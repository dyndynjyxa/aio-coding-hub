//! Usage: App-level Tauri commands (about info, lifecycle, etc.).

use tauri::utils::config::BundleType;
use tauri::Manager;

fn sanitize_text(input: Option<String>, max_len: usize) -> Option<String> {
    let value = input?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(max_len).collect())
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct AppAboutInfo {
    os: String,
    arch: String,
    profile: String,
    app_version: String,
    bundle_type: Option<String>,
    run_mode: String,
}

#[tauri::command]
pub(crate) fn app_about_get() -> AppAboutInfo {
    let bundle_type = tauri::utils::platform::bundle_type();
    let run_mode = match bundle_type {
        Some(BundleType::Nsis | BundleType::Msi | BundleType::Deb | BundleType::Rpm) => "installer",
        Some(BundleType::AppImage) => "portable",
        Some(BundleType::App | BundleType::Dmg) => "unknown",
        None => {
            // On Windows, BundleType::None means the exe is NOT running from an
            // MSI or NSIS install, so it must be a portable (ZIP) deployment.
            #[cfg(windows)]
            {
                "portable"
            }
            #[cfg(not(windows))]
            {
                "unknown"
            }
        }
    }
    .to_string();

    AppAboutInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        profile: if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        },
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        bundle_type: bundle_type.map(|t| t.to_string()),
        run_mode,
    }
}

#[tauri::command]
pub(crate) fn app_exit(app: tauri::AppHandle) -> Result<bool, String> {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        app.state::<crate::app::resident::ResidentState>()
            .begin_exit();
        app.exit(0);
    });
    Ok(true)
}

#[tauri::command]
pub(crate) fn app_restart(app: tauri::AppHandle) -> Result<bool, String> {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        app.state::<crate::app::resident::ResidentState>()
            .begin_restart();
        tauri::async_runtime::block_on(crate::app::cleanup::cleanup_before_exit(&app));
        app.request_restart();
    });
    Ok(true)
}

#[tauri::command]
pub(crate) fn app_heartbeat_pong(app: tauri::AppHandle) -> Result<bool, String> {
    let watchdog = app.state::<crate::app::heartbeat_watchdog::HeartbeatWatchdogState>();
    watchdog.record_pong();
    Ok(true)
}

#[tauri::command]
pub(crate) fn app_frontend_error_report(
    source: String,
    message: String,
    stack: Option<String>,
    details_json: Option<String>,
    href: Option<String>,
    user_agent: Option<String>,
) -> Result<bool, String> {
    let source = sanitize_text(Some(source), 128).unwrap_or_else(|| "unknown".to_string());
    let message = sanitize_text(Some(message), 4096).unwrap_or_else(|| "unknown".to_string());
    let stack = sanitize_text(stack, 16_384);
    let details_json = sanitize_text(details_json, 16_384);
    let href = sanitize_text(href, 2_048);
    let user_agent = sanitize_text(user_agent, 1_024);

    tracing::error!(
        target: "frontend",
        source = %source,
        href = %href.as_deref().unwrap_or_default(),
        user_agent = %user_agent.as_deref().unwrap_or_default(),
        stack = %stack.as_deref().unwrap_or_default(),
        details_json = %details_json.as_deref().unwrap_or_default(),
        "frontend runtime error: {}",
        message
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_text_returns_none_for_none_input() {
        assert!(sanitize_text(None, 100).is_none());
    }

    #[test]
    fn sanitize_text_returns_none_for_empty_string() {
        assert!(sanitize_text(Some("".to_string()), 100).is_none());
    }

    #[test]
    fn sanitize_text_returns_none_for_whitespace_only() {
        assert!(sanitize_text(Some("   \t\n  ".to_string()), 100).is_none());
    }

    #[test]
    fn sanitize_text_trims_whitespace() {
        assert_eq!(
            sanitize_text(Some("  hello  ".to_string()), 100),
            Some("hello".to_string())
        );
    }

    #[test]
    fn sanitize_text_truncates_to_max_len() {
        assert_eq!(
            sanitize_text(Some("abcdefgh".to_string()), 3),
            Some("abc".to_string())
        );
    }

    #[test]
    fn sanitize_text_truncates_after_trimming() {
        // Whitespace is trimmed first, then truncation applies to the trimmed result.
        assert_eq!(
            sanitize_text(Some("  abcdefgh  ".to_string()), 5),
            Some("abcde".to_string())
        );
    }

    #[test]
    fn sanitize_text_handles_multibyte_chars_by_char_count() {
        // Truncation is by char count, not byte count.
        let cjk = Some("\u{4f60}\u{597d}\u{4e16}\u{754c}".to_string()); // 4 CJK chars
        assert_eq!(sanitize_text(cjk, 2), Some("\u{4f60}\u{597d}".to_string()));
    }

    #[test]
    fn sanitize_text_returns_full_string_when_within_limit() {
        assert_eq!(
            sanitize_text(Some("short".to_string()), 100),
            Some("short".to_string())
        );
    }
}
