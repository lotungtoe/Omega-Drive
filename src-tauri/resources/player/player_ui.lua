local opt = require("mp.options")

local opts = {
    hide_timeout = 2.0,
    fade_duration = 0.2,
    accent = "#FF0000",
    knob_color = "#FFFFFF",
    buffer_color = "#5f5f5f",
    progress_bg = "#3d3d3d",
    progress_hover_bg = "#5f5f5f",
    volume_bg = "#8f8f8f",
    text = "#f4f4f5",
    text_font = "Segoe UI",
    icon_font = "Segoe UI Symbol",
    side_padding = 20,
    button_width = 48,
    button_height = 48,
    control_gap_min = 16,
    control_gap_max = 24,
    volume_gap = 14,
    time_gap = 20,
    progress_bottom = 16,
    progress_hit_height = 40,
    progress_height = 4,
    progress_hover_extra = 4,
    progress_knob_radius = 8,
    progress_hover_knob_radius = 11,
    volume_width_min = 80,
    volume_width_max = 150,
    volume_hit_height = 32,
    volume_height = 4,
    volume_knob_radius = 7,
    vignette_height = 96,
    vignette_steps = 16,
    top_vignette_height = 0,
    top_vignette_steps = 10,
    title_font_size = 24,
    title_top_margin = 20,
    title_side_margin = 24,
}

opt.read_options(opts, "youtube_osc")

local overlay = mp.create_osd_overlay and mp.create_osd_overlay("ass-events") or nil
if overlay then
    overlay.z = 1000
end

local state = {
    paused = false,
    position = 0,
    duration = 0,
    volume = 100,
    muted = false,
    fullscreen = false,
    hover_target = nil,
    hover_window = false,
    last_mouse_x = nil,
    last_mouse_y = nil,
    visible_until = 0,
    fade = 1,
    last_tick = mp.get_time(),
    title = "",
    cache_end = 0,
    menu = { page = nil, items = {} },
}

local layout = {
    progress = nil,
    volume = nil,
    buttons = {},
    time = nil,
    title_bar = nil,
    menu = nil,
}

local function clamp(value, min_value, max_value)
    if value < min_value then
        return min_value
    end
    if value > max_value then
        return max_value
    end
    return value
end

local function ass_color(hex)
    local clean = (hex or "#ffffff"):gsub("#", "")
    if #clean ~= 6 then
        clean = "ffffff"
    end
    return clean:sub(5, 6) .. clean:sub(3, 4) .. clean:sub(1, 2)
end

local function mix_alpha(base_alpha, fade)
    return math.floor(base_alpha + (255 - base_alpha) * (1 - clamp(fade, 0, 1)))
end

local function escape_ass(text)
    return (text or "")
        :gsub("\\", "\\\\")
        :gsub("{", "\\{")
        :gsub("}", "\\}")
        :gsub("\n", "\\N")
end

local function rect_path(x, y, width, height)
    return string.format(
        "m %.1f %.1f l %.1f %.1f l %.1f %.1f l %.1f %.1f",
        x,
        y,
        x + width,
        y,
        x + width,
        y + height,
        x,
        y + height
    )
end

local function circle_path(cx, cy, radius)
    local points = {}
    for i = 0, 15 do
        local angle = (math.pi * 2 * i) / 16
        local x = cx + math.cos(angle) * radius
        local y = cy + math.sin(angle) * radius
        points[#points + 1] = string.format("%.1f %.1f", x, y)
    end
    return "m " .. points[1] .. " l " .. table.concat(points, " l ")
end

local function append_shape(lines, x, y, width, height, color, alpha, blur)
    if width <= 0 or height <= 0 then
        return
    end
    lines[#lines + 1] = string.format(
        "{\\an7\\pos(%.1f,%.1f)\\blur%.1f\\bord0\\shad0\\1c&H%s&\\1a&H%02X&\\p1}%s",
        x,
        y,
        blur or 0,
        ass_color(color),
        clamp(alpha, 0, 255),
        rect_path(0, 0, width, height)
    )
end

local function append_circle(lines, cx, cy, radius, color, alpha, blur)
    if radius <= 0 then
        return
    end
    lines[#lines + 1] = string.format(
        "{\\an7\\pos(0,0)\\blur%.1f\\bord0\\shad0\\1c&H%s&\\1a&H%02X&\\p1}%s",
        blur or 0,
        ass_color(color),
        clamp(alpha, 0, 255),
        circle_path(cx, cy, radius)
    )
end

local function append_text(lines, x, y, align, size, color, alpha, text, bold, font)
    lines[#lines + 1] = string.format(
        "{\\an%d\\pos(%.1f,%.1f)\\fn%s\\fs%d\\bord0\\shad0\\1c&H%s&\\1a&H%02X&%s}%s",
        align,
        x,
        y,
        escape_ass(font or opts.text_font),
        size,
        ass_color(color),
        clamp(alpha, 0, 255),
        bold and "\\b1" or "",
        escape_ass(text)
    )
end

local function format_time(seconds)
    seconds = math.max(0, tonumber(seconds) or 0)
    local total = math.floor(seconds + 0.5)
    local hours = math.floor(total / 3600)
    local minutes = math.floor((total % 3600) / 60)
    local secs = total % 60

    if hours > 0 then
        return string.format("%d:%02d:%02d", hours, minutes, secs)
    end

    return string.format("%d:%02d", minutes, secs)
end

local function button_spec(id, label, width, icon)
    return {
        id = id,
        label = label,
        width = width,
        icon = icon or false,
    }
end

local function button_icon(button_id)
    if button_id == "play_pause" then
        return state.paused and "▶" or "⏸"
    end
    if button_id == "next" then
        return "⏭"
    end
    if button_id == "volume" then
        return (state.muted or state.volume <= 0.1) and "🔇" or "🔊"
    end
    if button_id == "subtitle" then
        return "CC"
    end
    if button_id == "settings" then
        return "⚙"
    end
    if button_id == "fullscreen" then
        return "⛶"
    end
    return ""
end

local function should_restart_from_eof()
    return state.duration > 0 and (
        mp.get_property_bool("eof-reached", false)
        or state.position >= math.max(state.duration - 0.35, 0)
    )
end

local function build_layout(osd_w, osd_h)
    local side_padding = opts.side_padding
    local button_w = opts.button_width
    local gap = clamp(osd_w * 0.012, opts.control_gap_min, opts.control_gap_max)
    local center_y = osd_h - 56
    local button_y = center_y - (opts.button_height / 2)
    local text_y = center_y + 1
    local time_text = string.format("%s / %s", format_time(state.position), format_time(state.duration))
    local time_width = clamp(26 + (#time_text * 8), 70, 132)

    local left_defs = {
        button_spec("play_pause", button_icon("play_pause"), button_w, true),
        button_spec("next", button_icon("next"), button_w, true),
        button_spec("volume", button_icon("volume"), button_w, true),
    }

    local right_defs = {
        button_spec("subtitle", button_icon("subtitle"), 30, false),
        button_spec("settings", button_icon("settings"), button_w, true),
        button_spec("fullscreen", button_icon("fullscreen"), button_w, true),
    }

    local right_width = 0
    for index, definition in ipairs(right_defs) do
        right_width = right_width + definition.width
        if index < #right_defs then
            right_width = right_width + gap
        end
    end

    local left_fixed = (button_w * 3) + (gap * 2) + opts.volume_gap + opts.time_gap
    local available = osd_w - (side_padding * 2) - right_width - left_fixed
    local volume_width = clamp(available - time_width, opts.volume_width_min, opts.volume_width_max)
    local max_time_width = math.max(56, available - volume_width)
    time_width = clamp(time_width, 56, math.max(56, max_time_width))

    layout.buttons = {}

    local cursor_x = side_padding
    for _, definition in ipairs(left_defs) do
        layout.buttons[#layout.buttons + 1] = {
            id = definition.id,
            label = definition.label,
            x = cursor_x,
            y = button_y,
            w = definition.width,
            h = opts.button_height,
            text_x = cursor_x + (definition.width / 2),
            text_y = text_y,
            icon = definition.icon,
        }
        cursor_x = cursor_x + definition.width + gap
    end

    layout.volume = {
        x = cursor_x - gap + opts.volume_gap,
        y = center_y - (opts.volume_hit_height / 2),
        w = volume_width,
        h = opts.volume_hit_height,
        rail_y = center_y - (opts.volume_height / 2),
        rail_h = opts.volume_height,
    }

    layout.time = {
        x = layout.volume.x + layout.volume.w + opts.time_gap,
        y = center_y + 1,
        text = time_text,
        w = time_width,
    }

    cursor_x = osd_w - side_padding - right_width
    for _, definition in ipairs(right_defs) do
        layout.buttons[#layout.buttons + 1] = {
            id = definition.id,
            label = definition.label,
            x = cursor_x,
            y = button_y,
            w = definition.width,
            h = opts.button_height,
            text_x = cursor_x + (definition.width / 2),
            text_y = text_y,
            icon = definition.icon,
        }
        cursor_x = cursor_x + definition.width + gap
    end

    layout.progress = {
        x = 0,
        y = osd_h - opts.progress_bottom - opts.progress_hit_height,
        w = osd_w,
        h = opts.progress_hit_height,
        rail_y = osd_h - opts.progress_bottom - opts.progress_height,
    }

    layout.title_bar = {
        x = opts.title_side_margin,
        y = opts.title_top_margin,
        max_w = osd_w - (opts.title_side_margin * 2),
    }

    layout.menu = nil
    if state.menu.page and #state.menu.items > 0 then
        local item_h = 44
        local padding = 16
        local max_chars = 14
        for _, item in ipairs(state.menu.items) do 
            local len = #item.label + (item.value and #item.value or 0)
            max_chars = math.max(max_chars, len) 
        end
        local w = clamp(max_chars * 12 + 64, 320, 480)
        local h = (#state.menu.items * item_h) + (padding * 2)

        local settings_btn = nil
        if layout.buttons then
            for _, b in ipairs(layout.buttons) do
                if b.id == "settings" then settings_btn = b; break end
            end
        end

        local x = osd_w - w - opts.side_padding
        if settings_btn then x = settings_btn.text_x - (w / 2) end
        if x + w > osd_w - opts.side_padding then x = osd_w - w - opts.side_padding end

        local y = center_y - (opts.button_height / 2) - h - 12
        layout.menu = { x = x, y = y, w = w, h = h, item_h = item_h, padding = padding }
    end
end

local function close_menu()
    state.menu.page = nil
    state.menu.items = {}
end

local function init_menu(page)
    state.menu.page = page
    state.menu.items = {}
    
    if page == "main" then
        local spd = mp.get_property_number("speed", 1)
        local spd_text = (spd == 1) and "Chuẩn" or string.format("%gx", spd)
        
        local sid = mp.get_property("sid", "no")
        local sub_text = (sid == "no" or sid == "auto") and "Tắt" or "Bật"

        local sub_scale = tonumber(mp.get_property("sub-scale", "1.0") or "1.0")
        local scale_text = sub_scale == 1.0 and "100%" or string.format("%.0f%%", sub_scale * 100)

        state.menu.items = {
            { label = "Tốc độ phát", value = spd_text, action = "menu:speed" },
            { label = "P.Giải (Video Track)", action = "menu:video_track" },
            { label = "L.Tiếng (Audio Track)", action = "menu:audio_track" },
            { label = "Phụ đề", value = sub_text, action = "menu:sub_track" },
            { label = "Cỡ chữ", value = scale_text, action = "menu:sub_size" },
        }
    elseif page == "speed" then
        local current = mp.get_property_number("speed", 1)
        state.menu.items = {
            { label = "0.25x", action = "prop:speed:0.25", selected = (current == 0.25) },
            { label = "0.5x", action = "prop:speed:0.5", selected = (current == 0.5) },
            { label = "0.75x", action = "prop:speed:0.75", selected = (current == 0.75) },
            { label = "Chuẩn (1x)", action = "prop:speed:1.0", selected = (current == 1) },
            { label = "1.25x", action = "prop:speed:1.25", selected = (current == 1.25) },
            { label = "1.5x", action = "prop:speed:1.5", selected = (current == 1.5) },
            { label = "2x", action = "prop:speed:2.0", selected = (current == 2.0) },
        }
    elseif page == "sub_size" then
        local current = tonumber(mp.get_property("sub-scale") or "1.0")
        state.menu.items = {
            { label = "Rất nhỏ (50%)", action = "prop:sub-scale:0.5", selected = (current == 0.5) },
            { label = "Nhỏ (75%)", action = "prop:sub-scale:0.75", selected = (current == 0.75) },
            { label = "Vừa (100%)", action = "prop:sub-scale:1.0", selected = (current == 1.0) },
            { label = "Lớn (125%)", action = "prop:sub-scale:1.25", selected = (current == 1.25) },
            { label = "Rất lớn (150%)", action = "prop:sub-scale:1.5", selected = (current == 1.5) },
        }
    elseif page == "sub_track" or page == "audio_track" or page == "video_track" then
        local track_list = mp.get_property_native("track-list") or {}
        local kind = page == "sub_track" and "sub" or (page == "audio_track" and "audio" or "video")
        
        if kind == "sub" then
            state.menu.items[#state.menu.items + 1] = {
                label = "Tắt (Off)",
                action = "prop:sid:no",
                selected = (mp.get_property("sid") == "no")
            }
        end
        
        for _, track in ipairs(track_list) do
            if track.type == kind then
                local title = track.title or track.lang or string.format("Track %d", track.id)
                if track.codec then title = title .. " [" .. track.codec .. "]" end
                state.menu.items[#state.menu.items + 1] = {
                    label = title,
                    action = "prop:" .. (kind == "sub" and "sid" or (kind == "audio" and "aid" or "vid")) .. ":" .. tostring(track.id),
                    selected = track.selected
                }
            end
        end

        -- Thêm nút để nạp file ngoài ở dưới cùng
        if kind == "sub" then
            state.menu.items[#state.menu.items + 1] = {
                label = "➕",
                action = "cmd:sub_add_picker"
            }
        elseif kind == "audio" then
            state.menu.items[#state.menu.items + 1] = {
                label = "➕",
                action = "cmd:audio_add_picker"
            }
        end
    end
end

local function hit_test(x, y)
    if layout.menu then
        if x >= layout.menu.x and x <= (layout.menu.x + layout.menu.w)
            and y >= layout.menu.y and y <= (layout.menu.y + layout.menu.h)
        then
            local relative_y = y - layout.menu.y - layout.menu.padding
            local index = math.floor(relative_y / layout.menu.item_h) + 1
            if index >= 1 and index <= #state.menu.items then
                return { kind = "menu_item", index = index }
            else
                return { kind = "menu_bg" }
            end
        end
    end

    for _, button in ipairs(layout.buttons) do
        if x >= button.x and x <= (button.x + button.w)
            and y >= button.y and y <= (button.y + button.h)
        then
            return { kind = "button", id = button.id }
        end
    end

    if layout.volume
        and x >= layout.volume.x and x <= (layout.volume.x + layout.volume.w)
        and y >= layout.volume.y and y <= (layout.volume.y + layout.volume.h)
    then
        return {
            kind = "volume_slider",
            ratio = clamp((x - layout.volume.x) / math.max(layout.volume.w, 1), 0, 1),
        }
    end

    if layout.progress
        and x >= layout.progress.x and x <= (layout.progress.x + layout.progress.w)
        and y >= layout.progress.y and y <= (layout.progress.y + layout.progress.h)
    then
        return {
            kind = "progress",
            ratio = clamp((x - layout.progress.x) / math.max(layout.progress.w, 1), 0, 1),
        }
    end

    return nil
end

local function current_mouse_target()
    local mouse = mp.get_property_native("mouse-pos")
    if not mouse or not mouse.hover then
        return nil
    end
    return hit_test(mouse.x or 0, mouse.y or 0)
end

local function set_visible_timeout()
    state.visible_until = mp.get_time() + opts.hide_timeout
end

local function seek_to_ratio(ratio, is_drag)
    if state.duration <= 0 then
        return
    end
    local target = clamp(state.duration * ratio, 0, state.duration)
    if is_drag then
        -- (Không làm gì cả) Bỏ lệnh seek liên tục để tránh thắt cổ chai tiến trình phát video làm lag chuột
    else
        mp.set_property("time-pos", string.format("%.3f", target))
    end
    set_visible_timeout()
end

local function seek_relative(delta)
    if state.duration <= 0 then
        return
    end
    local target = clamp((state.position or 0) + delta, 0, state.duration)
    mp.set_property("time-pos", string.format("%.3f", target))
    set_visible_timeout()
end

local function set_volume_ratio(ratio)
    local target = clamp(ratio, 0, 1) * 100
    mp.set_property("volume", string.format("%.1f", target))
    if target > 0 then
        mp.set_property("mute", "no")
    end
    set_visible_timeout()
end

local function update_state()
    state.paused = mp.get_property_bool("pause", false)
    state.position = mp.get_property_number("time-pos", 0) or 0
    state.duration = mp.get_property_number("duration", 0) or 0
    state.volume = mp.get_property_number("volume", 100) or 100
    state.muted = mp.get_property_bool("mute", false)
    state.fullscreen = mp.get_property_bool("fullscreen", false)
    state.title = mp.get_property("force-media-title", "") or mp.get_property("media-title", "") or ""
    local cache_dur = mp.get_property_number("demuxer-cache-duration", 0) or 0
    state.cache_end = state.position + cache_dur
end

local function push_overlay(osd_w, osd_h, ass)
    if overlay then
        overlay.res_x = osd_w
        overlay.res_y = osd_h
        overlay.data = ass
        overlay:update()
    elseif mp.set_osd_ass then
        mp.set_osd_ass(osd_w, osd_h, ass)
    end
end

local function clear_overlay()
    if overlay then
        overlay.data = ""
        overlay:update()
    elseif mp.set_osd_ass then
        mp.set_osd_ass(0, 0, "")
    end
end

local function trigger_button(button_id)
    if button_id == "play_pause" then
        if state.paused and should_restart_from_eof() then
            mp.commandv("seek", "0", "absolute", "exact")
            mp.set_property("pause", "no")
        else
            mp.commandv("cycle", "pause")
        end
    elseif button_id == "next" then
        mp.commandv("playlist-next")
    elseif button_id == "volume" then
        mp.commandv("cycle", "mute")
    elseif button_id == "subtitle" then
        mp.commandv("cycle", "sub-visibility")
    elseif button_id == "settings" then
        if state.menu.page then
            close_menu()
        else
            init_menu("main")
        end
    elseif button_id == "fullscreen" then
        mp.commandv("cycle", "fullscreen")
    end
    set_visible_timeout()
end

local function render_vignette(lines, osd_w, osd_h)
    -- Bottom frame (vừa đủ ôm controller)
    append_shape(
        lines,
        0,
        osd_h - opts.vignette_height,
        osd_w,
        opts.vignette_height,
        "#000000",
        mix_alpha(180, state.fade),
        0
    )
end

local function render()
    local osd_w = mp.get_property_number("osd-width", 0) or 0
    local osd_h = mp.get_property_number("osd-height", 0) or 0
    if osd_w <= 0 or osd_h <= 0 then
        clear_overlay()
        return
    end

    build_layout(osd_w, osd_h)

    if state.fade <= 0.01 then
        clear_overlay()
        return
    end

    local hovered_button_id = nil
    local progress_hovered = state.hover_target and state.hover_target.kind == "progress"
    local volume_hovered = state.hover_target and state.hover_target.kind == "volume_slider"
    if state.hover_target and state.hover_target.kind == "button" then
        hovered_button_id = state.hover_target.id
    end

    local lines = {}
    local idle_text_alpha = mix_alpha(72, state.fade)
    local hover_text_alpha = mix_alpha(0, state.fade)
    local time_alpha = mix_alpha(12, state.fade)
    local hover_fill_alpha = mix_alpha(232, state.fade)
    local volume_bg_alpha = mix_alpha(volume_hovered and 118 or 166, state.fade)
    local volume_fill_alpha = mix_alpha(14, state.fade)
    local progress_bg_alpha = mix_alpha(progress_hovered and 126 or 168, state.fade)

    local progress_ratio = 0
    if state.dragging_progress and state.last_seek_ratio then
        progress_ratio = state.last_seek_ratio
    elseif state.duration > 0 then
        progress_ratio = clamp(state.position / state.duration, 0, 1)
    end

    local progress_h = opts.progress_height + (progress_hovered and opts.progress_hover_extra or 0)
    local progress_y = layout.progress.rail_y - ((progress_h - opts.progress_height) / 2)
    local progress_fill = layout.progress.w * progress_ratio
    local knob_radius = progress_hovered and opts.progress_hover_knob_radius or opts.progress_knob_radius
    local knob_x = clamp(layout.progress.x + progress_fill, knob_radius, osd_w - knob_radius)

    local volume_ratio = clamp((state.muted and 0 or state.volume) / 100, 0, 1)
    if state.dragging_volume and state.last_volume_ratio then
        volume_ratio = state.last_volume_ratio
    end
    local volume_fill = layout.volume.w * volume_ratio
    local volume_knob_x = clamp(
        layout.volume.x + volume_fill,
        layout.volume.x + opts.volume_knob_radius,
        layout.volume.x + layout.volume.w - opts.volume_knob_radius
    )

    render_vignette(lines, osd_w, osd_h)

    -- Title bar at top
    if layout.title_bar and state.title ~= "" then
        append_text(
            lines,
            layout.title_bar.x,
            layout.title_bar.y,
            7,
            opts.title_font_size,
            opts.text,
            mix_alpha(16, state.fade),
            state.title,
            true,
            opts.text_font
        )
    end

    for _, button in ipairs(layout.buttons) do
        local hovered = hovered_button_id == button.id
        if hovered then
            -- Circular hover highlight (YouTube style)
            local hover_cx = button.x + (button.w / 2)
            local hover_cy = button.y + (button.h / 2)
            local hover_radius = math.max(button.w, button.h) / 2 + 4
            append_circle(
                lines,
                hover_cx,
                hover_cy,
                hover_radius,
                "#ffffff",
                hover_fill_alpha,
                1.0
            )
        end

        append_text(
            lines,
            button.text_x,
            button.text_y,
            5,
            button.icon and 36 or 24,
            opts.text,
            hovered and hover_text_alpha or idle_text_alpha,
            button.label,
            true,
            button.icon and opts.icon_font or opts.text_font
        )
    end

    append_shape(
        lines,
        layout.volume.x,
        layout.volume.rail_y,
        layout.volume.w,
        layout.volume.rail_h,
        opts.volume_bg,
        volume_bg_alpha,
        0
    )
    if volume_fill > 0 then
        append_shape(
            lines,
            layout.volume.x,
            layout.volume.rail_y,
            volume_fill,
            layout.volume.rail_h,
            "#ffffff",
            volume_fill_alpha,
            0
        )
    end
    append_circle(
        lines,
        volume_knob_x,
        layout.volume.rail_y + (layout.volume.rail_h / 2),
        opts.volume_knob_radius,
        "#ffffff",
        mix_alpha(0, state.fade),
        0
    )

    append_text(
        lines,
        layout.time.x,
        layout.time.y,
        4,
        22,
        opts.text,
        time_alpha,
        layout.time.text,
        true,
        opts.text_font
    )

    if layout.menu then
        local bg_alpha = mix_alpha(25, state.fade) -- 90% opacity (#1A in Hex)
        local r = 12
        local bg_path = string.format("m %.1f %.1f l %.1f %.1f l %.1f %.1f l %.1f %.1f",
            layout.menu.x + r, layout.menu.y + r,
            layout.menu.x + layout.menu.w - r, layout.menu.y + r,
            layout.menu.x + layout.menu.w - r, layout.menu.y + layout.menu.h - r,
            layout.menu.x + r, layout.menu.y + layout.menu.h - r)
        
        lines[#lines + 1] = string.format(
            "{\\an7\\pos(0,0)\\bord%d\\3c&H%s&\\1c&H%s&\\1a&H%02X&\\3a&H%02X&\\p1}%s",
            r, ass_color("#1f1f1f"), ass_color("#1f1f1f"), bg_alpha, bg_alpha, bg_path
        )

        for i, item in ipairs(state.menu.items) do
            local item_y = layout.menu.y + layout.menu.padding + ((i - 1) * layout.menu.item_h)
            local is_hovered = state.hover_target and state.hover_target.kind == "menu_item" and state.hover_target.index == i
            
            if is_hovered then
                local hr = 8
                local hpath = string.format("m %.1f %.1f l %.1f %.1f l %.1f %.1f l %.1f %.1f",
                    layout.menu.x + 8 + hr, item_y + hr,
                    layout.menu.x + layout.menu.w - 8 - hr, item_y + hr,
                    layout.menu.x + layout.menu.w - 8 - hr, item_y + layout.menu.item_h - hr,
                    layout.menu.x + 8 + hr, item_y + layout.menu.item_h - hr)
                lines[#lines + 1] = string.format(
                    "{\\an7\\pos(0,0)\\bord%d\\3c&H%s&\\1c&H%s&\\1a&H%02X&\\3a&H%02X&\\p1}%s",
                    hr, ass_color("#3e3e3e"), ass_color("#3e3e3e"), mix_alpha(0, state.fade), mix_alpha(0, state.fade), hpath
                )
            end

            -- Main Label
            append_text(
                lines,
                layout.menu.x + 24,
                item_y + (layout.menu.item_h / 2),
                4,
                22,
                "#ffffff",
                mix_alpha(0, state.fade),
                item.label,
                false,
                opts.text_font
            )
            
            -- Right Value / Navigate Arrow
            local right_text = ""
            if item.action and item.action:match("^menu:") then
                right_text = (item.value and (item.value .. "   > ") or " > ")
            else
                if item.selected then right_text = "✓" end
            end
            
            if right_text ~= "" then
                append_text(
                    lines,
                    layout.menu.x + layout.menu.w - 24,
                    item_y + (layout.menu.item_h / 2),
                    6,
                    18,
                    item.selected and "#ffffff" or "#c6c6c6",
                    mix_alpha(0, state.fade),
                    right_text,
                    false,
                    opts.text_font
                )
            end
        end
    end

    -- Progress bar track background
    append_shape(
        lines,
        layout.progress.x,
        progress_y,
        layout.progress.w,
        progress_h,
        progress_hovered and opts.progress_hover_bg or opts.progress_bg,
        progress_bg_alpha,
        0
    )
    -- Buffer indicator
    if state.duration > 0 and state.cache_end > state.position then
        local buffer_ratio = clamp(state.cache_end / state.duration, 0, 1)
        local buffer_fill = layout.progress.w * buffer_ratio
        if buffer_fill > progress_fill then
            append_shape(
                lines,
                layout.progress.x,
                progress_y,
                buffer_fill,
                progress_h,
                opts.buffer_color,
                mix_alpha(progress_hovered and 100 or 140, state.fade),
                0
            )
        end
    end
    -- Progress fill (red)
    if progress_fill > 0 then
        append_shape(
            lines,
            layout.progress.x,
            progress_y,
            progress_fill,
            progress_h,
            opts.accent,
            mix_alpha(0, state.fade),
            0
        )
    end
    -- Progress knob (white, YouTube style)
    append_circle(
        lines,
        knob_x,
        progress_y + (progress_h / 2),
        knob_radius,
        opts.knob_color,
        mix_alpha(0, state.fade),
        0
    )

    push_overlay(osd_w, osd_h, table.concat(lines, "\n"))
end

local function tick()
    update_state()

    local now = mp.get_time()
    local dt = now - state.last_tick
    state.last_tick = now

    local mouse = mp.get_property_native("mouse-pos")
    state.hover_window = mouse and mouse.hover or false
    if state.hover_window then
        local mouse_x = mouse.x or 0
        local mouse_y = mouse.y or 0
        local moved = state.last_mouse_x == nil
            or math.abs(mouse_x - state.last_mouse_x) >= 1
            or math.abs(mouse_y - state.last_mouse_y) >= 1

        if moved then
            state.last_mouse_x = mouse_x
            state.last_mouse_y = mouse_y
            set_visible_timeout()
            
            if state.dragging_progress and layout.progress then
                local ratio = clamp((mouse_x - layout.progress.x) / math.max(layout.progress.w, 1), 0, 1)
                if state.last_seek_ratio ~= ratio then
                    state.last_seek_ratio = ratio
                    seek_to_ratio(ratio, true)
                end
            elseif state.dragging_volume and layout.volume then
                local ratio = clamp((mouse_x - layout.volume.x) / math.max(layout.volume.w, 1), 0, 1)
                if state.last_volume_ratio ~= ratio then
                    state.last_volume_ratio = ratio
                    set_volume_ratio(ratio)
                end
            end
        end
    else
        state.last_mouse_x = nil
        state.last_mouse_y = nil
        state.dragging_progress = false
        state.dragging_volume = false
    end

    build_layout(
        mp.get_property_number("osd-width", 0) or 0,
        mp.get_property_number("osd-height", 0) or 0
    )

    state.hover_target = nil
    if state.hover_window and state.last_mouse_x and state.last_mouse_y then
        state.hover_target = hit_test(state.last_mouse_x, state.last_mouse_y)
        if state.hover_target then
            set_visible_timeout()
        end
    end

    local target_visible = state.paused or state.menu.page or (state.hover_window and now <= state.visible_until)
    local fade_step = opts.fade_duration > 0 and (dt / opts.fade_duration) or 1
    if target_visible then
        state.fade = clamp(state.fade + fade_step, 0, 1)
    else
        state.fade = clamp(state.fade - fade_step, 0, 1)
    end

    render()
end

local function on_left_click()
    local target = current_mouse_target()
    
    if state.menu.page then
        if not target then
            close_menu()
            return
        end
        if target.kind == "menu_item" then
            local item = state.menu.items[target.index]
            if item.action:sub(1, 5) == "menu:" then
                init_menu(item.action:sub(6))
            elseif item.action:sub(1, 5) == "prop:" then
                local parts = {}
                for part in string.gmatch(item.action:sub(6), "[^:]+") do table.insert(parts, part) end
                mp.set_property(parts[1], parts[2])
                close_menu()
            elseif item.action:sub(1, 4) == "cmd:" then
                local action = item.action:sub(5)
                if action == "sub_add_picker" then
                    mp.set_property("user-data/picker", "subtitle:" .. mp.get_time())
                elseif action == "audio_add_picker" then
                    mp.set_property("user-data/picker", "audio:" .. mp.get_time())
                end
                close_menu()
            end
            return
        elseif target.kind == "menu_bg" then
            return
        elseif target.kind == "button" and target.id == "settings" then
            -- Fallthrough to normal trigger_button logic which naturally toggles it off
        else
            close_menu()
        end
    end

    if not target then
        mp.commandv("cycle", "pause")
        return
    end

    if target.kind == "button" then
        trigger_button(target.id)
    end
end

local function handle_mouse_btn(e)
    if e.event == "down" then
        local target = current_mouse_target()
        if target and target.kind == "progress" then
            state.dragging_progress = true
            state.last_seek_ratio = target.ratio
            seek_to_ratio(target.ratio, true)
        elseif target and target.kind == "volume_slider" then
            state.dragging_volume = true
            state.last_volume_ratio = target.ratio
            set_volume_ratio(target.ratio)
        else
            on_left_click()
        end
    elseif e.event == "up" then
        if state.dragging_progress and state.last_seek_ratio then
            -- Thực hiện 1 cú tua frame chính xác khi nhả chuột
            seek_to_ratio(state.last_seek_ratio, false)
        end
        state.dragging_progress = false
        state.dragging_volume = false
    end
end

local function on_left_double_click()
    local target = current_mouse_target()
    if not target then
        mp.commandv("cycle", "fullscreen")
    end
end

local function on_right_click()
    local target = current_mouse_target()
    if not target then
        -- Bạn có thể ẩn menu ngữ cảnh hoặc tùy chỉnh thêm ở đây
    end
end

local function on_wheel(delta)
    local target = current_mouse_target()
    if target and target.kind == "progress" then
        seek_relative(delta > 0 and 5 or -5)
        return
    end

    local current_volume = clamp(state.volume + (delta > 0 and 5 or -5), 0, 100)
    mp.set_property("volume", string.format("%.1f", current_volume))
    if current_volume > 0 then
        mp.set_property("mute", "no")
    end
    set_visible_timeout()
end

mp.add_forced_key_binding("MBTN_LEFT", "youtube-osc-left-click", handle_mouse_btn, {complex = true})
mp.add_forced_key_binding("MBTN_LEFT_DBL", "youtube-osc-left-double", on_left_double_click)
mp.add_forced_key_binding("MBTN_RIGHT", "youtube-osc-right-click", on_right_click)
mp.add_forced_key_binding("WHEEL_UP", "youtube-osc-wheel-up", function() on_wheel(1) end)
mp.add_forced_key_binding("WHEEL_DOWN", "youtube-osc-wheel-down", function() on_wheel(-1) end)

local timer = mp.add_periodic_timer(1 / 60, tick)
timer:resume()

set_visible_timeout()
update_state()
render()
