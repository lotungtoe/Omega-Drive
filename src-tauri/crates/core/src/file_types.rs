pub use omega_drive_gateway::core::file_types::FileType;

pub fn normalize_extension(ext: &str) -> Option<String> {
    let trimmed = ext.trim().trim_start_matches('.').to_lowercase();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

pub fn file_type_from_extension(ext: &str) -> Option<FileType> {
    match ext {
        "pdf" | "doc" | "docx" | "txt" | "rtf" | "md" | "mdx" | "ppt" | "pptx" | "odt" | "epub"
        | "srt" | "vtt" | "ass" | "ssa" | "sub" => Some(FileType::Document),
        "xls" | "xlsx" | "csv" | "tsv" | "ods" => Some(FileType::Sheet),
        "json" | "jsonc" | "xml" | "html" | "htm" | "css" | "js" | "jsx" | "ts" | "tsx" | "py"
        | "java" | "c" | "cpp" | "h" | "hpp" | "rs" | "go" | "rb" | "php" | "swift" | "kt"
        | "dart" | "lua" | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "ini"
        | "cfg" | "toml" | "yaml" | "yml" | "vue" | "svelte" | "scss" | "sass" | "less" | "env"
        | "log" | "conf" | "properties" | "gradle" | "kts" | "stylus" | "styl" | "sql" => Some(FileType::Code),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "heic" | "avif" | "ico"
        | "tiff" | "tif" | "raw" | "dng" | "cr2" | "cr3" | "nef" | "arw" | "orf" | "sr2"
        | "srw" | "pef" | "rw2" | "raf" | "3fr" | "dcr" | "erf" | "fff" | "kdc" | "mos" | "mrw"
        | "nrw" | "x3f" => Some(FileType::Image),
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "m4v" | "flv" | "m2ts" | "mpeg" | "mpg"
        | "3gp" | "3g2" | "asf" | "wmv" | "vob" | "ogv" | "rm" | "rmvb" | "mxf" | "f4v" | "f4p"
        | "dv" | "nut" | "m3u8" => Some(FileType::Video),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => Some(FileType::Audio),
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => Some(FileType::Archive),
        _ => None,
    }
}

pub fn file_type_from_filename(filename: &str) -> FileType {
    use std::path::Path;
    Path::new(filename).extension().and_then(|v| v.to_str()).and_then(normalize_extension).and_then(|ext| file_type_from_extension(&ext)).unwrap_or(FileType::Unknown)
}

pub fn sniff_magic_bytes(buf: &[u8]) -> Option<FileType> {
    if buf.len() >= 4 {
        if buf.starts_with(b"%PDF-") { return Some(FileType::Document); }
        if buf.len() >= 8 && buf.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) { return Some(FileType::Image); }
        if buf.len() >= 3 && buf[0] == 0xFF && buf[1] == 0xD8 && buf[2] == 0xFF { return Some(FileType::Image); }
        if buf.starts_with(b"GIF87a") || buf.starts_with(b"GIF89a") { return Some(FileType::Image); }
        if buf.len() >= 12 && buf.starts_with(b"RIFF") && &buf[8..12] == b"WEBP" { return Some(FileType::Image); }
        if buf.starts_with(b"PK\x03\x04") || buf.starts_with(b"PK\x05\x06") || buf.starts_with(b"PK\x07\x08") { return Some(FileType::Archive); }
        if buf.starts_with(b"Rar!\x1A\x07\x00") || buf.starts_with(b"Rar!\x1A\x07\x01\x00") { return Some(FileType::Archive); }
        if buf.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) { return Some(FileType::Archive); }
        if buf.starts_with(b"fLaC") { return Some(FileType::Audio); }
        if buf.starts_with(b"ID3") || (buf.len() >= 2 && buf[0] == 0xFF && (buf[1] & 0xE0) == 0xE0) { return Some(FileType::Audio); }
        if buf.len() >= 12 && buf.starts_with(b"RIFF") {
            if &buf[8..12] == b"WAVE" { return Some(FileType::Audio); }
            if &buf[8..12] == b"AVI " { return Some(FileType::Video); }
        }
        if buf.len() >= 4 && buf.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) { return Some(FileType::Video); }
        if buf.len() >= 12 && &buf[4..8] == b"ftyp" { return Some(FileType::Video); }
    }
    None
}

pub fn normalize_storage_kind(kind: &str) -> &'static str {
    match kind.trim().to_ascii_lowercase().as_str() {
        "video" => "video", "image" => "image", "audio" => "audio",
        "document" => "document", "archive" => "archive", "code" => "code",
        "sheet" | "spreadsheet" => "sheet",
        "unknown" | "other" => "unknown", _ => "unknown",
    }
}

pub fn media_child_kind(kind: &str) -> &'static str {
    match normalize_storage_kind(kind) { "video" => "video", "audio" => "audio", "image" => "image", _ => "other" }
}

pub fn storage_kind_from_filename(filename: &str) -> &'static str { file_type_from_filename(filename).storage_kind() }

pub fn is_video_file(filename: &str) -> bool { matches!(filename.to_lowercase().as_str(), s if s.ends_with(".mp4") || s.ends_with(".mkv") || s.ends_with(".mov") || s.ends_with(".avi") || s.ends_with(".webm")) }
pub fn is_audio_file(filename: &str) -> bool { matches!(filename.to_lowercase().as_str(), s if s.ends_with(".mp3") || s.ends_with(".wav") || s.ends_with(".flac") || s.ends_with(".m4a") || s.ends_with(".aac") || s.ends_with(".ogg")) }
pub fn is_image_file(filename: &str) -> bool { matches!(filename.to_lowercase().as_str(), s if s.ends_with(".jpg") || s.ends_with(".jpeg") || s.ends_with(".png") || s.ends_with(".gif") || s.ends_with(".webp")) }
