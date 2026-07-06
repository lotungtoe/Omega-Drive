use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use serde_json::Value;

use crate::app_wiring::app_runtime::AppState;
use omega_drive_gateway::core::error::{report, AppError, AppResult};
use omega_drive_gateway::core::error_codes as codes;

use super::{
    context::{ExtensionContext, ScopedExtensionContext},
    contract::{InternalExtension, InvocationMeta},
};

pub struct ExtensionRegistry {
    extensions: HashMap<String, Arc<dyn InternalExtension>>,
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
        }
    }

    pub fn register(&mut self, extension: Arc<dyn InternalExtension>) -> AppResult<()> {
        let manifest = extension.manifest();
        manifest.validate()?;

        if self.extensions.contains_key(&manifest.id) {
            return Err(report(
                "extensions",
                AppError::new(
                    codes::E_CONFLICT,
                    format!("Duplicate extension id '{}'", manifest.id),
                ),
            ));
        }

        self.extensions.insert(manifest.id.clone(), extension);
        Ok(())
    }

    pub fn get(&self, extension_id: &str) -> Option<Arc<dyn InternalExtension>> {
        self.extensions.get(extension_id).cloned()
    }

    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.extensions.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn global() -> AppResult<Arc<Self>> {
        static REGISTRY: OnceLock<AppResult<Arc<ExtensionRegistry>>> = OnceLock::new();

        REGISTRY
            .get_or_init(|| {
                let mut registry = ExtensionRegistry::new();
                crate::app_wiring::extensions::generated::register_generated_extensions(&mut registry)?;
                Ok(Arc::new(registry))
            })
            .clone()
    }

    pub async fn dispatch(
        &self,
        state: Arc<AppState>,
        extension_id: &str,
        command_id: &str,
        payload: Value,
        window_label: Option<String>,
    ) -> AppResult<Value> {
        let extension = self.lookup_extension(extension_id, command_id)?;
        let context =
            ScopedExtensionContext::from_state(state, extension.manifest().dependencies.clone());

        self.dispatch_with_context(
            extension_id,
            command_id,
            payload,
            &context,
            InvocationMeta { window_label },
        )
        .await
    }

    pub async fn dispatch_with_context(
        &self,
        extension_id: &str,
        command_id: &str,
        payload: Value,
        ctx: &dyn ExtensionContext,
        meta: InvocationMeta,
    ) -> AppResult<Value> {
        let extension = self.lookup_extension(extension_id, command_id)?;
        extension.handle(command_id, payload, ctx, meta).await
    }

    fn lookup_extension(
        &self,
        extension_id: &str,
        command_id: &str,
    ) -> AppResult<Arc<dyn InternalExtension>> {
        let extension = self.get(extension_id).ok_or_else(|| {
            report(
                "extensions",
                AppError::new(
                    codes::E_NOT_FOUND,
                    format!("Unknown extension '{}'", extension_id),
                ),
            )
        })?;

        if !extension.manifest().supports_command(command_id) {
            return Err(report(
                "extensions",
                AppError::new(
                    codes::E_NOT_FOUND,
                    format!(
                        "Unknown extension command '{}.{}'",
                        extension_id, command_id
                    ),
                ),
            ));
        }

        Ok(extension)
    }
}
