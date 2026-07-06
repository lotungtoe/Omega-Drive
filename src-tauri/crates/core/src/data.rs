pub use omega_drive_gateway::core::data::*;

fn parse_u32(value: Option<&serde_json::Value>) -> Option<u32> {
    value.and_then(|raw| {
        raw.as_u64()
            .or_else(|| raw.as_str().and_then(|s| s.parse::<u64>().ok()))
    })
    .and_then(|raw| u32::try_from(raw).ok())
}

fn parse_i64(value: Option<&serde_json::Value>) -> Option<i64> {
    value.and_then(|raw| {
        raw.as_i64()
            .or_else(|| raw.as_str().and_then(|s| s.parse::<i64>().ok()))
    })
}

fn parse_f64(value: Option<&serde_json::Value>) -> Option<f64> {
    value
        .and_then(|raw| {
            raw.as_f64()
                .or_else(|| raw.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
        .filter(|raw| raw.is_finite() && *raw > 0.0)
}

fn parse_string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|raw| raw.as_str())
        .map(str::to_string)
        .filter(|raw| !raw.is_empty())
}

pub fn parse_media_summary(raw_json: &str) -> Option<ParsedMediaSummary> {
    let parsed: serde_json::Value = serde_json::from_str(raw_json).ok()?;
    let format = parsed.get("format");
    let streams = parsed.get("streams")?.as_array()?;

    let video_stream = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("video"));
    let audio_stream = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("audio"));

    Some(ParsedMediaSummary {
        duration_sec: parse_f64(format.and_then(|f| f.get("duration"))),
        width: parse_u32(video_stream.and_then(|s| s.get("width"))),
        height: parse_u32(video_stream.and_then(|s| s.get("height"))),
        bitrate_bps: parse_i64(video_stream.and_then(|s| s.get("bit_rate")))
            .or_else(|| parse_i64(format.and_then(|f| f.get("bit_rate")))),
        video_codec: parse_string(video_stream.and_then(|s| s.get("codec_name"))),
        audio_codec: parse_string(audio_stream.and_then(|s| s.get("codec_name"))),
        audio_bitrate_bps: parse_i64(audio_stream.and_then(|s| s.get("bit_rate")))
            .or_else(|| parse_i64(format.and_then(|f| f.get("bit_rate")))),
        audio_codec_only: parse_string(audio_stream.and_then(|s| s.get("codec_name"))),
        sample_rate_hz: parse_u32(audio_stream.and_then(|s| s.get("sample_rate"))),
        channels: parse_u32(audio_stream.and_then(|s| s.get("channels"))),
        container: parse_string(format.and_then(|f| f.get("format_name"))),
    })
}
