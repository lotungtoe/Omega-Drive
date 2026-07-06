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
    prefetch_concurrency: Option<usize>,
    prefetch_chunks: Option<u32>,
    prefetch_debounce_ms: Option<u64>,
    mpv_cache_secs: Option<u64>,
    mpv_demuxer_max_mb: Option<u64>,
    mpv_readahead_secs: Option<u64>,
    prevent_sleep_enabled: Option<bool>,
    bandwidth_limit_kbps: Option<u64>,
    adaptive_soft_limit: Option<bool>,
    soft_limit_ratio: Option<f64>,
    soft_limit_when_player_active: Option<bool>,
    soft_limit_when_minimized: Option<bool>,
    disk_check_interval_parts: Option<u32>,
    auto_resume_on_startup: Option<bool>,
    purge_days: Option<u32>,
    hwdec_method: Option<String>,
    d3d11_adapter: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawRam {
    playback_buffer_mb: Option<u64>,
    session_ttl_minutes: Option<u64>,
    gc_interval_minutes: Option<u64>,
    trash_ttl_days: Option<i64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawStream {
    ram_pool_mb: Option<u64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawServer {
    host: Option<String>,
    port: Option<u16>,
    log_level: Option<String>,
    keep_alive_s: Option<u64>,
    max_concurrency: Option<usize>,
    auto_sync_interval_s: Option<u64>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawData {
    history_file: Option<String>,
    folders_file: Option<String>,
    sessions_file: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
struct RawStartup {
    auto_sync: Option<bool>,
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
    stream: RawStream,
    #[serde(default)]
    server: RawServer,
    #[serde(default)]
    data: RawData,
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
    let stream = &r.stream;
    let s = &r.server;
    let dt = &r.data;
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
    let prefetch_concurrency = clamp!(d.prefetch_concurrency, 2, 1, 4);
    let prefetch_chunks = clamp!(d.prefetch_chunks, 5, 0, 20);
    let prefetch_debounce_ms = clamp!(d.prefetch_debounce_ms, 50, 0, 2000);
    let mpv_cache_secs = clamp!(d.mpv_cache_secs, 15, 1, 120);
    let mpv_demuxer_max_mb = clamp!(d.mpv_demuxer_max_mb, 64, 50, 2000);
    let mpv_readahead_secs = clamp!(d.mpv_readahead_secs, 15, 1, 120);
    let prevent_sleep_enabled = d.prevent_sleep_enabled.unwrap_or(true);
    let bandwidth_limit_kbps = clamp!(d.bandwidth_limit_kbps, 0, 0, 2_000_000);
    let adaptive_soft_limit = d.adaptive_soft_limit.unwrap_or(true);
    let soft_limit_ratio = d
        .soft_limit_ratio
        .filter(|v| (0.5..=0.95).contains(v))
        .unwrap_or(0.8);
    let soft_limit_when_player_active = d.soft_limit_when_player_active.unwrap_or(true);
    let soft_limit_when_minimized = d.soft_limit_when_minimized.unwrap_or(true);
    let disk_check_interval_parts = clamp!(d.disk_check_interval_parts, 5, 1, 50);
    let auto_resume_on_startup = d.auto_resume_on_startup.unwrap_or(true);
    let purge_days = clamp!(d.purge_days, 7, 1, 30);

    let d3d11_adapter = d
        .d3d11_adapter
        .clone()
        .unwrap_or_else(|| "Auto".to_string());

    let playback_buffer_mb = clamp!(m.playback_buffer_mb, 256, 64, 4096);
    let stream_ram_pool_mb = clamp!(stream.ram_pool_mb, playback_buffer_mb, 128, 2048);
    let session_ttl_minutes = clamp!(m.session_ttl_minutes, 60, 5, 1440);
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
        prefetch_concurrency,
        prefetch_chunks,
        prefetch_debounce_ms,
        mpv_cache_secs,
        mpv_demuxer_max_mb,
        mpv_readahead_secs,
        prevent_sleep_enabled,
        bandwidth_limit_kbps,
        adaptive_soft_limit,
        soft_limit_ratio,
        soft_limit_when_player_active,
        soft_limit_when_minimized,
        disk_check_interval_parts,
        auto_resume_on_startup,
        purge_days,

        playback_buffer_ram_bytes: stream_ram_pool_mb * 1024 * 1024,
        session_ttl_s: session_ttl_minutes * 60,
        gc_interval_s: gc_interval_minutes * 60,
        trash_ttl_days,

        host: s.host.clone().unwrap_or_else(|| "0.0.0.0".to_string()),
        port: s.port.unwrap_or(8000),
        log_level,
        keep_alive_s: clamp!(s.keep_alive_s, 600, 10, 3600),
        max_concurrency: clamp!(s.max_concurrency, 5, 1, 100),
        auto_sync_interval_s: clamp!(s.auto_sync_interval_s, 10, 5, 3600),

        history_file: dt
            .history_file
            .clone()
            .unwrap_or_else(|| "file_history.json".to_string()),
        folders_file: dt
            .folders_file
            .clone()
            .unwrap_or_else(|| "folders.json".to_string()),
        sessions_file: dt
            .sessions_file
            .clone()
            .unwrap_or_else(|| "upload_sessions.json".to_string()),

        auto_sync_on_startup: startup.auto_sync.unwrap_or(false),
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

    let raw = RawConfig {
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
            prefetch_concurrency: Some(config.prefetch_concurrency),
            prefetch_chunks: Some(config.prefetch_chunks),
            prefetch_debounce_ms: Some(config.prefetch_debounce_ms),
            mpv_cache_secs: Some(config.mpv_cache_secs),
            mpv_demuxer_max_mb: Some(config.mpv_demuxer_max_mb),
            mpv_readahead_secs: Some(config.mpv_readahead_secs),
            prevent_sleep_enabled: Some(config.prevent_sleep_enabled),
            bandwidth_limit_kbps: Some(config.bandwidth_limit_kbps),
            adaptive_soft_limit: Some(config.adaptive_soft_limit),
            soft_limit_ratio: Some(config.soft_limit_ratio),
            soft_limit_when_player_active: Some(config.soft_limit_when_player_active),
            soft_limit_when_minimized: Some(config.soft_limit_when_minimized),
            disk_check_interval_parts: Some(config.disk_check_interval_parts),
            auto_resume_on_startup: Some(config.auto_resume_on_startup),
            purge_days: Some(config.purge_days),
            d3d11_adapter: Some(config.d3d11_adapter.clone()),
            ..Default::default()
        },
        ram: RawRam {
            playback_buffer_mb: Some(config.playback_buffer_ram_bytes / 1024 / 1024),
            session_ttl_minutes: Some(config.session_ttl_s / 60),
            gc_interval_minutes: Some(config.gc_interval_s / 60),
            trash_ttl_days: Some(config.trash_ttl_days),
        },
        stream: RawStream {
            ram_pool_mb: Some(config.playback_buffer_ram_bytes / 1024 / 1024),
        },
        server: RawServer {
            host: Some(config.host.clone()),
            port: Some(config.port),
            log_level: Some(config.log_level.clone()),
            keep_alive_s: Some(config.keep_alive_s),
            max_concurrency: Some(config.max_concurrency),
            auto_sync_interval_s: Some(config.auto_sync_interval_s),
        },
        data: RawData {
            history_file: Some(config.history_file.clone()),
            folders_file: Some(config.folders_file.clone()),
            sessions_file: Some(config.sessions_file.clone()),
        },
        providers,
        logging: RawLogging {
            feature_enabled: config.logging.feature_enabled.clone(),
            frontend_enabled: Some(config.logging.frontend_enabled),
        },
        startup: RawStartup {
            auto_sync: Some(config.auto_sync_on_startup),
            persistent_video_bridge: Some(config.persistent_video_bridge),
        },
        backup: RawBackup {
            enabled: Some(config.backup_enabled),
            snapshot_interval_days: Some(config.backup_snapshot_interval_days),
        },
    };

    let json = serde_json::to_string_pretty(&raw)?;
    std::fs::write(path, json)?;
    Ok(())
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
        " RAM      : session_ttl={}min gc={}min",
        config.session_ttl_s / 60,
        config.gc_interval_s / 60
    );
    println!(
        " Server   : {}:{} log={} max_concurrency={}",
        config.host, config.port, config.log_level, config.max_concurrency
    );
    println!(
        " Backup   : {} snapshot={} days",
        if config.backup_enabled { "enabled" } else { "disabled" },
        config.backup_snapshot_interval_days
    );
    println!("{}", "".repeat(60));
}
