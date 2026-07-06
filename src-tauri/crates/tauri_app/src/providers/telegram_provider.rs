use std::{path::Path, sync::Arc, time::Duration};

use omega_drive_db::repos::DbFileRepository;
use omega_drive_gateway::provider::file_repository::FileRepository;
use omega_drive_telegram::installer::TelegramInstallInput;
use omega_drive_telegram::telegram_real::TelegramClient;

use crate::core::error::AppResult;
use crate::providers::install::{
    ProviderBootstrapHooks, ProviderInstallContext, ProviderInstallFuture,
    ProviderInstallOutput, ProviderInstaller,
};

const TELEGRAM_ENV_TEMPLATE: &str = r#"# [TUY CHON] Telegram MTProto - de trong neu khong dung
TELEGRAM_PHONE=
TELEGRAM_API_ID=
TELEGRAM_API_HASH=
"#;

pub fn build_provider_installer() -> ProviderInstaller {
    ProviderInstaller::new(
        "telegram",
        ProviderBootstrapHooks::new(Some(TELEGRAM_ENV_TEMPLATE), Some(cleanup_temp_files), None),
        install_entry,
    )
}

fn install_entry(ctx: ProviderInstallContext) -> ProviderInstallFuture {
    Box::pin(async move { install_telegram(ctx).await })
}

async fn install_telegram(ctx: ProviderInstallContext) -> AppResult<ProviderInstallOutput> {
    #[cfg(feature = "telegram")]
    {
        let configured = ctx.tenant.telegram_group_id.is_some() && has_telegram_env();
        let client = connect_telegram(&ctx.base_dir, ctx.tenant.telegram_group_id.as_deref()).await?;
        let file_repo: Arc<dyn FileRepository> =
            Arc::new(DbFileRepository::new(Arc::clone(&ctx.db_read), Arc::clone(&ctx.db_write)));
        let input = TelegramInstallInput {
            base_dir: ctx.base_dir.clone(),
            configured,
            client,
            file_repo,
            event_bus: Arc::clone(&ctx.event_bus),
        };
        let output = omega_drive_telegram::installer::install_telegram(input).await?;
        Ok(ProviderInstallOutput {
            storage_providers: output.storage_providers,
            provider_admin_gateways: output.provider_admin_gateways,
            part_store_gateways: output.part_store_gateways,
            stream_gateways: output.stream_gateways,
            remote_folder_gateways: output.remote_folder_gateways,
            remote_object_gateways: output.remote_object_gateways,
        })
    }

    #[cfg(not(feature = "telegram"))]
    {
        Ok(ProviderInstallOutput::default())
    }
}

#[cfg(feature = "telegram")]
fn has_telegram_env() -> bool {
    let phone = std::env::var("TELEGRAM_PHONE").unwrap_or_default();
    let api_id: i32 = std::env::var("TELEGRAM_API_ID")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let api_hash = std::env::var("TELEGRAM_API_HASH").unwrap_or_default();
    !phone.is_empty() && api_id != 0 && !api_hash.is_empty()
}

#[cfg(feature = "telegram")]
async fn connect_telegram(
    base_dir: &Path,
    chat_id: Option<&str>,
) -> AppResult<Option<Arc<TelegramClient>>> {
    let Some(chat_id) = chat_id else {
        return Ok(None);
    };
    if !has_telegram_env() {
        return Ok(None);
    }
    let phone = std::env::var("TELEGRAM_PHONE").unwrap_or_default();
    let api_id: i32 = std::env::var("TELEGRAM_API_ID")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let api_hash = std::env::var("TELEGRAM_API_HASH").unwrap_or_default();

    let session_path = omega_drive_telegram::telegram_session::telegram_session_path(base_dir)
        .to_string_lossy()
        .to_string();
    match TelegramClient::connect(
        api_id,
        &api_hash,
        &phone,
        chat_id,
        &session_path,
        &base_dir.join("cache"),
    )
    .await
    {
        Ok(client) => {
            tracing::info!("Telegram provider preconnected.");
            Ok(Some(client))
        }
        Err(err) => {
            eprintln!("Telegram provider connection failed: {err}");
            Ok(None)
        }
    }
}

fn cleanup_temp_files(max_age: Duration) {
    #[cfg(feature = "telegram")]
    omega_drive_telegram::telegram_real::cleanup_telegram_temp_files(max_age);
}
