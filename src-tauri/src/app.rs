use tauri::{Manager, App, AppHandle};
use log::{info, error, debug};
use crate::toolbar;
use crate::windows;
use crate::tray;
use crate::sunshine;
use crate::proxy_server;
use crate::update;

/// Application state
pub struct AppState {
    #[allow(dead_code)]
    pub main_window: std::sync::Mutex<Option<tauri::Window>>,
}

/// Application initialization / setup
pub fn setup_application(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let show_toolbar = args.iter().any(|arg| arg == "--toolbar" || arg == "-t");
    let show_desktop = args.iter().any(|arg| arg == "--desktop" || arg == "-d");
    let url_contains_pin = args.iter()
        .find(|arg| arg.starts_with("--url="))
        .map_or(false, |arg| arg.contains("/pin"));
    
    let app_handle = app.handle().clone();
    
    // Create window: skip main window in desktop or toolbar mode
    let main_window_created = if show_desktop {
        info!("🖥️ Detected --desktop argument — starting desktop UI mode");
        windows::create_desktop_window(&app_handle)?;
        windows::create_main_window_hidden(&app_handle)?;
        false
    } else if !show_toolbar && !url_contains_pin {
        windows::create_main_window(&app_handle)?;
        true
    } else {
        false
    };
    
    tray::create_system_tray(&app_handle)?;
    register_global_shortcuts(app)?;
    setup_menu_event_handler(app);
    start_proxy_server_async();
    
    // Start WebView heartbeat monitoring (detects renderer crashes and auto-recovers)
    windows::start_heartbeat_monitor(app.handle().clone());

    // Deferred tasks
    tauri::async_runtime::spawn(async move {
        // PIN pairing window
        if url_contains_pin {
            info!("🔐 Will open the PIN pairing window after app startup");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Err(e) = windows::open_pin_window(&app_handle) {
                error!("❌ Failed to create PIN pairing window: {}", e);
            }
        }

        // Toolbar window (non-desktop mode)
        if show_toolbar && !show_desktop {
            info!("🔧 Will open the toolbar after app startup");
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            if let Err(e) = toolbar::create_toolbar_window_internal(&app_handle) {
                error!("❌ Failed to create toolbar: {}", e);
            }
        }

        // Update check (only when the main window is created)
        if main_window_created {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            if let Err(e) = update::init_update_checker(&app_handle) {
                error!("❌ Failed to initialize update checker: {}", e);
            }
        }
    });
    
    Ok(())
}

/// Register global shortcuts
fn register_global_shortcuts(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
    
    let app_handle = app.handle().clone();
    
    match app.handle().global_shortcut().on_shortcut("CmdOrCtrl+Shift+Alt+T", move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            debug!("⌨️ Global shortcut triggered: CTRL+SHIFT+ALT+T");
            toggle_toolbar_window(&app_handle);
        }
    }) {
        Ok(_) => {
            info!("⌨️ Global shortcut registered: CTRL+SHIFT+ALT+T");
        }
        Err(e) => {
            log::warn!("⚠️  Failed to register global shortcut CTRL+SHIFT+ALT+T (possibly taken by another program): {}", e);
            log::warn!("⚠️  Toolbar shortcut is unavailable, but the application will continue running normally");
        }
    }
    
    Ok(())
}

/// Toggle the toolbar window show/hide
fn toggle_toolbar_window(app_handle: &AppHandle) {
    if let Some(toolbar_window) = app_handle.get_webview_window("toolbar") {
        debug!("🔧 Toolbar already exists — closing");
        let _ = toolbar_window.close();
    } else {
        debug!("🔧 Toolbar does not exist — creating");
        let app_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = toolbar::create_toolbar_window_internal(&app_clone) {
                error!("❌ Shortcut failed to create toolbar: {}", e);
            }
        });
    }
}

/// Set up the global menu event handler
fn setup_menu_event_handler(app: &mut App) {
    let app_handle = app.handle().clone();
    app.handle().on_menu_event(move |_app, event| {
        let event_id = event.id().as_ref();
        if event_id.starts_with("toolbar_") {
            debug!("🔧 Global menu event: {:?}", event.id());
            toolbar::handle_toolbar_menu_event(&app_handle, event_id);
        }
    });
}

/// Start the proxy server asynchronously
fn start_proxy_server_async() {
    tauri::async_runtime::spawn(async {
        // Check for the WEBUI_DEV_TARGET environment variable (used in dev mode)
        if let Ok(dev_target) = std::env::var("WEBUI_DEV_TARGET") {
            info!("🛠️ [dev mode] WEBUI_DEV_TARGET environment variable detected");
            info!("🎯 Proxy target: {}", dev_target);
            proxy_server::set_sunshine_target(dev_target);
        } else {
            // Get the Sunshine URL and configure the proxy target
            match sunshine::get_sunshine_url().await {
                Ok(url) => {
                    info!("🎯 Sunshine URL: {}", url);
                    let base_url = url.trim_end_matches('/').to_string();
                    proxy_server::set_sunshine_target(base_url);
                }
                Err(e) => {
                    log::warn!("⚠️  Failed to get Sunshine URL, using default: {}", e);
                }
            }
        }

        // Start the proxy server
        if let Err(e) = proxy_server::start_proxy_server().await {
            error!("❌ Failed to start proxy server: {}", e);
        }
    });
}

/// Handle single-instance logic
pub fn handle_single_instance(app: &AppHandle, args: Vec<String>) {
    info!("🔔 Second instance detected — activating existing window");
    info!("   Launch args: {:?}", args);

    // Diagnostic: list all current windows
    let windows: Vec<_> = app.webview_windows().keys().cloned().collect();
    info!("📋 Currently existing windows: {:?}", windows);

    // Check whether to open the desktop UI
    if args.iter().any(|arg| arg == "--desktop" || arg == "-d") {
        info!("🖥️ Detected --desktop argument — opening desktop UI");
        if let Err(e) = windows::open_desktop_window(app) {
            error!("❌ Failed to open desktop UI: {}", e);
        }
        return;
    }

    // Check whether to open the toolbar
    if args.iter().any(|arg| arg == "--toolbar" || arg == "-t") {
        info!("🔧 Detected --toolbar argument — opening toolbar");
        toggle_toolbar_window(app);
        return;
    }

    // Extract the URL argument and activate the main window
    let target_url = args.iter()
        .find(|arg| arg.starts_with("--url="))
        .map(|arg| arg.trim_start_matches("--url=").to_string());

    if let Some(url) = &target_url {
        info!("📍 Detected URL argument: {}", url);

        // Detect whether the URL contains the /pin path
        if url.contains("/pin") {
            info!("🔐 Detected /pin path — opening PIN pairing window");
            if let Err(e) = windows::open_pin_window(app) {
                error!("❌ Failed to open PIN window: {}", e);
            }
            return;
        }
    }

    windows::activate_main_window(app, target_url);
}
