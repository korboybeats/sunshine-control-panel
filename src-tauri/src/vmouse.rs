use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use log::{info, warn, debug};
use crate::sunshine;

/// Check whether the current process has administrator privileges
#[cfg(target_os = "windows")]
fn is_elevated() -> bool {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match output {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            s == "True"
        }
        Err(_) => false,
    }
}

/// Run a .bat script at appropriate privilege — run directly if already admin, otherwise elevate
#[cfg(target_os = "windows")]
fn run_bat_elevated(bat_path: &std::path::Path) -> Result<(), String> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    if is_elevated() {
        // Already elevated — run directly
        let output = Command::new("cmd")
            .args(&["/c", &bat_path.to_string_lossy()])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to start script: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Script failed: {}", stderr));
        }
    } else {
        // Needs elevation
        let ps_cmd = format!(
            r#"Start-Process cmd -ArgumentList '/c','""{bat}""' -Verb RunAs -WindowStyle Hidden -Wait"#,
            bat = bat_path.display()
        );

        let output = Command::new("powershell")
            .args(&["-NoProfile", "-Command", &ps_cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to start script: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Script failed: {}", stderr));
        }
    }
    Ok(())
}

/// vmouse driver status info
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VmouseStatus {
    /// Whether the driver is installed (device node exists)
    pub installed: bool,
    /// Whether the device is running (no error codes)
    pub running: bool,
    /// Device status description
    pub status_text: String,
    /// Driver file path (if any)
    pub driver_path: String,
    /// The `virtual_mouse` config value in sunshine.conf
    pub config_enabled: bool,
}

/// vmouse driver files directory
fn get_vmouse_tools_path() -> PathBuf {
    PathBuf::from(sunshine::get_sunshine_install_path())
        .join("tools")
        .join("vmouse")
}

/// vmouse bat-scripts directory (CMake installs these to scripts/vmouse/)
fn get_vmouse_scripts_path() -> PathBuf {
    PathBuf::from(sunshine::get_sunshine_install_path())
        .join("scripts")
        .join("vmouse")
}

/// Check whether the vmouse device node exists
#[cfg(target_os = "windows")]
fn check_device_installed() -> (bool, bool, String) {
    use std::process::Command;

    // Use PowerShell's Get-PnpDevice to check device status
    // Exclude ghost devices left behind after removal (empty FriendlyName)
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let output = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(&[
            "-NoProfile", "-Command",
            "Get-PnpDevice -InstanceId 'ROOT\\HIDCLASS\\*' -ErrorAction SilentlyContinue | Where-Object { ($_.FriendlyName -like '*Virtual Mouse*' -or $_.HardwareID -contains 'Root\\ZakoVirtualMouse') -and $_.FriendlyName -ne $null -and $_.FriendlyName -ne '' } | Select-Object -First 1 Status, FriendlyName, Problem | ConvertTo-Json -Compress"
        ])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout.is_empty() || stdout == "null" {
                return (false, false, "Not installed".to_string());
            }

            // Try to parse JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                let status = json.get("Status").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let friendly = json.get("FriendlyName").and_then(|v| v.as_str()).unwrap_or("Zako Virtual Mouse");
                let problem = json.get("Problem").and_then(|v| v.as_u64()).unwrap_or(0);

                let running = status == "OK" && problem == 0;
                let status_text = if running {
                    format!("{} - Running", friendly)
                } else if problem == 21 {
                    format!("{} - Needs restart", friendly)
                } else {
                    format!("{} - Status: {} (problem code: {})", friendly, status, problem)
                };

                (true, running, status_text)
            } else {
                // JSON parse failed but output exists — device is present
                (true, false, format!("Installed (status unknown)"))
            }
        }
        Err(e) => {
            warn!("Failed to detect vmouse device: {}", e);
            (false, false, "Detection failed".to_string())
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn check_device_installed() -> (bool, bool, String) {
    (false, false, "Windows only".to_string())
}

/// Read `virtual_mouse` from sunshine.conf
fn read_vmouse_config() -> bool {
    let config_path = PathBuf::from(sunshine::get_sunshine_install_path())
        .join("config")
        .join("sunshine.conf");

    if !config_path.exists() {
        return true; // Enabled by default
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    if key.trim() == "virtual_mouse" {
                        let val = value.trim().to_lowercase();
                        return val == "enabled" || val == "true" || val == "1" || val == "yes";
                    }
                }
            }
            true // Enabled by default
        }
        Err(_) => true,
    }
}

/// Get vmouse driver status
#[tauri::command]
pub async fn get_vmouse_status() -> Result<VmouseStatus, String> {
    let (installed, running, status_text) = check_device_installed();
    let driver_path = get_vmouse_tools_path().to_string_lossy().to_string();
    let config_enabled = read_vmouse_config();

    Ok(VmouseStatus {
        installed,
        running,
        status_text,
        driver_path,
        config_enabled,
    })
}

/// Install the vmouse driver (reuses install-vmouse.bat)
#[tauri::command]
pub async fn install_vmouse_driver() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let scripts_dir = get_vmouse_scripts_path();
        let install_bat = scripts_dir.join("install-vmouse.bat");

        if !install_bat.exists() {
            return Err(format!(
                "Install script not found: {}. Please verify your Sunshine installation.",
                install_bat.display()
            ));
        }

        info!("Calling install-vmouse.bat to install the virtual mouse driver...");
        run_bat_elevated(&install_bat)?;

        // Wait for the driver to load
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        info!("✅ vmouse driver install complete");
        Ok("Virtual mouse driver installed".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("This feature is only supported on Windows".to_string())
    }
}

/// Uninstall the vmouse driver (reuses uninstall-vmouse.bat)
#[tauri::command]
pub async fn uninstall_vmouse_driver() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let scripts_dir = get_vmouse_scripts_path();
        let uninstall_bat = scripts_dir.join("uninstall-vmouse.bat");

        if !uninstall_bat.exists() {
            return Err(format!(
                "Uninstall script not found: {}. Please verify your Sunshine installation.",
                uninstall_bat.display()
            ));
        }

        info!("Calling uninstall-vmouse.bat to uninstall the virtual mouse driver...");
        run_bat_elevated(&uninstall_bat)?;

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        info!("✅ vmouse driver uninstall complete");
        Ok("Virtual mouse driver uninstalled".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("This feature is only supported on Windows".to_string())
    }
}

/// Set `virtual_mouse` in sunshine.conf
#[tauri::command]
pub async fn set_vmouse_config(enabled: bool) -> Result<String, String> {
    // Read the current full config
    let mut config_map = crate::vdd::read_full_sunshine_config().await
        .unwrap_or_default();

    // Update the virtual_mouse field
    let value_str = if enabled { "enabled" } else { "disabled" };
    config_map.insert("virtual_mouse".to_string(), serde_json::json!(value_str));

    debug!("📝 Updating virtual_mouse = {}", value_str);

    // Save via the Sunshine API
    sunshine::post_sunshine_config(&config_map).await?;
    info!("✅ virtual_mouse config updated: {}", value_str);
    Ok(format!("Virtual mouse {}", if enabled { "enabled" } else { "disabled" }))
}
