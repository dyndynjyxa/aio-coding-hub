//! Usage: Community plugin management related Tauri commands.

use crate::app::plugin_service;
use crate::app_state::{ensure_db_ready, DbInitState};
use crate::domain::plugins::{PluginAuditLog, PluginDetail, PluginInstallSource};
use crate::infra::plugins::market::PluginMarketListing;
use crate::{blocking, plugins};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginGetInput {
    pub plugin_id: String,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginInstallFromFileInput {
    pub file_path: String,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginSaveConfigInput {
    pub plugin_id: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginGrantPermissionsInput {
    pub plugin_id: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginRevokePermissionInput {
    pub plugin_id: String,
    pub permission: String,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginListAuditLogsInput {
    pub plugin_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginMarketIndexInput {
    pub index_json: String,
    pub index_url: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginRollbackInput {
    pub plugin_id: String,
    pub version: String,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginInstallRemoteInput {
    pub plugin_id: String,
    pub download_url: String,
    pub checksum: String,
    pub signature: Option<String>,
    pub public_key: Option<String>,
    pub source: Option<String>,
}

fn official_resource_root<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("failed to resolve official plugin resources: {error}"))?;
    Ok(official_resource_root_from_resource_dir(&resource_dir))
}

fn official_resource_root_from_resource_dir(resource_dir: &std::path::Path) -> PathBuf {
    let bundled_root =
        resource_dir.join(crate::app::plugins::official::OFFICIAL_RESOURCE_RELATIVE_ROOT);
    if official_resource_root_exists(&bundled_root) {
        return bundled_root;
    }

    let dev_root = resource_dir.join("plugins/official");
    if official_resource_root_exists(&dev_root) {
        return dev_root;
    }

    #[cfg(test)]
    {
        let source_root = crate::app::plugins::official::official_source_resource_root();
        if official_resource_root_exists(&source_root) {
            return source_root;
        }
    }

    bundled_root
}

fn official_resource_root_exists(root: &std::path::Path) -> bool {
    root.join("privacy-filter").join("plugin.json").exists()
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<Vec<plugins::PluginSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_list", move || plugin_service::list_plugins(&db))
        .await
        .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginGetInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_get", move || {
        plugin_service::get_plugin_detail(&db, &input.plugin_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_install_from_file(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginInstallFromFileInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_install_from_file", move || {
        let path = PathBuf::from(&input.file_path);
        let cache_dir = crate::app_paths::plugins_cache_dir(&app)?;
        let installed_dir = crate::app_paths::plugins_installed_dir(&app)?;
        plugin_service::install_plugin_from_local_package(
            &db,
            &path,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
        )
        .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
        .and_then(|detail| ensure_plugin_runtime_dirs(&app, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_update_from_file(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginInstallFromFileInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_update_from_file", move || {
        let path = PathBuf::from(&input.file_path);
        let cache_dir = crate::app_paths::plugins_cache_dir(&app)?;
        let installed_dir = crate::app_paths::plugins_installed_dir(&app)?;
        plugin_service::update_plugin_from_local_package(
            &db,
            &path,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            plugin_service::LocalPackageInstallPolicy::default(),
        )
        .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
        .and_then(|detail| ensure_plugin_runtime_dirs(&app, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_rollback(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginRollbackInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_rollback", move || {
        plugin_service::rollback_plugin_to_version(&db, &input.plugin_id, &input.version)
            .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
    })
    .await
    .map_err(Into::into)
}

fn market_index_trusted_public_key(
    db: &crate::db::Db,
    input: &PluginMarketIndexInput,
) -> crate::shared::error::AppResult<Option<String>> {
    if input.signature.is_none() {
        return Ok(None);
    }
    let index_url = input.index_url.as_deref().ok_or_else(|| {
        crate::shared::error::AppError::new(
            "PLUGIN_MARKET_SOURCE_URL_REQUIRED",
            "signed market index parsing requires a market source URL",
        )
    })?;
    crate::infra::plugins::repository::trusted_market_public_key_for_url(db, index_url)?
        .ok_or_else(|| {
            crate::shared::error::AppError::new(
                "PLUGIN_MARKET_TRUSTED_PUBLIC_KEY_REQUIRED",
                "signed market index parsing requires a trusted market source public key",
            )
        })
        .map(Some)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_parse_market_index(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginMarketIndexInput,
) -> Result<Vec<PluginMarketListing>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_parse_market_index", move || {
        let installed: HashMap<String, String> = plugin_service::list_plugins(&db)?
            .into_iter()
            .filter_map(|plugin| {
                plugin
                    .current_version
                    .map(|version| (plugin.plugin_id, version))
            })
            .collect();
        match (
            input.signature.as_deref(),
            market_index_trusted_public_key(&db, &input)?.as_deref(),
        ) {
            (Some(signature), Some(public_key)) => {
                crate::infra::plugins::market::parse_signed_market_index(
                    input.index_json.as_bytes(),
                    signature,
                    public_key,
                    env!("CARGO_PKG_VERSION"),
                    &installed,
                )
            }
            (None, None) => crate::infra::plugins::market::parse_market_index(
                input.index_json.as_bytes(),
                env!("CARGO_PKG_VERSION"),
                &installed,
            ),
            _ => Err(crate::shared::error::AppError::new(
                "PLUGIN_MARKET_SIGNATURE_POLICY_INCOMPLETE",
                "market index signature verification requires signature and trusted public key",
            )),
        }
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_install_remote(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginInstallRemoteInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let package_bytes = download_remote_plugin_package(&input.download_url).await?;
    blocking::run("plugin_install_remote", move || {
        let cache_dir = crate::app_paths::plugins_cache_dir(&app)?;
        let installed_dir = crate::app_paths::plugins_installed_dir(&app)?;
        let install_source = remote_install_source(input.source.as_deref(), &input.download_url)?;
        plugin_service::install_plugin_from_remote_package_bytes(
            &db,
            package_bytes,
            &input.download_url,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            plugin_service::RemotePackageInstallPolicy {
                install_source,
                expected_plugin_id: input.plugin_id,
                expected_checksum: input.checksum,
                signature: input.signature,
                public_key: input.public_key,
            },
        )
        .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
        .and_then(|detail| ensure_plugin_runtime_dirs(&app, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_install_official(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginGetInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let official_resource_root = official_resource_root(&app)?;
    blocking::run("plugin_install_official", move || {
        let installed_dir = crate::app_paths::plugins_installed_dir(&app)?;
        plugin_service::install_official_plugin(
            &db,
            &input.plugin_id,
            &official_resource_root,
            env!("CARGO_PKG_VERSION"),
            &installed_dir,
        )
        .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
        .and_then(|detail| ensure_plugin_runtime_dirs(&app, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_quarantine_revoked(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginGetInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_quarantine_revoked", move || {
        plugin_service::quarantine_revoked_plugin(
            &db,
            &input.plugin_id,
            "Plugin revoked by market index",
        )
        .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_enable(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginGetInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_enable", move || {
        plugin_service::enable_plugin(&db, &input.plugin_id, env!("CARGO_PKG_VERSION"))
            .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
    })
    .await
    .map_err(Into::into)
}

fn ensure_plugin_runtime_dirs(
    app: &tauri::AppHandle,
    detail: PluginDetail,
) -> crate::shared::error::AppResult<PluginDetail> {
    let data_dir = crate::app_paths::plugin_data_dir(app, &detail.summary.plugin_id)?;
    let logs_dir = crate::app_paths::plugins_logs_dir(app)?;
    std::fs::create_dir_all(&data_dir).map_err(|e| {
        format!(
            "failed to create plugin data dir {}: {e}",
            data_dir.display()
        )
    })?;
    std::fs::create_dir_all(&logs_dir).map_err(|e| {
        format!(
            "failed to create plugin logs dir {}: {e}",
            logs_dir.display()
        )
    })?;
    Ok(detail)
}

fn refresh_running_gateway_plugins(
    app: &tauri::AppHandle,
    db: &crate::db::Db,
    detail: PluginDetail,
) -> crate::shared::error::AppResult<PluginDetail> {
    crate::app::gateway_control::app_refresh_gateway_plugins(app, db);
    Ok(detail)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_disable(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginGetInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_disable", move || {
        plugin_service::disable_plugin(&db, &input.plugin_id)
            .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_uninstall(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginGetInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_uninstall", move || {
        plugin_service::uninstall_plugin(&db, &input.plugin_id)
            .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_save_config(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginSaveConfigInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_save_config", move || {
        plugin_service::save_plugin_config(&db, &input.plugin_id, input.config)
            .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_grant_permissions(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginGrantPermissionsInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_grant_permissions", move || {
        plugin_service::grant_plugin_permissions(&db, &input.plugin_id, input.permissions)
            .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_revoke_permission(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginRevokePermissionInput,
) -> Result<PluginDetail, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("plugin_revoke_permission", move || {
        plugin_service::revoke_plugin_permission(&db, &input.plugin_id, &input.permission)
            .and_then(|detail| refresh_running_gateway_plugins(&app, &db, detail))
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn plugin_list_audit_logs(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    input: PluginListAuditLogsInput,
) -> Result<Vec<PluginAuditLog>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("plugin_list_audit_logs", move || {
        crate::infra::plugins::repository::list_audit_logs(
            &db,
            input.plugin_id.as_deref(),
            input.limit.unwrap_or(50),
        )
    })
    .await
    .map_err(Into::into)
}

const MAX_REMOTE_PLUGIN_PACKAGE_BYTES: usize = 32 * 1024 * 1024;

fn remote_install_source(
    source: Option<&str>,
    download_url: &str,
) -> crate::shared::error::AppResult<PluginInstallSource> {
    match source.unwrap_or_else(|| {
        if download_url.contains("github.com/") {
            "github_release"
        } else {
            "market"
        }
    }) {
        "market" => Ok(PluginInstallSource::Market),
        "github_release" => Ok(PluginInstallSource::GithubRelease),
        other => Err(crate::shared::error::AppError::new(
            "PLUGIN_REMOTE_SOURCE_INVALID",
            format!("unsupported remote plugin source: {other}"),
        )),
    }
}

async fn download_remote_plugin_package(download_url: &str) -> Result<Vec<u8>, String> {
    validate_remote_plugin_download_url(download_url)?;
    let parsed = reqwest::Url::parse(download_url)
        .map_err(|err| format!("PLUGIN_REMOTE_DOWNLOAD_URL_INVALID: {err}"))?;
    if parsed.scheme() == "file" {
        let path = parsed
            .to_file_path()
            .map_err(|_| "PLUGIN_REMOTE_DOWNLOAD_URL_INVALID: invalid file URL".to_string())?;
        let bytes =
            std::fs::read(&path).map_err(|err| format!("PLUGIN_REMOTE_DOWNLOAD_FAILED: {err}"))?;
        if bytes.len() > MAX_REMOTE_PLUGIN_PACKAGE_BYTES {
            return Err(format!(
                "PLUGIN_REMOTE_PACKAGE_TOO_LARGE: plugin package exceeds {} bytes",
                MAX_REMOTE_PLUGIN_PACKAGE_BYTES
            ));
        }
        return Ok(bytes);
    }

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|err| format!("PLUGIN_REMOTE_DOWNLOAD_FAILED: failed to build client: {err}"))?;
    let response = client
        .get(parsed)
        .send()
        .await
        .map_err(|err| format!("PLUGIN_REMOTE_DOWNLOAD_FAILED: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("PLUGIN_REMOTE_DOWNLOAD_FAILED: HTTP {status}"));
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_REMOTE_PLUGIN_PACKAGE_BYTES as u64)
    {
        return Err(format!(
            "PLUGIN_REMOTE_PACKAGE_TOO_LARGE: plugin package exceeds {} bytes",
            MAX_REMOTE_PLUGIN_PACKAGE_BYTES
        ));
    }
    let mut out = Vec::new();
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| format!("PLUGIN_REMOTE_DOWNLOAD_FAILED: {err}"))?
    {
        if out.len().saturating_add(chunk.len()) > MAX_REMOTE_PLUGIN_PACKAGE_BYTES {
            return Err(format!(
                "PLUGIN_REMOTE_PACKAGE_TOO_LARGE: plugin package exceeds {} bytes",
                MAX_REMOTE_PLUGIN_PACKAGE_BYTES
            ));
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

fn validate_remote_plugin_download_url(download_url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(download_url)
        .map_err(|err| format!("PLUGIN_REMOTE_DOWNLOAD_URL_INVALID: {err}"))?;
    if !matches!(parsed.scheme(), "https" | "file") {
        return Err("PLUGIN_REMOTE_DOWNLOAD_URL_INVALID: only https:// or file:// plugin packages are supported".to_string());
    }
    if parsed.scheme() == "https" && parsed.host_str().is_none() {
        return Err(
            "PLUGIN_REMOTE_DOWNLOAD_URL_INVALID: remote plugin URL must include a host".to_string(),
        );
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Err("PLUGIN_REMOTE_DOWNLOAD_URL_INVALID: credentials are not allowed in plugin download URLs".to_string());
    }
    if !parsed.path().ends_with(".aio-plugin") {
        return Err(
            "PLUGIN_REMOTE_DOWNLOAD_URL_INVALID: remote artifact must be a .aio-plugin file"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_resource_root_resolves_packaged_plugin_manifest() {
        let app = tauri::test::mock_app();

        let root = official_resource_root(app.handle()).expect("official resource root");

        let manifest = root.join("privacy-filter").join("plugin.json");
        assert!(
            manifest.exists(),
            "official plugin manifest should exist at {}",
            manifest.display()
        );
    }

    #[test]
    fn official_resource_root_falls_back_to_source_resources_for_tests() {
        let dir = tempfile::tempdir().unwrap();

        let root = official_resource_root_from_resource_dir(dir.path());

        assert_eq!(
            root,
            crate::app::plugins::official::official_source_resource_root()
        );
        assert!(
            root.join("privacy-filter").join("plugin.json").exists(),
            "official plugin manifest should exist at {}",
            root.display()
        );
    }

    #[test]
    fn market_index_signature_uses_trusted_source_public_key() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let conn = db.open_connection().unwrap();
        conn.execute(
            r#"
INSERT INTO plugin_market_sources(
  name,
  index_url,
  enabled,
  trusted_public_key,
  created_at,
  updated_at
) VALUES (?1, ?2, 1, ?3, 1, 1)
"#,
            rusqlite::params![
                "Community",
                "https://plugins.example.test/index.json",
                "trusted-key"
            ],
        )
        .unwrap();
        drop(conn);
        let input = PluginMarketIndexInput {
            index_json: "{}".to_string(),
            index_url: Some("https://plugins.example.test/index.json".to_string()),
            signature: Some("signature".to_string()),
        };

        let public_key = market_index_trusted_public_key(&db, &input).unwrap();

        assert_eq!(public_key.as_deref(), Some("trusted-key"));
    }
}
