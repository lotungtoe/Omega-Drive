use flate2::read::GzDecoder;
use std::path::Path;

pub fn extract_archive(archive: &Path, dest: &Path) -> Result<(), String> {
    let ext = archive
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "zip" => extract_zip(archive, dest),
        "gz" | "tgz" => extract_tar_gz(archive, dest),
        _ => Err(format!("Unsupported archive format: .{ext}")),
    }
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let file =
        std::fs::File::open(archive).map_err(|e| format!("Cannot open archive {}: {e}", archive.display()))?;
    let mut archive_zip =
        zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip {}: {e}", archive.display()))?;
    archive_zip
        .extract(dest)
        .map_err(|e| format!("Extract failed: {e}"))?;
    Ok(())
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), String> {
    let file =
        std::fs::File::open(archive).map_err(|e| format!("Cannot open archive {}: {e}", archive.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive_tar = tar::Archive::new(decoder);
    archive_tar
        .unpack(dest)
        .map_err(|e| format!("Extract failed: {e}"))?;
    Ok(())
}
