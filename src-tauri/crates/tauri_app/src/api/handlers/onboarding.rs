use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    api::handlers::tenants::{
        build_tenant_summary, map_tenant_error, rebind_runtime_to_tenant, ActiveTenantsState,
    },
    app_runtime::AppState,
    core::{
        error::{wrap_error, AppResult},
        error_codes as codes,
        tenant::{TenantDescriptor, TENANT_SCOPE_MY, TENANT_SCOPE_SHARED},
        tenant_registry::{discover_tenants, load_tenant_registry, save_tenant_registry, TenantRegistryFile},
    },
    db::{tenant_meta, Db},
    providers::{discord_auth, telegram_auth},
    providers::install::render_builtin_bot_env_template,
    providers::telegram_session::{legacy_telegram_session_path, telegram_session_path},
};

use omega_drive_gateway::core::types::ReachableDestination;

const BOT_ENV_FILE_NAME: &str = "bot.env";
const DISCORD_TOKEN_KEY: &str = "DISCORD_TOKEN";
const TELEGRAM_PHONE_KEY: &str = "TELEGRAM_PHONE";
const TELEGRAM_API_ID_KEY: &str = "TELEGRAM_API_ID";
const TELEGRAM_API_HASH_KEY: &str = "TELEGRAM_API_HASH";
const DESTINATION_FETCH_MIN_INTERVAL: Duration = Duration::from_secs(30);



struct DestinationCache {
    discord_token: String,
    telegram_cache_key: String,
    discord_guilds: Vec<ReachableDestination>,
    telegram_groups: Vec<ReachableDestination>,
    discord_error: Option<String>,
    telegram_error: Option<String>,
    telegram_authorized: bool,
    fetched_at: Option<Instant>,
    is_fetching: bool,
}

static DESTINATION_CACHE: tokio::sync::Mutex<DestinationCache> =
    tokio::sync::Mutex::const_new(DestinationCache {
        discord_token: String::new(),
        telegram_cache_key: String::new(),
        discord_guilds: Vec::new(),
        telegram_groups: Vec::new(),
        discord_error: None,
        telegram_error: None,
        telegram_authorized: false,
        fetched_at: None,
        is_fetching: false,
    });



#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingScopeState {
    pub active: Option<TenantDescriptor>,
    pub valid_tenants: Vec<TenantDescriptor>,
    pub invalid_tenants: Vec<TenantDescriptor>,
    pub needs_selection: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingTenantsState {
    pub my: OnboardingScopeState,
    pub shared: OnboardingScopeState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    pub discord_token_present: bool,
    pub telegram_credentials_present: bool,
    pub telegram_authorized: bool,
    pub destinations_loaded: bool,
    pub requires_onboarding: bool,
    pub can_skip: bool,
    pub telegram_login_step: String,
    pub telegram_password_hint: Option<String>,
    pub discord_error: Option<String>,
    pub telegram_error: Option<String>,
    pub discord_guilds: Vec<ReachableDestination>,
    pub telegram_groups: Vec<ReachableDestination>,
    pub tenants: OnboardingTenantsState,
    pub active_tenants: ActiveTenantsState,
    pub discord_token: Option<String>,
    pub telegram_phone: Option<String>,
    pub telegram_api_id: Option<i32>,
    pub telegram_api_hash: Option<String>,
}

#[derive(Clone)]
struct TelegramCredentials {
    phone: String,
    api_id: i32,
    api_hash: String,
}

struct ReachableContext {
    discord_token_present: bool,
    telegram_credentials_present: bool,
    telegram_authorized: bool,
    destinations_loaded: bool,
    discord_error: Option<String>,
    telegram_error: Option<String>,
    discord_guilds: Vec<ReachableDestination>,
    telegram_groups: Vec<ReachableDestination>,
}

fn onboarding_context(action: &str, extra: Value) -> Value {
    let mut context = serde_json::Map::from_iter([
        ("feature".to_string(), json!("onboarding")),
        ("action".to_string(), json!(action)),
    ]);

    if let Value::Object(extra) = extra {
        context.extend(extra);
    }

    Value::Object(context)
}

fn current_tenant_descriptor(state: &AppState) -> AppResult<TenantDescriptor> {
    state
        .active_tenant
        .lock()
        .map(|tenant| tenant.clone())
        .map_err(|err| {
            wrap_error(
                "onboarding",
                codes::E_UNKNOWN,
                "Khong the doc tenant hien tai.",
                onboarding_context("current_tenant", json!({})),
                anyhow!(err.to_string()),
            )
        })
}

fn bot_env_path(state: &AppState) -> PathBuf {
    state.base_dir.join(BOT_ENV_FILE_NAME)
}

fn ensure_bot_env_exists(state: &AppState) -> AppResult<PathBuf> {
    let path = bot_env_path(state);
    if !path.exists() {
        std::fs::write(&path, render_builtin_bot_env_template()).map_err(|err| {
            wrap_error(
                "onboarding",
                codes::E_IO,
                "Khong the tao bot.env.",
                onboarding_context(
                    "ensure_bot_env",
                    json!({ "path": path.display().to_string() }),
                ),
                err,
            )
        })?;
    }
    Ok(path)
}

fn normalize_env_value(value: &str) -> String {
    value.trim().to_string()
}

fn update_bot_env(state: &AppState, updates: &[(&str, String)]) -> AppResult<()> {
    let path = ensure_bot_env_exists(state)?;
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = if existing.is_empty() {
        render_builtin_bot_env_template()
            .lines()
            .map(str::to_string)
            .collect()
    } else {
        existing.lines().map(str::to_string).collect()
    };

    let update_map: HashMap<&str, String> = updates
        .iter()
        .map(|(key, value)| (*key, value.clone()))
        .collect();
    let mut seen = HashSet::new();

    for line in &mut lines {
        let trimmed = line.trim_start().to_string();
        for (key, value) in &update_map {
            if trimmed.starts_with(&format!("{key}=")) {
                *line = format!("{key}={value}");
                seen.insert(*key);
            }
        }
    }

    for (key, value) in &update_map {
        if !seen.contains(key) {
            lines.push(format!("{key}={value}"));
        }
    }

    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }

    std::fs::write(&path, content).map_err(|err| {
        wrap_error(
            "onboarding",
            codes::E_IO,
            "Khong the cap nhat bot.env.",
            onboarding_context(
                "update_bot_env",
                json!({ "path": path.display().to_string() }),
            ),
            err,
        )
    })?;

    for (key, value) in &update_map {
        let normalized = normalize_env_value(value);
        if normalized.is_empty() {
            std::env::remove_var(key);
        } else {
            std::env::set_var(key, normalized);
        }
    }

    Ok(())
}

fn read_discord_token() -> String {
    std::env::var(DISCORD_TOKEN_KEY)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn read_telegram_credentials() -> Option<TelegramCredentials> {
    let phone = std::env::var(TELEGRAM_PHONE_KEY).unwrap_or_default();
    let api_id = std::env::var(TELEGRAM_API_ID_KEY)
        .unwrap_or_default()
        .parse::<i32>()
        .unwrap_or(0);
    let api_hash = std::env::var(TELEGRAM_API_HASH_KEY).unwrap_or_default();

    if phone.trim().is_empty() || api_id <= 0 || api_hash.trim().is_empty() {
        return None;
    }

    Some(TelegramCredentials {
        phone: phone.trim().to_string(),
        api_id,
        api_hash: api_hash.trim().to_string(),
    })
}

fn normalize_optional_segment(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "0")
}

#[cfg(feature = "telegram")]
fn telegram_reachable_cache_key(base_dir: &Path, creds: &TelegramCredentials) -> String {
    let session_path = telegram_session_path(base_dir);
    format!(
        "{}|{}|{}",
        session_path.display(),
        creds.phone.trim(),
        creds.api_id
    )
}

#[cfg(feature = "telegram")]
async fn invalidate_telegram_reachable_groups_cache(_base_dir: &std::path::Path) {
    let mut cache = DESTINATION_CACHE.lock().await;
    cache.telegram_groups.clear();
    cache.telegram_error = None;
    cache.telegram_authorized = false;
    cache.telegram_cache_key = String::new();
    // Reset fetched_at so the next poll triggers a fresh network fetch
    cache.fetched_at = None;
}

#[cfg(not(feature = "telegram"))]
async fn invalidate_telegram_reachable_groups_cache(_base_dir: &std::path::Path) {
    let mut cache = DESTINATION_CACHE.lock().await;
    cache.telegram_groups.clear();
    cache.telegram_authorized = false;
    cache.telegram_cache_key = String::new();
    cache.fetched_at = None;
}

async fn trigger_destinations_fetch(base_dir: std::path::PathBuf, force_refresh: bool) {
    let discord_token = read_discord_token();
    let telegram_credentials = read_telegram_credentials();

    #[cfg(feature = "telegram")]
    let tg_key = telegram_credentials
        .as_ref()
        .map(|c| telegram_reachable_cache_key(&base_dir, c))
        .unwrap_or_default();
    #[cfg(not(feature = "telegram"))]
    let tg_key = String::new();

    {
        let mut cache = DESTINATION_CACHE.lock().await;

        // Invalidate if credentials changed
        if cache.discord_token != discord_token || cache.telegram_cache_key != tg_key {
            cache.discord_guilds.clear();
            cache.telegram_groups.clear();
            cache.discord_error = None;
            cache.telegram_error = None;
            cache.telegram_authorized = false;
            cache.fetched_at = None;
            cache.discord_token = discord_token.clone();
            cache.telegram_cache_key = tg_key.clone();
        }

        if cache.is_fetching {
            return;
        }

        let stale = match cache.fetched_at {
            None => true,
            Some(t) => force_refresh || t.elapsed() > DESTINATION_FETCH_MIN_INTERVAL,
        };

        if !stale {
            return;
        }

        cache.is_fetching = true;
    }

    tokio::spawn(async move {
        let mut discord_guilds: Vec<ReachableDestination> = Vec::new();
        let mut discord_error: Option<String> = None;
        let mut telegram_groups: Vec<ReachableDestination> = Vec::new();
        let mut telegram_error: Option<String> = None;
        let mut telegram_authorized = false;

        if !discord_token.is_empty() {
            match list_reachable_discord_guilds(&discord_token).await {
                Ok(guilds) => { discord_guilds = guilds; }
                Err(err) => { discord_error = Some(err.to_string()); }
            }
        }

        if let Some(creds) = telegram_credentials {
            match telegram_auth::list_reachable_groups(
                &base_dir, creds.api_id, &creds.api_hash, &creds.phone,
            ).await {
                Ok(groups) => {
                    telegram_authorized = true;
                    telegram_groups = groups;
                }
                Err(err) => { telegram_error = Some(err.to_string()); }
            }
        }

        let mut cache = DESTINATION_CACHE.lock().await;
        // Only update if credentials haven't changed while we were fetching
        if cache.discord_token == discord_token && cache.telegram_cache_key == tg_key {
            cache.discord_guilds = discord_guilds;
            cache.telegram_groups = telegram_groups;
            cache.discord_error = discord_error;
            cache.telegram_error = telegram_error;
            cache.telegram_authorized = telegram_authorized;
            cache.fetched_at = Some(Instant::now());
        }
        cache.is_fetching = false;
    });
}

async fn list_reachable_discord_guilds(
    token: &str,
) -> Result<Vec<ReachableDestination>, anyhow::Error> {
    discord_auth::list_reachable_discord_guilds(token).await
}

async fn validate_discord_token(token: &str) -> Result<(), anyhow::Error> {
    discord_auth::validate_discord_token(token).await
}


fn scope_label(scope: &str) -> &'static str {
    if scope.eq_ignore_ascii_case(TENANT_SCOPE_SHARED) {
        TENANT_SCOPE_SHARED
    } else {
        TENANT_SCOPE_MY
    }
}

fn choose_scope_active(
    registry: &TenantRegistryFile,
    scope: &str,
    valid_tenants: &[TenantDescriptor],
) -> Option<TenantDescriptor> {
    if let Some(remembered) = registry.active_tenant(scope) {
        if valid_tenants.iter().any(|tenant| tenant == &remembered) {
            return Some(remembered);
        }
    }

    valid_tenants.first().cloned()
}

fn tenant_is_valid(
    tenant: &TenantDescriptor,
    discord_ids: &HashSet<String>,
    telegram_ids: &HashSet<String>,
    check_discord: bool,
    check_telegram: bool,
    destinations_loaded: bool,
) -> bool {
    let discord_ok = if check_discord {
        tenant
            .discord_guild_id
            .as_ref()
            .map(|id| {
                if destinations_loaded {
                    discord_ids.contains(id)
                } else {
                    true
                }
            })
            .unwrap_or(false)
    } else {
        true
    };
    let telegram_ok = if check_telegram {
        tenant
            .telegram_group_id
            .as_ref()
            .map(|id| {
                if destinations_loaded {
                    telegram_ids.contains(id)
                } else {
                    true
                }
            })
            .unwrap_or(false)
    } else {
        true
    };

    discord_ok && telegram_ok
}

async fn collect_reachable_context(state: &AppState, force_refresh: bool) -> ReachableContext {
    let discord_token = read_discord_token();
    let discord_token_present = !discord_token.is_empty();
    let telegram_credentials = read_telegram_credentials();
    let telegram_credentials_present = telegram_credentials.is_some();

    // Trigger background fetch (non-blocking â€” returns immediately)
    trigger_destinations_fetch(state.base_dir.clone(), force_refresh).await;

    // Read from RAM cache immediately
    let cache = DESTINATION_CACHE.lock().await;
    let destinations_loaded = cache.fetched_at.is_some();

    ReachableContext {
        discord_token_present,
        telegram_credentials_present,
        telegram_authorized: cache.telegram_authorized,
        destinations_loaded,
        discord_error: cache.discord_error.clone(),
        telegram_error: cache.telegram_error.clone(),
        discord_guilds: cache.discord_guilds.clone(),
        telegram_groups: cache.telegram_groups.clone(),
    }
}

async fn resolve_onboarding_state_inner(
    state: &AppState,
    allow_rebind: bool,
    force_refresh: bool,
) -> AppResult<OnboardingState> {
    let reachable = collect_reachable_context(state, force_refresh).await;
    let discovered = discover_tenants(&state.base_dir);
    let discord_ids: HashSet<String> = reachable
        .discord_guilds
        .iter()
        .map(|guild| guild.id.clone())
        .collect();
    let telegram_ids: HashSet<String> = reachable
        .telegram_groups
        .iter()
        .map(|group| group.id.clone())
        .collect();

    let mut my_valid = Vec::new();
    let mut my_invalid = Vec::new();
    let mut shared_valid = Vec::new();
    let mut shared_invalid = Vec::new();

    let check_discord = reachable.discord_token_present && reachable.discord_error.is_none();
    let check_telegram = reachable.telegram_authorized;

    for tenant in discovered {
        let target = if tenant.scope == TENANT_SCOPE_SHARED {
            (&mut shared_valid, &mut shared_invalid)
        } else {
            (&mut my_valid, &mut my_invalid)
        };

        if tenant_is_valid(&tenant, &discord_ids, &telegram_ids, check_discord, check_telegram, reachable.destinations_loaded) {
            target.0.push(tenant);
        } else {
            target.1.push(tenant);
        }
    }

    let current = current_tenant_descriptor(state)?;
    let mut registry = load_tenant_registry(&state.base_dir);
    let resolved_my = choose_scope_active(&registry, TENANT_SCOPE_MY, &my_valid);
    let resolved_shared = choose_scope_active(&registry, TENANT_SCOPE_SHARED, &shared_valid);

    registry.set_active_db_file(
        TENANT_SCOPE_MY,
        resolved_my.as_ref().map(TenantDescriptor::db_file_name),
    );
    registry.set_active_db_file(
        TENANT_SCOPE_SHARED,
        resolved_shared.as_ref().map(TenantDescriptor::db_file_name),
    );
    save_tenant_registry(&registry, &state.base_dir).map_err(|err| {
        wrap_error(
            "onboarding",
            codes::E_IO,
            "Khong the cap nhat tenant registry.",
            onboarding_context("resolve_onboarding_state", json!({})),
            err,
        )
    })?;

    let scope_to_keep = scope_label(&current.scope);
    let desired_current = match scope_to_keep {
        TENANT_SCOPE_SHARED => resolved_shared.clone(),
        _ => resolved_my.clone(),
    };

    let current_tenant = if allow_rebind {
        if let Some(desired) = desired_current {
            if desired != current {
                rebind_runtime_to_tenant(state, desired.clone()).await?;
                desired
            } else {
                current
            }
        } else {
            current
        }
    } else {
        current
    };

    let active_tenants = ActiveTenantsState {
        my: resolved_my
            .clone()
            .map(|tenant| build_tenant_summary(&state.base_dir, tenant)),
        shared: resolved_shared
            .clone()
            .map(|tenant| build_tenant_summary(&state.base_dir, tenant)),
        current: build_tenant_summary(&state.base_dir, current_tenant),
    };
    let discord_token_raw = read_discord_token();
    let discord_token = (!discord_token_raw.is_empty()).then_some(discord_token_raw);
    let (telegram_phone, telegram_api_id, telegram_api_hash) = match read_telegram_credentials() {
        Some(creds) => (Some(creds.phone), Some(creds.api_id), Some(creds.api_hash)),
        None => (None, None, None),
    };
    let (telegram_login_step, telegram_password_hint) = telegram_auth::current_step().await;
    let needs_my_selection = active_tenants.my.is_none();
    let needs_shared_selection = active_tenants.shared.is_none();
    let current_scope_needs_selection = match scope_to_keep {
        TENANT_SCOPE_SHARED => needs_shared_selection,
        _ => needs_my_selection,
    };
    let requires_onboarding = !reachable.discord_token_present
        || !reachable.telegram_authorized
        || current_scope_needs_selection
        || reachable.discord_error.is_some()
        || reachable.telegram_error.is_some();

    Ok(OnboardingState {
        discord_token_present: reachable.discord_token_present,
        telegram_credentials_present: reachable.telegram_credentials_present,
        telegram_authorized: reachable.telegram_authorized,
        destinations_loaded: reachable.destinations_loaded,
        requires_onboarding,
        can_skip: true,
        telegram_login_step,
        telegram_password_hint,
        discord_error: reachable.discord_error,
        telegram_error: reachable.telegram_error,
        discord_guilds: reachable.discord_guilds,
        telegram_groups: reachable.telegram_groups,
        tenants: OnboardingTenantsState {
            my: OnboardingScopeState {
                active: resolved_my.clone(),
                valid_tenants: my_valid,
                invalid_tenants: my_invalid,
                needs_selection: needs_my_selection,
            },
            shared: OnboardingScopeState {
                active: resolved_shared.clone(),
                valid_tenants: shared_valid,
                invalid_tenants: shared_invalid,
                needs_selection: needs_shared_selection,
            },
        },
        active_tenants,
        discord_token,
        telegram_phone,
        telegram_api_id,
        telegram_api_hash,
    })
}

async fn refresh_current_runtime(state: &AppState) -> AppResult<()> {
    let current = current_tenant_descriptor(state)?;
    rebind_runtime_to_tenant(state, current).await.map(|_| ())
}

fn validate_scope(scope: &str) -> AppResult<String> {
    let normalized = scope_label(scope).to_string();
    if normalized == TENANT_SCOPE_MY || normalized == TENANT_SCOPE_SHARED {
        Ok(normalized)
    } else {
        Err(wrap_error(
            "onboarding",
            codes::E_INVALID_INPUT,
            "Scope khong hop le.",
            onboarding_context("validate_scope", json!({ "scope": scope })),
            anyhow!("invalid scope"),
        ))
    }
}

#[tauri::command]
pub async fn get_onboarding_state(st: tauri::State<'_, AppState>) -> AppResult<OnboardingState> {
    resolve_onboarding_state_inner(st.inner(), true, false).await
}

#[tauri::command]
pub async fn load_onboarding_destinations(
    st: tauri::State<'_, AppState>,
) -> AppResult<OnboardingState> {
    resolve_onboarding_state_inner(st.inner(), true, true).await
}

#[tauri::command]
pub async fn save_discord_token(
    st: tauri::State<'_, AppState>,
    token: String,
) -> AppResult<OnboardingState> {
    let trimmed = token.trim().to_string();
    if !trimmed.is_empty() {
        validate_discord_token(&trimmed)
            .await
            .map_err(|err| {
                wrap_error(
                    "onboarding",
                    codes::E_NETWORK,
                    "Discord token khong hop le hoac bot khong the xac thuc.",
                    onboarding_context("save_discord_token", json!({})),
                    err,
                )
            })?;
    }

    update_bot_env(st.inner(), &[(DISCORD_TOKEN_KEY, trimmed)])?;
    refresh_current_runtime(st.inner()).await?;
    resolve_onboarding_state_inner(st.inner(), true, true).await
}

#[tauri::command]
pub async fn save_telegram_credentials(
    st: tauri::State<'_, AppState>,
    phone: String,
    api_id: i32,
    api_hash: String,
) -> AppResult<OnboardingState> {
    let phone = phone.trim().to_string();
    let api_hash = api_hash.trim().to_string();
    if phone.is_empty() || api_id <= 0 || api_hash.is_empty() {
        return Err(wrap_error(
            "onboarding",
            codes::E_INVALID_INPUT,
            "Thong tin Telegram chua day du.",
            onboarding_context("save_telegram_credentials", json!({})),
            anyhow!("missing telegram credentials"),
        ));
    }

    let previous = read_telegram_credentials();
    let credentials_changed = previous
        .as_ref()
        .map(|creds| creds.phone != phone || creds.api_id != api_id || creds.api_hash != api_hash)
        .unwrap_or(true);

    update_bot_env(
        st.inner(),
        &[
            (TELEGRAM_PHONE_KEY, phone.clone()),
            (TELEGRAM_API_ID_KEY, api_id.to_string()),
            (TELEGRAM_API_HASH_KEY, api_hash.clone()),
        ],
    )?;

    #[cfg(feature = "telegram")]
    {
        telegram_auth::reset_state().await;
        invalidate_telegram_reachable_groups_cache(&st.base_dir).await;
        if credentials_changed {
            let session_path = telegram_session_path(&st.base_dir);
            let legacy_path = legacy_telegram_session_path(&st.base_dir);
            if session_path.exists() {
                let _ = tokio::fs::remove_file(&session_path).await;
            }
            if legacy_path.exists() {
                let _ = tokio::fs::remove_file(legacy_path).await;
            }
        }
    }

    refresh_current_runtime(st.inner()).await?;
    resolve_onboarding_state_inner(st.inner(), true, true).await
}

#[tauri::command]
pub async fn send_telegram_login_code(
    st: tauri::State<'_, AppState>,
) -> AppResult<OnboardingState> {
    #[cfg(not(feature = "telegram"))]
    {
        return Err(wrap_error(
            "onboarding",
            codes::E_UNAVAILABLE,
            "Telegram feature is disabled.",
            onboarding_context("send_telegram_login_code", json!({})),
            anyhow!("telegram feature disabled"),
        ));
    }

    #[cfg(feature = "telegram")]
    {
        let creds = read_telegram_credentials().ok_or_else(|| {
            wrap_error(
                "onboarding",
                codes::E_INVALID_INPUT,
                "Hay luu thong tin Telegram truoc khi gui ma dang nhap.",
                onboarding_context("send_telegram_login_code", json!({})),
                anyhow!("telegram credentials missing"),
            )
        })?;

        let already_authorized = telegram_auth::start_login(
            &st.base_dir, creds.api_id, &creds.api_hash, &creds.phone,
        ).await.map_err(|err| {
            wrap_error(
                "onboarding",
                codes::E_NETWORK,
                "Khong the ket noi Telegram MTProto.",
                onboarding_context("send_telegram_login_code", json!({})),
                err,
            )
        })?;

        if already_authorized {
            invalidate_telegram_reachable_groups_cache(&st.base_dir).await;
            refresh_current_runtime(st.inner()).await?;
            return resolve_onboarding_state_inner(st.inner(), true, true).await;
        }

        resolve_onboarding_state_inner(st.inner(), true, false).await
    }
}

#[tauri::command]
pub async fn submit_telegram_login_code(
    st: tauri::State<'_, AppState>,
    code: String,
) -> AppResult<OnboardingState> {
    #[cfg(not(feature = "telegram"))]
    {
        return Err(wrap_error(
            "onboarding",
            codes::E_UNAVAILABLE,
            "Telegram feature is disabled.",
            onboarding_context("submit_telegram_login_code", json!({})),
            anyhow!("telegram feature disabled"),
        ));
    }

    #[cfg(feature = "telegram")]
    {
        let trimmed_code = code.trim().to_string();
        if trimmed_code.is_empty() {
            return Err(wrap_error(
                "onboarding",
                codes::E_INVALID_INPUT,
                "Ma dang nhap Telegram khong duoc de trong.",
                onboarding_context("submit_telegram_login_code", json!({})),
                anyhow!("telegram code missing"),
            ));
        }

        let need_password = telegram_auth::submit_code(&trimmed_code).await.map_err(|err| {
            wrap_error(
                "onboarding",
                codes::E_INVALID_INPUT,
                "Dang nhap Telegram that bai.",
                onboarding_context("submit_telegram_login_code", json!({})),
                err,
            )
        })?;

        if need_password {
            Ok(resolve_onboarding_state_inner(st.inner(), true, false).await?)
        } else {
            invalidate_telegram_reachable_groups_cache(&st.base_dir).await;
            refresh_current_runtime(st.inner()).await?;
            resolve_onboarding_state_inner(st.inner(), true, true).await
        }
    }
}

#[tauri::command]
pub async fn submit_telegram_password(
    st: tauri::State<'_, AppState>,
    password: String,
) -> AppResult<OnboardingState> {
    #[cfg(not(feature = "telegram"))]
    {
        return Err(wrap_error(
            "onboarding",
            codes::E_UNAVAILABLE,
            "Telegram feature is disabled.",
            onboarding_context("submit_telegram_password", json!({})),
            anyhow!("telegram feature disabled"),
        ));
    }

    #[cfg(feature = "telegram")]
    {
        let trimmed_password = password.trim().to_string();
        if trimmed_password.is_empty() {
            return Err(wrap_error(
                "onboarding",
                codes::E_INVALID_INPUT,
                "Mat khau 2FA Telegram khong duoc de trong.",
                onboarding_context("submit_telegram_password", json!({})),
                anyhow!("telegram password missing"),
            ));
        }

        telegram_auth::submit_password(&trimmed_password).await.map_err(|err| {
            wrap_error(
                "onboarding",
                codes::E_INVALID_INPUT,
                "Xac thuc 2FA Telegram that bai.",
                onboarding_context("submit_telegram_password", json!({})),
                err,
            )
        })?;

        invalidate_telegram_reachable_groups_cache(&st.base_dir).await;
        refresh_current_runtime(st.inner()).await?;
        resolve_onboarding_state_inner(st.inner(), true, true).await
    }
}

#[tauri::command]
pub async fn create_onboarding_tenant(
    st: tauri::State<'_, AppState>,
    scope: String,
    discord_guild_id: Option<String>,
    telegram_group_id: Option<String>,
) -> AppResult<OnboardingState> {
    let scope = validate_scope(&scope)?;
    let discord_guild_id = normalize_optional_segment(discord_guild_id);
    let telegram_group_id = normalize_optional_segment(telegram_group_id);

    if discord_guild_id.is_none() && telegram_group_id.is_none() {
        return Err(wrap_error(
            "onboarding",
            codes::E_INVALID_INPUT,
            "Can it nhat mot Discord server hoac Telegram group de tao tenant.",
            onboarding_context("create_onboarding_tenant", json!({ "scope": scope })),
            anyhow!("missing tenant destinations"),
        ));
    }

    let reachable = collect_reachable_context(st.inner(), true).await;
    let discord_ids: HashSet<String> = reachable
        .discord_guilds
        .iter()
        .map(|guild| guild.id.clone())
        .collect();
    let telegram_ids: HashSet<String> = reachable
        .telegram_groups
        .iter()
        .map(|group| group.id.clone())
        .collect();

    if let Some(discord_id) = discord_guild_id.as_ref() {
        if !discord_ids.contains(discord_id) {
            return Err(wrap_error(
                "onboarding",
                codes::E_INVALID_INPUT,
                "Discord server da chon khong thuoc bot hien tai.",
                onboarding_context(
                    "create_onboarding_tenant",
                    json!({ "scope": scope, "discordGuildId": discord_id }),
                ),
                anyhow!("discord guild is not reachable by current bot token"),
            ));
        }
    }

    if let Some(telegram_id) = telegram_group_id.as_ref() {
        if !telegram_ids.contains(telegram_id) {
            return Err(wrap_error(
                "onboarding",
                codes::E_INVALID_INPUT,
                "Telegram group da chon khong thuoc session hien tai.",
                onboarding_context(
                    "create_onboarding_tenant",
                    json!({ "scope": scope, "telegramGroupId": telegram_id }),
                ),
                anyhow!("telegram group is not reachable by current session"),
            ));
        }
    }

    let tenant = TenantDescriptor::new(scope.clone(), discord_guild_id, telegram_group_id);
    let db_path = crate::core::tenant_registry::tenant_db_path(&st.base_dir, &tenant);
    let db = Db::open(&db_path).map_err(|err| {
        map_tenant_error(
            "create_onboarding_tenant",
            json!({ "tenant": tenant, "dbPath": db_path.display().to_string() }),
            err,
        )
    })?;
    tenant_meta::upsert_tenant_meta(db.conn(), &tenant).map_err(|err| {
        map_tenant_error("create_onboarding_tenant", json!({ "tenant": tenant }), err)
    })?;

    let current_scope = current_tenant_descriptor(st.inner())?.scope;
    if current_scope == scope {
        rebind_runtime_to_tenant(st.inner(), tenant).await?;
    } else {
        let mut registry = load_tenant_registry(&st.base_dir);
        registry.set_active_db_file(&scope, Some(tenant.db_file_name()));
        save_tenant_registry(&registry, &st.base_dir).map_err(|err| {
            wrap_error(
                "onboarding",
                codes::E_IO,
                "Khong the cap nhat tenant registry.",
                onboarding_context("create_onboarding_tenant", json!({ "tenant": tenant })),
                err,
            )
        })?;
    }

    resolve_onboarding_state_inner(st.inner(), true, false).await
}
