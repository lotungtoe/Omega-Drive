use std::path::{Path, PathBuf};

const APP_SUBDIR: &str = "omega-drive";

pub(super) fn resolve_base_dir() -> PathBuf {
    // Dev mode: walk up from CARGO_MANIFEST_DIR to workspace root, then project root
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut p = PathBuf::from(&manifest);
        loop {
            let cargo_toml = p.join("Cargo.toml");
            if cargo_toml.is_file() {
                if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                    if content.contains("[workspace]") {
                        p.pop();
                        return p;
                    }
                }
            }
            if !p.pop() {
                break;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let dir = PathBuf::from(appdata).join(APP_SUBDIR);
            if std::fs::create_dir_all(&dir).is_ok() {
                return dir;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var("HOME").ok() {
            let dir = PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_SUBDIR);
            if std::fs::create_dir_all(&dir).is_ok() {
                return dir;
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let xdg_data = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".local").join("share")
            });
        let dir = xdg_data.join(APP_SUBDIR);
        if std::fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }

    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(super) fn resolve_tenant_db_path(
    base_dir: &Path,
    tenant: &omega_drive_gateway::core::tenant::TenantDescriptor,
) -> PathBuf {
    omega_drive_core::tenant_registry::tenant_db_path(base_dir, tenant)
}

pub(super) fn resolve_startup_tenant(
    base_dir: &Path,
) -> omega_drive_gateway::core::tenant::TenantDescriptor {
    omega_drive_core::tenant_registry::resolve_startup_tenant(base_dir)
}


