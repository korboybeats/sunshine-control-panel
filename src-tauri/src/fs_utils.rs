use std::path::PathBuf;
use crate::sunshine;
use log::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

/// Scanned application info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedApp {
    pub name: String,
    pub cmd: String,
    #[serde(rename = "working-dir")]
    pub working_dir: String,
    pub source_path: String,
    #[serde(rename = "app-type")]
    pub app_type: String,
    #[serde(rename = "is-game", skip_serializing_if = "Option::is_none")]
    pub is_game: Option<bool>,
}

/// Platform game library scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformGame {
    pub name: String,
    pub app_id: String,
    pub platform: String,        // "steam", "epic", "gog"
    pub install_dir: String,
    pub exe_path: String,
    pub cmd: String,
    #[serde(rename = "working-dir")]
    pub working_dir: String,
    #[serde(rename = "cover-url", skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(rename = "size-on-disk", skip_serializing_if = "Option::is_none")]
    pub size_on_disk: Option<u64>,
}

/// Shortcut resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LnkInfo {
    pub name: String,
    #[serde(rename = "targetPath")]
    pub target_path: String,
    #[serde(rename = "workingDir")]
    pub working_dir: String,
    pub arguments: String,
}

/// Generic text file save (shows the system save dialog).
/// Callable by the remote WebUI page via __TAURI_INTERNALS__.invoke.
#[tauri::command]
pub async fn save_text_file(
    app: tauri::AppHandle,
    content: String,
    default_name: String,
    filter_name: String,
    extensions: Vec<String>,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
    let (tx, rx) = oneshot::channel();

    app.dialog()
        .file()
        .set_file_name(&default_name)
        .add_filter(&filter_name, &ext_refs)
        .save_file(move |file_path_opt| {
            let _ = tx.send(file_path_opt);
        });

    let file_path = rx.await
        .map_err(|_| "dialog channel error".to_string())?
        .ok_or_else(|| "cancelled".to_string())?;

    let path_buf = std::path::PathBuf::from(file_path.to_string());
    std::fs::write(&path_buf, &content)
        .map_err(|e| format!("write failed: {}", e))?;

    Ok(path_buf.display().to_string())
}

/// Get the list of ICC color profile files
#[tauri::command]
pub async fn get_icc_file_list() -> Result<Vec<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let color_dir = std::env::var("windir")
            .map(|windir| PathBuf::from(windir).join("System32\\spool\\drivers\\color"))
            .unwrap_or_else(|_| PathBuf::from("C:\\Windows\\System32\\spool\\drivers\\color"));

        match std::fs::read_dir(&color_dir) {
            Ok(entries) => {
                let mut files = Vec::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Some(file_name) = entry.file_name().to_str() {
                            // Only include .icc and .icm files
                            if file_name.ends_with(".icc") || file_name.ends_with(".icm") {
                                files.push(file_name.to_string());
                            }
                        }
                    }
                }
                files.sort();  // Sort alphabetically
                Ok(files)
            }
            Err(e) => Err(format!("Failed to read directory: {}", e)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(vec![])  // Return empty list on non-Windows systems
    }
}

/// Read the file list of the given directory
#[tauri::command]
pub async fn read_directory(path: String) -> Result<Vec<String>, String> {
    match std::fs::read_dir(&path) {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(file_name) = entry.file_name().to_str() {
                        files.push(file_name.to_string());
                    }
                }
            }
            files.sort();
            Ok(files)
        }
        Err(e) => Err(format!("Failed to read directory: {}", e)),
    }
}

/// Read an image file and return it as a Base64-encoded Data URL
#[tauri::command]
pub async fn read_image_as_data_url(path: String) -> Result<String, String> {
    use std::fs;
    use std::path::Path;

    // Read the file
    let file_bytes = fs::read(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    debug!("📖 File read successfully: {}, size: {} bytes", path, file_bytes.len());

    // Determine MIME type from extension
    let path_obj = Path::new(&path);
    let extension = path_obj.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime_type = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/png", // default
    };

    // Convert to Base64
    use base64::{Engine as _, engine::general_purpose};
    let base64 = general_purpose::STANDARD.encode(&file_bytes);

    // Build the Data URL
    let data_url = format!("data:{};base64,{}", mime_type, base64);

    debug!("✅ Data URL generated, MIME: {}, Base64 length: {}", mime_type, base64.len());

    Ok(data_url)
}

/// Copy an image file to the Sunshine assets directory.
/// Returns the URL path relative to the Sunshine web server (/boxart/xxx.jpg).
#[tauri::command]
pub async fn copy_image_to_assets(source_path: String) -> Result<String, String> {
    use std::fs;
    use std::path::Path;

    let source = Path::new(&source_path);

    // Verify the source file exists
    if !source.exists() {
        return Err(format!("Source file does not exist: {}", source_path));
    }

    let assets_dir = sunshine::assets_dir();

    // Create the assets directory if it does not exist
    fs::create_dir_all(&assets_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    // Get the file name
    let file_name = source.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid file name".to_string())?;

    // Generate a unique file name (to avoid overwriting)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let extension = source.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let unique_name = format!("bg_{}_{}.{}", timestamp, file_name.replace(|c: char| !c.is_alphanumeric(), "_"), extension);

    // Destination path
    let dest_path = assets_dir.join(&unique_name);

    // Copy the file
    fs::copy(source, &dest_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    info!("✅ Image copied to: {:?}", dest_path);

    // Return the URL path relative to the Sunshine web root
    let web_url = format!("/boxart/{}", unique_name);

    Ok(web_url)
}

/// Clean up unused cover images in the covers directory
#[tauri::command]
pub async fn cleanup_unused_covers() -> Result<serde_json::Value, String> {
    use std::fs;
    use std::collections::HashSet;
    use serde_json::json;

    info!("🧹 Starting cleanup of unused covers...");

    let covers_dir = sunshine::covers_dir();
    let apps_json_path = sunshine::config_dir().join("apps.json");

    debug!("📂 Using covers dir: {:?}", covers_dir);
    debug!("📄 Using apps.json path: {:?}", apps_json_path);

    // Read apps.json to collect all images currently in use
    let used_images: HashSet<String> = if apps_json_path.exists() {
        match fs::read_to_string(&apps_json_path) {
            Ok(content) => {
                // Check whether the file content is empty or whitespace-only
                let trimmed_content = content.trim();
                if trimmed_content.is_empty() {
                    warn!("⚠️  apps.json is empty; skipping parse");
                    HashSet::new()
                } else {
                    // Try to parse JSON
                    match serde_json::from_str::<serde_json::Value>(trimmed_content) {
                        Ok(apps) => {
                            let mut images = HashSet::new();

                            if let Some(apps_array) = apps.get("apps").and_then(|a| a.as_array()) {
                                for app in apps_array {
                                    if let Some(image_path) = app.get("image-path").and_then(|p| p.as_str()) {
                                        // Skip invalid or default images
                                        if image_path.is_empty() || image_path == "desktop" {
                                            continue;
                                        }

                                        // Extract file name (strip path)
                                        let filename = image_path.split('/').last()
                                            .or_else(|| image_path.split('\\').last())
                                            .unwrap_or(image_path);

                                        if !filename.is_empty() && filename != "desktop" {
                                            // Always save the file name
                                            images.insert(filename.to_string());

                                            // If the path contains a separator, also store the full path
                                            if image_path.contains('/') || image_path.contains('\\') {
                                                images.insert(image_path.to_string());
                                                debug!("  📌 In use: {} (full path: {})", filename, image_path);
                                            } else {
                                                debug!("  📌 In use: {}", filename);
                                            }
                                        }
                                    }
                                }
                            }
                            images
                        }
                        Err(e) => {
                            warn!("⚠️  Failed to parse apps.json: {}; skipping parse", e);
                            HashSet::new()
                        }
                    }
                }
            }
            Err(e) => {
                warn!("⚠️  Failed to read apps.json: {}; skipping parse", e);
                HashSet::new()
            }
        }
    } else {
        debug!("📄 apps.json does not exist; skipping parse");
        HashSet::new()
    };

    debug!("  covers in use: {}", used_images.len());

    let mut deleted_count = 0;
    let mut freed_space: u64 = 0;
    let mut errors = Vec::new();

    // === 1. Clean up unused covers in the covers directory ===
    if covers_dir.exists() {
        debug!("\n📂 Scanning covers directory...");
        let entries = fs::read_dir(&covers_dir)
            .map_err(|e| format!("Failed to read covers directory: {}", e))?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Safer check: see if the file name is referenced by any used path
                        let is_used = {
                            // Direct file name check
                            used_images.contains(filename) ||
                            // Check for any path that ends with this file name
                            used_images.iter().any(|used_path| {
                                used_path.ends_with(&format!("/{}", filename)) ||
                                used_path.ends_with(&format!("\\{}", filename)) ||
                                used_path == filename
                            })
                        };

                        if !is_used {
                            // Get file size
                            let size = fs::metadata(&path)
                                .map(|m| m.len())
                                .unwrap_or(0);

                            // Delete the file
                            match fs::remove_file(&path) {
                                Ok(_) => {
                                    debug!("  🗑️  [cover] {}", filename);
                                    deleted_count += 1;
                                    freed_space += size;
                                }
                                Err(e) => {
                                    let error_msg = format!("Failed to delete cover {}: {}", filename, e);
                                    error!("  ❌ {}", error_msg);
                                    errors.push(error_msg);
                                }
                            }
                        } else {
                            debug!("  ✅ [kept] {} (in use)", filename);
                        }
                    }
                }
            }
        }
    }

    // === 2. Clean up temp_ temporary files in the config directory ===
    let config_dir = sunshine::config_dir();
    debug!("\n📂 Scanning config directory for temporary files...");
    if config_dir.exists() {
        match fs::read_dir(&config_dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();

                        if path.is_file() {
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                // Remove temporary files starting with temp_
                                if filename.starts_with("temp_") {
                                    let size = fs::metadata(&path)
                                        .map(|m| m.len())
                                        .unwrap_or(0);

                                    match fs::remove_file(&path) {
                                        Ok(_) => {
                                            debug!("  🗑️  [temp] {}", filename);
                                            deleted_count += 1;
                                            freed_space += size;
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to delete temp file {}: {}", filename, e);
                                            error!("  ❌ {}", error_msg);
                                            errors.push(error_msg);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to read config directory: {}", e);
                warn!("  ⚠️  {}", error_msg);
                // Do not return an error; continue execution
            }
        }
    }

    let message = if deleted_count > 0 {
        format!("Deleted {} unused file(s), freed {:.2} KB", deleted_count, freed_space as f64 / 1024.0)
    } else {
        "No files found that needed cleaning up".to_string()
    };

    info!("\n✅ Cleanup complete: {}", message);
    
    Ok(json!({
        "success": true,
        "message": message,
        "deleted_count": deleted_count,
        "freed_space": freed_space,
        "errors": errors
    }))
}

/// Resolve a Windows shortcut (.lnk) file
#[tauri::command]
pub async fn resolve_lnk_target(lnk_path: String) -> Result<LnkInfo, String> {
    #[cfg(target_os = "windows")]
    {
        resolve_lnk_windows(&lnk_path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Shortcut resolution is only supported on Windows".to_string())
    }
}

#[cfg(target_os = "windows")]
fn resolve_lnk_windows(lnk_path: &str) -> Result<LnkInfo, String> {
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize,
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::Interface;
    use std::path::Path;
    
    info!("🔗 Resolving shortcut: {}", lnk_path);

    // Initialize COM
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let result = (|| -> Result<LnkInfo, String> {
        // Create a ShellLink object
        let shell_link: IShellLinkW = unsafe {
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("Failed to create ShellLink: {:?}", e))?
        };

        // Query the IPersistFile interface
        let persist_file: IPersistFile = shell_link.cast()
            .map_err(|e| format!("Failed to query IPersistFile: {:?}", e))?;

        // Load the .lnk file
        let wide_path: Vec<u16> = OsStr::new(lnk_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            persist_file.Load(
                windows::core::PCWSTR(wide_path.as_ptr()),
                STGM_READ,
            ).map_err(|e| format!("Failed to load .lnk file: {:?}", e))?;
        }

        // Fetch the target path
        let mut target_path_buf: [u16; 260] = [0; 260];
        let mut find_data: windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };

        unsafe {
            shell_link.GetPath(
                &mut target_path_buf,
                &mut find_data,
                windows::Win32::UI::Shell::SLGP_RAWPATH.0 as u32,
            ).map_err(|e| format!("Failed to get target path: {:?}", e))?;
        }

        let target_path = String::from_utf16_lossy(
            &target_path_buf[..target_path_buf.iter().position(|&c| c == 0).unwrap_or(target_path_buf.len())]
        );

        // Fetch the working directory
        let mut working_dir_buf: [u16; 260] = [0; 260];
        unsafe {
            let _ = shell_link.GetWorkingDirectory(&mut working_dir_buf);
        }

        let working_dir = String::from_utf16_lossy(
            &working_dir_buf[..working_dir_buf.iter().position(|&c| c == 0).unwrap_or(working_dir_buf.len())]
        );

        // Fetch the arguments
        let mut arguments_buf: [u16; 1024] = [0; 1024];
        unsafe {
            let _ = shell_link.GetArguments(&mut arguments_buf);
        }

        let arguments = String::from_utf16_lossy(
            &arguments_buf[..arguments_buf.iter().position(|&c| c == 0).unwrap_or(arguments_buf.len())]
        );

        // Derive the name from the .lnk file name
        let lnk_file_path = Path::new(lnk_path);
        let name = lnk_file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        debug!("✅ Shortcut resolved:");
        debug!("   name: {}", name);
        debug!("   target: {}", target_path);
        debug!("   working dir: {}", working_dir);
        debug!("   arguments: {}", arguments);

        Ok(LnkInfo {
            name,
            target_path,
            working_dir,
            arguments,
        })
    })();

    // Uninitialize COM
    unsafe {
        CoUninitialize();
    }

    result
}

/// Scan a directory for executables and shortcuts.
/// Returns the list of apps found.
#[tauri::command]
pub async fn scan_directory_for_apps(directory: String) -> Result<Vec<ScannedApp>, String> {
    use std::path::Path;

    info!("📂 Starting directory scan: {}", directory);

    let dir_path = Path::new(&directory);
    if !dir_path.exists() {
        return Err(format!("Directory does not exist: {}", directory));
    }

    if !dir_path.is_dir() {
        return Err(format!("Path is not a directory: {}", directory));
    }

    let mut apps: Vec<ScannedApp> = Vec::new();

    // Supported file extensions
    let supported_extensions = [".lnk", ".exe", ".bat", ".cmd", ".url"];

    // Recursively scan the directory
    scan_directory_recursive(dir_path, &supported_extensions, &mut apps)?;

    info!("✅ Scan complete, found {} app(s)", apps.len());
    Ok(apps)
}

/// Detect whether an app is a game.
/// Based on path, file name, and common game-platform directories.
fn detect_if_game(file_path: &str, name: &str, target_path: Option<&str>) -> bool {
    let path_lower = file_path.to_lowercase();
    let name_lower = name.to_lowercase();
    let target_lower = target_path.map(|s| s.to_lowercase()).unwrap_or_default();

    // Non-.exe files are definitely not games.
    // Check whether the file path or target path ends with .exe.
    let is_exe = path_lower.ends_with(".exe") ||
                 target_lower.ends_with(".exe") ||
                 // For .lnk shortcuts, check whether the target is a .exe
                 (path_lower.ends_with(".lnk") && target_lower.ends_with(".exe"));

    if !is_exe && !path_lower.ends_with(".lnk") {
        return false;
    }

    // For .lnk files, if the target is not a .exe, it's not a game either.
    if path_lower.ends_with(".lnk") && !target_lower.is_empty() && !target_lower.ends_with(".exe") {
        return false;
    }

    // First exclude apps that are clearly not games
    let exclude_keywords = [
        "uninstall", "卸载", "setup", "安装", "installer",
        "update", "更新", "updater", "patch",
        "config", "配置", "settings", "设置",
        "crash", "崩溃", "reporter", "report",
        "helper", "service", "daemon",
        "redist", "redistributable", "vcredist", "directx",
        "launcher_helper", "bootstrapper",
        "ue4prereqsetup", "dxsetup", "dotnet",
        // Common non-game apps
        "chrome", "firefox", "edge", "opera", "brave",
        "word", "excel", "powerpoint", "outlook", "onenote", "access",
        "visual studio", "vscode", "code", "notepad", "sublime",
        "git", "node", "python", "java", "ruby",
        "adobe", "photoshop", "illustrator", "premiere", "after effects",
        "spotify", "discord", "telegram", "wechat", "微信", "qq",
        "obs", "vlc", "potplayer", "media player",
        "7-zip", "winrar", "bandizip",
        "driver", "nvidia", "amd ", "intel",
        "antivirus", "defender", "kaspersky", "avast",
        "office", "onedrive", "teams",
        "terminal", "powershell", "cmd",
        "control panel", "控制面板",
        "explorer", "task manager", "任务管理器",
        "calculator", "计算器", "paint", "画图",
        "snipping", "截图",
    ];

    for keyword in &exclude_keywords {
        if name_lower.contains(keyword) || path_lower.ends_with(&format!("\\{}.exe", keyword)) {
            return false;
        }
    }

    // Game-platform path keywords (high confidence)
    let high_confidence_paths = [
        "\\steamapps\\common\\",
        "\\steam\\steamapps\\common\\",
        "\\epic games\\",
        "\\gog galaxy\\games\\",
        "\\gog games\\",
        "\\ubisoft\\ubisoft game launcher\\games\\",
        "\\origin games\\",
        "\\ea games\\",
        "\\battle.net\\",
        "\\riot games\\",
        "\\xbox games\\",
        "\\playnite\\",
    ];
    
    // 检查路径中是否包含高置信度的游戏平台路径
    for keyword in &high_confidence_paths {
        if path_lower.contains(keyword) || target_lower.contains(keyword) {
            return true;
        }
    }
    
    // 中等置信度：检查是否在 Program Files 下的 games 目录
    let medium_confidence_paths = [
        "\\program files\\games\\",
        "\\program files\\game\\",
        "\\program files (x86)\\games\\",
        "\\program files (x86)\\game\\",
    ];
    
    for keyword in &medium_confidence_paths {
        if path_lower.contains(keyword) || target_lower.contains(keyword) {
            // 额外检查：确保不是工具类应用
            let tool_indicators = ["tool", "editor", "sdk", "dev", "debug", "server", "manager", "launcher"];
            let is_tool = tool_indicators.iter().any(|t| name_lower.contains(t));
            if !is_tool {
                return true;
            }
        }
    }
    
    // 检查快捷方式来源目录（如果是从开始菜单的游戏文件夹扫描的）
    if path_lower.contains("\\start menu\\programs\\games\\") ||
       path_lower.contains("\\开始菜单\\程序\\游戏\\") {
        return true;
    }
    
    // 低置信度：仅基于文件名判断（需要更严格的条件）
    // 不再仅凭 "game" 关键词判断，因为误报率太高
    
    false
}

/// 递归扫描目录
fn scan_directory_recursive(
    dir_path: &std::path::Path,
    supported_extensions: &[&str],
    apps: &mut Vec<ScannedApp>,
) -> Result<(), String> {
    use std::fs;
    
    // 读取目录内容
    let entries = fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;
    
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let path = entry.path();
        
        // 如果是目录，递归扫描
        if path.is_dir() {
            // 跳过一些常见的系统目录和隐藏目录
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name.starts_with('.') || 
                   dir_name.eq_ignore_ascii_case("$RECYCLE.BIN") ||
                   dir_name.eq_ignore_ascii_case("System Volume Information") {
                    continue;
                }
            }
            
            // 递归扫描子目录，忽略权限错误
            let _ = scan_directory_recursive(&path, supported_extensions, apps);
            continue;
        }
        
        // 只处理文件
        if !path.is_file() {
            continue;
        }
        
        let _file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => format!(".{}", e.to_lowercase()),
            None => continue,
        };
        
        // 检查是否是支持的扩展名
        if !supported_extensions.contains(&ext.as_str()) {
            continue;
        }
        
        let file_path = path.to_string_lossy().to_string();
        debug!("📄 找到文件: {}", file_path);
        
        // 根据文件类型处理
        let scanned_app = match ext.as_str() {
            ".lnk" => {
                #[cfg(target_os = "windows")]
                {
                    process_lnk_file(&file_path)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    None
                }
            }
            ".exe" => {
                process_exe_file(&file_path)
            }
            ".bat" | ".cmd" => {
                process_batch_file(&file_path)
            }
            ".url" => {
                process_url_file(&file_path)
            }
            _ => None,
        };
        
        if let Some(mut app) = scanned_app {
            // 检测是否是游戏
            let target_path = if app.app_type == "shortcut" {
                #[cfg(target_os = "windows")]
                {
                    resolve_lnk_windows(&file_path).ok()
                        .map(|lnk| lnk.target_path)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    None
                }
            } else {
                None
            };
            
            let is_game = detect_if_game(&file_path, &app.name, target_path.as_deref());
            app.is_game = Some(is_game);
            apps.push(app);
        }
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
fn process_lnk_file(file_path: &str) -> Option<ScannedApp> {
    let lnk_info = resolve_lnk_windows(file_path).ok()?;
    
    let cmd = format!("\"{}\"", file_path);
    
    Some(ScannedApp {
        name: lnk_info.name,
        cmd,
        working_dir: String::new(),
        source_path: file_path.to_string(),
        app_type: "shortcut".to_string(),
        is_game: None, // 将在扫描时检测
    })
}

fn process_exe_file(file_path: &str) -> Option<ScannedApp> {
    use std::path::Path;
    
    let path = Path::new(file_path);
    let name = path.file_stem()?.to_str()?.to_string();
    let working_dir = path.parent()?.to_string_lossy().to_string();
    let cmd = format!("\"{}\"", file_path);
    
    Some(ScannedApp {
        name,
        cmd,
        working_dir,
        source_path: file_path.to_string(),
        app_type: "executable".to_string(),
        is_game: None, // 将在扫描时检测
    })
}

fn process_batch_file(file_path: &str) -> Option<ScannedApp> {
    use std::path::Path;
    
    let path = Path::new(file_path);
    let name = path.file_stem()?.to_str()?.to_string();
    let working_dir = path.parent()?.to_string_lossy().to_string();
    let cmd = format!("cmd /c \"{}\"", file_path);
    let ext = path.extension()?.to_str()?.to_lowercase();
    let app_type = if ext == "bat" { "batch" } else { "command" };
    
    Some(ScannedApp {
        name,
        cmd,
        working_dir,
        source_path: file_path.to_string(),
        app_type: app_type.to_string(),
        is_game: None, // 批处理和命令脚本通常不是游戏
    })
}

fn process_url_file(file_path: &str) -> Option<ScannedApp> {
    use std::path::Path;
    
    let path = Path::new(file_path);
    let name = path.file_stem()?.to_str()?.to_string();
    let cmd = format!("start \"\" \"{}\"", file_path);
    
    Some(ScannedApp {
        name,
        cmd,
        working_dir: String::new(),
        source_path: file_path.to_string(),
        app_type: "url".to_string(),
        is_game: None, // URL 文件通常不是游戏
    })
}

// ======================================================================
// 平台游戏库扫描
// ======================================================================

/// 扫描结果汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLibraryScanResult {
    pub steam: Vec<PlatformGame>,
    pub epic: Vec<PlatformGame>,
    pub gog: Vec<PlatformGame>,
    pub total: usize,
    pub scan_time_ms: u64,
}

/// 统一扫描所有游戏平台库
#[tauri::command]
pub async fn scan_game_libraries() -> Result<GameLibraryScanResult, String> {
    use std::time::Instant;

    let start = Instant::now();
    info!("🎮 开始扫描游戏平台库...");

    let steam = scan_steam_library();
    let epic = scan_epic_library();
    let gog = scan_gog_library();

    let total = steam.len() + epic.len() + gog.len();
    let elapsed = start.elapsed().as_millis() as u64;

    info!("✅ 游戏库扫描完成: Steam={}, Epic={}, GOG={}, 总计={}, 耗时={}ms",
        steam.len(), epic.len(), gog.len(), total, elapsed);

    Ok(GameLibraryScanResult {
        steam,
        epic,
        gog,
        total,
        scan_time_ms: elapsed,
    })
}

// ==================== Steam ====================

/// 扫描 Steam 游戏库
fn scan_steam_library() -> Vec<PlatformGame> {
    let mut games = Vec::new();

    // 查找 Steam 安装路径
    let steam_path = find_steam_path();
    let steam_path = match steam_path {
        Some(p) => p,
        None => {
            info!("Steam 未安装或未找到");
            return games;
        }
    };

    info!("📂 Steam 路径: {}", steam_path.display());

    // 读取 libraryfolders.vdf 获取所有库路径
    let library_folders = get_steam_library_folders(&steam_path);
    info!("📚 找到 {} 个 Steam 库路径", library_folders.len());

    for lib_path in &library_folders {
        let steamapps = lib_path.join("steamapps");
        if !steamapps.exists() {
            continue;
        }

        // 扫描 appmanifest_*.acf
        if let Ok(entries) = std::fs::read_dir(&steamapps) {
            for entry in entries.flatten() {
                let fname = entry.file_name();
                let fname = fname.to_string_lossy();
                if fname.starts_with("appmanifest_") && fname.ends_with(".acf") {
                    if let Some(game) = parse_steam_acf(&entry.path(), &steamapps) {
                        games.push(game);
                    }
                }
            }
        }
    }

    games
}

/// 查找 Steam 安装路径
fn find_steam_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // 尝试注册表
        use winreg::RegKey;
        use winreg::enums::HKEY_LOCAL_MACHINE;

        if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\WOW6432Node\\Valve\\Steam")
        {
            if let Ok(path) = hklm.get_value::<String, _>("InstallPath") {
                let p = PathBuf::from(&path);
                if p.exists() {
                    return Some(p);
                }
            }
        }

        // 备选：默认路径
        let default = PathBuf::from("C:\\Program Files (x86)\\Steam");
        if default.exists() {
            return Some(default);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").ok()?;
        let paths = [
            format!("{}/.steam/steam", home),
            format!("{}/.local/share/Steam", home),
        ];
        for p in &paths {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// 从 libraryfolders.vdf 读取所有 Steam 库路径
fn get_steam_library_folders(steam_path: &PathBuf) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    // Steam 自身路径永远是一个库
    folders.push(steam_path.clone());

    let vdf_path = steam_path.join("steamapps").join("libraryfolders.vdf");
    if !vdf_path.exists() {
        // 旧版本路径
        let alt = steam_path.join("config").join("libraryfolders.vdf");
        if alt.exists() {
            parse_library_folders_vdf(&alt, &mut folders);
        }
        return folders;
    }

    parse_library_folders_vdf(&vdf_path, &mut folders);
    folders
}

/// 简单的 VDF 解析器 — 提取 "path" 字段
fn parse_library_folders_vdf(path: &PathBuf, folders: &mut Vec<PathBuf>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("读取 VDF 失败: {}: {}", path.display(), e);
            return;
        }
    };

    // VDF 格式: "path"		"D:\\SteamLibrary"
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"path\"") {
            let value = rest.trim().trim_matches('"');
            if !value.is_empty() {
                let p = PathBuf::from(value);
                if p.exists() && !folders.contains(&p) {
                    folders.push(p);
                }
            }
        }
    }
}

/// 解析 Steam appmanifest ACF 文件
fn parse_steam_acf(acf_path: &PathBuf, steamapps_dir: &PathBuf) -> Option<PlatformGame> {
    let content = std::fs::read_to_string(acf_path).ok()?;

    let app_id = extract_vdf_value(&content, "appid")?;
    let name = extract_vdf_value(&content, "name")?;
    let install_dir_name = extract_vdf_value(&content, "installdir")?;
    let size_str = extract_vdf_value(&content, "SizeOnDisk");

    // 排除 Steamworks 工具类
    let app_id_num: u64 = app_id.parse().unwrap_or(0);
    if app_id_num < 10 {
        return None; // Steam 自身的工具
    }

    // 排除名称包含工具/SDK/运行时等关键词的条目
    let name_lower = name.to_lowercase();
    let exclude_keywords = [
        "redistributable", " sdk", "dedicated server", "proton ",
        "steam linux runtime", "steamworks",
        "directx", "vcredist", "visual c++",
        "common redist", "mod tool", "editor",
        "soundtrack", "ost", "artbook", "art book",
        "benchmark", "demo", " test",
        "developer tool", "devkit",
    ];
    if exclude_keywords.iter().any(|kw| name_lower.contains(kw)) {
        return None;
    }

    // 检查 Steam ACF 中的 apptype（如果有的话），排除 tool / demo / music 类型
    let app_type = extract_vdf_value(&content, "apptype")
        .unwrap_or_default()
        .to_lowercase();
    if matches!(app_type.as_str(), "tool" | "demo" | "music" | "dlc" | "config" | "media") {
        return None;
    }

    let install_dir = steamapps_dir.join("common").join(&install_dir_name);
    let install_dir_str = install_dir.to_string_lossy().to_string();

    // 尝试找到主 exe
    let exe_path = find_main_exe(&install_dir).unwrap_or_default();

    // 使用 steam:// URL 启动（最可靠的方式）
    let cmd = format!("steam://rungameid/{}", app_id);

    let cover_url = Some(format!(
        "https://cdn.akamai.steamstatic.com/steam/apps/{}/header.jpg",
        app_id
    ));

    let size_on_disk = size_str.and_then(|s| s.parse::<u64>().ok());

    Some(PlatformGame {
        name,
        app_id,
        platform: "steam".to_string(),
        install_dir: install_dir_str,
        exe_path,
        cmd,
        working_dir: install_dir.to_string_lossy().to_string(),
        cover_url,
        size_on_disk,
    })
}

/// 从 VDF/ACF 内容中提取键值对
fn extract_vdf_value(content: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&pattern) {
            let value = rest.trim().trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// 在安装目录中查找主可执行文件
fn find_main_exe(install_dir: &PathBuf) -> Option<String> {
    if !install_dir.exists() {
        return None;
    }

    // 只搜索根目录和一层子目录
    let mut candidates: Vec<(PathBuf, u64)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(install_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "exe" {
                        let name_lower = path.file_name()
                            .map(|n| n.to_string_lossy().to_lowercase())
                            .unwrap_or_default();
                        // 排除工具
                        if !is_tool_exe(&name_lower) {
                            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            candidates.push((path, size));
                        }
                    }
                }
            }
        }
    }

    // 按大小排序，取最大的（通常是主程序）
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    candidates.first().map(|(p, _)| p.to_string_lossy().to_string())
}

/// 检查 exe 是否是工具/辅助程序
fn is_tool_exe(name_lower: &str) -> bool {
    let tools = [
        "uninstall", "uninst", "setup", "install", "update", "updater",
        "crash", "reporter", "helper", "service", "launcher_helper",
        "redist", "vcredist", "dxsetup", "dotnet", "ue4prereq",
        "bootstrapper", "cleanup", "repair",
    ];
    tools.iter().any(|t| name_lower.contains(t))
}

// ==================== Epic Games ====================

/// 扫描 Epic Games 库
fn scan_epic_library() -> Vec<PlatformGame> {
    let mut games = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // Epic 清单目录
        let manifests_dir = std::env::var("ProgramData")
            .map(|pd| PathBuf::from(pd).join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests"))
            .unwrap_or_else(|_| PathBuf::from("C:\\ProgramData\\Epic\\EpicGamesLauncher\\Data\\Manifests"));

        if !manifests_dir.exists() {
            info!("Epic Games 清单目录不存在: {}", manifests_dir.display());
            return games;
        }

        info!("📂 Epic Games 清单目录: {}", manifests_dir.display());

        if let Ok(entries) = std::fs::read_dir(&manifests_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "item").unwrap_or(false) {
                    if let Some(game) = parse_epic_manifest(&path) {
                        games.push(game);
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        info!("Epic Games 扫描仅支持 Windows");
    }

    games
}

/// 解析 Epic Games .item 清单文件（JSON 格式）
#[cfg(target_os = "windows")]
fn parse_epic_manifest(manifest_path: &PathBuf) -> Option<PlatformGame> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let display_name = json.get("DisplayName")?.as_str()?.to_string();
    let install_location = json.get("InstallLocation")?.as_str()?.to_string();
    let app_name = json.get("AppName")?.as_str()?.to_string();
    let launch_executable = json.get("LaunchExecutable")?.as_str()?.to_string();

    let install_dir = PathBuf::from(&install_location);
    let exe_path = install_dir.join(&launch_executable);
    let exe_str = exe_path.to_string_lossy().to_string();

    // Epic 启动命令
    let cmd = format!("com.epicgames.launcher://apps/{}?action=launch&silent=true", app_name);

    let size_on_disk = json.get("InstallSize").and_then(|v| v.as_u64());

    Some(PlatformGame {
        name: display_name,
        app_id: app_name,
        platform: "epic".to_string(),
        install_dir: install_location,
        exe_path: exe_str,
        cmd,
        working_dir: install_dir.to_string_lossy().to_string(),
        cover_url: None, // Epic 没有简单的封面 URL
        size_on_disk,
    })
}

// ==================== GOG Galaxy ====================

/// 扫描 GOG Galaxy 游戏库
fn scan_gog_library() -> Vec<PlatformGame> {
    #[cfg(target_os = "windows")]
    {
        // GOG Galaxy 数据库是加密的 SQLite，改用注册表方式
        return scan_gog_from_registry();
    }

    #[cfg(not(target_os = "windows"))]
    {
        info!("GOG 扫描仅支持 Windows");
        return Vec::new();
    }
}

/// 通过 Windows 注册表扫描 GOG 游戏
#[cfg(target_os = "windows")]
fn scan_gog_from_registry() -> Vec<PlatformGame> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let mut games = Vec::new();

    let gog_key = match RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SOFTWARE\\WOW6432Node\\GOG.com\\Games")
    {
        Ok(key) => key,
        Err(_) => {
            info!("GOG 注册表键不存在");
            return games;
        }
    };

    if let Ok(subkeys) = gog_key.enum_keys().collect::<Result<Vec<String>, _>>() {
        for game_id in &subkeys {
            if let Ok(game_key) = gog_key.open_subkey(game_id) {
                let name: String = game_key.get_value("gameName").unwrap_or_default();
                let path: String = game_key.get_value("path").unwrap_or_default();
                let exe: String = game_key.get_value("exe").unwrap_or_default();

                if name.is_empty() || path.is_empty() {
                    continue;
                }

                let exe_path = if exe.is_empty() {
                    find_main_exe(&PathBuf::from(&path)).unwrap_or_default()
                } else {
                    exe.clone()
                };

                let cmd = if exe_path.is_empty() {
                    format!("goggalaxy://openGameView/{}", game_id)
                } else {
                    format!("\"{}\"", exe_path)
                };

                games.push(PlatformGame {
                    name,
                    app_id: game_id.clone(),
                    platform: "gog".to_string(),
                    install_dir: path.clone(),
                    exe_path,
                    cmd,
                    working_dir: path,
                    cover_url: None,
                    size_on_disk: None,
                });
            }
        }
    }

    games
}

/// Steam Store 搜索结果（返回给前端选择）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamCoverCandidate {
    pub steam_id: u64,
    pub name: String,
    /// header.jpg 完整 URL
    pub header_url: String,
    /// 小缩略图 URL（适用于列表预览）
    pub tiny_image: String,
}

#[derive(Debug, Deserialize)]
struct SteamSearchResponse {
    items: Vec<SteamSearchItem>,
}

#[derive(Debug, Deserialize)]
struct SteamSearchItem {
    id: u64,
    name: String,
    #[serde(default)]
    tiny_image: String,
}

/// 通过 Steam Store 搜索 API 按名称查找游戏，返回候选列表供用户选择
#[tauri::command]
pub async fn search_steam_covers(query: String) -> Result<Vec<SteamCoverCandidate>, String> {
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    let client = crate::commands::cdn_client();

    let search_url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&l=english&cc=US",
        url_percent_encode(&query)
    );
    info!("🔍 搜索 Steam Store: {}", search_url);

    let resp = client.get(&search_url).send().await
        .map_err(|e| format!("Steam search failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Steam search failed: HTTP {}", resp.status()));
    }

    let data: SteamSearchResponse = resp.json().await
        .map_err(|e| format!("Failed to parse search results: {}", e))?;

    if data.items.is_empty() {
        return Err(format!("\"{}\" was not found on Steam", query));
    }

    let candidates: Vec<SteamCoverCandidate> = data.items.into_iter().take(6).map(|item| {
        SteamCoverCandidate {
            header_url: format!(
                "https://cdn.akamai.steamstatic.com/steam/apps/{}/header.jpg",
                item.id
            ),
            steam_id: item.id,
            name: item.name,
            tiny_image: item.tiny_image,
        }
    }).collect();

    info!("✅ 找到 {} 个候选封面", candidates.len());
    Ok(candidates)
}

/// URL percent encoding（与 JS encodeURIComponent 行为一致）
fn url_percent_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' | b'!' | b'\'' | b'(' | b')' | b'*' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

/// 下载指定 Steam 封面并通过 Sunshine HTTP API 上传
/// 通过 Sunshine 服务进程写入 covers 目录（解决权限问题）
#[tauri::command]
pub async fn upload_steam_cover(
    header_url: String,
    app_name: String,
    proxy_url: String,
) -> Result<String, String> {
    use base64::Engine as _;

    // 安全检查：仅允许 Steam CDN
    if !header_url.starts_with("https://cdn.akamai.steamstatic.com/") {
        return Err(format!("Downloads from this domain are not allowed: {}", header_url));
    }

    let client = crate::commands::cdn_client();

    // 1. 下载封面图片
    let resp = client.get(&header_url).send().await
        .map_err(|e| format!("Failed to download cover: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().await
        .map_err(|e| format!("Failed to read cover: {}", e))?;

    if bytes.is_empty() {
        return Err("Downloaded cover is empty".to_string());
    }

    // 2. 转 base64
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    // 3. 通过 Sunshine API 上传
    let upload_url = format!("{}/api/covers/upload", proxy_url);
    let payload = serde_json::json!({
        "key": app_name,
        "data": b64,
    });

    let upload_resp = client.post(&upload_url)
        .json(&payload)
        .send().await
        .map_err(|e| format!("Failed to upload cover: {}", e))?;

    if !upload_resp.status().is_success() {
        return Err(format!("Upload failed: HTTP {}", upload_resp.status()));
    }

    info!("✅ 封面已上传: {} ({} bytes)", app_name, bytes.len());
    Ok(app_name)
}
