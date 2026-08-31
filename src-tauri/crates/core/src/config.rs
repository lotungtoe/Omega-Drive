use anyhow::Context;
use std::collections::HashMap;

pub use omega_drive_gateway::core::config::{
    Config, GroupSettings, LoggingConfig, ProviderConfig, ProviderLimitConfig, ProviderRetryConfig,
    ProviderTransferConfig, UploadMode,
};

#[derive(Clone, Copy)]
pub struct ProviderConfigDefaults {
    pub parallel_sends: usize,
    pub parallel_sends_min: usize,
    pub parallel_sends_max: usize,
    pub chunk_mb: Option<u64>,
    pub chunk_mb_min: u64,
    pub chunk_mb_max: u64,
    pub batch_size: Option<usize>,
    pub batch_size_min: usize,
    pub batch_size_max: usize,
    pub send_retries: u32,
    pub send_retries_min: u32,
    pub send_retries_max: u32,
    pub retry_base_delay_s: u64,
    pub retry_base_delay_s_min: u64,
    pub retry_base_delay_s_max: u64,
    pub hard_limit_mb: u64,
    pub file_limit_mb: u64,
    pub limit_mb_min: u64,
    pub limit_mb_max: u64,
}

impl ProviderConfigDefaults {
    pub const DISCORD: Self = Self {
        parallel_sends: 1,  parallel_sends_min: 1, parallel_sends_max: 10,
        chunk_mb: Some(10), chunk_mb_min: 0,       chunk_mb_max: 10,
        batch_size: Some(10), batch_size_min: 1,   batch_size_max: 10,
        send_retries: 3,     send_retries_min: 1,  send_retries_max: 10,
        retry_base_delay_s: 2, retry_base_delay_s_min: 1, retry_base_delay_s_max: 30,
        hard_limit_mb: 100,  file_limit_mb: 100,   limit_mb_min: 8, limit_mb_max: 4000,
    };

    pub const TELEGRAM: Self = Self {
        parallel_sends: 3,  parallel_sends_min: 1, parallel_sends_max: 10,
        chunk_mb: Some(20), chunk_mb_min: 0,       chunk_mb_max: 2000,
        batch_size: Some(1), batch_size_min: 1,    batch_size_max: 10,
        send_retries: 3,    send_retries_min: 1,   send_retries_max: 10,
        retry_base_delay_s: 2, retry_base_delay_s_min: 1, retry_base_delay_s_max: 30,
        hard_limit_mb: 0,   file_limit_mb: 2000,   limit_mb_min: 8, limit_mb_max: 4000,
    };
}

impl Default for ProviderConfigDefaults {
    fn default() -> Self {
        Self {
            parallel_sends: 1, parallel_sends_min: 1, parallel_sends_max: 10,
            chunk_mb: None, chunk_mb_min: 0, chunk_mb_max: 2000,
            batch_size: None, batch_size_min: 1, batch_size_max: 10,
            send_retries: 3, send_retries_min: 1, send_retries_max: 10,
            retry_base_delay_s: 2, retry_base_delay_s_min: 1, retry_base_delay_s_max: 30,
            hard_limit_mb: 0, file_limit_mb: 0, limit_mb_min: 8, limit_mb_max: 4000,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ProviderConfigDescriptor {
    pub id: &'static str,
    pub defaults: fn(&GroupSettings) -> ProviderConfigDefaults,
    pub apply_legacy_json: Option<fn(&mut serde_json::Value)>,
}

impl ProviderConfigDescriptor {
    pub const fn new(
        id: &'static str,
        defaults: fn(&GroupSettings) -> ProviderConfigDefaults,
        apply_legacy_json: Option<fn(&mut serde_json::Value)>,
    ) -> Self {
        Self { id, defaults, apply_legacy_json }
    }
}

macro_rules! clamp {
    ($val:expr, $default:expr, $lo:expr, $hi:expr) => {{
        #[allow(unused_comparisons)]
        let out = {
            let v = $val.unwrap_or($default);
            if !($lo..=$hi).contains(&v) {
                eprintln!("Config {} outside allowed range [{},{}] ? Using default {}",
                    stringify!($val), $lo, $hi, $default);
                $default
            } else { v }
        };
        out
    }};
}

macro_rules! clamp_min {
    ($val:expr, $default:expr, $lo:expr) => {{
        let v = $val.unwrap_or($default);
        if v < $lo {
            eprintln!("Config {} below minimum {} ? Using default {}", stringify!($val), $lo, $default);
            $default
        } else { v }
    }};
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawGroup {
    chunk_mb: Option<u64>,
    parallel_sends: Option<usize>,
    zip_level: Option<u32>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawUpload {
    upload_mode: Option<UploadMode>,
    #[serde(default)]
    general: RawGroup,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawDownload {
    http_timeout_s: Option<u64>,
    retry_count: Option<u32>,
    retry_base_delay_s: Option<u64>,
    part_delay_ms: Option<u64>,
    stream_buffer_kb: Option<usize>,
    large_file_threshold_mb: Option<u64>,
    mpv_cache_secs: Option<u64>,
    mpv_demuxer_max_mb: Option<u64>,
    mpv_readahead_secs: Option<u64>,
    prevent_sleep_enabled: Option<bool>,
    bandwidth_limit_kbps: Option<u64>,
    adaptive_soft_limit: Option<bool>,
    soft_limit_ratio: Option<f64>,
    disk_check_interval_parts: Option<u32>,
    auto_resume_on_startup: Option<bool>,
    purge_days: Option<u32>,
    hwdec_method: Option<String>,
    d3d11_adapter: Option<String>,
    cache_preview_max_mb: Option<u64>,
    cache_video_max_mb: Option<u64>,
    cache_audio_max_mb: Option<u64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawRam {
    gc_interval_minutes: Option<u64>,
    trash_ttl_days: Option<i64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawServer {
    log_level: Option<String>,
    keep_alive_s: Option<u64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawStartup {
    persistent_video_bridge: Option<bool>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawProviderTransfer {
    parallel_sends: Option<usize>,
    chunk_mb: Option<u64>,
    batch_size: Option<usize>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawProviderRetry {
    send_retries: Option<u32>,
    retry_base_delay_s: Option<u64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawProviderLimits {
    hard_limit_mb: Option<u64>,
    file_limit_mb: Option<u64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawProviderConfig {
    #[serde(default)]
    transfer: RawProviderTransfer,
    #[serde(default)]
    retry: RawProviderRetry,
    #[serde(default)]
    limits: RawProviderLimits,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawLogging {
    #[serde(default)]
    feature_enabled: HashMap<String, bool>,
    frontend_enabled: Option<bool>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawBackup {
    enabled: Option<bool>,
    snapshot_interval_days: Option<u64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawConfig {
    #[serde(default)]
    upload: RawUpload,
    #[serde(default)]
    download: RawDownload,
    #[serde(default)]
    ram: RawRam,
    #[serde(default)]
    server: RawServer,
    #[serde(default)]
    providers: HashMap<String, RawProviderConfig>,
    #[serde(default)]
    logging: RawLogging,
    #[serde(default)]
    startup: RawStartup,
    #[serde(default)]
    backup: RawBackup,
}

fn parse_limit_mb(provider_id: &str, field_name: &str, value: Option<u64>, default: u64, min: u64, max: u64) -> u64 {
    let resolved = value.unwrap_or(default);
    if resolved == 0 {
        0
    } else if !(min..=max).contains(&resolved) {
        eprintln!("Provider {}.{}={} outside allowed range [{},{}] ? Using default {}",
            provider_id, field_name, resolved, min, max, default);
        default
    } else {
        resolved
    }
}

fn build_provider_config(provider_id: &str, raw: RawProviderConfig, defaults: ProviderConfigDefaults) -> ProviderConfig {
    ProviderConfig {
        transfer: ProviderTransferConfig {
            parallel_sends: clamp!(raw.transfer.parallel_sends, defaults.parallel_sends, defaults.parallel_sends_min, defaults.parallel_sends_max),
            chunk_mb: raw.transfer.chunk_mb.or(defaults.chunk_mb).map(|v| {
                v.clamp(defaults.chunk_mb_min, defaults.chunk_mb_max)
            }),
            batch_size: raw.transfer.batch_size.or(defaults.batch_size).map(|value| value.clamp(defaults.batch_size_min, defaults.batch_size_max)),
        },
        retry: ProviderRetryConfig {
            send_retries: clamp!(raw.retry.send_retries, defaults.send_retries, defaults.send_retries_min, defaults.send_retries_max),
            retry_base_delay_s: clamp!(raw.retry.retry_base_delay_s, defaults.retry_base_delay_s, defaults.retry_base_delay_s_min, defaults.retry_base_delay_s_max),
        },
        limits: ProviderLimitConfig {
            hard_limit_bytes: parse_limit_mb(provider_id, "limits.hard_limit_mb", raw.limits.hard_limit_mb, defaults.hard_limit_mb, defaults.limit_mb_min, defaults.limit_mb_max) * 1024 * 1024,
            file_limit_bytes: parse_limit_mb(provider_id, "limits.file_limit_mb", raw.limits.file_limit_mb, defaults.file_limit_mb, defaults.limit_mb_min, defaults.limit_mb_max) * 1024 * 1024,
        },
    }
}

fn strip_comment_keys(val: &mut serde_json::Value) {
    if let serde_json::Value::Object(map) = val {
        map.retain(|k, _| !k.starts_with('_'));
        for v in map.values_mut() {
            strip_comment_keys(v);
        }
    }
}

pub fn load_config(
    base_dir: &std::path::Path,
    provider_descriptors: &[ProviderConfigDescriptor],
) -> Config {
    let path = base_dir.join("config.json");
    let raw: RawConfig = if path.exists() {
        match std::fs::read_to_string(&path)
            .context("Cannot read config.json file")
            .and_then(|s| {
                let mut val: serde_json::Value = serde_json::from_str(&s)?;
                strip_comment_keys(&mut val);
                for descriptor in provider_descriptors {
                    if let Some(apply_legacy_json) = descriptor.apply_legacy_json {
                        apply_legacy_json(&mut val);
                    }
                }
                serde_json::from_value(val).map_err(Into::into)
            }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error reading config.json: {e} → Using defaults");
                RawConfig::default()
            }
        }
    } else {
        eprintln!("config.json not found → Using defaults");
        RawConfig::default()
    };
    config_from_raw(raw, provider_descriptors)
}

fn config_from_raw(r: RawConfig, provider_descriptors: &[ProviderConfigDescriptor]) -> Config {
    let u = &r.upload;
    let d = &r.download;
    let m = &r.ram;
    let s = &r.server;
    let logging = &r.logging;
    let startup = &r.startup;
    let mut raw_provider_configs = r.providers.clone();

    let upload_mode = u.upload_mode.unwrap_or(UploadMode::Safe);

    let general = GroupSettings {
        chunk_bytes: clamp!(u.general.chunk_mb, 10, 1, 100) * 1024 * 1024,
        parallel_sends: clamp!(u.general.parallel_sends, 4, 1, 10),
        zip_level: clamp!(u.general.zip_level, 0, 0, 9),
    };

    let http_timeout_s = clamp!(d.http_timeout_s, 600, 30, 3600);
    let download_retry = clamp!(d.retry_count, 3, 1, 10);
    let download_retry_base_s = clamp!(d.retry_base_delay_s, 2, 1, 30);
    let part_delay_ms = clamp!(d.part_delay_ms, 150, 0, 5000);
    let stream_buffer_kb = clamp!(d.stream_buffer_kb, 64, 8, 4096);
    let large_file_threshold_mb = clamp_min!(d.large_file_threshold_mb, 500, 50);
    let mpv_cache_secs = clamp!(d.mpv_cache_secs, 15, 1, 120);
    let mpv_demuxer_max_mb = clamp!(d.mpv_demuxer_max_mb, 64, 50, 2000);
    let mpv_readahead_secs = clamp!(d.mpv_readahead_secs, 15, 1, 120);    let prevent_sleep_enabled = d.prevent_sleep_enabled.unwrap_or(true);
    let bandwidth_limit_kbps = clamp!(d.bandwidth_limit_kbps, 0, 0, 2_000_000);
    let adaptive_soft_limit = d.adaptive_soft_limit.unwrap_or(true);
    let soft_limit_ratio = d
        .soft_limit_ratio
        .filter(|v| (0.5..=0.95).contains(v))
        .unwrap_or(0.8);
    let disk_check_interval_parts = clamp!(d.disk_check_interval_parts, 5, 1, 50);
    let auto_resume_on_startup = d.auto_resume_on_startup.unwrap_or(true);
    let purge_days = clamp!(d.purge_days, 7, 1, 30);

    let d3d11_adapter = d
        .d3d11_adapter
        .clone()
        .unwrap_or_else(|| "Auto".to_string());

    let cache_preview_max_bytes = clamp!(d.cache_preview_max_mb, 50, 10, 2000) * 1024 * 1024;
    let cache_video_max_bytes = clamp!(d.cache_video_max_mb, 400, 50, 5000) * 1024 * 1024;
    let cache_audio_max_bytes = clamp!(d.cache_audio_max_mb, 100, 10, 1000) * 1024 * 1024;
    let gc_interval_minutes = clamp!(m.gc_interval_minutes, 10, 1, 120);
    let trash_ttl_days = clamp!(m.trash_ttl_days, 30, 1, 365);

    let backup_enabled = r.backup.enabled.unwrap_or(false);
    let backup_snapshot_interval_days = clamp!(r.backup.snapshot_interval_days, 7, 1, 365);

    let log_level = {
        let raw = s.log_level.clone().unwrap_or_else(|| "warn".to_string());
        if ["debug", "info", "warning", "error", "critical"].contains(&raw.as_str()) {
            raw
        } else {
            eprintln!("Invalid log_level: {raw} ? Using 'info'");
            "warn".to_string()
        }
    };

    let mut providers = HashMap::new();

    for descriptor in provider_descriptors {
        let defaults = (descriptor.defaults)(&general);
        let raw_provider = raw_provider_configs
            .remove(descriptor.id)
            .unwrap_or_default();
        providers.insert(
            descriptor.id.to_string(),
            build_provider_config(descriptor.id, raw_provider, defaults),
        );
    }

    for (provider_id, raw_provider) in raw_provider_configs {
        providers.insert(
            provider_id.clone(),
            build_provider_config(
                &provider_id,
                raw_provider,
                ProviderConfigDefaults::default(),
            ),
        );
    }

    Config {
        general,
        upload_mode,
        providers,

        http_timeout_s,
        download_retry,
        download_retry_base_s,
        part_delay_ms,
        read_buffer_bytes: stream_buffer_kb * 1024,
        large_file_threshold_mb,
        mpv_cache_secs,
        mpv_demuxer_max_mb,
        mpv_readahead_secs,
        prevent_sleep_enabled,
        bandwidth_limit_kbps,
        adaptive_soft_limit,
        soft_limit_ratio,
        disk_check_interval_parts,
        auto_resume_on_startup,
        purge_days,

        cache_preview_max_bytes,
        cache_video_max_bytes,
        cache_audio_max_bytes,
        gc_interval_s: gc_interval_minutes * 60,
        trash_ttl_days,

        log_level,
        keep_alive_s: clamp!(s.keep_alive_s, 600, 10, 3600),

        persistent_video_bridge: startup.persistent_video_bridge.unwrap_or(true),

        logging: LoggingConfig {
            feature_enabled: logging.feature_enabled.clone(),
            frontend_enabled: logging.frontend_enabled.unwrap_or(true),
        },

        backup_enabled,
        backup_snapshot_interval_days,

        d3d11_adapter,
    }
}

pub fn save_config_to_file(config: &Config, base_dir: &std::path::Path) -> anyhow::Result<()> {
    let path = base_dir.join("config.json");
    let raw = raw_from_config(config);
    let json = serde_json::to_string_pretty(&raw)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Serialise a Config into the RawConfig shape used on disk and by the
/// Settings UI. This is the same conversion save_config_to_file performs
/// (so the on-disk format and the in-memory default map stay in lockstep),
/// exposed publicly so the get_settings handler can build a defaults map
/// for fields the user has not yet set. Returns a generic JSON Value so the
/// RawConfig type itself can stay private.
pub fn raw_from_config(config: &Config) -> serde_json::Value {
    let raw = raw_from_config_inner(config);
    serde_json::to_value(&raw).unwrap_or(serde_json::json!({}))
}

fn raw_from_config_inner(config: &Config) -> RawConfig {
    let providers = config
        .providers
        .iter()
        .map(|(provider_id, provider)| {
            (
                provider_id.clone(),
                RawProviderConfig {
                    transfer: RawProviderTransfer {
                        parallel_sends: Some(provider.transfer.parallel_sends),
                        chunk_mb: provider.transfer.chunk_mb,
                        batch_size: provider.transfer.batch_size,
                    },
                    retry: RawProviderRetry {
                        send_retries: Some(provider.retry.send_retries),
                        retry_base_delay_s: Some(provider.retry.retry_base_delay_s),
                    },
                    limits: RawProviderLimits {
                        hard_limit_mb: (provider.limits.hard_limit_bytes > 0)
                            .then_some(provider.limits.hard_limit_bytes / 1024 / 1024),
                        file_limit_mb: (provider.limits.file_limit_bytes > 0)
                            .then_some(provider.limits.file_limit_bytes / 1024 / 1024),
                    },
                },
            )
        })
        .collect::<HashMap<_, _>>();

    RawConfig {
        upload: RawUpload {
            upload_mode: Some(config.upload_mode),
            general: RawGroup {
                chunk_mb: Some(config.general.chunk_bytes / 1024 / 1024),
                parallel_sends: Some(config.general.parallel_sends),
                zip_level: Some(config.general.zip_level),
            },
        },
        download: RawDownload {
            http_timeout_s: Some(config.http_timeout_s),
            retry_count: Some(config.download_retry),
            retry_base_delay_s: Some(config.download_retry_base_s),
            part_delay_ms: Some(config.part_delay_ms),
            stream_buffer_kb: Some(config.read_buffer_bytes / 1024),
            large_file_threshold_mb: Some(config.large_file_threshold_mb),
            mpv_cache_secs: Some(config.mpv_cache_secs),
            mpv_demuxer_max_mb: Some(config.mpv_demuxer_max_mb),
            mpv_readahead_secs: Some(config.mpv_readahead_secs),
            prevent_sleep_enabled: Some(config.prevent_sleep_enabled),
            bandwidth_limit_kbps: Some(config.bandwidth_limit_kbps),
            adaptive_soft_limit: Some(config.adaptive_soft_limit),
            soft_limit_ratio: Some(config.soft_limit_ratio),
            disk_check_interval_parts: Some(config.disk_check_interval_parts),
            auto_resume_on_startup: Some(config.auto_resume_on_startup),
            purge_days: Some(config.purge_days),
            d3d11_adapter: Some(config.d3d11_adapter.clone()),
            cache_preview_max_mb: Some(config.cache_preview_max_bytes / 1024 / 1024),
            cache_video_max_mb: Some(config.cache_video_max_bytes / 1024 / 1024),
            cache_audio_max_mb: Some(config.cache_audio_max_bytes / 1024 / 1024),
            ..Default::default()
        },
        ram: RawRam {
            gc_interval_minutes: Some(config.gc_interval_s / 60),
            trash_ttl_days: Some(config.trash_ttl_days),
        },
        server: RawServer {
            log_level: Some(config.log_level.clone()),
            keep_alive_s: Some(config.keep_alive_s),
        },
        providers,
        logging: RawLogging {
            feature_enabled: config.logging.feature_enabled.clone(),
            frontend_enabled: Some(config.logging.frontend_enabled),
        },
        startup: RawStartup {
            persistent_video_bridge: Some(config.persistent_video_bridge),
        },
        backup: RawBackup {
            enabled: Some(config.backup_enabled),
            snapshot_interval_days: Some(config.backup_snapshot_interval_days),
        },
    }
}

pub fn print_config_summary(config: &Config) {
    println!("{}", "".repeat(60));
    println!("Application configuration");

    let gen_mb = config.general.chunk_bytes / 1024 / 1024;
    println!(
        " [General]  : chunk={}MB parallel={} zip={}",
        gen_mb, config.general.parallel_sends, config.general.zip_level
    );

    let mut provider_ids = config.providers.keys().cloned().collect::<Vec<_>>();
    provider_ids.sort();
    for provider_id in provider_ids {
        if let Some(provider) = config.providers.get(&provider_id) {
            let hard_limit_label = if provider.limits.hard_limit_bytes == 0 {
                "-".to_string()
            } else {
                format!("{}MB", provider.limits.hard_limit_bytes / 1024 / 1024)
            };
            let file_limit_label = if provider.limits.file_limit_bytes == 0 {
                "-".to_string()
            } else {
                format!("{}MB", provider.limits.file_limit_bytes / 1024 / 1024)
            };
            println!(
                " Provider {}: parallel={} retry={} base={}s hard_limit={} file_limit={}",
                provider_id,
                provider.transfer.parallel_sends,
                provider.retry.send_retries,
                provider.retry.retry_base_delay_s,
                hard_limit_label,
                file_limit_label
            );
        }
    }
    println!(
        " Download : timeout={}s retry={} large_file>={}MB",
        config.http_timeout_s, config.download_retry, config.large_file_threshold_mb
    );
    println!(
        " RAM      : gc={}min",
        config.gc_interval_s / 60
    );
    println!(
        " Server   : log={}",
        config.log_level
    );
    println!(
        " Backup   : {} snapshot={} days",
        if config.backup_enabled { "enabled" } else { "disabled" },
        config.backup_snapshot_interval_days
    );
    println!("{}", "".repeat(60));
}
