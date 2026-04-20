use std::process::Command;
use crate::sunshine;
use std::env;
use tauri::Manager;
use log::{info, error, debug};

#[tauri::command]
pub async fn restart_graphics_driver() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        
        // Dynamically read the Sunshine install path from the registry
        let sunshine_path = std::path::PathBuf::from(sunshine::get_sunshine_install_path());
        let restart_exe = sunshine_path.join("tools").join("restart64.exe");

        if !restart_exe.exists() {
            return Err("restart64.exe not found".to_string());
        }

        // Run via PowerShell with admin privileges
        let ps_command = format!(
            r#"Start-Process '{}' -Verb RunAs -WindowStyle Hidden"#,
            restart_exe.display()
        );

        // CREATE_NO_WINDOW = 0x08000000, used to hide the PowerShell window
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        Command::new("powershell")
            .args(&["-Command", &ps_command])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;

        Ok("Graphics driver restart requested".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("This feature is only supported on Windows".to_string())
    }
}

/// Restart the GUI with administrator privileges
#[tauri::command]
pub async fn restart_as_admin(app_handle: tauri::AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        
        // Get the current executable path
        let current_exe = env::current_exe()
            .map_err(|e| format!("Failed to get current program path: {}", e))?;

        info!("🔄 Preparing to restart GUI with admin privileges");
        debug!("   Current program: {:?}", current_exe);

        // Use PowerShell's Start-Process -Verb RunAs to elevate privileges
        let exe_path = current_exe.to_string_lossy().to_string();

        // Build the PowerShell command to start as administrator
        let ps_command = format!(
            "Start-Sleep -Milliseconds 500; Start-Process -FilePath '{}' -Verb RunAs",
            exe_path.replace("'", "''")  // Escape single quotes
        );

        debug!("   PowerShell command: {}", ps_command);

        // CREATE_NO_WINDOW = 0x08000000
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Launch a new elevated instance (PowerShell waits 500ms before starting it)
        Command::new("powershell")
            .args(&["-NoProfile", "-Command", &ps_command])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to launch admin instance: {}", e))?;

        info!("✅ New instance requested with admin privileges (starting in 500ms)");

        // Exit the current instance immediately so the new one can bind the port
        tokio::spawn(async move {
            info!("🚪 Preparing to exit current instance...");

            // Close the main window first
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.close();
                debug!("   Closing main window");
            }

            // Short delay, then exit so the window can close and release resources
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            info!("🚪 Exiting current instance, releasing resources");
            app_handle.exit(0);
        });

        Ok("Restarting with admin privileges...".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("This feature is only supported on Windows".to_string())
    }
}


/// Check whether the current process is running with administrator privileges
#[tauri::command]
pub fn is_running_as_admin() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        
        unsafe {
            let mut token: HANDLE = HANDLE::default();
            let process = GetCurrentProcess();
            
            // Open the access token for the current process
            if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
                return Ok(false);
            }

            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut return_length = 0u32;

            // Get the token elevation info
            let result = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_length,
            );
            
            CloseHandle(token).ok();
            
            if result.is_err() {
                return Ok(false);
            }
            
            Ok(elevation.TokenIsElevated != 0)
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows systems, check for root privileges
        Ok(unsafe { libc::geteuid() == 0 })
    }
}

/// Open a URL in the external browser
pub fn open_url_in_browser(url: &str) {
    let url = url.to_string();

    tauri::async_runtime::spawn(async move {
        info!("🌐 Opening external browser...");

        #[cfg(target_os = "windows")]
        {
            if let Err(e) = Command::new("cmd")
                .args(&["/c", "start", "", &url])
                .spawn()
            {
                error!("❌ Failed to open URL: {}", e);
            } else {
                info!("✅ Opened in external browser: {}", url);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Err(e) = Command::new("xdg-open")
                .arg(&url)
                .spawn()
            {
                error!("❌ Failed to open URL: {}", e);
            } else {
                info!("✅ Opened in external browser: {}", url);
            }
        }
    });
}

/// Tauri command: open a URL in the external browser
#[tauri::command]
pub async fn open_external_url(url: String) -> Result<bool, String> {
    if !url.starts_with("http") {
        return Ok(false);
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(&["/c", "start", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(true)
}

/// Execute a PowerShell command (internal helper)
#[cfg(target_os = "windows")]
pub fn execute_powershell_command(command: &str, error_context: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    
    let ps_command = format!(
        "Start-Process powershell -ArgumentList '-NoProfile', '-Command', '{}' -Verb RunAs -WindowStyle Hidden",
        command.replace("'", "''")
    );
    
    Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_command])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| {
            error!("❌ {}: {}", error_context, e);
            format!("{}: {}", error_context, e)
        })?;
    
    Ok(())
}
