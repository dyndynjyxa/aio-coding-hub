//! Usage: Image generation orchestration (read config from DB, inject the API
//! key server-side, send via the shared proxy-aware HTTP client).

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::blocking;
use crate::domain::image_gen;

pub(crate) async fn config_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
) -> Result<image_gen::ImageGenConfigView, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("image_gen_config_get", move || {
        image_gen::config_get(&db, &adapter_id)
    })
    .await
    .map_err(Into::into)
}

pub(crate) async fn config_set(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<image_gen::ImageGenConfigView, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("image_gen_config_set", move || {
        image_gen::config_set(&db, &adapter_id, &base_url, &model, api_key.as_deref())
    })
    .await
    .map_err(Into::into)
}

async fn connection(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
) -> Result<(String, String), String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("image_gen_connection_get", move || {
        image_gen::config_connection(&db, &adapter_id)
    })
    .await
    .map_err(Into::into)
}

pub(crate) async fn post_json(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
    path: String,
    body: serde_json::Value,
    timeout_secs: Option<u32>,
) -> Result<image_gen::ImageGenHttpResponse, String> {
    let (base_url, api_key) = connection(app, db_state, adapter_id).await?;
    let client = crate::gateway::http_client::get();
    image_gen::post_json(&client, &base_url, &api_key, &path, &body, timeout_secs).await
}

pub(crate) async fn post_multipart(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    adapter_id: String,
    path: String,
    fields: Vec<(String, String)>,
    files: Vec<image_gen::ImageGenMultipartFile>,
    timeout_secs: Option<u32>,
) -> Result<image_gen::ImageGenHttpResponse, String> {
    let (base_url, api_key) = connection(app, db_state, adapter_id).await?;
    let client = crate::gateway::http_client::get();
    image_gen::post_multipart(
        &client,
        &base_url,
        &api_key,
        &path,
        &fields,
        &files,
        timeout_secs,
    )
    .await
}

pub(crate) async fn fetch_image(
    url: String,
    timeout_secs: Option<u32>,
) -> Result<image_gen::ImageGenFetchedImage, String> {
    let client = crate::gateway::http_client::get();
    image_gen::fetch_image(&client, &url, timeout_secs).await
}
