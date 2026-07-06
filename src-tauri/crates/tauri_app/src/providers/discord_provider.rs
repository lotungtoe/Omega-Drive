use std::sync::Arc;

use anyhow::anyhow;
use omega_drive_core::tenant_registry::{persist_active_tenant, resolve_active_tenant_for_scope, tenant_db_path};
use omega_drive_db::Db;
use omega_drive_db::repos::DbFileRepository;
use omega_drive_gateway::core::{error_codes as codes, tenant::TENANT_SCOPE_SHARED};
use omega_drive_discord::discord_types::{DiscordGuildId, DiscordHttp};
use tokio::sync::OnceCell;

use crate::core::error::{wrap_error, AppResult};
use crate::providers::install::{
    ProviderBootstrapHooks, ProviderInstallContext, ProviderInstallFuture,
    ProviderInstallOutput, ProviderInstaller,
};

pub use omega_drive_discord::DiscordBackupGateway;
pub use omega_drive_discord::globals::discord_backup_gateway;
pub use omega_drive_discord::DISCORD_CONNECTED;

const DISCORD_ENV_TEMPLATE: &str = r#"# [REQUIRED] Discord Bot Token
# Get at: https://discord.com/developers/applications -> Bot -> Token
DISCORD_TOKEN=
"#;

static DISCORD_HTTP: OnceCell<Arc<DiscordHttp>> = OnceCell::const_new();

pub fn build_provider_installer() -> ProviderInstaller {
    ProviderInstaller::new(
        "discord",
        ProviderBootstrapHooks::new(Some(DISCORD_ENV_TEMPLATE), None, None),
        install_entry,
    )
}

fn install_entry(ctx: ProviderInstallContext) -> ProviderInstallFuture {
    Box::pin(async move { install_discord(ctx).await })
}

async fn install_discord(ctx: ProviderInstallContext) -> AppResult<ProviderInstallOutput> {
    let file_repo: Arc<dyn omega_drive_gateway::provider::file_repository::FileRepository> =
        Arc::new(DbFileRepository::new(Arc::clone(&ctx.db_read), Arc::clone(&ctx.db_write)));
    let input = omega_drive_discord::installer::DiscordInstallInput {
        discord_token: std::env::var("DISCORD_TOKEN").unwrap_or_default(),
        tenant_guild_id: ctx.tenant.discord_guild_id.clone(),
        tenant_scope: ctx.tenant.scope.clone(),
        file_repo,
        event_bus: Arc::clone(&ctx.event_bus),
    };
    let output = omega_drive_discord::installer::install_discord(input).await?;

    let _ = DISCORD_HTTP.set(Arc::clone(&output.http));

    Ok(ProviderInstallOutput {
        storage_providers: output.storage_providers,
        provider_admin_gateways: output.provider_admin_gateways,
        part_store_gateways: output.part_store_gateways,
        stream_gateways: output.stream_gateways,
        remote_folder_gateways: output.remote_folder_gateways,
        remote_object_gateways: output.remote_object_gateways,
    })
}

#[derive(serde::Serialize)]
pub struct SharedDriveStatus {
    pub is_configured: bool,
    pub guild_id: Option<String>,
    pub main_discord_connected: bool,
}

pub async fn check_shared_drive_status_internal(
    st: tauri::State<'_, crate::app_wiring::app_runtime::AppState>,
) -> AppResult<SharedDriveStatus> {
    let shared_tenant = resolve_active_tenant_for_scope(&st.base_dir, TENANT_SCOPE_SHARED);
    let main_connected = *DISCORD_CONNECTED.read().await;

    Ok(SharedDriveStatus {
        is_configured: shared_tenant
            .as_ref()
            .map(|tenant| tenant.discord_guild_id.is_some() || tenant.telegram_group_id.is_some())
            .unwrap_or(false),
        guild_id: shared_tenant.and_then(|tenant| tenant.discord_guild_id),
        main_discord_connected: main_connected,
    })
}

pub async fn setup_shared_drive_internal(
    st: tauri::State<'_, crate::app_wiring::app_runtime::AppState>,
    guild_id: String,
    tg_chat_id: String,
) -> AppResult<String> {
    use omega_drive_gateway::core::tenant::TenantDescriptor;

    let ctx = serde_json::json!({
        "feature": "discord_provider",
        "action": "setup_shared_drive",
    });
    let guild_id = guild_id.trim().to_string();
    let tg_chat_id = tg_chat_id.trim().to_string();

    if guild_id.is_empty() && tg_chat_id.is_empty() {
        return Err(wrap_error(
            "discord",
            codes::E_INVALID_INPUT,
            "Can it nhat mot Discord guild id hoac Telegram chat id cho Shared Drive.",
            ctx.clone(),
            anyhow!("missing shared tenant destination"),
        ));
    }

    let mut guild_name = "Shared Drive".to_string();

    if !guild_id.is_empty() {
        let guild_id_num = guild_id.trim().parse().map_err(|e| {
            wrap_error(
                "discord",
                codes::E_INVALID_INPUT,
                "Invalid Discord server ID.",
                ctx.clone(),
                anyhow!("Parse error: {}", e),
            )
        })?;

        let http = DISCORD_HTTP.get().ok_or_else(|| {
            wrap_error(
                "discord",
                codes::E_NOT_READY,
                "Discord bot is not ready. Configure the main bot first.",
                ctx.clone(),
                anyhow!("Discord HTTP client not initialized"),
            )
        })?;

        let guild_obj = http
            .get_guild(DiscordGuildId::new(guild_id_num))
            .await
            .map_err(|e| {
                wrap_error(
                    "discord",
                    codes::E_NOT_FOUND,
                    "Bot could not find that Discord server.",
                    ctx.clone(),
                    e,
                )
            })?;
        guild_name = guild_obj.name.clone();
    }

    let shared_tenant = TenantDescriptor::new(
        TENANT_SCOPE_SHARED,
        (!guild_id.is_empty()).then_some(guild_id.clone()),
        (!tg_chat_id.is_empty()).then_some(tg_chat_id.clone()),
    );
    let db_path = tenant_db_path(&st.base_dir, &shared_tenant);
    let db = Db::open(&db_path).map_err(|e| {
        wrap_error(
            "discord",
            codes::E_DB,
            "Khong the tao hoac mo DB tenant Shared Drive.",
            ctx.clone(),
            e,
        )
    })?;
    omega_drive_db::tenant_meta::upsert_tenant_meta(db.conn(), &shared_tenant).map_err(|e| {
        wrap_error(
            "discord",
            codes::E_DB,
            "Khong the luu tenant metadata cho Shared Drive.",
            ctx.clone(),
            e,
        )
    })?;

    let active_scope = st
        .active_tenant
        .lock()
        .map(|tenant| tenant.scope.clone())
        .unwrap_or_else(|_| "my".to_string());

    if active_scope == TENANT_SCOPE_SHARED {
        crate::api::handlers::rebind_runtime_to_tenant(st.inner(), shared_tenant.clone())
            .await
            .map_err(|e| {
                wrap_error(
                    "discord",
                    codes::E_UNAVAILABLE,
                    "Khong the rebind runtime sang Shared Drive moi.",
                    ctx.clone(),
                    anyhow!(e.to_string()),
                )
            })?;
    } else {
        persist_active_tenant(&st.base_dir, &shared_tenant).map_err(|e| {
            wrap_error(
                "discord",
                codes::E_IO,
                "Khong the cap nhat tenant registry cho Shared Drive.",
                ctx.clone(),
                e,
            )
        })?;
    }

    Ok(guild_name)
}

pub async fn forward_file_to_shared_internal(
    st: tauri::State<'_, crate::app_wiring::app_runtime::AppState>,
    file_id: i64,
) -> AppResult<()> {
    let active_scope = st
        .active_tenant
        .lock()
        .map(|tenant| tenant.scope.clone())
        .unwrap_or_else(|_| "my".to_string());

    if active_scope == "shared" {
        return Ok(());
    }

    Err(wrap_error(
        "move",
        codes::E_UNAVAILABLE,
        "Forward sang Shared Drive da bi vo hieu hoa trong che do multi-DB. Hay chuyen sang tenant Shared roi tai lai file o do.",
        serde_json::json!({ "file_id": file_id, "active_scope": active_scope }),
        anyhow!("forward_file_to_shared is not supported after tenant split"),
    ))
}
