//! Usage: MCP server management related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::{blocking, mcp};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpSecretPatchInput {
    #[serde(default)]
    pub preserve_keys: Vec<String>,
    #[serde(default)]
    pub replace: BTreeMap<String, String>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServerUpsertInput {
    pub server_id: Option<i64>,
    #[serde(default)]
    pub server_key: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: McpSecretPatchInput,
    pub cwd: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub headers: McpSecretPatchInput,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServersListInput {
    pub workspace_id: i64,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServerEnabledInput {
    pub workspace_id: i64,
    pub server_id: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpServerDeleteInput {
    pub server_id: i64,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpParseJsonInput {
    pub json_text: String,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpImportServersInput {
    pub workspace_id: i64,
    pub servers: Vec<mcp::McpImportServer>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpImportFromWorkspaceCliInput {
    pub workspace_id: i64,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct McpServerSummaryView {
    pub id: i64,
    pub server_key: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env_keys: Vec<String>,
    pub cwd: Option<String>,
    pub url: Option<String>,
    pub header_keys: Vec<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<mcp::McpServerSummary> for McpServerSummaryView {
    fn from(value: mcp::McpServerSummary) -> Self {
        let env_keys = value.env_keys();
        let header_keys = value.header_keys();
        Self {
            id: value.id,
            server_key: value.server_key,
            name: value.name,
            transport: value.transport,
            command: value.command,
            args: value.args,
            env_keys,
            cwd: value.cwd,
            url: value.url,
            header_keys,
            enabled: value.enabled,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn mcp_servers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: McpServersListInput,
) -> Result<Vec<McpServerSummaryView>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("mcp_servers_list", move || {
        mcp::list_for_workspace(&db, input.workspace_id)
            .map(|items| items.into_iter().map(McpServerSummaryView::from).collect())
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn mcp_server_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: McpServerUpsertInput,
) -> Result<McpServerSummaryView, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("mcp_server_upsert", move || {
        let McpServerUpsertInput {
            server_id,
            server_key,
            name,
            transport,
            command,
            args,
            env,
            cwd,
            url,
            headers,
        } = input;
        mcp::upsert(
            &app,
            &db,
            server_id,
            &server_key,
            &name,
            &transport,
            command.as_deref(),
            args,
            env.preserve_keys,
            env.replace,
            cwd.as_deref(),
            url.as_deref(),
            headers.preserve_keys,
            headers.replace,
        )
        .map(McpServerSummaryView::from)
    })
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn mcp_server_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: McpServerEnabledInput,
) -> Result<McpServerSummaryView, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("mcp_server_set_enabled", move || {
        mcp::set_enabled(
            &app,
            &db,
            input.workspace_id,
            input.server_id,
            input.enabled,
        )
        .map(McpServerSummaryView::from)
    })
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn mcp_server_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: McpServerDeleteInput,
) -> Result<bool, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run(
        "mcp_server_delete",
        move || -> crate::shared::error::AppResult<bool> {
            mcp::delete(&app, &db, input.server_id)?;
            Ok(true)
        },
    )
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}

#[tauri::command]
#[specta::specta]
pub(crate) fn mcp_parse_json(input: McpParseJsonInput) -> Result<mcp::McpParseResult, String> {
    mcp::parse_json(&input.json_text).map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn mcp_import_servers(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: McpImportServersInput,
) -> Result<mcp::McpImportReport, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("mcp_import_servers", move || {
        mcp::import_servers(&app, &db, input.workspace_id, input.servers)
    })
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn mcp_import_from_workspace_cli(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: McpImportFromWorkspaceCliInput,
) -> Result<mcp::McpImportReport, String> {
    #[cfg(windows)]
    let app_for_wsl = app.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("mcp_import_from_workspace_cli", move || {
        mcp::import_servers_from_workspace_cli(&app, &db, input.workspace_id)
    })
    .await
    .map_err(Into::into);
    #[cfg(windows)]
    if result.is_ok() {
        super::wsl::wsl_sync_trigger::trigger(app_for_wsl);
    }
    result
}
