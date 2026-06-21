use crate::db;
use crate::domain::plugins::{
    validate_manifest, PluginAuditLog, PluginDetail, PluginInstallSource, PluginManifest,
    PluginPermissionRisk, PluginRuntime, PluginRuntimeFailure, PluginStatus, PluginSummary,
};
use crate::shared::error::{db_err, AppResult};
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, OptionalExtension};

pub(crate) struct InsertPluginInput {
    pub manifest: PluginManifest,
    pub install_source: PluginInstallSource,
    pub status: PluginStatus,
    pub installed_dir: Option<String>,
}

pub(crate) struct AppendPluginAuditLogInput {
    pub plugin_id: Option<String>,
    pub trace_id: Option<String>,
    pub event_type: String,
    pub risk_level: String,
    pub message: String,
    pub details: serde_json::Value,
}

pub(crate) struct RecordPluginRuntimeFailureInput {
    pub plugin_id: String,
    pub hook_name: Option<String>,
    pub failure_kind: String,
    pub message: String,
    pub trace_id: Option<String>,
}

pub(crate) fn trusted_market_public_key_for_url(
    db: &db::Db,
    url: &str,
) -> AppResult<Option<String>> {
    let conn = db.open_connection()?;
    let url_host = reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase));
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT index_url, trusted_public_key
FROM plugin_market_sources
WHERE enabled = 1
  AND trusted_public_key IS NOT NULL
  AND TRIM(trusted_public_key) != ''
"#,
        )
        .map_err(|e| db_err!("failed to prepare plugin market source query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| db_err!("failed to query plugin market sources: {e}"))?;

    for row in rows {
        let (index_url, public_key) =
            row.map_err(|e| db_err!("failed to read plugin market source: {e}"))?;
        if index_url == url {
            return Ok(Some(public_key));
        }
        let source_host = reqwest::Url::parse(&index_url)
            .ok()
            .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase));
        if source_host.is_some() && source_host == url_host {
            return Ok(Some(public_key));
        }
    }

    Ok(None)
}

pub(crate) fn list_plugins(db: &db::Db) -> AppResult<Vec<PluginSummary>> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  id,
  plugin_id,
  name,
  current_version,
  install_source,
  status,
  manifest_json,
  last_error,
  created_at,
  updated_at
FROM plugins
WHERE status != 'uninstalled'
ORDER BY updated_at DESC, id DESC
"#,
        )
        .map_err(|e| db_err!("failed to prepare plugin list query: {e}"))?;

    let rows = stmt
        .query_map([], summary_from_row)
        .map_err(|e| db_err!("failed to query plugins: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read plugin row: {e}"))?);
    }
    Ok(out)
}

pub(crate) fn get_plugin(db: &db::Db, plugin_id: &str) -> AppResult<PluginDetail> {
    let conn = db.open_connection()?;
    get_plugin_with_conn(&conn, plugin_id)
}

fn get_plugin_with_conn(conn: &rusqlite::Connection, plugin_id: &str) -> AppResult<PluginDetail> {
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  id,
  plugin_id,
  name,
  current_version,
  install_source,
  status,
  manifest_json,
  config_json,
  granted_permissions_json,
  installed_dir,
  last_error,
  created_at,
  updated_at
FROM plugins
WHERE plugin_id = ?1
"#,
        )
        .map_err(|e| db_err!("failed to prepare plugin detail query: {e}"))?;

    let row = stmt
        .query_row(params![plugin_id], detail_from_row)
        .optional()
        .map_err(|e| db_err!("failed to query plugin detail: {e}"))?
        .ok_or_else(|| crate::shared::error::AppError::new("DB_NOT_FOUND", "plugin not found"))?;

    let config = load_plugin_config(conn, plugin_id)?.unwrap_or(row.config);
    let (granted_permissions, pending_permissions) =
        load_plugin_permissions(conn, plugin_id)?.unwrap_or((row.granted_permissions, Vec::new()));
    let audit_logs = list_audit_logs_with_conn(conn, Some(plugin_id), 50)?;
    let runtime_failures = list_runtime_failures_with_conn(conn, plugin_id, 50)?;

    Ok(PluginDetail {
        summary: row.summary,
        manifest: row.manifest,
        install_source: row.install_source,
        installed_dir: row.installed_dir,
        config,
        granted_permissions,
        pending_permissions,
        audit_logs,
        runtime_failures,
    })
}

pub(crate) fn insert_plugin(db: &db::Db, input: InsertPluginInput) -> AppResult<PluginDetail> {
    validate_manifest(&input.manifest, env!("CARGO_PKG_VERSION"))?;
    let conn = db.open_connection()?;
    let now = now_unix_seconds();
    let manifest_json = serde_json::to_string(&input.manifest)
        .map_err(|e| format!("PLUGIN_INVALID_MANIFEST: failed to serialize manifest: {e}"))?;

    conn.execute(
        r#"
INSERT INTO plugins(
  plugin_id,
  name,
  current_version,
  install_source,
  status,
  manifest_json,
  config_json,
  granted_permissions_json,
  installed_dir,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', '[]', ?7, ?8, ?9)
ON CONFLICT(plugin_id) DO UPDATE SET
  name = excluded.name,
  current_version = excluded.current_version,
  install_source = excluded.install_source,
  status = excluded.status,
  manifest_json = excluded.manifest_json,
  installed_dir = excluded.installed_dir,
  updated_at = excluded.updated_at
"#,
        params![
            input.manifest.id,
            input.manifest.name,
            input.manifest.version,
            input.install_source.as_str(),
            input.status.as_str(),
            manifest_json,
            input.installed_dir,
            now,
            now
        ],
    )
    .map_err(|e| db_err!("failed to insert plugin: {e}"))?;

    conn.execute(
        r#"
INSERT OR IGNORE INTO plugin_versions(
  plugin_id,
  version,
  manifest_json,
  installed_dir,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5)
"#,
        params![
            input.manifest.id,
            input.manifest.version,
            serde_json::to_string(&input.manifest).unwrap_or_else(|_| "{}".to_string()),
            input.installed_dir,
            now
        ],
    )
    .map_err(|e| db_err!("failed to insert plugin version: {e}"))?;

    get_plugin_with_conn(&conn, &input.manifest.id)
}

pub(crate) fn update_plugin_status(
    db: &db::Db,
    plugin_id: &str,
    status: PluginStatus,
    last_error: Option<&str>,
) -> AppResult<PluginDetail> {
    let conn = db.open_connection()?;
    let changed = conn
        .execute(
            r#"
UPDATE plugins
SET status = ?1, last_error = ?2, updated_at = ?3
WHERE plugin_id = ?4
"#,
            params![status.as_str(), last_error, now_unix_seconds(), plugin_id],
        )
        .map_err(|e| db_err!("failed to update plugin status: {e}"))?;
    if changed == 0 {
        return Err(crate::shared::error::AppError::new(
            "DB_NOT_FOUND",
            "plugin not found",
        ));
    }
    get_plugin_with_conn(&conn, plugin_id)
}

pub(crate) fn update_plugin_manifest(
    db: &db::Db,
    manifest: PluginManifest,
    installed_dir: Option<String>,
) -> AppResult<PluginDetail> {
    validate_manifest(&manifest, env!("CARGO_PKG_VERSION"))?;
    let conn = db.open_connection()?;
    let now = now_unix_seconds();
    let plugin_id = manifest.id.clone();
    let version = manifest.version.clone();
    let name = manifest.name.clone();
    let manifest_json = serde_json::to_string(&manifest)
        .map_err(|e| format!("PLUGIN_INVALID_MANIFEST: failed to serialize manifest: {e}"))?;

    let changed = conn
        .execute(
            r#"
UPDATE plugins
SET name = ?1,
    current_version = ?2,
    status = ?3,
    manifest_json = ?4,
    installed_dir = ?5,
    last_error = NULL,
    updated_at = ?6
WHERE plugin_id = ?7
"#,
            params![
                name,
                version.clone(),
                PluginStatus::Disabled.as_str(),
                manifest_json,
                installed_dir.clone(),
                now,
                plugin_id.clone()
            ],
        )
        .map_err(|e| db_err!("failed to update plugin manifest: {e}"))?;
    if changed == 0 {
        return Err(crate::shared::error::AppError::new(
            "DB_NOT_FOUND",
            "plugin not found",
        ));
    }

    conn.execute(
        r#"
INSERT OR IGNORE INTO plugin_versions(
  plugin_id,
  version,
  manifest_json,
  installed_dir,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5)
"#,
        params![
            plugin_id.clone(),
            version.clone(),
            serde_json::to_string(&manifest).unwrap_or_else(|_| "{}".to_string()),
            installed_dir,
            now
        ],
    )
    .map_err(|e| db_err!("failed to insert plugin version: {e}"))?;

    get_plugin_with_conn(&conn, &manifest.id)
}

pub(crate) fn get_plugin_version(
    db: &db::Db,
    plugin_id: &str,
    version: &str,
) -> AppResult<(PluginManifest, Option<String>)> {
    let conn = db.open_connection()?;
    conn.query_row(
        r#"
SELECT manifest_json, installed_dir
FROM plugin_versions
WHERE plugin_id = ?1 AND version = ?2
"#,
        params![plugin_id, version],
        |row| {
            let manifest_json: String = row.get(0)?;
            let manifest: PluginManifest = serde_json::from_str(&manifest_json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    manifest_json.len(),
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            Ok((manifest, row.get(1)?))
        },
    )
    .optional()
    .map_err(|e| db_err!("failed to query plugin version: {e}"))?
    .ok_or_else(|| crate::shared::error::AppError::new("DB_NOT_FOUND", "plugin version not found"))
}

pub(crate) fn save_plugin_config(
    db: &db::Db,
    plugin_id: &str,
    config_version: u32,
    config: &serde_json::Value,
    sensitive_keys: &[String],
) -> AppResult<PluginDetail> {
    let conn = db.open_connection()?;
    ensure_plugin_exists(&conn, plugin_id)?;
    let now = now_unix_seconds();
    let config_json = serde_json::to_string(config)
        .map_err(|e| format!("PLUGIN_INVALID_CONFIG: failed to serialize config: {e}"))?;
    let sensitive_keys_json = serde_json::to_string(sensitive_keys)
        .map_err(|e| format!("PLUGIN_INVALID_CONFIG: failed to serialize sensitive keys: {e}"))?;

    conn.execute(
        r#"
INSERT INTO plugin_configs(
  plugin_id,
  config_version,
  config_json,
  sensitive_keys_json,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5)
ON CONFLICT(plugin_id) DO UPDATE SET
  config_version = excluded.config_version,
  config_json = excluded.config_json,
  sensitive_keys_json = excluded.sensitive_keys_json,
  updated_at = excluded.updated_at
"#,
        params![
            plugin_id,
            config_version,
            config_json,
            sensitive_keys_json,
            now
        ],
    )
    .map_err(|e| db_err!("failed to save plugin config: {e}"))?;

    conn.execute(
        "UPDATE plugins SET config_json = ?1, updated_at = ?2 WHERE plugin_id = ?3",
        params![config_json, now, plugin_id],
    )
    .map_err(|e| db_err!("failed to mirror plugin config: {e}"))?;

    get_plugin_with_conn(&conn, plugin_id)
}

pub(crate) fn plugin_config_version(db: &db::Db, plugin_id: &str) -> AppResult<Option<u32>> {
    let conn = db.open_connection()?;
    conn.query_row(
        "SELECT config_version FROM plugin_configs WHERE plugin_id = ?1",
        params![plugin_id],
        |row| row.get::<_, u32>(0),
    )
    .optional()
    .map_err(|e| db_err!("failed to query plugin config version: {e}"))
}

pub(crate) fn save_plugin_permissions(
    db: &db::Db,
    plugin_id: &str,
    permissions: &[String],
    pending_permissions: &[String],
) -> AppResult<PluginDetail> {
    let conn = db.open_connection()?;
    ensure_plugin_exists(&conn, plugin_id)?;
    let now = now_unix_seconds();
    let permissions_json = serde_json::to_string(permissions)
        .map_err(|e| format!("PLUGIN_INVALID_PERMISSION: failed to serialize permissions: {e}"))?;
    let pending_permissions_json = serde_json::to_string(pending_permissions).map_err(|e| {
        format!("PLUGIN_INVALID_PERMISSION: failed to serialize pending permissions: {e}")
    })?;

    conn.execute(
        r#"
INSERT INTO plugin_permissions(
  plugin_id,
  permissions_json,
  pending_permissions_json,
  updated_at
) VALUES (?1, ?2, ?3, ?4)
ON CONFLICT(plugin_id) DO UPDATE SET
  permissions_json = excluded.permissions_json,
  pending_permissions_json = excluded.pending_permissions_json,
  updated_at = excluded.updated_at
"#,
        params![plugin_id, permissions_json, pending_permissions_json, now],
    )
    .map_err(|e| db_err!("failed to save plugin permissions: {e}"))?;

    conn.execute(
        "UPDATE plugins SET granted_permissions_json = ?1, updated_at = ?2 WHERE plugin_id = ?3",
        params![permissions_json, now, plugin_id],
    )
    .map_err(|e| db_err!("failed to mirror plugin permissions: {e}"))?;

    get_plugin_with_conn(&conn, plugin_id)
}

pub(crate) fn append_audit_log(
    db: &db::Db,
    input: AppendPluginAuditLogInput,
) -> AppResult<PluginAuditLog> {
    let conn = db.open_connection()?;
    let details_json = serde_json::to_string(&input.details)
        .map_err(|e| format!("PLUGIN_INVALID_AUDIT: failed to serialize details: {e}"))?;
    let now = now_unix_seconds();
    conn.execute(
        r#"
INSERT INTO plugin_audit_logs(
  plugin_id,
  trace_id,
  event_type,
  risk_level,
  message,
  details_json,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
"#,
        params![
            input.plugin_id,
            input.trace_id,
            input.event_type,
            input.risk_level,
            input.message,
            details_json,
            now
        ],
    )
    .map_err(|e| db_err!("failed to append plugin audit log: {e}"))?;

    let id = conn.last_insert_rowid();
    get_audit_log_by_id(&conn, id)
}

pub(crate) fn list_audit_logs(
    db: &db::Db,
    plugin_id: Option<&str>,
    limit: usize,
) -> AppResult<Vec<PluginAuditLog>> {
    let conn = db.open_connection()?;
    list_audit_logs_with_conn(&conn, plugin_id, limit)
}

pub(crate) fn record_runtime_failure(
    db: &db::Db,
    input: RecordPluginRuntimeFailureInput,
) -> AppResult<PluginRuntimeFailure> {
    let conn = db.open_connection()?;
    ensure_plugin_exists(&conn, &input.plugin_id)?;
    let now = now_unix_seconds();
    conn.execute(
        r#"
INSERT INTO plugin_runtime_failures(
  plugin_id,
  hook_name,
  failure_kind,
  message,
  trace_id,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
"#,
        params![
            input.plugin_id,
            input.hook_name,
            input.failure_kind,
            input.message,
            input.trace_id,
            now
        ],
    )
    .map_err(|e| db_err!("failed to record plugin runtime failure: {e}"))?;

    let id = conn.last_insert_rowid();
    get_runtime_failure_by_id(&conn, id)
}

struct PluginDetailRow {
    summary: PluginSummary,
    manifest: PluginManifest,
    install_source: PluginInstallSource,
    installed_dir: Option<String>,
    config: serde_json::Value,
    granted_permissions: Vec<String>,
}

fn summary_from_row(row: &rusqlite::Row<'_>) -> Result<PluginSummary, rusqlite::Error> {
    let manifest_json: String = row.get("manifest_json")?;
    let manifest = parse_manifest_lossy(&manifest_json);
    let status_raw: String = row.get("status")?;
    let status = PluginStatus::parse(&status_raw).unwrap_or(PluginStatus::Incompatible);
    let runtime = manifest
        .as_ref()
        .map(runtime_name)
        .unwrap_or_else(|| "unknown".to_string());
    let permission_risk = manifest
        .as_ref()
        .map(|manifest| highest_permission_risk(&manifest.permissions))
        .unwrap_or(PluginPermissionRisk::Low);

    Ok(PluginSummary {
        id: row.get("id")?,
        plugin_id: row.get("plugin_id")?,
        name: row.get("name")?,
        current_version: row.get("current_version")?,
        status,
        runtime,
        permission_risk,
        update_available: status == PluginStatus::UpdateAvailable,
        last_error: row.get("last_error")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn detail_from_row(row: &rusqlite::Row<'_>) -> Result<PluginDetailRow, rusqlite::Error> {
    let manifest_json: String = row.get("manifest_json")?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_json).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            manifest_json.len(),
            rusqlite::types::Type::Text,
            Box::new(err),
        )
    })?;
    let install_source_raw: String = row.get("install_source")?;
    let install_source =
        PluginInstallSource::parse(&install_source_raw).unwrap_or(PluginInstallSource::Local);
    let config_json: String = row.get("config_json")?;
    let config = parse_json_value(&config_json);
    let permissions_json: String = row.get("granted_permissions_json")?;
    let granted_permissions = parse_string_array(&permissions_json);

    Ok(PluginDetailRow {
        summary: summary_from_row(row)?,
        manifest,
        install_source,
        installed_dir: row.get("installed_dir")?,
        config,
        granted_permissions,
    })
}

fn runtime_name(manifest: &PluginManifest) -> String {
    match manifest.runtime {
        PluginRuntime::DeclarativeRules { .. } => "declarativeRules".to_string(),
        PluginRuntime::Native { ref engine } => format!("native:{engine}"),
        PluginRuntime::Wasm { .. } => "wasm".to_string(),
    }
}

fn highest_permission_risk(permissions: &[String]) -> PluginPermissionRisk {
    use crate::domain::plugins::permission_risk;
    permissions
        .iter()
        .filter_map(|permission| permission_risk(permission))
        .max_by_key(|risk| match risk {
            PluginPermissionRisk::Low => 0,
            PluginPermissionRisk::Medium => 1,
            PluginPermissionRisk::High => 2,
            PluginPermissionRisk::Critical => 3,
        })
        .unwrap_or(PluginPermissionRisk::Low)
}

fn parse_manifest_lossy(raw: &str) -> Option<PluginManifest> {
    serde_json::from_str(raw).ok()
}

fn parse_json_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}))
}

fn parse_string_array(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn ensure_plugin_exists(conn: &rusqlite::Connection, plugin_id: &str) -> AppResult<()> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM plugins WHERE plugin_id = ?1 LIMIT 1",
            params![plugin_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| db_err!("failed to query plugin existence: {e}"))?
        .unwrap_or(false);
    if exists {
        Ok(())
    } else {
        Err(crate::shared::error::AppError::new(
            "DB_NOT_FOUND",
            "plugin not found",
        ))
    }
}

fn load_plugin_config(
    conn: &rusqlite::Connection,
    plugin_id: &str,
) -> AppResult<Option<serde_json::Value>> {
    conn.query_row(
        "SELECT config_json FROM plugin_configs WHERE plugin_id = ?1",
        params![plugin_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| db_err!("failed to query plugin config: {e}"))
    .map(|raw| raw.map(|value| parse_json_value(&value)))
}

fn load_plugin_permissions(
    conn: &rusqlite::Connection,
    plugin_id: &str,
) -> AppResult<Option<(Vec<String>, Vec<String>)>> {
    conn.query_row(
        "SELECT permissions_json, pending_permissions_json FROM plugin_permissions WHERE plugin_id = ?1",
        params![plugin_id],
        |row| {
            Ok((
                parse_string_array(&row.get::<_, String>(0)?),
                parse_string_array(&row.get::<_, String>(1)?),
            ))
        },
    )
    .optional()
    .map_err(|e| db_err!("failed to query plugin permissions: {e}"))
}

fn list_audit_logs_with_conn(
    conn: &rusqlite::Connection,
    plugin_id: Option<&str>,
    limit: usize,
) -> AppResult<Vec<PluginAuditLog>> {
    let limit = limit.clamp(1, 500) as i64;
    let mut out = Vec::new();
    if let Some(plugin_id) = plugin_id {
        let mut stmt = conn
            .prepare_cached(
                r#"
SELECT id, plugin_id, trace_id, event_type, risk_level, message, details_json, created_at
FROM plugin_audit_logs
WHERE plugin_id = ?1
ORDER BY created_at DESC, id DESC
LIMIT ?2
"#,
            )
            .map_err(|e| db_err!("failed to prepare plugin audit query: {e}"))?;
        let rows = stmt
            .query_map(params![plugin_id, limit], audit_log_from_row)
            .map_err(|e| db_err!("failed to query plugin audit logs: {e}"))?;
        for row in rows {
            out.push(row.map_err(|e| db_err!("failed to read plugin audit log: {e}"))?);
        }
    } else {
        let mut stmt = conn
            .prepare_cached(
                r#"
SELECT id, plugin_id, trace_id, event_type, risk_level, message, details_json, created_at
FROM plugin_audit_logs
ORDER BY created_at DESC, id DESC
LIMIT ?1
"#,
            )
            .map_err(|e| db_err!("failed to prepare plugin audit query: {e}"))?;
        let rows = stmt
            .query_map(params![limit], audit_log_from_row)
            .map_err(|e| db_err!("failed to query plugin audit logs: {e}"))?;
        for row in rows {
            out.push(row.map_err(|e| db_err!("failed to read plugin audit log: {e}"))?);
        }
    }
    Ok(out)
}

fn get_audit_log_by_id(conn: &rusqlite::Connection, id: i64) -> AppResult<PluginAuditLog> {
    conn.query_row(
        r#"
SELECT id, plugin_id, trace_id, event_type, risk_level, message, details_json, created_at
FROM plugin_audit_logs
WHERE id = ?1
"#,
        params![id],
        audit_log_from_row,
    )
    .map_err(|e| db_err!("failed to query inserted plugin audit log: {e}"))
}

fn audit_log_from_row(row: &rusqlite::Row<'_>) -> Result<PluginAuditLog, rusqlite::Error> {
    let details_json: String = row.get("details_json")?;
    Ok(PluginAuditLog {
        id: row.get("id")?,
        plugin_id: row.get("plugin_id")?,
        trace_id: row.get("trace_id")?,
        event_type: row.get("event_type")?,
        risk_level: row.get("risk_level")?,
        message: row.get("message")?,
        details: parse_json_value(&details_json),
        created_at: row.get("created_at")?,
    })
}

fn list_runtime_failures_with_conn(
    conn: &rusqlite::Connection,
    plugin_id: &str,
    limit: usize,
) -> AppResult<Vec<PluginRuntimeFailure>> {
    let limit = limit.clamp(1, 500) as i64;
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT id, plugin_id, hook_name, failure_kind, message, trace_id, created_at
FROM plugin_runtime_failures
WHERE plugin_id = ?1
ORDER BY created_at DESC, id DESC
LIMIT ?2
"#,
        )
        .map_err(|e| db_err!("failed to prepare plugin runtime failure query: {e}"))?;
    let rows = stmt
        .query_map(params![plugin_id, limit], runtime_failure_from_row)
        .map_err(|e| db_err!("failed to query plugin runtime failures: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read plugin runtime failure: {e}"))?);
    }
    Ok(out)
}

fn get_runtime_failure_by_id(
    conn: &rusqlite::Connection,
    id: i64,
) -> AppResult<PluginRuntimeFailure> {
    conn.query_row(
        r#"
SELECT id, plugin_id, hook_name, failure_kind, message, trace_id, created_at
FROM plugin_runtime_failures
WHERE id = ?1
"#,
        params![id],
        runtime_failure_from_row,
    )
    .map_err(|e| db_err!("failed to query inserted plugin runtime failure: {e}"))
}

fn runtime_failure_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<PluginRuntimeFailure, rusqlite::Error> {
    Ok(PluginRuntimeFailure {
        id: row.get("id")?,
        plugin_id: row.get("plugin_id")?,
        hook_name: row.get("hook_name")?,
        failure_kind: row.get("failure_kind")?,
        message: row.get("message")?,
        trace_id: row.get("trace_id")?,
        created_at: row.get("created_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plugins::{PluginInstallSource, PluginManifest, PluginStatus};

    fn manifest() -> PluginManifest {
        serde_json::from_value(serde_json::json!({
            "id": "community.prompt-helper",
            "name": "Community Prompt Helper",
            "version": "1.0.0",
            "apiVersion": "1.0.0",
            "runtime": {
                "kind": "declarativeRules",
                "rules": ["rules/main.json"]
            },
            "hooks": [
                {
                    "name": "gateway.request.afterBodyRead",
                    "priority": 100,
                    "failurePolicy": "fail-open"
                }
            ],
            "permissions": ["request.body.read", "request.body.write"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            }
        }))
        .unwrap()
    }

    #[test]
    fn repository_round_trips_plugin_state_config_permissions_and_audit() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let manifest = manifest();

        insert_plugin(
            &db,
            InsertPluginInput {
                manifest: manifest.clone(),
                install_source: PluginInstallSource::Local,
                status: PluginStatus::Disabled,
                installed_dir: None,
            },
        )
        .unwrap();

        save_plugin_config(
            &db,
            "community.prompt-helper",
            1,
            &serde_json::json!({"mode": "append_instruction"}),
            &[],
        )
        .unwrap();
        save_plugin_permissions(
            &db,
            "community.prompt-helper",
            &[
                "request.body.read".to_string(),
                "request.body.write".to_string(),
            ],
            &[],
        )
        .unwrap();
        update_plugin_status(&db, "community.prompt-helper", PluginStatus::Enabled, None).unwrap();
        append_audit_log(
            &db,
            AppendPluginAuditLogInput {
                plugin_id: Some("community.prompt-helper".to_string()),
                trace_id: Some("trace-1".to_string()),
                event_type: "plugin.enabled".to_string(),
                risk_level: "low".to_string(),
                message: "Plugin enabled".to_string(),
                details: serde_json::json!({"source": "test"}),
            },
        )
        .unwrap();
        record_runtime_failure(
            &db,
            RecordPluginRuntimeFailureInput {
                plugin_id: "community.prompt-helper".to_string(),
                hook_name: Some("gateway.request.afterBodyRead".to_string()),
                failure_kind: "timeout".to_string(),
                message: "Hook timed out".to_string(),
                trace_id: Some("trace-1".to_string()),
            },
        )
        .unwrap();

        let list = list_plugins(&db).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].plugin_id, "community.prompt-helper");
        assert_eq!(list[0].status, PluginStatus::Enabled);
        assert_eq!(list[0].runtime, "declarativeRules");

        let detail = get_plugin(&db, "community.prompt-helper").unwrap();
        assert_eq!(detail.manifest, manifest);
        assert_eq!(detail.config["mode"], "append_instruction");
        assert_eq!(
            detail.granted_permissions,
            vec![
                "request.body.read".to_string(),
                "request.body.write".to_string()
            ]
        );
        assert_eq!(detail.audit_logs.len(), 1);
        assert_eq!(detail.runtime_failures.len(), 1);
    }

    #[test]
    fn repository_resolves_enabled_market_source_public_key_by_url_host() {
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
) VALUES (?1, ?2, ?3, ?4, 1, 1)
"#,
            rusqlite::params![
                "Community",
                "https://plugins.example.test/index.json",
                1,
                "trusted-key"
            ],
        )
        .unwrap();
        conn.execute(
            r#"
INSERT INTO plugin_market_sources(
  name,
  index_url,
  enabled,
  trusted_public_key,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, 1, 1)
"#,
            rusqlite::params![
                "Disabled",
                "https://disabled.example.test/index.json",
                0,
                "disabled-key"
            ],
        )
        .unwrap();
        drop(conn);

        let exact =
            trusted_market_public_key_for_url(&db, "https://plugins.example.test/index.json")
                .unwrap();
        let same_host = trusted_market_public_key_for_url(
            &db,
            "https://plugins.example.test/download/plugin.aio-plugin",
        )
        .unwrap();
        let disabled =
            trusted_market_public_key_for_url(&db, "https://disabled.example.test/index.json")
                .unwrap();

        assert_eq!(exact.as_deref(), Some("trusted-key"));
        assert_eq!(same_host.as_deref(), Some("trusted-key"));
        assert_eq!(disabled, None);
    }

    #[test]
    fn repository_maps_missing_plugin_to_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let err = get_plugin(&db, "official.missing").unwrap_err();
        assert!(err.to_string().starts_with("DB_NOT_FOUND:"));
    }

    #[test]
    fn insert_plugin_validates_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let mut manifest = manifest();
        manifest.permissions.push("unknown.permission".to_string());

        let err = insert_plugin(
            &db,
            InsertPluginInput {
                manifest,
                install_source: PluginInstallSource::Local,
                status: PluginStatus::Disabled,
                installed_dir: None,
            },
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_UNKNOWN_PERMISSION:"));
    }
}
