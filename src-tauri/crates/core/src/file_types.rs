pub use omega_drive_gateway::core::file_types::FileType;

pub fn normalize_extension(ext: &str) -> Option<String> {
    let trimmed = ext.trim().trim_start_matches('.').to_lowercase();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

pub fn file_type_from_extension(ext: &str) -> Option<FileType> {
    match ext {
        "pdf" | "doc" | "docx" | "rtf" | "md" | "mdx" | "ppt" | "pptx" | "odt"
        | "srt" | "vtt" | "ass" | "ssa" | "sub" => Some(FileType::Document),
        "xls" | "xlsx" | "csv" | "tsv" | "ods" | "odp" => Some(FileType::Sheet),
        "json" | "jsonc" | "xml" | "html" | "htm" | "css" | "js" | "jsx" | "ts" | "tsx" | "py"
        | "java" | "c" | "cpp" | "h" | "hpp" | "rs" | "go" | "rb" | "php" | "swift" | "kt"
        | "dart" | "lua" | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "ini"
        | "cfg" | "toml" | "yaml" | "yml" | "vue" | "svelte" | "scss" | "sass" | "less" | "env"
        | "log" | "conf" | "properties" | "gradle" | "kts" | "stylus" | "styl" | "sql" => Some(FileType::Text),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "heic" | "heif" | "avif"
        | "ico" | "tiff" | "tif" | "raw" | "dng" | "cr2" | "cr3" | "nef" | "arw" | "orf"
        | "sr2" | "srw" | "pef" | "rw2" | "raf" | "3fr" | "dcr" | "erf" | "fff" | "kdc"
        | "mos" | "mrw" | "nrw" | "x3f" | "jxr" | "psd" | "ora" | "djvu" => Some(FileType::Image),
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "m4v" | "flv" | "m2ts" | "mpeg" | "mpg"
        | "3gp" | "3g2" | "asf" | "wmv" | "vob" | "ogv" | "rm" | "rmvb" | "mxf" | "f4v" | "f4p"
        | "dv" | "nut" | "m3u8" => Some(FileType::Video),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "mid" | "amr" | "aiff"
        | "dsf" | "ape" => Some(FileType::Audio),
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "Z" | "lz" | "lz4" | "zst"
        | "cab" | "dcm" | "msi" | "cpio" | "par2" | "swf" | "ps" | "sqlite" | "nes" | "crx"
        | "rpm" | "bz3" => Some(FileType::Archive),
        "epub" | "mobi" => Some(FileType::Book),
        "ttf" | "otf" | "woff" | "woff2" | "eot" => Some(FileType::Font),
        "wasm" | "exe" | "dll" | "elf" | "bc" | "mach" | "class" | "dex" | "dey" | "der"
        | "obj" => Some(FileType::App),
        "txt" => Some(FileType::Text),
        _ => None,
    }
}

pub fn file_type_from_filename(filename: &str) -> FileType {
    use std::path::Path;
    Path::new(filename).extension().and_then(|v| v.to_str()).and_then(normalize_extension).and_then(|ext| file_type_from_extension(&ext)).unwrap_or(FileType::Unknown)
}

pub fn sniff_magic_bytes(buf: &[u8]) -> Option<FileType> {
    let kind = infer::get(buf)?;
    match kind.matcher_type() {
        infer::MatcherType::Video => Some(FileType::Video),
        infer::MatcherType::Audio => Some(FileType::Audio),
        infer::MatcherType::Image => Some(FileType::Image),
        infer::MatcherType::Doc => Some(FileType::Document),
        infer::MatcherType::Book => Some(FileType::Book),
        infer::MatcherType::Font => Some(FileType::Font),
        infer::MatcherType::App => file_type_from_extension(kind.extension()),
        infer::MatcherType::Text => Some(FileType::Text),
        _ => file_type_from_extension(kind.extension()).or(Some(FileType::Archive)),
    }
}

pub fn normalize_storage_kind(kind: &str) -> &'static str {
    match kind.trim().to_ascii_lowercase().as_str() {
        "video" => "video", "image" => "image", "audio" => "audio",
        "document" => "document", "archive" => "archive",
        "sheet" | "spreadsheet" => "sheet",
        "book" => "book", "font" => "font", "app" => "app",
        "text" => "text",
        "unknown" | "other" => "unknown", _ => "unknown",
    }
}

pub fn media_child_kind(kind: &str) -> &'static str {
    match normalize_storage_kind(kind) { "video" => "video", "audio" => "audio", "image" => "image", _ => "other" }
}

pub fn storage_kind_from_filename(filename: &str) -> &'static str { file_type_from_filename(filename).storage_kind() }

pub fn is_video_file(filename: &str) -> bool { file_type_from_filename(filename).storage_kind() == "video" }
pub fn is_audio_file(filename: &str) -> bool { file_type_from_filename(filename).storage_kind() == "audio" }
pub fn is_image_file(filename: &str) -> bool { file_type_from_filename(filename).storage_kind() == "image" }
