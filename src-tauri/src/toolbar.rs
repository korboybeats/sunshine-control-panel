// Toolbar-window management module

use tauri::{AppHandle, Manager, Runtime, Emitter};
use std::path::PathBuf;
use std::fs;
use log::{warn, error, debug};
use crate::windows;

// Get toolbar config file path
fn get_toolbar_config_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Ensure the directory exists
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }

    Ok(app_data_dir.join("toolbar_config.json"))
}

// Internal function to save toolbar position (used by window event handler)
pub fn save_toolbar_position_internal<R: Runtime>(app: &AppHandle<R>, x: f64, y: f64) {
    if let Ok(config_path) = get_toolbar_config_path(app) {
        let config = serde_json::json!({
            "x": x,
            "y": y
        });

        if let Err(e) = fs::write(&config_path, config.to_string()) {
            error!("❌ Failed to save toolbar position: {}", e);
        } else {
            debug!("💾 Toolbar position saved: ({}, {})", x, y);
        }
    }
}

// Save toolbar position (Tauri command)
#[tauri::command]
pub async fn save_toolbar_position(app: AppHandle, x: f64, y: f64) -> Result<(), String> {
    save_toolbar_position_internal(&app, x, y);
    Ok(())
}

// Load toolbar position
fn load_toolbar_position<R: Runtime>(app: &AppHandle<R>) -> Option<(f64, f64)> {
    let config_path = match get_toolbar_config_path(app) {
        Ok(path) => path,
        Err(e) => {
            error!("❌ Failed to get config path: {}", e);
            return None;
        }
    };

    if !config_path.exists() {
        return None;
    }

    match fs::read_to_string(&config_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(config) => {
                    let x = config["x"].as_f64()?;
                    let y = config["y"].as_f64()?;
                    debug!("📂 Loaded toolbar position: ({}, {})", x, y);
                    Some((x, y))
                }
                Err(e) => {
                    error!("❌ Failed to parse toolbar config: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to read toolbar config: {}", e);
            None
        }
    }
}

// Helper: create a tool window
pub fn create_tool_window_internal<R: Runtime>(app: &AppHandle<R>, tool_type: &str) {
    const TOOL_WINDOW_ID: &str = "tool_window";

    // If the window already exists, close it first
    if let Some(window) = app.get_webview_window(TOOL_WINDOW_ID) {
        let _ = window.close();
    }

    // Create the tool window, passing the tool type via URL param
    let url = format!("tool-window/index.html?tool={}", tool_type);
    let title = format!("ZakoToolsWindow - {}", tool_type);
    debug!("🔧 Creating tool window URL: {}", url);

    match tauri::WebviewWindowBuilder::new(
        app,
        TOOL_WINDOW_ID,
        tauri::WebviewUrl::App(url.into())
    )
    .title(&title)
    .fullscreen(true)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)  // Hide first to avoid white flash
    .build()
    {
        Ok(window) => {
            // Disable the right-click menu in production
            windows::disable_context_menu(&window);

            // Disable autofill and password save prompts
            #[cfg(target_os = "windows")]
            windows::configure_webview_security(&window);

            // Auto-open DevTools in debug
            #[cfg(debug_assertions)]
            {
                window.open_devtools();
                let _ = window.set_always_on_top(false);
                debug!("🔧 [dev mode] DevTools auto-opened on tool window");
            }

            // Wait briefly for content to load, then show the window
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let _ = window.show();
            });
        }
        Err(e) => {
            error!("❌ Failed to create tool window: {}", e);
        }
    }
}

// Handle toolbar menu events
pub fn handle_toolbar_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    fn show_main_window<R: Runtime>(window: &tauri::WebviewWindow<R>) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }

    fn ensure_main_window<R: Runtime>(app: &AppHandle<R>) -> Option<tauri::WebviewWindow<R>> {
        if let Some(window) = app.get_webview_window("main") {
            show_main_window(&window);
            Some(window)
        } else {
            if let Err(e) = windows::create_main_window(app) {
                error!("❌ Failed to create main window: {}", e);
                return None;
            }
            app.get_webview_window("main")
        }
    }

    match event_id {
        "main" | "toolbar_main" => {
            ensure_main_window(app);
        }
        "vdd" | "toolbar_vdd" => {
            if let Some(window) = ensure_main_window(app) {
                let _ = window.emit("open-vdd-settings", ());
            }
        }
        "dpi" | "toolbar_dpi" => {
            create_tool_window_internal(app, "dpi");
        }
        "bitrate" | "toolbar_bitrate" => {
            create_tool_window_internal(app, "bitrate");
        }
        "shortcuts" | "toolbar_shortcuts" => {
            create_tool_window_internal(app, "shortcuts");
        }
        "close" | "toolbar_close" => {
            if let Some(window) = app.get_webview_window("toolbar") {
                let _ = window.close();
            }
        }
        _ => {}
    }
}

// Internal generic function to create the toolbar window
pub fn create_toolbar_window_internal<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    const TOOLBAR_WINDOW_ID: &str = "toolbar";

    // Check whether the toolbar window already exists
    if app.get_webview_window(TOOLBAR_WINDOW_ID).is_some() {
        debug!("🔧 Toolbar window already exists");
        return Ok(());
    }

    debug!("🔧 Creating toolbar window");

    // Window size and margin
    let toolbar_size = 240.0;  // Window size (compact layout: 80px icon + 80px bubble radius × 2)
    let margin = 20.0;         // Margin from screen edge

    // Create the window at the default position first
    let window = match tauri::WebviewWindowBuilder::new(
        app,
        TOOLBAR_WINDOW_ID,
        tauri::WebviewUrl::App("toolbar/index.html".into())
    )
    .title("Toolbar")
    .inner_size(toolbar_size, toolbar_size)
    .max_inner_size(toolbar_size, toolbar_size)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)  // Hide first; show after positioning
    .build()
    {
        Ok(win) => {
            // Disable right-click menu in production
            windows::disable_context_menu(&win);

            // Auto-open DevTools in debug
            #[cfg(debug_assertions)]
            {
                win.open_devtools();
                debug!("🔧 [dev mode] DevTools auto-opened");
            }

            // After 500ms, verify the window size (WebView2 init may unexpectedly enlarge it)
            let win_check = win.clone();
            let target = toolbar_size;
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Ok(size) = win_check.inner_size() {
                    let sf = win_check.scale_factor().unwrap_or(1.0);
                    let expected_phys = (target * sf) as u32;
                    if size.width != expected_phys || size.height != expected_phys {
                        warn!("⚠️ Unexpected window size! expected {}x{}, actual {}x{}", expected_phys, expected_phys, size.width, size.height);
                        let _ = win_check.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(expected_phys, expected_phys)));
                    }
                }
            });

            win
        }
        Err(e) => {
            error!("❌ Failed to create toolbar window: {}", e);
            return Err(format!("Failed to create toolbar window: {}", e));
        }
    };

    // Try to load a saved position; if none, fall back to the default (bottom-right)
    if let Some((saved_x, saved_y)) = load_toolbar_position(app) {
        // The saved coordinates are in physical pixels; validate they are within the screen
        debug!("📂 Read saved toolbar position: ({}, {})", saved_x, saved_y);

        // Get current monitor info for bounds checking
        if let Ok(monitor) = window.current_monitor() {
            if let Some(monitor) = monitor {
                let size = monitor.size();
                let scale_factor = monitor.scale_factor();

                // Logical pixel dimensions
                let screen_width = size.width as f64 / scale_factor;
                let screen_height = size.height as f64 / scale_factor;

                // Convert saved physical coords to logical (for bounds checking)
                let logical_x = saved_x / scale_factor;
                let logical_y = saved_y / scale_factor;

                // Bounds guard: make sure at least part of the toolbar is visible
                let min_visible = 50.0;  // At least 50px visible
                let max_x = screen_width - min_visible;
                let max_y = screen_height - min_visible;

                // Check whether out of bounds
                let is_out_of_bounds =
                    logical_x < -toolbar_size + min_visible ||
                    logical_y < -toolbar_size + min_visible ||
                    logical_x > max_x ||
                    logical_y > max_y;

                if is_out_of_bounds {
                    warn!("⚠️  Saved position is out of bounds — using default");
                    debug!("   Screen size: {}x{}, saved position (logical): ({}, {})",
                             screen_width, screen_height, logical_x, logical_y);
                    // Use the default (bottom-right)
                    let x = screen_width - toolbar_size - margin - 60.0;
                    let y = screen_height - toolbar_size - margin - 80.0;

                    if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                        (x * scale_factor) as i32,
                        (y * scale_factor) as i32
                    )) {
                        error!("❌ Failed to set default position: {}", e);
                    }
                } else {
                    // Position is valid — use it
                    debug!("✅ Position is valid — applying saved position");
                    if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                        saved_x as i32,
                        saved_y as i32
                    )) {
                        error!("❌ Failed to set toolbar position: {}", e);
                    }
                }
            } else {
                // Couldn't read monitor info — use saved position directly
                warn!("⚠️  Could not read monitor info — using saved position directly");
                if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                    saved_x as i32,
                    saved_y as i32
                )) {
                        error!("❌ Failed to set toolbar position: {}", e);
                }
            }
        } else {
            // Couldn't get monitor — use saved position directly
            warn!("⚠️  Could not get current monitor — using saved position directly");
            if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                saved_x as i32,
                saved_y as i32
            )) {
                        error!("❌ Failed to set toolbar position: {}", e);
            }
        }
    } else {
        // Get primary monitor info and compute bottom-right position
        if let Ok(monitor) = window.current_monitor() {
            if let Some(monitor) = monitor {
                let size = monitor.size();
                let scale_factor = monitor.scale_factor();

                // Logical pixel dimensions
                let screen_width = size.width as f64 / scale_factor;
                let screen_height = size.height as f64 / scale_factor;

                // Compute bottom-right position (account for taskbar)
                let x = screen_width - toolbar_size - margin - 60.0;
                let y = screen_height - toolbar_size - margin - 80.0;

                debug!("📍 Screen size: {}x{}, scale: {}, default toolbar position: ({}, {})",
                         screen_width, screen_height, scale_factor, x, y);

                // Convert to physical coords
                if let Err(e) = window.set_position(tauri::PhysicalPosition::new(
                    (x * scale_factor) as i32,
                    (y * scale_factor) as i32
                )) {
                        error!("❌ Failed to set toolbar position: {}", e);
                }
            }
        }
    }

    // Show the window
    if let Err(e) = window.show() {
        error!("❌ Failed to show toolbar window: {}", e);
    }

    debug!("✅ Toolbar window created");
    Ok(())
}

// Tauri command: create toolbar window
#[tauri::command]
pub async fn create_toolbar_window(app: AppHandle) -> Result<(), String> {
    create_toolbar_window_internal(&app)
}

// Tauri command: handle toolbar menu action
#[tauri::command]
pub async fn handle_toolbar_menu_action(app: AppHandle, action: String) -> Result<(), String> {
    debug!("🔧 Handling menu action: {}", action);
    handle_toolbar_menu_event(&app, &action);
    Ok(())
}

