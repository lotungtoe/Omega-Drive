use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use crate::{
    core::{
        error::{AppError, AppResult},
        tenant::TenantDescriptor,
    },
    db::{DbWriteQueue, ReadDbPool},
};

use omega_drive_gateway::provider::{
    part_store::{PartStoreGateway, PartStoreRegistry},
    provider_admin::{ProviderAdminGateway, ProviderAdminRegistry},
    remote_folder::{RemoteFolderGateway, RemoteFolderRegistry},
    remote_object::{RemoteObjectGateway, RemoteObjectRegistry},
    storage::{ProviderRegistry, StorageProvider},
    stream::{StreamGateway, StreamRegistry},
};

#[derive(Clone, Default)]
pub struct PreparedProviderState {}

#[derive(Clone)]
pub struct ProviderInstallContext {
    pub base_dir: PathBuf,
    pub db_write: Arc<DbWriteQueue>,
    pub db_read: Arc<ReadDbPool>,
    pub prepared: PreparedProviderState,
    pub event_bus: Arc<omega_drive_gateway::core::events::EventBus>,
    pub tenant: TenantDescriptor,
}

pub type ProviderInstallFuture =
    Pin<Box<dyn Future<Output = AppResult<ProviderInstallOutput>> + Send>>;
pub type ProviderPrepareFuture =
    Pin<Box<dyn Future<Output = AppResult<PreparedProviderState>> + Send>>;

#[derive(Clone, Copy, Default)]
pub struct ProviderBootstrapHooks {
    pub env_template: Option<&'static str>,
    pub cleanup_temp_files: Option<fn(Duration)>,
    pub prepare_state:
        Option<fn(PathBuf, TenantDescriptor, PreparedProviderState) -> ProviderPrepareFuture>,
}

impl ProviderBootstrapHooks {
    pub const fn new(
        env_template: Option<&'static str>,
        cleanup_temp_files: Option<fn(Duration)>,
        prepare_state: Option<
            fn(PathBuf, TenantDescriptor, PreparedProviderState) -> ProviderPrepareFuture,
        >,
    ) -> Self {
        Self {
            env_template,
            cleanup_temp_files,
            prepare_state,
        }
    }
}

pub struct ProviderInstaller {
    pub id: &'static str,
    bootstrap: ProviderBootstrapHooks,
    install: fn(ProviderInstallContext) -> ProviderInstallFuture,
}

impl ProviderInstaller {
    pub const fn new(
        id: &'static str,
        bootstrap: ProviderBootstrapHooks,
        install: fn(ProviderInstallContext) -> ProviderInstallFuture,
    ) -> Self {
        Self {
            id,
            bootstrap,
            install,
        }
    }

    pub async fn install(&self, ctx: ProviderInstallContext) -> AppResult<ProviderInstallOutput> {
        (self.install)(ctx).await
    }

    pub fn env_template(&self) -> Option<&'static str> {
        self.bootstrap.env_template
    }

    pub fn cleanup_temp_files(&self, max_age: Duration) {
        if let Some(cleanup) = self.bootstrap.cleanup_temp_files {
            cleanup(max_age);
        }
    }

    pub async fn prepare_state(
        &self,
        base_dir: PathBuf,
        tenant: TenantDescriptor,
        prepared: PreparedProviderState,
    ) -> AppResult<PreparedProviderState> {
        match self.bootstrap.prepare_state {
            Some(prepare) => prepare(base_dir, tenant, prepared).await,
            None => Ok(prepared),
        }
    }
}

pub async fn prepare_builtin_provider_state(
    base_dir: &Path,
    tenant: &TenantDescriptor,
) -> AppResult<PreparedProviderState> {
    let mut prepared = PreparedProviderState::default();

    for installer in crate::providers::builtin_installers() {
        prepared = installer
            .prepare_state(base_dir.to_path_buf(), tenant.clone(), prepared)
            .await?;
    }

    Ok(prepared)
}

pub fn render_builtin_bot_env_template() -> String {
    let mut template =
        String::from("# Discord Drive - Cau hinh bot\n# Dien thong tin vao day roi restart app.\n");

    for installer in crate::providers::builtin_installers() {
        let Some(section) = installer.env_template() else {
            continue;
        };

        if !template.ends_with("\n\n") {
            if !template.ends_with('\n') {
                template.push('\n');
            }
            template.push('\n');
        }

        template.push_str(section.trim_end());
        template.push('\n');
    }

    template
}

#[derive(Default)]
pub struct ProviderInstallOutput {
    pub storage_providers: Vec<Arc<dyn StorageProvider>>,
    pub provider_admin_gateways: Vec<Arc<dyn ProviderAdminGateway>>,
    pub part_store_gateways: Vec<Arc<dyn PartStoreGateway>>,
    pub stream_gateways: Vec<Arc<dyn StreamGateway>>,
    pub remote_folder_gateways: Vec<Arc<dyn RemoteFolderGateway>>,
    pub remote_object_gateways: Vec<Arc<dyn RemoteObjectGateway>>,
}

#[derive(Default)]
pub struct ProviderInstallResults {
    pub storage_registry: ProviderRegistry,
    pub provider_admin_registry: ProviderAdminRegistry,
    pub part_store_registry: PartStoreRegistry,
    pub stream_registry: StreamRegistry,
    pub remote_folder_registry: RemoteFolderRegistry,
    pub remote_object_registry: RemoteObjectRegistry,
}

impl ProviderInstallResults {
    fn merge(&mut self, installer_id: &str, output: ProviderInstallOutput) -> AppResult<()> {
        for provider in output.storage_providers {
            let provider_id = provider.metadata().id;
            if self.storage_registry.get(&provider_id).is_some() {
                return Err(AppError::new(
                    "E_PROVIDER_INSTALL",
                    format!(
                        "Duplicate storage provider '{provider_id}' contributed by '{installer_id}'"
                    ),
                ));
            }
            self.storage_registry.register(provider);
        }

        for gateway in output.provider_admin_gateways {
            let provider_id = gateway.provider_id().to_string();
            if self.provider_admin_registry.get(&provider_id).is_some() {
                return Err(AppError::new(
                    "E_PROVIDER_INSTALL",
                    format!(
                        "Duplicate provider-admin gateway '{provider_id}' contributed by '{installer_id}'"
                    ),
                ));
            }
            self.provider_admin_registry.register(gateway);
        }

        for gateway in output.part_store_gateways {
            let provider_id = gateway.provider_id().to_string();
            if self.part_store_registry.get(&provider_id).is_some() {
                return Err(AppError::new(
                    "E_PROVIDER_INSTALL",
                    format!(
                        "Duplicate part-store gateway '{provider_id}' contributed by '{installer_id}'"
                    ),
                ));
            }
            self.part_store_registry.register(gateway);
        }

        for gateway in output.stream_gateways {
            let provider_id = gateway.provider_id().to_string();
            if self.stream_registry.get(&provider_id).is_some() {
                return Err(AppError::new(
                    "E_PROVIDER_INSTALL",
                    format!(
                        "Duplicate stream gateway '{provider_id}' contributed by '{installer_id}'"
                    ),
                ));
            }
            self.stream_registry.register(gateway);
        }

        for gateway in output.remote_folder_gateways {
            let provider_id = gateway.provider_id().to_string();
            if self.remote_folder_registry.get(&provider_id).is_some() {
                return Err(AppError::new(
                    "E_PROVIDER_INSTALL",
                    format!(
                        "Duplicate remote-folder gateway '{provider_id}' contributed by '{installer_id}'"
                    ),
                ));
            }
            self.remote_folder_registry.register(gateway);
        }

        for gateway in output.remote_object_gateways {
            let provider_id = gateway.provider_id().to_string();
            if self.remote_object_registry.get(&provider_id).is_some() {
                return Err(AppError::new(
                    "E_PROVIDER_INSTALL",
                    format!(
                        "Duplicate remote-object gateway '{provider_id}' contributed by '{installer_id}'"
                    ),
                ));
            }
            self.remote_object_registry.register(gateway);
        }

        Ok(())
    }
}

pub async fn install_builtin_providers(
    ctx: ProviderInstallContext,
) -> AppResult<ProviderInstallResults> {
    let mut installed = ProviderInstallResults::default();

    for installer in crate::providers::builtin_installers() {
        let output = installer.install(ctx.clone()).await?;
        installed.merge(installer.id, output)?;
    }

    Ok(installed)
}

pub fn build_provider_runtime(
    installed: ProviderInstallResults,
) -> Arc<crate::providers::runtime::ProviderRuntime> {
    Arc::new(crate::providers::runtime::ProviderRuntime::with_registries(
        Arc::new(installed.storage_registry),
        Arc::new(installed.part_store_registry),
        Arc::new(installed.stream_registry),
        Arc::new(installed.provider_admin_registry),
        Arc::new(installed.remote_folder_registry),
        Arc::new(installed.remote_object_registry),
    ))
}
