use anyhow::{Context, Result};
use omega_drive_gateway::core::filemeta::{ChapterInfo, FormatInfo, FullMetadata, StreamInfo};
use serde_json::Value;
use std::path::PathBuf;
use tracing::{info, warn};

use crate::context::UploadContext;

fn parse_u32(value: Option<&Value>) -> Option<u32> {
    value
        .and_then(|raw| {
            raw.as_u64()
                .or_else(|| raw.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
        .and_then(|raw| u32::try_from(raw).ok())
}

pub(crate) async fn extract_full_metadata(state: &UploadContext, path: &PathBuf) -> Result<FullMetadata> {
    let sidecar = state
        .sidecar
        .as_deref()
        .context("SidecarProvider not available for ffprobe")?;
    let sidecar_probe = sidecar
        .sidecar_output(
            "ffprobe",
            &[
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
                "-show_chapters",
                "-analyzeduration",
                "150M",
                "-probesize",
                "150M",
                path.to_str().context("Path error")?,
            ],
        )
        .await?;

    let raw: serde_json::Value = serde_json::from_slice(&sidecar_probe)?;

    let mut format = FormatInfo {
        duration: raw["format"]["duration"].as_str().map(|s| s.to_string()),
        size: raw["format"]["size"].as_str().map(|s| s.to_string()),
        bit_rate: raw["format"]["bit_rate"].as_str().map(|s| s.to_string()),
        format_name: raw["format"]["format_name"].as_str().map(|s| s.to_string()),
        encoder: raw["format"]["tags"]["encoder"]
            .as_str()
            .map(|s| s.to_string()),
    };

    let duration_f64 = format
        .duration
        .as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let size_bytes = format
        .size
        .as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let is_suspicious = if duration_f64 > 0.0 && size_bytes > 50 * 1024 * 1024 {
        let bitrate_mbps = (size_bytes as f64 * 8.0) / duration_f64 / 1_000_000.0;
        bitrate_mbps > 150.0 || (size_bytes > 1024 * 1024 * 1024 && duration_f64 < 1800.0)
    } else {
        duration_f64 < 1.0
    };

    if is_suspicious {
        if let Some(tail_d) = probe_duration_tail(state, path).await {
            format.duration = Some(tail_d.to_string());
        } else {
            warn!("Header sai & Duoi hong. Ap dung 12 gio de Discovery.");
            format.duration = Some("43200.0".to_string());
        }
    }

    let mut streams = Vec::new();
    if let Some(arr) = raw["streams"].as_array() {
        for s in arr {
            streams.push(StreamInfo {
                index: parse_u32(s.get("index")).unwrap_or(0),
                codec_type: s["codec_type"].as_str().unwrap_or("unknown").to_string(),
                codec_name: s["codec_name"].as_str().map(|v| v.to_string()),
                language: s["tags"]["language"].as_str().map(|v| v.to_string()),
                width: parse_u32(s.get("width")),
                height: parse_u32(s.get("height")),
                bit_rate: s["bit_rate"].as_str().map(|v| v.to_string()),
                sample_rate: s["sample_rate"].as_str().map(|v| v.to_string()),
                channels: parse_u32(s.get("channels")),
            });
        }
    }

    let mut chapters = Vec::new();
    if let Some(arr) = raw["chapters"].as_array() {
        for c in arr {
            chapters.push(ChapterInfo {
                start_time: c["start_time"].as_str().unwrap_or("0").to_string(),
                end_time: c["end_time"].as_str().unwrap_or("0").to_string(),
                title: c["tags"]["title"].as_str().map(|v| v.to_string()),
            });
        }
    }

    Ok(FullMetadata {
        format,
        streams,
        chapters,
    })
}

pub(crate) async fn probe_duration_tail(state: &UploadContext, path: &PathBuf) -> Option<f64> {
    let sidecar = state.sidecar.as_deref()?;
    let output = sidecar
        .sidecar_output(
            "ffprobe",
            &[
                "-v",
                "error",
                "-read_intervals",
                "%-1",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                path.to_str()?,
            ],
        )
        .await
        .ok()?;

    info!(
        "Tail probing: scanning last frames for accurate duration for {:?}",
        path.file_name()
    );

    let val = String::from_utf8_lossy(&output).trim().to_string();
    if let Ok(d) = val.parse::<f64>() {
        info!("Tail probing succeeded: {:.2}s", d);
        return Some(d);
    }

    warn!("Tail probing failed or no end timestamp found.");
    None
}
