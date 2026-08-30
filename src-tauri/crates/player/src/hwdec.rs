/// Enumerate GPU adapters for hardware decoding.
/// Returns list of adapter names; first entry is always "Auto".

fn gpu_names_from_powershell() -> Vec<String> {
    // ponytail: CREATE_NO_WINDOW (0x08000000) to hide powershell.exe console flash
    // when opening the Decode settings tab.
    #[cfg(windows)]
    let output = {
        use std::os::windows::process::CommandExt;
        match std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
            ])
            .creation_flags(0x08000000)
            .output()
        {
            Ok(o) if o.status.success() => o,
            _ => return vec![],
        }
    };
    #[cfg(not(windows))]
    let output = match std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
        ])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[cfg(windows)]
pub fn enumerate_gpu_adapters() -> Vec<String> {
    let mut list = vec!["Auto".to_string()];
    for name in gpu_names_from_powershell() {
        // deduplicate while preserving order
        if !list.contains(&name) {
            list.push(name);
        }
    }
    list
}

#[cfg(not(windows))]
pub fn enumerate_gpu_adapters() -> Vec<String> {
    vec!["Auto".to_string()]
}
