use tauri::{AppHandle, Manager, Emitter};
use log::{info, warn};
use crate::windows;
use base64::Engine as _;
use std::sync::OnceLock;
use serde::{Serialize, Deserialize};

/// Shared CDN HTTP client (reuses connection pool to avoid CDN rejecting frequent TLS handshakes)
pub fn cdn_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(5)
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create CDN HTTP client")
    })
}

/// CDN response (distinguishes "not modified" vs. "fresh data")
enum CdnResult {
    /// 304 Not Modified — resource unchanged
    NotModified,
    /// 200 OK — returns the response and ETag
    Fresh(reqwest::Response, Option<String>),
}

/// CDN GET with one retry; supports ETag conditional requests
async fn cdn_get(url: &str, if_none_match: Option<&str>) -> Result<CdnResult, String> {
    let client = cdn_client();
    
    let build_request = || {
        let mut req = client.get(url);
        if let Some(etag) = if_none_match {
            req = req.header("If-None-Match", etag);
        }
        req
    };
    
    let result = build_request().send().await;
    let response = match result {
        Ok(resp) => resp,
        Err(_first_err) => {
            warn!("⚠️  CDN request failed — retrying in 1s");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            build_request().send().await
                .map_err(|e| format!("Request failed (still failed after retry): {}", e))?
        }
    };

    // 304 Not Modified
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(CdnResult::NotModified);
    }

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }
    
    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    Ok(CdnResult::Fresh(response, etag))
}

#[tauri::command]
pub async fn toggle_dark_mode(_window: tauri::Window) -> Result<bool, String> {
    // Tauri controls the theme via the frontend — this is just a stub
    Ok(true)
}

#[tauri::command]
pub async fn open_tool_window(app: AppHandle, tool_name: String) -> Result<(), String> {
    info!("🔧 Opening tool window: {}", tool_name);
    
    match tool_name.as_str() {
        "main" => {
            if let Some(window) = app.get_webview_window("main") {
                windows::show_and_activate_window(&window);
            }
        }
        "vdd" => {
            if let Some(window) = app.get_webview_window("main") {
                windows::show_and_activate_window(&window);
                let _ = window.emit("open-vdd-settings", ());
            }
        }
        "about" => {
            windows::open_about_window(&app)?;
        }
        _ => return Err(format!("Unknown tool name: {}", tool_name)),
    }
    Ok(())
}

#[tauri::command]
pub async fn launch_app(cmd: String, working_dir: Option<String>, elevated: Option<bool>) -> Result<(), String> {
    if cmd.trim().is_empty() {
        return Err("Launch command must not be empty".to_string());
    }
    info!("🚀 Launching app: {}", cmd);

    let is_elevated = elevated.unwrap_or(false);

    tokio::task::spawn_blocking(move || {
        use ::windows::core::PCWSTR;
        use ::windows::Win32::Foundation::HWND;
        use ::windows::Win32::UI::Shell::ShellExecuteW;
        use ::windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        fn to_wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0u16)).collect()
        }

        fn shell_exec(
            operation: &[u16], file: &str, params: &str,
            dir_wide: Option<&Vec<u16>>,
        ) -> isize {
            let file_wide = to_wide(file);
            let params_wide = to_wide(params);
            let result = unsafe {
                ShellExecuteW(
                    Some(HWND(std::ptr::null_mut())),
                    PCWSTR(operation.as_ptr()),
                    PCWSTR(file_wide.as_ptr()),
                    PCWSTR(params_wide.as_ptr()),
                    dir_wide.map_or(PCWSTR::null(), |d| PCWSTR(d.as_ptr())),
                    SW_SHOWNORMAL,
                )
            };
            result.0 as isize
        }

        fn error_desc(code: isize) -> &'static str {
            match code {
                0 | 8 => "Out of memory",
                2 => "File not found",
                3 => "Path not found",
                5 => "Access denied",
                11 => "Invalid format",
                26 => "Sharing violation",
                27 => "Incomplete file association",
                28 => "DDE request timeout",
                29 => "DDE transaction failed",
                30 => "DDE busy",
                31 => "No associated application",
                32 => "DLL not found",
                _ => "Unknown error",
            }
        }

        let operation = to_wide(if is_elevated { "runas" } else { "open" });
        let trimmed = cmd.trim();
        let dir_wide: Option<Vec<u16>> = working_dir.as_ref().filter(|d| !d.is_empty()).map(|d| to_wide(d));

        // Parse quoted path
        if trimmed.starts_with('"') {
            if let Some(end) = trimmed[1..].find('"') {
                let file = &trimmed[1..=end];
                let params = if end + 2 < trimmed.len() { trimmed[end + 2..].trim() } else { "" };
                info!("  📂 [quoted] file: {}, params: {}", file, params);
                let rc = shell_exec(&operation, file, params, dir_wide.as_ref());
                return if rc > 32 {
                    info!("✅ App launched: {}", file);
                    Ok(())
                } else {
                    Err(format!("Launch failed: {} (error code: {})", error_desc(rc), rc))
                };
            }
        }

        // Strategy: try the whole string as `file` first; on failure, split and retry.
        // This way "J:\DESSERT Soft\WannabeCN.exe" and "steam://run/123" both succeed directly.
        info!("  📂 [attempt 1] Using the whole command as `file`: {}", trimmed);
        let rc1 = shell_exec(&operation, trimmed, "", dir_wide.as_ref());
        if rc1 > 32 {
            info!("✅ App launched: {}", trimmed);
            return Ok(());
        }

        // First attempt failed — try splitting at the first space (file + params)
        if let Some(space_pos) = trimmed.find(' ') {
            let file = &trimmed[..space_pos];
            let params = trimmed[space_pos + 1..].trim();
            info!("  📂 [attempt 2] file: {}, params: {} (attempt 1 error code: {})", file, params, rc1);
            let rc2 = shell_exec(&operation, file, params, dir_wide.as_ref());
            if rc2 > 32 {
                info!("✅ App launched: {}", file);
                return Ok(());
            }
            // Both attempts failed — return the more meaningful error
            let (best_rc, _context) = if rc1 == 2 || rc1 == 3 { (rc2, file) } else { (rc1, trimmed) };
            Err(format!("Launch failed: {} (error code: {})", error_desc(best_rc), best_rc))
        } else {
            Err(format!("Launch failed: {} (error code: {})", error_desc(rc1), rc1))
        }
    }).await.map_err(|e| format!("Task execution failed: {}", e))?
}

/// Speech phrases response (includes ETag for conditional requests)
#[derive(Serialize)]
pub struct SpeechPhrasesResponse {
    pub phrases: Vec<String>,
    pub etag: Option<String>,
}

#[tauri::command]
pub async fn fetch_speech_phrases(if_none_match: Option<String>) -> Result<Option<SpeechPhrasesResponse>, String> {
    let url = "https://assets.alkaidlab.com/speech-phrases.json";
    
    let result = cdn_get(url, if_none_match.as_deref()).await?;
    let (response, etag) = match result {
        CdnResult::NotModified => return Ok(None),
        CdnResult::Fresh(resp, etag) => (resp, etag),
    };
    
    let body_text = response.text().await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // Strip UTF-8 BOM (CDN-returned JSON may include a BOM, which serde_json does not accept)
    let body_text = body_text.trim_start_matches('\u{FEFF}');

    let phrases: Vec<String> = serde_json::from_str(body_text)
        .map_err(|e| {
            let preview = if body_text.len() > 200 { &body_text[..200] } else { body_text };
            format!("JSON parse failed: {}; first 200 chars of response: {}", e, preview)
        })?;

    info!("✅ Speech phrases loaded — {} total", phrases.len());
    Ok(Some(SpeechPhrasesResponse { phrases, etag }))
}

/// Remote resource response (includes ETag for conditional requests)
#[derive(Serialize)]
pub struct RemoteBytesResponse {
    pub data_url: String,
    pub etag: Option<String>,
}

/// Download a remote resource via the Rust backend (bypasses WebView CORS).
/// Supports ETag conditional requests: if `if_none_match` is provided, data is only returned when the resource changed.
/// Returns None for 304 Not Modified.
#[tauri::command]
pub async fn fetch_remote_bytes(url: String, if_none_match: Option<String>) -> Result<Option<RemoteBytesResponse>, String> {
    // Safety check: only allow specific domains
    if !url.starts_with("https://assets.alkaidlab.com/") {
        return Err(format!("Proxy not allowed for URL: {}", url));
    }
    
    let result = cdn_get(&url, if_none_match.as_deref()).await?;
    let (response, etag) = match result {
        CdnResult::NotModified => return Ok(None),
        CdnResult::Fresh(resp, etag) => (resp, etag),
    };
    
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    
    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let data_url = format!("data:{};base64,{}", content_type, b64);

    info!("✅ Proxy download succeeded ({} bytes)", bytes.len());
    Ok(Some(RemoteBytesResponse { data_url, etag }))
}

// ===== AI API proxy (bypasses CORS) =====

/// Dedicated HTTP client for the AI API proxy
fn ai_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(3)
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("Failed to create AI HTTP client")
    })
}

#[derive(Debug, Deserialize)]
pub struct AiProxyRequest {
    pub url: String,
    pub method: String,  // "GET" or "POST"
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
}

/// Generic AI API proxy: forwards HTTP requests to external AI services, bypassing WebView CORS
#[tauri::command]
pub async fn ai_api_proxy(request: AiProxyRequest) -> Result<String, String> {
    let client = ai_client();

    let mut req_builder = match request.method.to_uppercase().as_str() {
        "GET" => client.get(&request.url),
        "POST" => client.post(&request.url),
        _ => return Err(format!("Unsupported HTTP method: {}", request.method)),
    };

    // Set request headers
    for (key, value) in &request.headers {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    // Set request body
    if let Some(body) = &request.body {
        req_builder = req_builder.header("Content-Type", "application/json");
        req_builder = req_builder.body(body.clone());
    }

    let response = req_builder.send().await
        .map_err(|e| format!("AI API request failed: {}", e))?;

    let status = response.status().as_u16();
    let body = response.text().await
        .map_err(|e| format!("Failed to read AI API response: {}", e))?;

    if status >= 400 {
        return Err(format!("{} - {}", status, &body[..body.len().min(500)]));
    }

    Ok(body)
}

// ===== Desktop screen capture (used for the AI desktop pet's vision) =====

/// Capture the primary display, returning a base64-encoded JPEG (quality 60, max 1024px wide)
#[tauri::command]
pub async fn capture_screenshot() -> Result<String, String> {
    use xcap::Monitor;
    use std::io::Cursor;

    // Run the capture on a blocking thread (xcap uses a synchronous API internally)
    tokio::task::spawn_blocking(|| {
        let monitors = Monitor::all().map_err(|e| format!("Failed to enumerate monitors: {}", e))?;
        let monitor = monitors.into_iter().next().ok_or("No monitor found")?;

        let image = monitor.capture_image().map_err(|e| format!("Screen capture failed: {}", e))?;

        // Scale to a max of 1024px wide to keep token usage low
        let (w, h) = (image.width(), image.height());
        let max_width = 1024u32;
        let resized = if w > max_width {
            let new_h = (h as f64 * max_width as f64 / w as f64) as u32;
            image::imageops::resize(&image, max_width, new_h, image::imageops::FilterType::Triangle)
        } else {
            image::imageops::resize(&image, w, h, image::imageops::FilterType::Triangle)
        };

        // RGBA → RGB (JPEG does not support alpha)
        let rgb_image = image::DynamicImage::ImageRgba8(resized).to_rgb8();

        // Encode as JPEG (quality 60)
        let mut buf = Cursor::new(Vec::new());
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 60);
        rgb_image.write_with_encoder(encoder).map_err(|e| format!("JPEG encoding failed: {}", e))?;

        let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
        Ok(format!("data:image/jpeg;base64,{}", b64))
    })
    .await
    .map_err(|e| format!("Screen capture task failed: {}", e))?
}
