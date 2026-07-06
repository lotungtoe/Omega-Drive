use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::tenant::{TenantDescriptor, TENANT_SCOPE_MY, TENANT_SCOPE_SHARED};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDbFiles {
    pub my: Option<String>,
    pub shared: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TenantRegistryFile {
    pub active_db_files: ActiveDbFiles,
}

impl TenantRegistryFile {
    pub fn active_db_file(&self, scope: &str) -> Option<&str> {
        match scope {
            TENANT_SCOPE_SHARED => self.active_db_files.shared.as_deref(),
            _ => self.active_db_files.my.as_deref(),
        }
    }

    pub fn set_active_db_file(&mut self, scope: &str, db_file: Option<String>) {
        let value = db_file
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        match scope {
            TENANT_SCOPE_SHARED => self.active_db_files.shared = value,
            _ => self.active_db_files.my = value,
        }
    }

    pub fn active_tenant(&self, scope: &str) -> Option<TenantDescriptor> {
        self.active_db_file(scope)
            .and_then(TenantDescriptor::from_db_file_name)
    }
}

const TENANT_REGISTRY_FILE_NAME: &str = "tenant_registry.json";
const TENANT_DB_DIR_NAME: &str = "db";

pub fn load_tenant_registry(base_dir: &Path) -> TenantRegistryFile {
    let path = base_dir.join(TENANT_REGISTRY_FILE_NAME);
    let Ok(content) = std::fs::read_to_string(path) else {
        return TenantRegistryFile::default();
    };
    serde_json::from_str::<TenantRegistryFile>(&content).unwrap_or_default()
}

pub fn save_tenant_registry(registry: &TenantRegistryFile, base_dir: &Path) -> std::io::Result<()> {
    let path = base_dir.join(TENANT_REGISTRY_FILE_NAME);
    let content = serde_json::to_string_pretty(registry)
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    std::fs::write(path, content)
}

pub fn registry_path(base_dir: &Path) -> PathBuf {
    base_dir.join(TENANT_REGISTRY_FILE_NAME)
}

pub fn tenant_db_dir(base_dir: &Path) -> PathBuf {
    let dir = base_dir.join(TENANT_DB_DIR_NAME);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn tenant_db_path(base_dir: &Path, tenant: &TenantDescriptor) -> PathBuf {
    tenant_db_dir(base_dir).join(tenant.db_file_name())
}

pub fn discover_tenants(base_dir: &Path) -> Vec<TenantDescriptor> {
    let dir = tenant_db_dir(base_dir);
    let mut tenants = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if let Some(tenant) = TenantDescriptor::from_db_file_name(&file_name) {
                tenants.push(tenant);
            }
        }
    }

    tenants.sort_by_key(|t| t.key());
    tenants.dedup_by(|left, right| left.key() == right.key());
    tenants
}

pub fn resolve_active_tenant_for_scope(base_dir: &Path, scope: &str) -> Option<TenantDescriptor> {
    let registry = load_tenant_registry(base_dir);
    let discovered = discover_tenants(base_dir);

    if let Some(active) = registry.active_tenant(scope) {
        return Some(active);
    }

    discovered
        .into_iter()
        .find(|tenant| tenant.scope == normalize_scope_key(scope))
}

pub fn resolve_startup_tenant(base_dir: &Path) -> TenantDescriptor {
    resolve_active_tenant_for_scope(base_dir, TENANT_SCOPE_MY)
        .or_else(|| resolve_active_tenant_for_scope(base_dir, TENANT_SCOPE_SHARED))
        .unwrap_or_default()
}

pub fn persist_active_tenant(
    base_dir: &Path,
    tenant: &TenantDescriptor,
) -> std::io::Result<TenantRegistryFile> {
    let mut registry = load_tenant_registry(base_dir);
    registry.set_active_db_file(&tenant.scope, Some(tenant.db_file_name()));
    save_tenant_registry(&registry, base_dir)?;
    Ok(registry)
}

fn normalize_scope_key(scope: &str) -> &'static str {
    if scope.eq_ignore_ascii_case(TENANT_SCOPE_SHARED) {
        TENANT_SCOPE_SHARED
    } else {
        TENANT_SCOPE_MY
    }
}
