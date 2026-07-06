use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // ponytail: hardcoded path to mpv.lib, same pattern as tauri_app/build.rs
    let libs_dir = manifest_dir.parent().unwrap().parent().unwrap().join("libs");
    if libs_dir.join("mpv.lib").exists() {
        println!("cargo:rustc-link-search={}", libs_dir.display());
    }

    // ponytail: find sqlite3.lib from libsqlite3-sys build output
    // rusqlite transitively links it but may not propagate search path for test bins
    let target_dir = manifest_dir.parent().unwrap().parent().unwrap().join("target");
    let build_dir = target_dir.join("debug").join("build");
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let sqlite_out = entry.path().join("out").join("sqlite3.lib");
            if sqlite_out.exists() {
                println!("cargo:rustc-link-search={}", entry.path().join("out").display());
                break;
            }
        }
    }
}
