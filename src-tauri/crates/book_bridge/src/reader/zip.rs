use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use bytes::Bytes;

use futures_util::FutureExt;
use omega_drive_gateway::provider::{
    stream::StreamRegistry,
    storage::PartMetadata,
};
use omega_drive_gateway::player::cache::ByteCache;
use omega_drive_gateway::player::singleflight::PartSingleFlight;

struct ZipEntryMeta {
    local_header_offset: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    compression_method: u16,
}

pub struct ZipReader {
    file_id: i64,
    entries: HashMap<String, ZipEntryMeta>,
    parts: Vec<PartMetadata>,
    chunk_offsets: Vec<u64>,
    pub opf_dir: String,
    registry: Arc<StreamRegistry>,
    byte_cache: Arc<dyn ByteCache>,
    singleflight: Arc<dyn PartSingleFlight>,
}

fn read_u16le(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}
fn read_u32le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

const EOCD_SIG: &[u8; 4] = b"PK\x05\x06";
const CD_SIG: &[u8; 4] = b"PK\x01\x02";
const LFH_SIG: &[u8; 4] = b"PK\x03\x04";

impl ZipReader {
    /// Open a ZIP file by fetching its Central Directory from remote parts.
    pub async fn open(
        file_id: i64,
        parts: Vec<PartMetadata>,
        registry: Arc<StreamRegistry>,
        byte_cache: Arc<dyn ByteCache>,
        singleflight: Arc<dyn PartSingleFlight>,
    ) -> Result<Self, String> {
        let mut parts: Vec<PartMetadata> = parts
            .into_iter()
            .filter(|p| p.part_type == "chunk")
            .fold(BTreeMap::<u32, PartMetadata>::new(), |mut map, p| {
                match map.get(&p.part_index) {
                    Some(ex) if ex.platform == "discord" && p.platform != "discord" => {
                        map.insert(p.part_index, p);
                    }
                    None => { map.insert(p.part_index, p); }
                    _ => {}
                }
                map
            })
            .into_values()
            .collect();
        parts.sort_by_key(|p| p.part_index);

        if parts.is_empty() {
            return Err("no chunk parts found".into());
        }

        // Build cumulative offsets
        let mut chunk_offsets = Vec::with_capacity(parts.len());
        let mut total: u64 = 0;
        for p in &parts {
            chunk_offsets.push(total);
            total += p.size as u64;
        }
        let total_size = total;

        tracing::info!(
            part_count = parts.len(),
            total_size,
            "zip open: parts filtered/sorted"
        );
        for p in &parts {
            tracing::info!(
                part_index = p.part_index,
                platform = p.platform,
                size = p.size,
                "zip open: part"
            );
        }

        // Fetch EOCD (last 128KB — covers max comment 65535B)
        let eocd_size = 128u64.min(total_size);
        let eocd_offset = total_size - eocd_size;
        tracing::info!(eocd_offset, eocd_size, "zip open: fetching EOCD");
        let eocd_data = Self::read_range(
            file_id, &parts, &chunk_offsets, eocd_offset, eocd_size,
            &registry, &byte_cache, &singleflight,
        ).await?;

        tracing::info!(
            data_len = eocd_data.len(),
            first_4 = ?&eocd_data[..4.min(eocd_data.len())],
            last_4 = ?&eocd_data[eocd_data.len().saturating_sub(4)..],
            "zip open: EOCD data received"
        );

        // Find EOCD signature from the end (inclusive range — EOCD can be at data.len()-22)
        let search_end = eocd_data.len().saturating_sub(22);
        let eocd_pos = (0..=search_end)
            .rev()
            .find(|&i| &eocd_data[i..i + 4] == EOCD_SIG)
            .ok_or_else(|| format!("EOCD signature not found (data_len={})", eocd_data.len()))?;

        let cd_offset = read_u32le(&eocd_data, eocd_pos + 16) as u64;
        let cd_size = read_u32le(&eocd_data, eocd_pos + 12) as u64;
        let total_entries = read_u16le(&eocd_data, eocd_pos + 10) as usize;

        // Fetch Central Directory (may share parts with EOCD — cached)
        let cd_data = Self::read_range(
            file_id, &parts, &chunk_offsets, cd_offset, cd_size,
            &registry, &byte_cache, &singleflight,
        ).await?;

        // Parse CD entries
        let mut entries = HashMap::with_capacity(total_entries);
        let mut pos = 0usize;
        while pos + 46 <= cd_data.len() {
            if &cd_data[pos..pos + 4] != CD_SIG {
                break;
            }
            let compression_method = read_u16le(&cd_data, pos + 10);
            let compressed_size = read_u32le(&cd_data, pos + 20);
            let uncompressed_size = read_u32le(&cd_data, pos + 24);
            let filename_len = read_u16le(&cd_data, pos + 28) as usize;
            let extra_len = read_u16le(&cd_data, pos + 30) as usize;
            let comment_len = read_u16le(&cd_data, pos + 32) as usize;
            let local_header_offset = read_u32le(&cd_data, pos + 42);

            if filename_len > 0 && pos + 46 + filename_len <= cd_data.len() {
                let name = String::from_utf8_lossy(&cd_data[pos + 46..pos + 46 + filename_len]).to_string();

                // Skip __MACOSX and directory entries
                if !name.starts_with("__MACOSX") && !name.ends_with('/') {
                    entries.insert(name, ZipEntryMeta {
                        local_header_offset,
                        compressed_size,
                        uncompressed_size,
                        compression_method,
                    });
                }
            }

            pos += 46 + filename_len + extra_len + comment_len;
        }

        tracing::info!(entry_count = entries.len(), "zip open: CD entries");

        // Determine opf_dir by reading META-INF/container.xml (may share parts — cached)
        let opf_dir = if let Some(meta) = entries.get("META-INF/container.xml") {
            let raw = Self::read_entry_raw(
                file_id, &parts, &chunk_offsets, meta, &registry,
                &byte_cache, &singleflight, false,
            ).await?;
            let s = String::from_utf8_lossy(&raw);
            let prefix = "full-path=\"";
            if let Some(start) = s.find(prefix) {
                let start = start + prefix.len();
                if let Some(end) = s[start..].find('"') {
                    let opf_path = &s[start..start + end];
                    opf_path.rsplit_once('/').map(|(d, _)| format!("{}/", d)).unwrap_or_default()
                } else { String::new() }
            } else { String::new() }
        } else { String::new() };

        Ok(Self {
            file_id,
            entries,
            parts,
            chunk_offsets,
            opf_dir,
            registry,
            byte_cache,
            singleflight,
        })
    }

    /// Read + decompress a named entry.
    pub async fn read_entry(&self, name: &str) -> Result<Vec<u8>, String> {
        let try_names = if !self.opf_dir.is_empty() && !name.starts_with(&self.opf_dir) {
            vec![format!("{}{}", self.opf_dir, name), name.to_string()]
        } else {
            vec![name.to_string()]
        };
        for n in &try_names {
            if let Some(m) = self.entries.get(n.as_str()) {
                return Self::read_and_decompress(
                    self.file_id, m,
                    &self.parts, &self.chunk_offsets,
                    &self.registry,
                    &self.byte_cache, &self.singleflight,
                ).await;
            }
        }
        Err(format!("entry not found: {name}"))
    }

    pub async fn read_entry_str(&self, name: &str) -> Result<String, String> {
        let data = self.read_entry(name).await?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    // ── Internal helpers ──

    /// Download a byte range from the assembled file.
    async fn read_range(
        file_id: i64,
        parts: &[PartMetadata],
        chunk_offsets: &[u64],
        offset: u64,
        size: u64,
        registry: &StreamRegistry,
        byte_cache: &Arc<dyn ByteCache>,
        singleflight: &Arc<dyn PartSingleFlight>,
    ) -> Result<Vec<u8>, String> {
        let end = offset + size;
        let mut current = offset;
        let mut result = Vec::with_capacity(size as usize);

        while current < end {
            let chunk_idx = match chunk_offsets.binary_search(&current) {
                Ok(i) => i,
                Err(0) => return Err(format!("offset {current} before first chunk")),
                Err(i) => i - 1,
            };
            let part = &parts[chunk_idx];
            let file_offset = chunk_offsets[chunk_idx];
            let chunk_end = file_offset + part.size as u64;
            let local_len = (end - current).min(chunk_end - current);

            // Check byte cache
            if byte_cache.is_range_filled(file_id, current, local_len).await {
                let data = byte_cache.wait_range(file_id, current, local_len).await?;
                result.extend_from_slice(&data);
                current += local_len;
                continue;
            }

            // Not in cache — download the full part via singleflight
            let part_clone = part.clone();
            let bc = byte_cache.clone();
            let gw = registry.get(&part_clone.platform)
                .ok_or_else(|| format!("gateway {} not found", part_clone.platform))?;
            let _ = singleflight.run((file_id, part.part_index), Box::new(move || {
                async move {
                    let raw = gw.download_part_range(&part_clone, None).await
                        .map_err(|e| format!("download part {}: {e}", part_clone.part_index))?;
                    bc.write(file_id, file_offset, Bytes::from(raw.clone())).await;
                    Ok(Bytes::from(raw))
                }.boxed()
            })).await?;

            // Read from cache after write
            let data = byte_cache.wait_range(file_id, current, local_len).await?;
            result.extend_from_slice(&data);
            current += local_len;
        }

        Ok(result)
    }

    /// Read raw bytes for an entry at its local header offset.
    /// If `expand`, small reads are expanded to 512KB for better cache locality.
    async fn read_entry_raw(
        file_id: i64,
        parts: &[PartMetadata],
        chunk_offsets: &[u64],
        meta: &ZipEntryMeta,
        registry: &StreamRegistry,
        byte_cache: &Arc<dyn ByteCache>,
        singleflight: &Arc<dyn PartSingleFlight>,
        expand: bool,
    ) -> Result<Vec<u8>, String> {
        let lfh_size = 30u64;
        let data_offset = meta.local_header_offset as u64;
        let read_size = lfh_size + meta.compressed_size as u64 + 65536;

        let raw = if expand {
            let total = chunk_offsets.last().copied().unwrap_or(0)
                + parts.last().map_or(0, |p| p.size as u64);
            let mid = data_offset + read_size / 2;
            let half = 262144u64;
            let mut new_start = mid.saturating_sub(half);
            let mut new_end = new_start + 524288;
            if new_end > total { new_end = total; }
            new_start = new_end.saturating_sub(524288);

            let expanded = Self::read_range(
                file_id, parts, chunk_offsets, new_start, new_end - new_start,
                registry, byte_cache, singleflight,
            ).await?;
            let off = (data_offset - new_start) as usize;
            if off + read_size as usize <= expanded.len() {
                expanded[off..off + read_size as usize].to_vec()
            } else {
                Self::read_range(
                    file_id, parts, chunk_offsets, data_offset, read_size,
                    registry, byte_cache, singleflight,
                ).await?
            }
        } else {
            Self::read_range(
                file_id, parts, chunk_offsets, data_offset, read_size,
                registry, byte_cache, singleflight,
            ).await?
        };

        if raw.len() < 30 || &raw[..4] != LFH_SIG {
            return Err(format!("bad LFH at offset {}", meta.local_header_offset));
        }
        let filename_len = read_u16le(&raw, 26) as usize;
        let extra_len = read_u16le(&raw, 28) as usize;
        let data_start = 30 + filename_len + extra_len;
        let data_end = data_start + meta.compressed_size as usize;

        if data_end > raw.len() {
            let exact_size = data_end as u64;
            let raw2 = Self::read_range(
                file_id, parts, chunk_offsets,
                meta.local_header_offset as u64, exact_size,
                registry, byte_cache, singleflight,
            ).await?;
            Ok(raw2[data_start..data_start + meta.compressed_size as usize].to_vec())
        } else {
            Ok(raw[data_start..data_end].to_vec())
        }
    }

    /// Read + decompress an entry.
    async fn read_and_decompress(
        file_id: i64,
        meta: &ZipEntryMeta,
        parts: &[PartMetadata],
        chunk_offsets: &[u64],
        registry: &StreamRegistry,
        byte_cache: &Arc<dyn ByteCache>,
        singleflight: &Arc<dyn PartSingleFlight>,
    ) -> Result<Vec<u8>, String> {
        let compressed = Self::read_entry_raw(
            file_id, parts, chunk_offsets, meta, registry,
            byte_cache, singleflight, true,
        ).await?;

        if meta.compression_method == 0 {
            return Ok(compressed);
        }

        if meta.compression_method != 8 {
            return Err(format!("unsupported compression method {}", meta.compression_method));
        }

        use std::io::Read;
        let mut decoder = flate2::read::DeflateDecoder::new(&compressed[..]);
        let mut out = Vec::with_capacity(meta.uncompressed_size as usize);
        decoder.read_to_end(&mut out)
            .map_err(|e| format!("deflate decompress: {e}"))?;
        Ok(out)
    }

    /// Map a byte range in the assembled file to (part_idx, local_start, len) tuples.
    #[allow(dead_code)]
    fn map_range(&self, offset: u64, size: u64) -> Vec<(usize, u64, u64)> {
        let mut result = Vec::new();
        let end = offset + size;
        let mut cur = offset;
        while cur < end {
            let ci = match self.chunk_offsets.binary_search(&cur) {
                Ok(i) => i,
                Err(0) => break,
                Err(i) => i - 1,
            };
            let ce = self.chunk_offsets[ci] + self.parts[ci].size as u64;
            let ls = cur - self.chunk_offsets[ci];
            let ll = (end - cur).min(ce - cur);
            result.push((ci, ls, ll));
            cur += ll;
        }
        result
    }
}
