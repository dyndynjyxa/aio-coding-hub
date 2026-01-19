//! Notice（系统通知）模块。
//!
//! 用法：
//! - 前端：`invoke("notice_send", { level, title?, body })` 触发通知
//! - Rust 后台：调用 `notice::emit(app, payload)` 触发通知事件（由前端统一监听并发送系统通知）

use tauri::Emitter;

pub const NOTICE_EVENT_NAME: &str = "notice:notify";

const NOTICE_PREFIX: &str = "AIO Coding Hub";

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoticeLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NoticeEventPayload {
    pub level: NoticeLevel,
    pub title: String,
    pub body: String,
}

fn default_title(level: NoticeLevel) -> &'static str {
    match level {
        NoticeLevel::Info => "提示",
        NoticeLevel::Success => "成功",
        NoticeLevel::Warning => "提醒",
        NoticeLevel::Error => "错误",
    }
}

fn normalize_optional_title(title: Option<String>) -> Option<String> {
    let title = title?;
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn format_title(level: NoticeLevel, title: Option<String>) -> String {
    let title = normalize_optional_title(title).unwrap_or_else(|| default_title(level).to_string());
    format!("{NOTICE_PREFIX} · {title}")
}

pub fn build(level: NoticeLevel, title: Option<String>, body: String) -> NoticeEventPayload {
    NoticeEventPayload {
        level,
        title: format_title(level, title),
        body,
    }
}

pub fn emit(app: &tauri::AppHandle, payload: NoticeEventPayload) -> Result<(), String> {
    app.emit(NOTICE_EVENT_NAME, payload)
        .map_err(|e| format!("NOTICE_EMIT: {e}"))?;
    Ok(())
}
