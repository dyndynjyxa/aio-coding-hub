//! Usage: Thin IPC wrappers for the image generation page (config + transport
//! proxy commands). The API key never appears in IPC arguments or results.

use crate::app::image_gen_service;
use crate::app_state::DbInitState;
use crate::blocking;
use crate::domain::image_gen::{
    ImageGenConfigView, ImageGenFetchedImage, ImageGenHttpResponse, ImageGenMultipartFile,
    ImageGenStorageView, ImageGenTaskPersistPayload, ImageGenTaskRow,
};
use base64::Engine as _;

const IMAGE_GEN_SAVE_MAX_BYTES: usize = 64 * 1024 * 1024;

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_config_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
) -> Result<ImageGenConfigView, String> {
    image_gen_service::config_get(app, db_state, adapter_id).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_config_set(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<ImageGenConfigView, String> {
    image_gen_service::config_set(app, db_state, adapter_id, base_url, model, api_key).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_post_json(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
    path: String,
    body: serde_json::Value,
    timeout_secs: Option<u32>,
) -> Result<ImageGenHttpResponse, String> {
    image_gen_service::post_json(app, db_state, adapter_id, path, body, timeout_secs).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_post_multipart(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
    path: String,
    fields: Vec<(String, String)>,
    files: Vec<ImageGenMultipartFile>,
    timeout_secs: Option<u32>,
) -> Result<ImageGenHttpResponse, String> {
    image_gen_service::post_multipart(app, db_state, adapter_id, path, fields, files, timeout_secs)
        .await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_fetch_image(
    url: String,
    timeout_secs: Option<u32>,
) -> Result<ImageGenFetchedImage, String> {
    image_gen_service::fetch_image(url, timeout_secs).await
}

fn write_image_gen_file(file_path: String, data_b64: String) -> Result<bool, String> {
    let file_path = file_path.trim().to_string();
    if file_path.is_empty() {
        return Err("SEC_INVALID_INPUT: file_path is required".to_string());
    }

    let data_b64 = data_b64.trim();
    if data_b64.is_empty() {
        return Err("SEC_INVALID_INPUT: data_b64 is required".to_string());
    }
    if data_b64.len() > IMAGE_GEN_SAVE_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: image data is too large (max {IMAGE_GEN_SAVE_MAX_BYTES} bytes)"
        ));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_b64.as_bytes())
        .map_err(|e| format!("SEC_INVALID_INPUT: data_b64 is invalid: {e}"))?;

    std::fs::write(&file_path, bytes)
        .map_err(|err| format!("SYSTEM_ERROR: failed to write image file: {err}"))?;
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_save_image(path: String, data_b64: String) -> Result<bool, String> {
    blocking::run("image_gen_save_image", move || {
        write_image_gen_file(path, data_b64)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_task_persist(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    payload: ImageGenTaskPersistPayload,
) -> Result<ImageGenTaskRow, String> {
    image_gen_service::task_persist(app, db_state, payload).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_tasks_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    before_created_at: Option<i64>,
    limit: u32,
) -> Result<Vec<ImageGenTaskRow>, String> {
    image_gen_service::tasks_list(app, db_state, before_created_at, limit).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_task_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    id: String,
) -> Result<(), String> {
    image_gen_service::task_delete(app, db_state, id).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_tasks_clear(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<u32, String> {
    image_gen_service::tasks_clear(app, db_state).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_read_image(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    path: String,
) -> Result<ImageGenFetchedImage, String> {
    image_gen_service::read_image(app, db_state, path).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_storage_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<ImageGenStorageView, String> {
    image_gen_service::storage_get(app, db_state).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_storage_set_dir(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    dir: String,
) -> Result<ImageGenStorageView, String> {
    image_gen_service::storage_set_dir(app, db_state, dir).await
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn image_gen_storage_cleanup(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    keep_count: u32,
) -> Result<u32, String> {
    image_gen_service::storage_cleanup(app, db_state, keep_count).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_gen_save_image_writes_decoded_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.png");

        let ok = write_image_gen_file(
            path.to_string_lossy().to_string(),
            "aGVsbG8=".to_string(), // "hello"
        )
        .expect("save should succeed");

        assert!(ok);
        assert_eq!(std::fs::read(path).expect("read file"), b"hello");
    }

    #[test]
    fn image_gen_save_image_rejects_empty_path_and_data() {
        let err = write_image_gen_file("   ".to_string(), "aGVsbG8=".to_string())
            .expect_err("empty path should fail");
        assert!(err.contains("SEC_INVALID_INPUT: file_path is required"));

        let err = write_image_gen_file("/tmp/out.png".to_string(), "  ".to_string())
            .expect_err("empty data should fail");
        assert!(err.contains("SEC_INVALID_INPUT: data_b64 is required"));
    }

    #[test]
    fn image_gen_save_image_rejects_oversized_and_invalid_base64() {
        let err = write_image_gen_file(
            "/tmp/out.png".to_string(),
            "A".repeat(IMAGE_GEN_SAVE_MAX_BYTES + 1),
        )
        .expect_err("oversized data should fail");
        assert!(err.contains("SEC_INVALID_INPUT: image data is too large"));

        let err = write_image_gen_file("/tmp/out.png".to_string(), "!!bad!!".to_string())
            .expect_err("invalid base64 should fail");
        assert!(err.contains("SEC_INVALID_INPUT: data_b64 is invalid"));
    }
}
