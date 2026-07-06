#[cfg(not(feature = "zip"))]
use anyhow::anyhow;
#[cfg(feature = "zip")]
use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "zip")]
use std::fs::File;
#[cfg(feature = "zip")]
use std::io::{Cursor, Read, Write};
#[cfg(feature = "zip")]
use std::path::Path;
#[cfg(feature = "zip")]
use zip::{write::FileOptions, AesMode, CompressionMethod, ZipArchive, ZipWriter};

#[cfg(not(feature = "zip"))]
use crate::core::error::AppError;
#[cfg(not(feature = "zip"))]
use std::path::Path;

#[cfg(feature = "zip")]
pub fn zip_bytes(data: &[u8], entry_name: &str, compress_level: u32) -> Result<Vec<u8>> {
    let initial_cap = if compress_level == 0 {
        data.len() + 512
    } else {
        (data.len() * 3 / 4).max(512)
    };

    let cursor = Cursor::new(Vec::with_capacity(initial_cap));
    let mut zip = ZipWriter::new(cursor);

    let method = if compress_level == 0 {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Zstd
    };

    let opts: FileOptions<()> = FileOptions::default()
        .compression_method(method)
        .compression_level(if compress_level == 0 {
            None
        } else {
            Some(compress_level as i64)
        });

    zip.start_file(entry_name, opts)?;
    zip.write_all(data)?;
    let cursor = zip.finish()?;

    Ok(cursor.into_inner())
}

#[cfg(not(feature = "zip"))]
pub fn zip_bytes(_data: &[u8], _entry_name: &str, _compress_level: u32) -> Result<Vec<u8>> {
    Err(anyhow!(AppError::feature_disabled("zip")))
}

#[cfg(feature = "zip")]
pub fn unzip_or_raw(data: Vec<u8>) -> Result<Vec<u8>> {
    if data.len() < 4 || &data[..4] != b"PK\x03\x04" {
        return Ok(data);
    }

    let cursor = Cursor::new(&data);
    let mut archive = ZipArchive::new(cursor).context("Loi mo file nen ZIP")?;

    let mut entry = archive
        .by_index(0)
        .context("Loi doc noi dung ben trong ZIP")?;

    let capacity = entry.size() as usize;
    let mut out = Vec::with_capacity(capacity);
    entry
        .read_to_end(&mut out)
        .context("Loi giai ma du lieu ZIP")?;

    Ok(out)
}

#[cfg(not(feature = "zip"))]
pub fn unzip_or_raw(_data: Vec<u8>) -> Result<Vec<u8>> {
    Err(anyhow!(AppError::feature_disabled("zip")))
}

#[cfg(feature = "zip")]
pub fn zip_file_to_path(
    src_path: &Path,
    dst_path: &Path,
    entry_name: &str,
    compress_level: u32,
    aes_password: Option<&str>,
) -> Result<()> {
    let method = if compress_level == 0 {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Zstd
    };

    let mut opts: FileOptions<()> = FileOptions::default()
        .compression_method(method)
        .compression_level(if compress_level == 0 {
            None
        } else {
            Some(compress_level as i64)
        });

    if let Some(password) = aes_password {
        opts = opts.with_aes_encryption(AesMode::Aes256, password);
    }

    let dst_file = File::create(dst_path).context("Unable to create zip output file")?;
    let mut zip = ZipWriter::new(dst_file);

    zip.start_file(entry_name, opts)?;
    let mut src_file = File::open(src_path).context("Unable to open source file for zip")?;
    std::io::copy(&mut src_file, &mut zip).context("Failed to write zip data")?;
    zip.finish()?;

    Ok(())
}

#[cfg(not(feature = "zip"))]
pub fn zip_file_to_path(
    _src_path: &Path,
    _dst_path: &Path,
    _entry_name: &str,
    _compress_level: u32,
    _aes_password: Option<&str>,
) -> Result<()> {
    Err(anyhow!(AppError::feature_disabled("zip")))
}

pub struct EngineZipService;

impl crate::core::engine_context::ZipService for EngineZipService {
    fn unzip_or_raw(&self, data: Vec<u8>) -> Result<Vec<u8>, String> {
        unzip_or_raw(data).map_err(|e| e.to_string())
    }

    fn zip_file_to_path(
        &self,
        src: &std::path::Path,
        dst: &std::path::Path,
        entry_name: &str,
        compress_level: u32,
        aes_password: Option<&str>,
    ) -> Result<(), String> {
        zip_file_to_path(src, dst, entry_name, compress_level, aes_password)
            .map_err(|e| e.to_string())
    }
}
