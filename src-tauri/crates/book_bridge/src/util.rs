/// Tìm vị trí kết thúc của tag (có xử lý nesting).
pub fn find_closing_tag(s: &str, tag: &str) -> Option<usize> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let tag_end = s.find('>')?;          // skip qua tag mở đầu
    let mut depth = 1;                   // đã bên trong tag đầu
    let mut pos = tag_end + 1;
    let mut iter = 0;
    loop {
        iter += 1;
        if iter % 200000 == 0 {
            tracing::info!(tag, pos, depth, s_len = s.len(), "find_closing_tag: still scanning");
        }
        if pos >= s.len() {
            tracing::warn!(tag, depth, pos, iter, "find_closing_tag: reached end without match");
            return None;
        }
        if s[pos..].starts_with(&close) {
            depth -= 1;
            if depth == 0 {
                return Some(pos + close.len());
            }
            pos += close.len();
        } else if s[pos..].starts_with(&open)
            && (s[pos + open.len()..].starts_with(' ')
                || s[pos + open.len()..].starts_with('>')
                || s[pos + open.len()..].starts_with('\n')
                || s[pos + open.len()..].starts_with('/'))
        {
            depth += 1;
            pos += open.len();
        } else {
            let c = s[pos..].chars().next().unwrap_or(' ');
            pos += c.len_utf8();
        }
    }
}

/// Lấy nội dung giữa cặp <tag>...</tag>.
pub fn extract_tag_body<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open_start = xml.find(&format!("<{}", tag))?;
    let content_start = xml[open_start..].find('>')? + open_start + 1;
    let close = format!("</{}>", tag);
    let content_end = xml[content_start..].find(&close)?;
    Some(&xml[content_start..content_start + content_end])
}

/// Lấy giá trị của attribute (prefix="...").
pub fn extract_attr_value<'a>(s: &'a str, prefix: &str) -> Option<String> {
    let start = s.find(prefix)? + prefix.len();
    let end = s[start..].find('"')?;
    Some(s[start..start + end].to_string())
}

/// Content type từ extension.
pub fn content_type(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm") {
        "application/xhtml+xml"
    } else if lower.ends_with(".css") {
        "text/css"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".ttf") { "font/ttf" }
    else if lower.ends_with(".woff") { "font/woff" }
    else if lower.ends_with(".woff2") { "font/woff2" }
    else if lower.ends_with(".otf") { "font/otf" }
    else if lower.ends_with(".ncx") { "application/x-dtbncx+xml" }
    else { "application/octet-stream" }
}

/// URL percent-decode (chỉ giải mã %XX).
pub fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hi = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
            let lo = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
            out.push(char::from((hi * 16 + lo) as u8));
        } else {
            out.push(c);
        }
    }
    out
}

/// Lấy text của tag con đầu tiên.
pub fn extract_first_tag_text(s: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let start = s.find(&open)?;
    let after = s[start..].find('>')? + start + 1;
    let end = s[after..].find("</")?;
    let t = s[after..after + end].trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}
