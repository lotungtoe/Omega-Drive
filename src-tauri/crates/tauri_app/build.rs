use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

fn required_env(name: &str) -> String {
    match env::var(name) {
        Ok(value) => value,
        Err(err) => panic!("{name} must exist: {err}"),
    }
}

fn write_generated_file(path: &Path, contents: &str, label: &str) {
    if let Err(err) = fs::write(path, contents) {
        panic!("failed to write {label} '{}': {err}", path.display());
    }
}

#[derive(Debug, Deserialize)]
struct BuildExtensionManifest {
    id: String,
    version: String,
    description: String,
    commands: Vec<String>,
}

fn is_valid_extension_id(value: &str) -> bool {
    value.contains('.')
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '.')
}

fn is_valid_command_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn escape_raw_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn generate_internal_extension_registry(manifest_dir: &Path) {
    let extensions_dir = manifest_dir.join("src").join("extensions");
    println!("cargo:rerun-if-changed={}", extensions_dir.display());

    let out_dir = PathBuf::from(required_env("OUT_DIR"));
    let generated_path = out_dir.join("extensions_registry.g.rs");

    if !extensions_dir.exists() {
        write_generated_file(
            &generated_path,
            "pub(crate) fn register_generated_extensions(\n    _registry: &mut crate::app_wiring::extensions::registry::ExtensionRegistry,\n) -> omega_drive_gateway::core::error::AppResult<()> {\n    Ok(())\n}\n",
            "empty extension registry",
        );
        return;
    }

    let mut discovered = Vec::new();
    let mut seen_ids = HashSet::new();

    let entries = match fs::read_dir(&extensions_dir) {
        Ok(entries) => entries,
        Err(err) => panic!("failed to read '{}': {err}", extensions_dir.display()),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => panic!("failed to read extension dir entry: {err}"),
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("extension.toml");
        if !manifest_path.exists() {
            continue;
        }
        let module_path = path.join("mod.rs");

        println!("cargo:rerun-if-changed={}", manifest_path.display());
        println!("cargo:rerun-if-changed={}", module_path.display());

        if !module_path.exists() {
            panic!(
                "Extension '{}' is missing module file at {}",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown"),
                module_path.display()
            );
        }

        let raw = fs::read_to_string(&manifest_path)
            .unwrap_or_else(|err| panic!("Failed to read {}: {err}", manifest_path.display()));
        let manifest: BuildExtensionManifest = toml::from_str(&raw)
            .unwrap_or_else(|err| panic!("Invalid manifest {}: {err}", manifest_path.display()));

        if !is_valid_extension_id(&manifest.id) {
            panic!(
                "Invalid extension id '{}' in {}",
                manifest.id,
                manifest_path.display()
            );
        }
        if manifest.version.trim().is_empty() {
            panic!(
                "Extension '{}' is missing version in {}",
                manifest.id,
                manifest_path.display()
            );
        }
        if manifest.description.trim().is_empty() {
            panic!(
                "Extension '{}' is missing description in {}",
                manifest.id,
                manifest_path.display()
            );
        }

        if !seen_ids.insert(manifest.id.clone()) {
            panic!("Duplicate extension id '{}'", manifest.id);
        }

        let mut seen_commands = HashSet::new();
        for command in &manifest.commands {
            if !is_valid_command_id(command) {
                panic!(
                    "Invalid command '{}' in extension '{}'",
                    command, manifest.id
                );
            }
            if !seen_commands.insert(command.clone()) {
                panic!(
                    "Duplicate command '{}' in extension '{}'",
                    command, manifest.id
                );
            }
        }

        let module_name = format!("generated_extension_{}", discovered.len());
        discovered.push((module_name, module_path, manifest.id));
    }

    discovered.sort_by(|a, b| a.2.cmp(&b.2));

    let mut generated = String::new();

    for (module_name, module_path, _extension_id) in &discovered {
        generated.push_str(&format!(
            "#[path = r\"{}\"]\nmod {};\n",
            escape_raw_path(module_path),
            module_name
        ));
    }

    generated.push_str(
        "\npub(crate) fn register_generated_extensions(\n    registry: &mut crate::app_wiring::extensions::registry::ExtensionRegistry,\n) -> omega_drive_gateway::core::error::AppResult<()> {\n",
    );

    for (module_name, _module_path, extension_id) in &discovered {
        generated.push_str(&format!(
            "    registry.register({}::build_extension())?; // {}\n",
            module_name, extension_id
        ));
    }

    generated.push_str("    Ok(())\n}\n");

    write_generated_file(&generated_path, &generated, "generated extension registry");
}

fn main() {
    tauri_build::build();

    // ponytail: hardcoded path to mpv.lib, switch to dynamic discovery if env changes
    let libs_dir = PathBuf::from(required_env("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("libs");
    // ponytail: /DELAYLOAD lets ensure_binaries() download libmpv-2.dll before mpv init
    if libs_dir.join("mpv.lib").exists() {
        println!("cargo:rustc-link-search={}", libs_dir.display());
        println!("cargo:rustc-link-arg=/DELAYLOAD:libmpv-2.dll");
        println!("cargo:rustc-link-lib=delayimp");
    }

    let manifest_dir = PathBuf::from(required_env("CARGO_MANIFEST_DIR"));
    generate_internal_extension_registry(&manifest_dir);
}
