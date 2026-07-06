use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    app_runtime::AppState,
    core::{
        error::{wrap_error, AppError, AppResult},
        error_codes as codes,
        events::OmegaEvent,
        tenant::{TenantDescriptor, TENANT_SCOPE_MY, TENANT_SCOPE_SHARED},
        tenant_registry::{
            discover_tenants, persist_active_tenant, resolve_active_tenant_for_scope,
            tenant_db_path,
        },
    },
    db::tenant_meta,
    providers::install::{
        build_provider_runtime, install_builtin_providers, prepare_builtin_provider_state,
        ProviderInstallContext,
    },
};

fn tenant_context(action: &str, extra: Value) -> Value {
    let mut context = serde_json::Map::from_iter([
        ("feature".to_string(), json!("tenant")),
        ("action".to_string(), json!(action)),
    ]);

    if let Value::Object(extra) = extra {
        context.extend(extra);
    }

    Value::Object(context)
}

fn current_tenant(state: &AppState) -> AppResult<TenantDescriptor> {
    state
        .active_tenant
        .lock()
        .map(|tenant| tenant.clone())
        .map_err(|err| {
            wrap_error(
                "tenant",
                codes::E_UNKNOWN,
                "Khong the doc tenant hien tai.",
                tenant_context("get_active_tenant", json!({})),
                anyhow::anyhow!(err.to_string()),
            )
        })
}

pub(crate) fn map_tenant_error(
    action: &str,
    extra: Value,
    err: impl Into<anyhow::Error>,
) -> AppError {
    wrap_error(
        "tenant",
        codes::E_UNKNOWN,
        "Tenant operation failed.",
        tenant_context(action, extra),
        err,
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTenantsState {
    pub my: Option<TenantSummary>,
    pub shared: Option<TenantSummary>,
    pub current: TenantSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantSummary {
    pub scope: String,
    pub discord_guild_id: Option<String>,
    pub telegram_group_id: Option<String>,
    pub display_name: Option<String>,
    pub db_file_name: String,
}

impl TenantSummary {
    pub(crate) fn from_descriptor(tenant: TenantDescriptor, display_name: Option<String>) -> Self {
        Self {
            db_file_name: tenant.db_file_name(),
            scope: tenant.scope,
            discord_guild_id: tenant.discord_guild_id,
            telegram_group_id: tenant.telegram_group_id,
            display_name,
        }
    }
}

pub(crate) fn build_tenant_summary(base_dir: &Path, tenant: TenantDescriptor) -> TenantSummary {
    let db_path = tenant_db_path(base_dir, &tenant);
    let display_name = tenant_meta::read_tenant_display_name_from_db(&db_path);
    TenantSummary::from_descriptor(tenant, display_name)
}

fn open_tenant_db(base_dir: &Path, tenant: &TenantDescriptor) -> AppResult<Connection> {
    let db_path = tenant_db_path(base_dir, tenant);
    if !db_path.exists() {
        return Err(map_tenant_error(
            "rename_tenant_display_name",
            json!({ "tenant": tenant, "dbPath": db_path.display().to_string() }),
            anyhow::anyhow!("Tenant DB does not exist."),
        ));
    }

    Connection::open(&db_path).map_err(|err| {
        map_tenant_error(
            "rename_tenant_display_name",
            json!({ "tenant": tenant, "dbPath": db_path.display().to_string() }),
            err,
        )
    })
}

pub async fn rebind_runtime_to_tenant(
    state: &AppState,
    tenant: TenantDescriptor,
) -> AppResult<TenantDescriptor> {
    let tenant = TenantDescriptor::new(
        tenant.scope.clone(),
        tenant.discord_guild_id.clone(),
        tenant.telegram_group_id.clone(),
    );
    let db_path = tenant_db_path(&state.base_dir, &tenant);
    let db_path_display = db_path.display().to_string();

    state.db_write.reopen(&db_path).await.map_err(|err| {
        map_tenant_error(
            "switch_tenant",
            json!({ "tenant": tenant, "dbPath": db_path_display }),
            err,
        )
    })?;

    state.drive_db_read.reopen(&db_path).await.map_err(|err| {
        map_tenant_error(
            "switch_tenant",
            json!({ "tenant": tenant, "dbPath": db_path_display }),
            err,
        )
    })?;

    {
        let db = state.db_write.lock().await;
        tenant_meta::upsert_tenant_meta(db.conn(), &tenant).map_err(|err| {
            map_tenant_error(
                "switch_tenant",
                json!({ "tenant": tenant, "dbPath": db_path_display }),
                err,
            )
        })?;
    }

    let prepared = prepare_builtin_provider_state(&state.base_dir, &tenant)
        .await
        .map_err(|err| map_tenant_error("switch_tenant", json!({ "tenant": tenant }), err))?;
    let install_ctx = ProviderInstallContext {
        base_dir: state.base_dir.clone(),
        db_write: state.db_write.clone(),
        db_read: state.db_read.clone(),
        prepared,
        event_bus: state.events.clone(),
        tenant: tenant.clone(),
    };
    let install_results = install_builtin_providers(install_ctx)
        .await
        .map_err(|err| map_tenant_error("switch_tenant", json!({ "tenant": tenant }), err))?;
    state.replace_provider_runtime(build_provider_runtime(install_results));

    state
        .active_tenant
        .lock()
        .map(|mut current| *current = tenant.clone())
        .map_err(|err| {
            map_tenant_error(
                "switch_tenant",
                json!({ "tenant": tenant }),
                anyhow::anyhow!(err.to_string()),
            )
        })?;

    persist_active_tenant(&state.base_dir, &tenant)
        .map_err(|err| map_tenant_error("switch_tenant", json!({ "tenant": tenant }), err))?;

    state.events.emit(OmegaEvent::FilesTableChanged);
    Ok(tenant)
}

#[tauri::command]
pub async fn list_tenants(st: tauri::State<'_, AppState>) -> AppResult<Vec<TenantSummary>> {
    Ok(discover_tenants(&st.base_dir)
        .into_iter()
        .map(|tenant| build_tenant_summary(&st.base_dir, tenant))
        .collect())
}

#[tauri::command]
pub async fn get_active_tenant(st: tauri::State<'_, AppState>) -> AppResult<TenantDescriptor> {
    current_tenant(st.inner())
}

#[tauri::command]
pub async fn get_active_tenants(st: tauri::State<'_, AppState>) -> AppResult<ActiveTenantsState> {
    Ok(ActiveTenantsState {
        my: resolve_active_tenant_for_scope(&st.base_dir, TENANT_SCOPE_MY)
            .map(|tenant| build_tenant_summary(&st.base_dir, tenant)),
        shared: resolve_active_tenant_for_scope(&st.base_dir, TENANT_SCOPE_SHARED)
            .map(|tenant| build_tenant_summary(&st.base_dir, tenant)),
        current: build_tenant_summary(&st.base_dir, current_tenant(st.inner())?),
    })
}

#[tauri::command]
pub async fn switch_tenant(
    st: tauri::State<'_, AppState>,
    tenant: TenantDescriptor,
) -> AppResult<TenantDescriptor> {
    rebind_runtime_to_tenant(st.inner(), tenant).await
}

#[tauri::command]
pub async fn rename_tenant_display_name(
    st: tauri::State<'_, AppState>,
    tenant: TenantDescriptor,
    display_name: Option<String>,
) -> AppResult<TenantSummary> {
    let conn = open_tenant_db(&st.base_dir, &tenant)?;
    tenant_meta::set_tenant_display_name(&conn, display_name.as_deref()).map_err(|err| {
        map_tenant_error(
            "rename_tenant_display_name",
            json!({ "tenant": tenant.clone(), "displayName": display_name.clone() }),
            err,
        )
    })?;
    Ok(build_tenant_summary(&st.base_dir, tenant))
}
