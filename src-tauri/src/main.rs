// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod vdd;
mod vmouse;
mod rtss;
mod hwinfo;
mod system;
mod sunshine;
mod utils;
mod proxy_server;
mod fs_utils;
mod toolbar;
mod update;
mod logger;
mod tray;
mod windows;
mod app;
mod commands;
mod moonlight_web;

use log::info;

fn main() {
    // Set WebView2 browser arguments to optimize GPU usage and security policy
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", [
            // Security: ignore self-signed certificate errors (for local Sunshine connections)
            "--ignore-certificate-errors",
            // Throttling: aggressive background / hidden-tab timer throttling
            "--enable-features=IntensiveWakeUpThrottling,ThrottleDisplayNoneAndVisibilityHiddenCrossOriginIframes",
            // GPU optimization: disable Edge-specific UI overlays (fewer unnecessary GPU compositor layers)
            "--disable-features=msWebOOUI",
            // GPU optimization: disable the GPU shader disk cache to reduce VRAM usage
            "--disable-gpu-shader-disk-cache",
            // GPU optimization: turn off GPU rasterization anti-aliasing (control panel UI does not need MSAA)
            "--gpu-rasterization-msaa-sample-count=0",
            // GPU optimization: limit the number of renderer processes, reducing GPU context switch overhead
            "--renderer-process-limit=1",
        ].join(" "));
    }
    
    tauri::Builder::default()
        .manage(app::AppState {
            main_window: std::sync::Mutex::new(None),
        })
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            app::handle_single_instance(app, args);
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Initialize the logging system (needs the app handle from setup)
            logger::init_logger(app.handle().clone());
            info!("🚀 Sunshine Control Panel starting...");
            
            app::setup_application(app)
        })
        .on_window_event(|window, event| {
            windows::handle_window_event(window, event);
        })
        .invoke_handler(tauri::generate_handler![
            commands::toggle_dark_mode,
            toolbar::handle_toolbar_menu_action,
            toolbar::save_toolbar_position,
            system::get_current_dpi,
            system::set_desktop_dpi,
            commands::open_tool_window,
            commands::launch_app,
            toolbar::create_toolbar_window,
            commands::fetch_speech_phrases,
            commands::fetch_remote_bytes,
            commands::ai_api_proxy,
            commands::capture_screenshot,
            vdd::get_vdd_settings_file_path,
            vdd::get_vdd_tools_dir_path,
            vdd::get_vdd_edid_file_path,
            vdd::load_vdd_settings,
            vdd::save_vdd_settings,
            vdd::exec_pipe_cmd,
            vdd::upload_edid_file,
            vdd::read_edid_file,
            vdd::delete_edid_file,
            system::get_gpus,
            system::get_system_info,
            system::get_process_memory_info,
            system::get_sunshine_start_time,
            sunshine::get_sunshine_install_path,
            sunshine::get_sunshine_version,
            sunshine::parse_sunshine_config,
            sunshine::get_sunshine_url,
            sunshine::get_command_line_url,
            sunshine::get_sunshine_locale,
            sunshine::set_sunshine_locale,
            sunshine::get_active_sessions,
            sunshine::change_bitrate,
            sunshine::toggle_sunshine_mode,
            sunshine::is_sunshine_running_in_user_mode,
            sunshine::restart_sunshine_in_user_mode,
            sunshine::restart_sunshine_service,
            proxy_server::get_proxy_url_command,
            utils::open_external_url,
            utils::restart_graphics_driver,
            utils::restart_as_admin,
            utils::is_running_as_admin,
            vdd::uninstall_vdd_driver,
            vmouse::get_vmouse_status,
            vmouse::install_vmouse_driver,
            vmouse::uninstall_vmouse_driver,
            vmouse::set_vmouse_config,
            fs_utils::get_icc_file_list,
            fs_utils::read_directory,
            fs_utils::read_image_as_data_url,
            fs_utils::copy_image_to_assets,
            fs_utils::cleanup_unused_covers,
            fs_utils::resolve_lnk_target,
            fs_utils::scan_directory_for_apps,
            fs_utils::scan_game_libraries,
            fs_utils::search_steam_covers,
            fs_utils::upload_steam_cover,
            fs_utils::save_text_file,
            update::check_for_updates,
            update::get_include_prerelease_preference,
            update::set_include_prerelease_preference,
            update::download_update,
            update::install_update,
            logger::get_all_logs,
            logger::clear_logs,
            logger::export_logs,
            moonlight_web::moonlight_web_get_status,
            moonlight_web::moonlight_web_start,
            moonlight_web::moonlight_web_stop,
            moonlight_web::moonlight_web_get_config,
            moonlight_web::moonlight_web_save_config,
            moonlight_web::moonlight_web_check_release,
            moonlight_web::moonlight_web_download,
            moonlight_web::moonlight_web_get_install_path,
            moonlight_web::moonlight_web_generate_cert,
            windows::_webview_heartbeat,
            rtss::get_rtss_status,
            rtss::rtss_set_osd,
            rtss::rtss_clear_osd,
            rtss::rtss_set_framerate_limit,
            rtss::rtss_get_framerate_limit,
            rtss::rtss_toggle_limiter,
            rtss::rtss_get_limiter_status,
            rtss::rtss_toggle_overlay,
            rtss::rtss_download_cli,
            rtss::rtss_get_available_metrics,
            rtss::rtss_start_monitoring,
            rtss::rtss_stop_monitoring,
            rtss::rtss_get_monitoring_status,
            rtss::rtss_get_osd_properties,
            rtss::rtss_set_osd_property,
            hwinfo::hwinfo_get_sensors,
            hwinfo::hwinfo_get_readings,
            hwinfo::hwinfo_check_available,
            tray::set_tray_locale,
            tray::get_tray_locale,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
