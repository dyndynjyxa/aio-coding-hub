//! Usage: Backend-owned desktop capability proxy commands.
//!
//! This module keeps sensitive or high-risk desktop capabilities behind one
//! handwritten IPC family so the renderer does not call plugin commands
//! directly.

use crate::shared::blocking;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{Manager, ResourceId, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, FileAccessMode, FilePath, PickerMode};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DesktopThemeMode {
    Light,
    Dark,
    System,
}

impl DesktopThemeMode {
    fn into_tauri_theme(self) -> Option<tauri::Theme> {
        match self {
            Self::Light => Some(tauri::Theme::Light),
            Self::Dark => Some(tauri::Theme::Dark),
            Self::System => None,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
pub(crate) struct DesktopNotificationPayload {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) sound: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DesktopNotificationPermissionState {
    Granted,
    Denied,
    Prompt,
    PromptWithRationale,
}

impl From<tauri::plugin::PermissionState> for DesktopNotificationPermissionState {
    fn from(value: tauri::plugin::PermissionState) -> Self {
        match value {
            tauri::plugin::PermissionState::Granted => Self::Granted,
            tauri::plugin::PermissionState::Denied => Self::Denied,
            tauri::plugin::PermissionState::Prompt => Self::Prompt,
            tauri::plugin::PermissionState::PromptWithRationale => Self::PromptWithRationale,
        }
    }
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopUpdaterMetadata {
    rid: u32,
    current_version: String,
    version: String,
    date: Option<String>,
    body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub(crate) enum DesktopUpdaterDownloadEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        content_length: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        chunk_length: usize,
    },
    Finished,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopDialogFilter {
    name: String,
    extensions: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DesktopDialogPickerMode {
    Document,
    Media,
    Image,
    Video,
}

impl From<DesktopDialogPickerMode> for PickerMode {
    fn from(value: DesktopDialogPickerMode) -> Self {
        match value {
            DesktopDialogPickerMode::Document => PickerMode::Document,
            DesktopDialogPickerMode::Media => PickerMode::Media,
            DesktopDialogPickerMode::Image => PickerMode::Image,
            DesktopDialogPickerMode::Video => PickerMode::Video,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DesktopDialogFileAccessMode {
    Copy,
    Scoped,
}

impl From<DesktopDialogFileAccessMode> for FileAccessMode {
    fn from(value: DesktopDialogFileAccessMode) -> Self {
        match value {
            DesktopDialogFileAccessMode::Copy => FileAccessMode::Copy,
            DesktopDialogFileAccessMode::Scoped => FileAccessMode::Scoped,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopDialogOpenRequest {
    title: Option<String>,
    filters: Option<Vec<DesktopDialogFilter>>,
    default_path: Option<String>,
    multiple: Option<bool>,
    directory: Option<bool>,
    recursive: Option<bool>,
    can_create_directories: Option<bool>,
    picker_mode: Option<DesktopDialogPickerMode>,
    file_access_mode: Option<DesktopDialogFileAccessMode>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopDialogSaveRequest {
    title: Option<String>,
    filters: Option<Vec<DesktopDialogFilter>>,
    default_path: Option<String>,
    can_create_directories: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopOpenUrlRequest {
    url: String,
    with: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopOpenPathRequest {
    path: String,
    with: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopRevealItemRequest {
    path: String,
}

fn trim_to_non_empty(input: &str, max_len: usize) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.chars().take(max_len).collect())
}

fn to_duration(timeout_ms: Option<u64>) -> Option<Duration> {
    timeout_ms.map(Duration::from_millis)
}

fn simplify_path(path: PathBuf) -> PathBuf {
    path.components().collect()
}

fn normalize_existing_path(path: PathBuf) -> PathBuf {
    if path.exists() {
        return std::fs::canonicalize(&path)
            .map(simplify_path)
            .unwrap_or_else(|_| simplify_path(path));
    }

    simplify_path(path)
}

fn sanitize_dialog_filters(
    filters: Option<Vec<DesktopDialogFilter>>,
) -> Result<Vec<DesktopDialogFilter>, String> {
    let Some(filters) = filters else {
        return Ok(Vec::new());
    };

    let mut sanitized = Vec::new();
    for filter in filters {
        let name = trim_to_non_empty(&filter.name, 128).ok_or_else(|| {
            "DESKTOP_DIALOG_INVALID_FILTER_NAME: filter name cannot be empty".to_string()
        })?;
        let extensions = filter
            .extensions
            .into_iter()
            .filter_map(|item| trim_to_non_empty(&item, 64))
            .map(|item| item.trim_start_matches('.').to_string())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if extensions.is_empty() {
            return Err(
                "DESKTOP_DIALOG_INVALID_FILTER: filter extensions cannot be empty".to_string(),
            );
        }
        sanitized.push(DesktopDialogFilter { name, extensions });
    }

    Ok(sanitized)
}

fn apply_dialog_default_path<R: tauri::Runtime>(
    mut dialog: tauri_plugin_dialog::FileDialogBuilder<R>,
    default_path: Option<String>,
) -> Result<tauri_plugin_dialog::FileDialogBuilder<R>, String> {
    let Some(default_path) = default_path else {
        return Ok(dialog);
    };

    let default_path = trim_to_non_empty(&default_path, 4_096).ok_or_else(|| {
        "DESKTOP_DIALOG_INVALID_DEFAULT_PATH: defaultPath cannot be empty".to_string()
    })?;
    let path = simplify_path(PathBuf::from(default_path));
    if path.is_file() || !path.exists() {
        if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) {
            if parent.components().count() > 0 {
                dialog = dialog.set_directory(parent);
            }
            dialog = dialog.set_file_name(file_name.to_string_lossy());
            return Ok(dialog);
        }
    }

    Ok(dialog.set_directory(path))
}

fn build_open_dialog(
    window: &WebviewWindow,
    request: DesktopDialogOpenRequest,
) -> Result<tauri_plugin_dialog::FileDialogBuilder<tauri::Wry>, String> {
    let mut dialog = window.dialog().file();
    #[cfg(any(windows, target_os = "macos"))]
    {
        dialog = dialog.set_parent(window);
    }

    if let Some(title) = request
        .title
        .and_then(|value| trim_to_non_empty(&value, 256))
    {
        dialog = dialog.set_title(title);
    }

    dialog = apply_dialog_default_path(dialog, request.default_path)?;

    if let Some(can_create_directories) = request.can_create_directories {
        dialog = dialog.set_can_create_directories(can_create_directories);
    }
    if let Some(picker_mode) = request.picker_mode {
        dialog = dialog.set_picker_mode(picker_mode.into());
    }
    if let Some(file_access_mode) = request.file_access_mode {
        dialog = dialog.set_file_access_mode(file_access_mode.into());
    }

    for filter in sanitize_dialog_filters(request.filters)? {
        let extensions = filter
            .extensions
            .iter()
            .map(|item| item.as_str())
            .collect::<Vec<_>>();
        dialog = dialog.add_filter(filter.name, &extensions);
    }

    let _ = request.recursive;
    Ok(dialog)
}

fn build_save_dialog(
    window: &WebviewWindow,
    request: DesktopDialogSaveRequest,
) -> Result<tauri_plugin_dialog::FileDialogBuilder<tauri::Wry>, String> {
    let mut dialog = window.dialog().file();
    #[cfg(any(windows, target_os = "macos"))]
    {
        dialog = dialog.set_parent(window);
    }

    if let Some(title) = request
        .title
        .and_then(|value| trim_to_non_empty(&value, 256))
    {
        dialog = dialog.set_title(title);
    }

    dialog = apply_dialog_default_path(dialog, request.default_path)?;

    if let Some(can_create_directories) = request.can_create_directories {
        dialog = dialog.set_can_create_directories(can_create_directories);
    }

    for filter in sanitize_dialog_filters(request.filters)? {
        let extensions = filter
            .extensions
            .iter()
            .map(|item| item.as_str())
            .collect::<Vec<_>>();
        dialog = dialog.add_filter(filter.name, &extensions);
    }

    Ok(dialog)
}

fn file_path_to_string(path: FilePath) -> String {
    path.to_string()
}

fn sanitize_optional_program(input: Option<String>) -> Option<String> {
    input.and_then(|value| trim_to_non_empty(&value, 256))
}

fn sanitize_url(input: String) -> Result<String, String> {
    let url = trim_to_non_empty(&input, 2_048)
        .ok_or_else(|| "DESKTOP_OPEN_URL_EMPTY: url cannot be empty".to_string())?;
    let parsed = tauri::Url::parse(&url)
        .map_err(|error| format!("DESKTOP_OPEN_URL_INVALID: invalid url: {error}"))?;
    match parsed.scheme() {
        "http" | "https" | "mailto" | "tel" => Ok(url),
        scheme => Err(format!(
            "DESKTOP_OPEN_URL_SCHEME_DENIED: unsupported url scheme: {scheme}"
        )),
    }
}

fn sanitize_open_path(input: String) -> Result<PathBuf, String> {
    let path = trim_to_non_empty(&input, 4_096)
        .ok_or_else(|| "DESKTOP_OPEN_PATH_EMPTY: path cannot be empty".to_string())?;
    Ok(normalize_existing_path(PathBuf::from(path)))
}

fn path_is_within_root(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn push_desktop_open_root(roots: &mut Vec<PathBuf>, root: PathBuf) {
    let root = normalize_existing_path(root);
    if roots.iter().any(|existing| existing == &root) {
        return;
    }
    roots.push(root);
}

fn desktop_open_allowed_roots<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Vec<PathBuf>, String> {
    let home_dir = crate::infra::app_paths::home_dir(app).map_err(|error| error.to_string())?;
    let app_data_dir =
        crate::infra::app_paths::app_data_dir(app).map_err(|error| error.to_string())?;
    let user_default_codex_home_dir = crate::infra::codex_paths::codex_home_dir_user_default(app)
        .map_err(|error| error.to_string())?;
    let follow_codex_home_dir =
        crate::infra::codex_paths::codex_home_dir_follow_env_or_default(app)
            .map_err(|error| error.to_string())?;
    let effective_codex_home_dir =
        crate::infra::codex_paths::codex_home_dir(app).map_err(|error| error.to_string())?;
    let configured_codex_home_dir = crate::infra::codex_paths::configured_codex_home_dir(app);

    let mut roots = Vec::new();
    push_desktop_open_root(&mut roots, app_data_dir);
    push_desktop_open_root(&mut roots, home_dir.join(".claude"));
    push_desktop_open_root(&mut roots, home_dir.join(".gemini"));
    push_desktop_open_root(&mut roots, user_default_codex_home_dir);
    push_desktop_open_root(&mut roots, follow_codex_home_dir);
    push_desktop_open_root(&mut roots, effective_codex_home_dir);
    if let Some(configured_codex_home_dir) = configured_codex_home_dir {
        push_desktop_open_root(&mut roots, configured_codex_home_dir);
    }

    Ok(roots)
}

fn ensure_desktop_open_path_allowed<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    path: &Path,
) -> Result<(), String> {
    let normalized_path = normalize_existing_path(path.to_path_buf());
    let allowed = desktop_open_allowed_roots(app)?;
    if allowed
        .iter()
        .any(|root| path_is_within_root(&normalized_path, root))
    {
        return Ok(());
    }

    Err(format!(
        "DESKTOP_OPEN_PATH_DENIED: path is outside allowed desktop roots: {}",
        normalized_path.display()
    ))
}

#[tauri::command]
#[specta::specta]
pub(crate) fn desktop_clipboard_write_text(
    app: tauri::AppHandle,
    text: String,
) -> Result<bool, String> {
    let text = trim_to_non_empty(&text, 1_000_000)
        .ok_or_else(|| "CLIPBOARD_EMPTY_TEXT: text cannot be empty".to_string())?;

    app.clipboard()
        .write_text(Cow::Owned(text))
        .map_err(|error| format!("failed to write clipboard text: {error}"))?;

    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn desktop_dialog_open(
    window: WebviewWindow,
    options: DesktopDialogOpenRequest,
) -> Result<Option<Vec<String>>, String> {
    let multiple = options.multiple.unwrap_or(false);
    let directory = options.directory.unwrap_or(false);
    let dialog = build_open_dialog(&window, options)?;
    let (tx, rx) = oneshot::channel();

    match (directory, multiple) {
        (true, true) => {
            dialog.pick_folders(move |selection| {
                let _ = tx.send(selection.map(|paths| {
                    paths
                        .into_iter()
                        .map(file_path_to_string)
                        .collect::<Vec<_>>()
                }));
            });
        }
        (true, false) => {
            dialog.pick_folder(move |selection| {
                let _ = tx.send(selection.map(|path| vec![file_path_to_string(path)]));
            });
        }
        (false, true) => {
            dialog.pick_files(move |selection| {
                let _ = tx.send(selection.map(|paths| {
                    paths
                        .into_iter()
                        .map(file_path_to_string)
                        .collect::<Vec<_>>()
                }));
            });
        }
        (false, false) => {
            dialog.pick_file(move |selection| {
                let _ = tx.send(selection.map(|path| vec![file_path_to_string(path)]));
            });
        }
    }

    rx.await
        .map_err(|_| "DESKTOP_DIALOG_OPEN_CANCELLED: dialog response channel dropped".to_string())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn desktop_dialog_save(
    window: WebviewWindow,
    options: DesktopDialogSaveRequest,
) -> Result<Option<String>, String> {
    let dialog = build_save_dialog(&window, options)?;
    let (tx, rx) = oneshot::channel();

    dialog.save_file(move |selection| {
        let _ = tx.send(selection.map(file_path_to_string));
    });

    rx.await
        .map_err(|_| "DESKTOP_DIALOG_SAVE_CANCELLED: dialog response channel dropped".to_string())
}

#[tauri::command]
#[specta::specta]
pub(crate) fn desktop_window_set_theme(
    window: WebviewWindow,
    theme: DesktopThemeMode,
) -> Result<bool, String> {
    window
        .set_theme(theme.into_tauri_theme())
        .map_err(|error| format!("failed to set desktop window theme: {error}"))?;

    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn desktop_opener_open_url(
    app: tauri::AppHandle,
    input: DesktopOpenUrlRequest,
) -> Result<bool, String> {
    let url = sanitize_url(input.url)?;
    let with = sanitize_optional_program(input.with);

    blocking::run("desktop_opener_open_url", move || {
        let with = with.as_deref();
        app.opener()
            .open_url(url, with)
            .map_err(|error| format!("failed to open desktop url: {error}"))?;
        Ok::<bool, crate::shared::error::AppError>(true)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn desktop_opener_open_path(
    app: tauri::AppHandle,
    input: DesktopOpenPathRequest,
) -> Result<bool, String> {
    let path = sanitize_open_path(input.path)?;
    ensure_desktop_open_path_allowed(&app, &path)?;
    let with = sanitize_optional_program(input.with);
    let path_string = path.display().to_string();

    blocking::run("desktop_opener_open_path", move || {
        let with = with.as_deref();
        app.opener()
            .open_path(path_string, with)
            .map_err(|error| format!("failed to open desktop path: {error}"))?;
        Ok::<bool, crate::shared::error::AppError>(true)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn desktop_opener_reveal_item_in_dir(
    app: tauri::AppHandle,
    input: DesktopRevealItemRequest,
) -> Result<bool, String> {
    let path = sanitize_open_path(input.path)?;
    ensure_desktop_open_path_allowed(&app, &path)?;

    blocking::run("desktop_opener_reveal_item_in_dir", move || {
        app.opener()
            .reveal_item_in_dir(path)
            .map_err(|error| format!("failed to reveal desktop item: {error}"))?;
        Ok::<bool, crate::shared::error::AppError>(true)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) fn desktop_notification_is_permission_granted(
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let granted = matches!(
        app.notification()
            .permission_state()
            .map_err(|error| format!("failed to read notification permission: {error}"))?,
        tauri_plugin_notification::PermissionState::Granted
    );

    Ok(granted)
}

#[tauri::command]
#[specta::specta]
pub(crate) fn desktop_notification_request_permission(
    app: tauri::AppHandle,
) -> Result<DesktopNotificationPermissionState, String> {
    let permission = app
        .notification()
        .request_permission()
        .map_err(|error| format!("failed to request notification permission: {error}"))?;

    Ok(permission.into())
}

#[tauri::command]
#[specta::specta]
pub(crate) fn desktop_notification_notify(
    app: tauri::AppHandle,
    options: DesktopNotificationPayload,
) -> Result<bool, String> {
    let title = trim_to_non_empty(&options.title, 256)
        .ok_or_else(|| "NOTICE_INVALID_TITLE: title cannot be empty".to_string())?;
    let body = trim_to_non_empty(&options.body, 4_096)
        .ok_or_else(|| "NOTICE_INVALID_BODY: body cannot be empty".to_string())?;
    let sound = options
        .sound
        .as_deref()
        .and_then(|value| trim_to_non_empty(value, 128));

    let mut builder = app.notification().builder().title(title).body(body);
    if let Some(sound) = sound {
        builder = builder.sound(sound);
    }

    builder
        .show()
        .map_err(|error| format!("failed to show desktop notification: {error}"))?;

    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn desktop_updater_check(
    app: tauri::AppHandle,
    timeout: Option<u64>,
) -> Result<Option<DesktopUpdaterMetadata>, String> {
    let mut builder = app.updater_builder();
    if let Some(timeout) = to_duration(timeout) {
        builder = builder.timeout(timeout);
    }

    let updater = builder
        .build()
        .map_err(|error| format!("failed to build updater: {error}"))?;
    let update = updater
        .check()
        .await
        .map_err(|error| format!("failed to check updater: {error}"))?;

    if let Some(update) = update {
        let current_version = update.current_version.clone();
        let version = update.version.clone();
        let body = update.body.clone();
        let date = update.date.map(|value| value.to_string());
        let rid = app.resources_table().add(update);

        return Ok(Some(DesktopUpdaterMetadata {
            rid,
            current_version,
            version,
            date,
            body,
        }));
    }

    Ok(None)
}

#[tauri::command]
pub(crate) async fn desktop_updater_download_and_install(
    app: tauri::AppHandle,
    rid: ResourceId,
    on_event: Channel<DesktopUpdaterDownloadEvent>,
    timeout: Option<u64>,
) -> Result<bool, String> {
    let update = app
        .resources_table()
        .get::<Update>(rid)
        .map_err(|error| format!("failed to resolve updater resource: {error}"))?;
    let mut update = (*update).clone();
    update.timeout = to_duration(timeout);

    let mut first_chunk = true;
    update
        .download_and_install(
            |chunk_length, content_length| {
                if first_chunk {
                    first_chunk = false;
                    let _ = on_event.send(DesktopUpdaterDownloadEvent::Started { content_length });
                }
                let _ = on_event.send(DesktopUpdaterDownloadEvent::Progress { chunk_length });
            },
            || {
                let _ = on_event.send(DesktopUpdaterDownloadEvent::Finished);
            },
        )
        .await
        .map_err(|error| format!("failed to download and install update: {error}"))?;

    let _ = app.resources_table().close(rid);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{
        desktop_open_allowed_roots, ensure_desktop_open_path_allowed, normalize_existing_path,
    };
    use crate::infra::settings::{self, AppSettings, CodexHomeMode};
    use crate::test_support::{clear_settings_cache, test_env_lock};
    use std::ffi::OsString;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ENV_SEQ: AtomicU64 = AtomicU64::new(1);

    #[derive(Default)]
    struct EnvRestore {
        saved: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvRestore {
        fn save_once(&mut self, key: &'static str) {
            if self.saved.iter().any(|(saved_key, _)| *saved_key == key) {
                return;
            }
            self.saved.push((key, std::env::var_os(key)));
        }

        fn set_var(&mut self, key: &'static str, value: impl Into<OsString>) {
            self.save_once(key);
            std::env::set_var(key, value.into());
        }

        fn remove_var(&mut self, key: &'static str) {
            self.save_once(key);
            std::env::remove_var(key);
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, value) in self.saved.drain(..).rev() {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    struct DesktopCommandTestApp {
        #[allow(dead_code)]
        env_lock: std::sync::MutexGuard<'static, ()>,
        #[allow(dead_code)]
        env_restore: EnvRestore,
        #[allow(dead_code)]
        home_dir: tempfile::TempDir,
        app: tauri::App<tauri::test::MockRuntime>,
    }

    impl DesktopCommandTestApp {
        fn new() -> Self {
            let env_lock = test_env_lock();
            let home_dir = tempfile::tempdir().expect("tempdir");
            let seq = TEST_ENV_SEQ.fetch_add(1, Ordering::Relaxed);
            let mut env_restore = EnvRestore::default();
            env_restore.set_var(
                "AIO_CODING_HUB_HOME_DIR",
                home_dir.path().as_os_str().to_os_string(),
            );
            env_restore.set_var(
                "AIO_CODING_HUB_DOTDIR_NAME",
                format!(".aio-coding-hub-desktop-test-{seq}"),
            );
            env_restore.remove_var("AIO_CODING_HUB_TEST_HOME");
            clear_settings_cache();

            Self {
                env_lock,
                env_restore,
                home_dir,
                app: tauri::test::mock_app(),
            }
        }

        fn handle(&self) -> tauri::AppHandle<tauri::test::MockRuntime> {
            self.app.handle().clone()
        }
    }

    fn write_custom_codex_home<R: tauri::Runtime>(app: &tauri::AppHandle<R>, custom_home: &Path) {
        let settings = AppSettings {
            codex_home_mode: CodexHomeMode::Custom,
            codex_home_override: custom_home.display().to_string(),
            ..AppSettings::default()
        };
        settings::write(app, &settings).expect("write settings");
    }

    #[test]
    fn desktop_open_allowed_roots_include_custom_codex_home() {
        let test_app = DesktopCommandTestApp::new();
        let app_handle = test_app.handle();
        let custom_home = test_app.home_dir.path().join("custom-codex-home");
        write_custom_codex_home(&app_handle, &custom_home);

        let allowed_roots = desktop_open_allowed_roots(&app_handle).expect("allowed roots");

        assert!(allowed_roots.contains(&normalize_existing_path(custom_home)));
    }

    #[test]
    fn desktop_open_path_allows_paths_under_custom_codex_home() {
        let test_app = DesktopCommandTestApp::new();
        let app_handle = test_app.handle();
        let custom_home = test_app.home_dir.path().join("custom-codex-home");
        write_custom_codex_home(&app_handle, &custom_home);

        let config_path = custom_home.join("config.toml");

        assert!(ensure_desktop_open_path_allowed(&app_handle, &config_path).is_ok());
    }
}
