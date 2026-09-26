use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::error::{UploadError, UploadResult};

pub async fn extract_and_save_idx(
    file_path: &Path,
    file_size: u64,
    file_id: i64,
    idx_cache_dir: &Path,
) -> UploadResult<Option<()>> {
    let mut file = tokio::fs::File::open(file_path).await
        .map_err(|e| UploadError::io("Cannot open file", e))?;
    let mut head = vec![0u8; 8192];
    let n = file.read(&mut head).await
        .map_err(|e| UploadError::io("Cannot read head", e))?;
    head.truncate(n);

    let container = detect_container(&head);
    if container == ContainerFormat::Unknown {
        return Ok(None);
    }

    let (offset, idx_bytes) = match container {
        ContainerFormat::Mp4 => extract_mp4(&mut file, file_size).await?,
        ContainerFormat::Mkv => extract_mkv(&mut file, file_size).await?,
        ContainerFormat::Avi => extract_avi(&mut file, file_size).await?,
        ContainerFormat::Unknown => unreachable!(),
    };

    let idx_path = idx_cache_dir.join(format!("{}.idx", file_id));
    if let Some(parent) = idx_path.parent() {
        tokio::fs::create_dir_all(parent).await
            .map_err(|e| UploadError::io("Cannot create idx dir", e))?;
    }

    let mut out = Vec::with_capacity(20 + idx_bytes.len());
    out.extend_from_slice(b"IDXC");
    out.extend_from_slice(&offset.to_be_bytes());
    out.extend_from_slice(&(idx_bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(&idx_bytes);

    tokio::fs::write(&idx_path, &out).await
        .map_err(|e| UploadError::io("Cannot write idx file", e))?;

    tracing::info!(target: "idx_cache", "saved: file_id={} container={:?} offset={} size={}",
        file_id, container, offset, idx_bytes.len());
    Ok(Some(()))
}

#[derive(Debug, Eq, PartialEq)]
enum ContainerFormat { Mp4, Mkv, Avi, Unknown }

fn detect_container(head: &[u8]) -> ContainerFormat {
    if head.len() >= 8 && &head[4..8] == b"ftyp" { ContainerFormat::Mp4 }
    else if head.len() >= 4 && &head[0..4] == &[0x1A, 0x45, 0xDF, 0xA3] { ContainerFormat::Mkv }
    else if head.len() >= 4 && &head[0..4] == b"RIFF" { ContainerFormat::Avi }
    else { ContainerFormat::Unknown }
}

// ── MP4: moov atom ──────────────────────────────────────────────

async fn extract_mp4(
    file: &mut tokio::fs::File,
    file_size: u64,
) -> UploadResult<(u64, Vec<u8>)> {
    let scan_size = (1024 * 1024).min(file_size as usize);

    // Scan head (faststart: moov at beginning)
    file.seek(std::io::SeekFrom::Start(0)).await
        .map_err(|e| UploadError::io("Seek", e))?;
    let mut buf = vec![0u8; scan_size];
    file.read_exact(&mut buf).await
        .map_err(|e| UploadError::io("Read", e))?;
    if let Some(result) = find_moov(&buf, 0) {
        return Ok(result);
    }

    // Scan tail (moov at end — common for non-faststart)
    let tail_start = file_size.saturating_sub(scan_size as u64);
    file.seek(std::io::SeekFrom::Start(tail_start)).await
        .map_err(|e| UploadError::io("Seek", e))?;
    buf.resize(scan_size, 0);
    file.read_exact(&mut buf).await
        .map_err(|e| UploadError::io("Read", e))?;
    if let Some(result) = find_moov(&buf, tail_start) {
        return Ok(result);
    }

    Err(UploadError::internal_message("moov atom not found"))
}

fn find_moov(buf: &[u8], buf_offset: u64) -> Option<(u64, Vec<u8>)> {
    let pos = buf.windows(4).position(|w| w == b"moov")?;
    if pos < 4 { return None; }
    let box_size = u32::from_be_bytes(buf[pos-4..pos].try_into().ok()?) as usize;
    if box_size < 8 { return None; }
    let start = pos - 4;
    if start + box_size > buf.len() { return None; }
    let file_off = buf_offset + start as u64;
    Some((file_off, buf[start..start + box_size].to_vec()))
}

// ── MKV: Cues element ───────────────────────────────────────────

const CUES_ID: [u8; 4] = [0x1C, 0x53, 0xBB, 0x6B];

async fn extract_mkv(
    file: &mut tokio::fs::File,
    file_size: u64,
) -> UploadResult<(u64, Vec<u8>)> {
    // Cues is almost always in first few MB (right after SeekHead/Info/Tracks)
    let scan_size = (1024 * 1024 * 2).min(file_size as usize);

    file.seek(std::io::SeekFrom::Start(0)).await
        .map_err(|e| UploadError::io("Seek", e))?;
    let mut buf = vec![0u8; scan_size];
    file.read_exact(&mut buf).await
        .map_err(|e| UploadError::io("Read", e))?;

    let id_pos = buf.windows(4).position(|w| w == &CUES_ID)
        .ok_or_else(|| UploadError::internal_message("Cues not found"))?;

    // EBML data size VINT starts right after the 4-byte ID
    let size_pos = id_pos + 4;
    let (data_size, vint_len) = parse_ebml_vint(&buf[size_pos..])?;

    // EBML unknown size: all data bits set to 1 (after marker bit)
    let unknown_mask = (1u64 << (vint_len * 7)) - 1;
    if data_size == unknown_mask {
        return Err(UploadError::internal_message("Cues has unknown size, cannot extract"));
    }

    let total = 4 + vint_len + data_size as usize;
    if id_pos + total > buf.len() {
        return Err(UploadError::internal_message("Cues extends beyond scan buffer"));
    }

    let file_off = id_pos as u64;
    Ok((file_off, buf[id_pos..id_pos + total].to_vec()))
}

// EBML VINT: leading 1-bits indicate byte length
// 1xxx xxxx → 1 byte, 01xx xxxx → 2 bytes, 001x xxxx → 3 bytes, etc.
fn vint_byte_count(first: u8) -> UploadResult<usize> {
    if first & 0x80 != 0 { Ok(1) }
    else if first & 0x40 != 0 { Ok(2) }
    else if first & 0x20 != 0 { Ok(3) }
    else if first & 0x10 != 0 { Ok(4) }
    else if first & 0x08 != 0 { Ok(5) }
    else if first & 0x04 != 0 { Ok(6) }
    else if first & 0x02 != 0 { Ok(7) }
    else if first & 0x01 != 0 { Ok(8) }
    else { Err(UploadError::internal_message("EBML VINT all-zero first byte")) }
}

fn parse_ebml_vint(data: &[u8]) -> UploadResult<(u64, usize)> {
    let first = *data.first().ok_or_else(|| UploadError::internal_message("empty VINT data"))?;
    let n = vint_byte_count(first)?;
    if data.len() < n {
        return Err(UploadError::internal_message("VINT truncated"));
    }
    let mut val = 0u64;
    for i in 0..n {
        val = (val << 8) | data[i] as u64;
    }
    // Clear the marker bit (first 1 bit)
    let mask = 1u64 << (n * 8 - n);
    val &= !mask;
    Ok((val, n))
}

// ── AVI: idx1 chunk ─────────────────────────────────────────────

async fn extract_avi(
    file: &mut tokio::fs::File,
    file_size: u64,
) -> UploadResult<(u64, Vec<u8>)> {
    // idx1 is always at the end of the file
    let tail_size = (1024 * 1024).min(file_size as usize);
    let tail_start = file_size.saturating_sub(tail_size as u64);

    file.seek(std::io::SeekFrom::Start(tail_start)).await
        .map_err(|e| UploadError::io("Seek", e))?;
    let mut buf = vec![0u8; tail_size];
    file.read_exact(&mut buf).await
        .map_err(|e| UploadError::io("Read", e))?;

    let pos = buf.windows(4).position(|w| w == b"idx1")
        .ok_or_else(|| UploadError::internal_message("idx1 not found"))?;

    let chunk_size = u32::from_le_bytes(
        buf[pos+4..pos+8].try_into().map_err(|_| UploadError::internal_message("idx1: bad size field"))?
    ) as usize;

    let total = 8 + chunk_size;
    if pos + total > buf.len() {
        return Err(UploadError::internal_message("idx1 extends beyond tail buffer"));
    }

    let file_off = tail_start + pos as u64;
    Ok((file_off, buf[pos..pos + total].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::UploadError;

    #[tokio::test]
    async fn unparseable_mp4_returns_typed_internal_error() {
        let dir = std::env::temp_dir().join("omega_idx_test");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        // ftyp box but no moov: passes detection, fails extraction.
        let mut garbage = b"\x00\x00\x00\x20ftypisom".to_vec();
        garbage.resize(100, 0);
        let path = dir.join("garbage.mp4");
        tokio::fs::write(&path, &garbage).await.unwrap();
        let err = extract_and_save_idx(&path, 100, 1, &dir).await.unwrap_err();
        assert!(matches!(err, UploadError::Internal { .. }), "unexpected: {err:?}");
        tokio::fs::remove_file(&path).await.ok();
    }
}
