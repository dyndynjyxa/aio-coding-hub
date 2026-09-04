//! Codex-specific CLI proxy configuration helpers.

use crate::shared::error::AppResult;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::{
    build_manifest_from_captured, build_manifest_with_current_target_paths,
    capture_current_target_state, read_cli_proxy_file, read_cli_proxy_file_with_max_len,
    read_optional_cli_proxy_file, read_optional_cli_proxy_file_with_max_len,
    restore_file_snapshots, snapshot_backup_files, snapshot_target_files, write_captured_backups,
    write_cli_proxy_file_atomic, write_cli_proxy_file_atomic_if_changed_with_max_len,
    write_manifest, CliProxyResult, PLACEHOLDER_KEY,
};

pub(super) const CODEX_PROVIDER_KEY: &str = "aio";
pub(super) const CODEX_MODEL_CATALOG_KIND: &str = "codex_model_catalog_json";

static CODEX_CONFIG_TRANSACTION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static CODEX_CONFIG_GENERATION: AtomicU64 = AtomicU64::new(0);

pub(super) fn transaction_lock() -> AppResult<MutexGuard<'static, ()>> {
    CODEX_CONFIG_TRANSACTION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "CODEX_CONFIG_TRANSACTION_LOCK_POISONED".into())
}

#[derive(Debug)]
pub(super) struct CatalogRefreshIdentity {
    base_origin: String,
    config_path: PathBuf,
    catalog_path: PathBuf,
    generation: u64,
}

/// The caller must hold `transaction_lock` while capturing this identity.
pub(super) fn catalog_refresh_identity_unlocked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> AppResult<CatalogRefreshIdentity> {
    Ok(CatalogRefreshIdentity {
        base_origin: base_origin.to_string(),
        config_path: codex_config_path(app)?,
        catalog_path: codex_model_catalog_path(app)?,
        generation: CODEX_CONFIG_GENERATION.load(Ordering::Relaxed),
    })
}

pub(super) fn bump_config_generation_unlocked() {
    CODEX_CONFIG_GENERATION.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn codex_model_catalog_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<PathBuf> {
    Ok(crate::codex_paths::codex_home_dir(app)?
        .join(crate::infra::codex_model_catalog::projection::AIO_CODEX_MODEL_CATALOG_FILENAME))
}

#[derive(Debug, Default)]
pub(super) struct CatalogApplyPlan {
    pub(super) catalog_bytes: Option<Vec<u8>>,
    pub(super) catalog_pointer: Option<String>,
}

fn manifest_original_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    kind: &str,
    current: Option<Vec<u8>>,
) -> AppResult<Option<Vec<u8>>> {
    let Some(manifest) = super::read_manifest(app, "codex")? else {
        return Ok(current);
    };
    let Some(entry) = manifest.files.iter().find(|entry| entry.kind == kind) else {
        return Ok(current);
    };
    if !entry.existed {
        return Ok(None);
    }
    let Some(rel) = entry.backup_rel.as_ref() else {
        return Err(format!("missing backup_rel for {kind}").into());
    };
    let root = super::cli_proxy_root_dir(app, "codex")?;
    let path = super::cli_proxy_files_dir(&root).join(rel);
    let max_bytes = super::managed_file_max_bytes("codex", kind);
    read_cli_proxy_file_with_max_len(&path, max_bytes).map(Some)
}

fn root_model_catalog_value(config: Option<&[u8]>) -> Option<String> {
    let text = config.and_then(|bytes| std::str::from_utf8(bytes).ok())?;
    let lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    find_root_key_value(&lines, "model_catalog_json")
}

fn parse_model_catalog_pointer_value(value: &str) -> Option<String> {
    format!("model_catalog_json = {value}")
        .parse::<toml::Value>()
        .ok()?
        .get("model_catalog_json")?
        .as_str()
        .map(str::to_string)
}

fn is_aio_owned_catalog_value(value: &str, codex_home: &Path) -> bool {
    parse_model_catalog_pointer_value(value).is_some_and(|pointer| {
        crate::infra::codex_model_catalog::projection::is_aio_owned_catalog_pointer(
            &pointer, codex_home,
        )
    })
}

/// Projection is an enhancement on top of the proxy takeover: enabling/syncing the
/// proxy must not fail just because the Codex CLI is missing or too old to export
/// its bundled catalog. The refresh path stays strict so the UI can report that
/// model mappings did not apply.
pub(super) fn resolve_projection(
    result: AppResult<Option<crate::infra::codex_model_catalog::projection::CatalogProjection>>,
    degrade_failure: bool,
) -> AppResult<Option<crate::infra::codex_model_catalog::projection::CatalogProjection>> {
    match result {
        Err(error) if degrade_failure => {
            tracing::warn!(
                error = %error,
                "codex catalog projection failed; applying proxy config without projection"
            );
            Ok(None)
        }
        other => other,
    }
}

fn prepare_catalog_apply_plan<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    current_config: Option<Vec<u8>>,
    current_catalog: Option<Vec<u8>>,
    db_override: Option<&crate::db::Db>,
    degrade_projection_failure: bool,
) -> AppResult<CatalogApplyPlan> {
    let original_config = manifest_original_bytes(app, "codex_config_toml", current_config)?;
    let original_catalog = manifest_original_bytes(app, CODEX_MODEL_CATALOG_KIND, current_catalog)?;
    let projection = resolve_projection(
        crate::infra::codex_model_catalog::projection::build_for_proxy(
            app,
            original_config.as_deref(),
            original_catalog.as_deref(),
            db_override,
        ),
        degrade_projection_failure,
    )?;

    let original_pointer = root_model_catalog_value(original_config.as_deref());
    let codex_home = crate::codex_paths::codex_home_dir(app)?;
    let catalog_pointer = if projection.is_some() {
        Some({
            format!(
                "\"{}\"",
                crate::infra::codex_model_catalog::projection::AIO_CODEX_MODEL_CATALOG_FILENAME
            )
        })
    } else {
        original_pointer.filter(|pointer| {
            original_catalog.is_some() || !is_aio_owned_catalog_value(pointer, &codex_home)
        })
    };
    let catalog_bytes = projection
        .map(|projection| projection.bytes)
        .or(original_catalog);

    Ok(CatalogApplyPlan {
        catalog_bytes,
        catalog_pointer,
    })
}

fn build_codex_catalog_pointer_config(
    current: Option<Vec<u8>>,
    catalog_pointer: Option<&str>,
) -> Vec<u8> {
    let input = current
        .as_deref()
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .unwrap_or_default();
    let mut lines = input.lines().map(str::to_string).collect::<Vec<_>>();
    revert_root_key(&mut lines, "model_catalog_json", catalog_pointer);
    let mut output = lines.join("\n");
    output.push('\n');
    output.into_bytes()
}

/// (path, current bytes, new bytes) for `auth.json`.
type AuthFileWrite<'a> = (&'a Path, Option<Vec<u8>>, &'a [u8]);

/// Only the proxy enable path writes `auth`; the catalog refresh path passes `None`.
fn apply_catalog_and_config_unlocked(
    config_path: &Path,
    current_config: Option<Vec<u8>>,
    config_bytes: &[u8],
    catalog_path: &Path,
    current_catalog: Option<Vec<u8>>,
    catalog_bytes: Option<&[u8]>,
    auth: Option<AuthFileWrite<'_>>,
) -> AppResult<bool> {
    let mut snapshots = vec![
        super::FileSnapshot {
            path: config_path.to_path_buf(),
            max_bytes: super::CLI_PROXY_FILE_MAX_BYTES,
            existed: current_config.is_some(),
            bytes: current_config,
        },
        super::FileSnapshot {
            path: catalog_path.to_path_buf(),
            max_bytes: crate::infra::codex_model_catalog::CODEX_CATALOG_MAX_BYTES,
            existed: current_catalog.is_some(),
            bytes: current_catalog,
        },
    ];
    if let Some((auth_path, current_auth, _)) = &auth {
        snapshots.push(super::FileSnapshot {
            path: auth_path.to_path_buf(),
            max_bytes: super::CLI_PROXY_FILE_MAX_BYTES,
            existed: current_auth.is_some(),
            bytes: current_auth.clone(),
        });
    }

    let write_result = (|| -> AppResult<bool> {
        let mut changed = false;
        if let Some(bytes) = catalog_bytes {
            changed |= write_cli_proxy_file_atomic_if_changed_with_max_len(
                catalog_path,
                bytes,
                crate::infra::codex_model_catalog::CODEX_CATALOG_MAX_BYTES,
            )?;
        }
        if let Some((auth_path, _, auth_bytes)) = &auth {
            changed |= write_cli_proxy_file_atomic_if_changed_with_max_len(
                auth_path,
                auth_bytes,
                super::CLI_PROXY_FILE_MAX_BYTES,
            )?;
        }
        changed |= write_cli_proxy_file_atomic_if_changed_with_max_len(
            config_path,
            config_bytes,
            super::CLI_PROXY_FILE_MAX_BYTES,
        )?;
        if catalog_bytes.is_none() && catalog_path.exists() {
            std::fs::remove_file(catalog_path)
                .map_err(|error| format!("failed to remove {}: {error}", catalog_path.display()))?;
            changed = true;
        }
        Ok(changed)
    })();

    match write_result {
        Ok(changed) => Ok(changed),
        Err(error) => {
            if let Err(restore_error) = restore_file_snapshots(&snapshots) {
                return Err(format!("{error}; rollback failed: {restore_error}").into());
            }
            Err(error)
        }
    }
}

fn apply_catalog_and_config(
    config_path: &Path,
    current_config: Option<Vec<u8>>,
    config_bytes: &[u8],
    catalog_path: &Path,
    current_catalog: Option<Vec<u8>>,
    catalog_bytes: Option<&[u8]>,
    auth: Option<AuthFileWrite<'_>>,
) -> AppResult<bool> {
    let _transaction = transaction_lock()?;
    bump_config_generation_unlocked();
    apply_catalog_and_config_unlocked(
        config_path,
        current_config,
        config_bytes,
        catalog_path,
        current_catalog,
        catalog_bytes,
        auth,
    )
}

pub(super) fn commit_catalog_refresh_if_active<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    identity: &CatalogRefreshIdentity,
    catalog_plan: CatalogApplyPlan,
) -> AppResult<Option<bool>> {
    let _transaction = transaction_lock()?;
    if CODEX_CONFIG_GENERATION.load(Ordering::Relaxed) != identity.generation
        || codex_config_path(app)? != identity.config_path
        || codex_model_catalog_path(app)? != identity.catalog_path
    {
        return Ok(None);
    }

    let still_active = super::read_manifest(app, "codex")?.is_some_and(|manifest| {
        manifest.enabled && manifest.base_origin.as_deref() == Some(identity.base_origin.as_str())
    });
    if !still_active || !is_proxy_config_applied(app, &identity.base_origin) {
        return Ok(None);
    }

    let current_config = read_optional_cli_proxy_file_with_max_len(
        &identity.config_path,
        super::CLI_PROXY_FILE_MAX_BYTES,
    )?;
    let current_catalog = read_optional_cli_proxy_file_with_max_len(
        &identity.catalog_path,
        crate::infra::codex_model_catalog::CODEX_CATALOG_MAX_BYTES,
    )?;
    let config_bytes = build_codex_catalog_pointer_config(
        current_config.clone(),
        catalog_plan.catalog_pointer.as_deref(),
    );

    apply_catalog_and_config_unlocked(
        &identity.config_path,
        current_config,
        &config_bytes,
        &identity.catalog_path,
        current_catalog,
        catalog_plan.catalog_bytes.as_deref(),
        None,
    )
    .map(Some)
}

pub(super) fn refresh_model_catalog<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
    identity: CatalogRefreshIdentity,
) -> AppResult<Option<bool>> {
    let current_config = read_optional_cli_proxy_file_with_max_len(
        &identity.config_path,
        super::CLI_PROXY_FILE_MAX_BYTES,
    )?;
    let current_catalog = read_optional_cli_proxy_file_with_max_len(
        &identity.catalog_path,
        crate::infra::codex_model_catalog::CODEX_CATALOG_MAX_BYTES,
    )?;
    let catalog_plan = prepare_catalog_apply_plan(
        app,
        current_config.clone(),
        current_catalog.clone(),
        Some(db),
        false,
    )?;
    commit_catalog_refresh_if_active(app, &identity, catalog_plan)
}

pub(super) fn apply_proxy_config<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> AppResult<()> {
    let config_path = codex_config_path(app)?;
    let catalog_path = codex_model_catalog_path(app)?;
    let current_config =
        read_optional_cli_proxy_file_with_max_len(&config_path, super::CLI_PROXY_FILE_MAX_BYTES)?;
    let current_catalog = read_optional_cli_proxy_file_with_max_len(
        &catalog_path,
        crate::infra::codex_model_catalog::CODEX_CATALOG_MAX_BYTES,
    )?;
    let catalog_plan = prepare_catalog_apply_plan(
        app,
        current_config.clone(),
        current_catalog.clone(),
        None,
        true,
    )?;

    let config_bytes = if super::codex_oauth_compatible_proxy_mode(app) {
        build_codex_config_toml_for_proxy(
            current_config.clone(),
            &format!("{base_origin}/v1"),
            CodexConfigPlatform::current(),
            true,
            catalog_plan.catalog_pointer.as_deref(),
        )?
    } else {
        build_codex_config_toml_for_proxy(
            current_config.clone(),
            &format!("{base_origin}/v1"),
            CodexConfigPlatform::current(),
            false,
            catalog_plan.catalog_pointer.as_deref(),
        )?
    };
    let auth_path = codex_auth_path(app)?;
    let current_auth = if super::codex_oauth_compatible_proxy_mode(app) {
        None
    } else {
        Some(read_optional_cli_proxy_file(&auth_path)?)
    };
    let auth_bytes = match current_auth.as_ref() {
        Some(current) => match build_codex_auth_json(current.clone()) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                if let Some(original_bytes) = current.as_ref() {
                    let backup_path = auth_path.with_extension("json.invalid-backup");
                    let _ = write_cli_proxy_file_atomic(&backup_path, original_bytes);
                }
                return Err(error);
            }
        },
        None => None,
    };

    let auth = match (current_auth, auth_bytes.as_deref()) {
        (Some(current), Some(bytes)) => Some((auth_path.as_path(), current, bytes)),
        _ => None,
    };

    apply_catalog_and_config(
        &config_path,
        current_config,
        &config_bytes,
        &catalog_path,
        current_catalog,
        catalog_plan.catalog_bytes.as_deref(),
        auth,
    )?;
    Ok(())
}

pub(super) fn codex_config_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<PathBuf> {
    crate::codex_paths::codex_config_toml_path(app)
}

pub(super) fn codex_auth_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    crate::codex_paths::codex_auth_json_path(app)
}

pub(super) fn is_codex_proxy_target_state<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    let config_path = match codex_config_path(app) {
        Ok(path) => path,
        Err(_) => return false,
    };

    let config = match read_cli_proxy_file(&config_path) {
        Ok(content) => String::from_utf8_lossy(&content).to_string(),
        Err(_) => return false,
    };

    // Check for either normal mode ("aio") or remote_compaction mode ("OpenAI")
    let has_proxy_provider = check_provider_config_basic(&config, CODEX_PROVIDER_KEY)
        || check_provider_config_basic(&config, "OpenAI");
    if super::codex_oauth_compatible_proxy_mode(app) {
        return has_proxy_provider;
    }

    let auth_path = match codex_auth_path(app) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let auth_bytes = match read_cli_proxy_file(&auth_path) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let auth = match serde_json::from_slice::<serde_json::Value>(&auth_bytes) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let has_proxy_auth = auth.get("OPENAI_API_KEY").and_then(|value| value.as_str())
        == Some(PLACEHOLDER_KEY)
        && auth.get("auth_mode").and_then(|value| value.as_str()) == Some("apikey");

    has_proxy_provider && has_proxy_auth
}

/// Basic check for model_provider and model_providers table (without base_url check).
fn check_provider_config_basic(config: &str, provider_key: &str) -> bool {
    let expected_provider = format!("model_provider = \"{provider_key}\"");
    let expected_table_unquoted = format!("[model_providers.{provider_key}]");
    let expected_table_double = format!("[model_providers.\"{provider_key}\"]");
    let expected_table_single = format!("[model_providers.'{provider_key}']");

    config.contains(&expected_provider)
        && (config.contains(&expected_table_unquoted)
            || config.contains(&expected_table_double)
            || config.contains(&expected_table_single))
}

pub(super) fn rebind_codex_manifest_after_home_change<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    mut manifest: super::CliProxyManifest,
    base_origin: &str,
    apply_live: bool,
    trace_id: String,
) -> AppResult<CliProxyResult> {
    let captured = capture_current_target_state(app, "codex")?;
    let previous_manifest = manifest.clone();
    let target_already_proxy_managed = is_proxy_config_applied(app, base_origin)
        || previous_manifest
            .base_origin
            .as_deref()
            .is_some_and(|origin| is_proxy_config_applied(app, origin))
        || is_codex_proxy_target_state(app);

    let origin = Some(base_origin.to_string());
    let rebind_msg = |live: bool| {
        if live {
            "已重绑 Codex 目录并写入当前网关配置".to_string()
        } else {
            "已重绑 Codex 目录基线，待网关启动后接管".to_string()
        }
    };

    if target_already_proxy_managed {
        let target_snapshots = snapshot_target_files(&captured)?;
        manifest = build_manifest_with_current_target_paths(app, &manifest, base_origin)?;

        if let Err(err) = write_manifest(app, "codex", &manifest) {
            return Ok(CliProxyResult::failure(
                trace_id,
                "codex",
                true,
                "CLI_PROXY_REBIND_MANIFEST_WRITE_FAILED",
                err.to_string(),
                origin,
            ));
        }

        if let Err(err) = super::restore_backups_exactly_from_manifest(app, &manifest) {
            let _ = write_manifest(app, "codex", &previous_manifest);
            let _ = restore_file_snapshots(&target_snapshots);
            return Ok(CliProxyResult::failure(
                trace_id,
                "codex",
                true,
                "CLI_PROXY_REBIND_RESTORE_FAILED",
                err.to_string(),
                origin,
            ));
        }

        if apply_live {
            if let Err(err) = super::apply_proxy_config(app, "codex", base_origin) {
                let _ = write_manifest(app, "codex", &previous_manifest);
                let _ = restore_file_snapshots(&target_snapshots);
                return Ok(CliProxyResult::failure(
                    trace_id,
                    "codex",
                    true,
                    "CLI_PROXY_REBIND_APPLY_FAILED",
                    err.to_string(),
                    origin,
                ));
            }
        }

        return Ok(CliProxyResult::success(
            trace_id,
            "codex",
            true,
            rebind_msg(apply_live),
            origin,
        ));
    }

    let backup_snapshots = snapshot_backup_files(app, "codex", &captured)?;
    let target_snapshots = snapshot_target_files(&captured)?;

    write_captured_backups(app, "codex", &captured)?;
    manifest = build_manifest_from_captured(&manifest, base_origin, captured);

    if let Err(err) = write_manifest(app, "codex", &manifest) {
        let _ = restore_file_snapshots(&backup_snapshots);
        return Ok(CliProxyResult::failure(
            trace_id,
            "codex",
            true,
            "CLI_PROXY_REBIND_MANIFEST_WRITE_FAILED",
            err.to_string(),
            origin,
        ));
    }

    if apply_live {
        if let Err(err) = super::apply_proxy_config(app, "codex", base_origin) {
            let _ = write_manifest(app, "codex", &previous_manifest);
            let _ = restore_file_snapshots(&backup_snapshots);
            let _ = restore_file_snapshots(&target_snapshots);
            return Ok(CliProxyResult::failure(
                trace_id,
                "codex",
                true,
                "CLI_PROXY_REBIND_APPLY_FAILED",
                err.to_string(),
                origin,
            ));
        }
    }

    Ok(CliProxyResult::success(
        trace_id,
        "codex",
        true,
        rebind_msg(apply_live),
        origin,
    ))
}

/// Merge-restore Codex `auth.json`: only revert the proxy-managed keys
/// (`OPENAI_API_KEY`, `auth_mode`) and restore `tokens` / `last_refresh` from
/// the backup if they existed, while preserving any other user changes.
pub(super) fn merge_restore_codex_auth_json(
    target_path: &Path,
    backup_path: &Path,
) -> AppResult<()> {
    const PROXY_INSERTED_KEYS: &[&str] = &["OPENAI_API_KEY", "auth_mode"];
    const PROXY_REMOVED_KEYS: &[&str] = &["tokens", "last_refresh"];

    let current_bytes = read_optional_cli_proxy_file(target_path)?;
    let backup_bytes = read_cli_proxy_file(backup_path)?;

    let mut current: serde_json::Value = match current_bytes {
        Some(b) if !b.is_empty() => {
            serde_json::from_slice(&b).unwrap_or_else(|_| serde_json::json!({}))
        }
        _ => serde_json::json!({}),
    };

    let backup: serde_json::Value =
        serde_json::from_slice(&backup_bytes).unwrap_or_else(|_| serde_json::json!({}));

    if let Some(obj) = current.as_object_mut() {
        let backup_obj = backup.as_object();

        // Revert inserted keys
        for key in PROXY_INSERTED_KEYS {
            if let Some(original) = backup_obj.and_then(|b| b.get(*key)) {
                obj.insert(key.to_string(), original.clone());
            } else {
                obj.remove(*key);
            }
        }

        // Restore keys that the proxy removed
        for key in PROXY_REMOVED_KEYS {
            if let Some(original) = backup_obj.and_then(|b| b.get(*key)) {
                obj.insert(key.to_string(), original.clone());
            }
        }
    }

    let mut bytes = serde_json::to_vec_pretty(&current)
        .map_err(|e| format!("failed to serialize auth.json: {e}"))?;
    bytes.push(b'\n');
    write_cli_proxy_file_atomic(target_path, &bytes)?;
    Ok(())
}

/// Merge-restore Codex `config.toml`: revert the proxy-managed root keys
/// (`model_provider`, `preferred_auth_method`, `model_catalog_json`) and the
/// `[model_providers.aio]` section / `[windows] sandbox` while preserving user
/// changes.
pub(super) fn merge_restore_codex_config_toml(
    target_path: &Path,
    backup_path: &Path,
    original_aio_catalog_existed: bool,
) -> AppResult<()> {
    let current_bytes = read_optional_cli_proxy_file(target_path)?;
    let backup_bytes = read_cli_proxy_file(backup_path)?;

    let current_str = current_bytes
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();
    let backup_str = String::from_utf8_lossy(&backup_bytes).to_string();

    let mut lines: Vec<String> = if current_str.is_empty() {
        Vec::new()
    } else {
        current_str.lines().map(|l| l.to_string()).collect()
    };

    let backup_lines: Vec<String> = if backup_str.is_empty() {
        Vec::new()
    } else {
        backup_str.lines().map(|l| l.to_string()).collect()
    };

    // --- Revert root `model_provider` ---
    let backup_model_provider = find_root_key_value(&backup_lines, "model_provider");
    revert_root_key(
        &mut lines,
        "model_provider",
        backup_model_provider.as_deref(),
    );

    // --- Revert root `preferred_auth_method` ---
    let backup_auth_method = find_root_key_value(&backup_lines, "preferred_auth_method");
    revert_root_key(
        &mut lines,
        "preferred_auth_method",
        backup_auth_method.as_deref(),
    );

    // --- Revert AIO `model_catalog_json` pointer ---
    let backup_model_catalog =
        find_root_key_value(&backup_lines, "model_catalog_json").filter(|value| {
            original_aio_catalog_existed
                || !target_path
                    .parent()
                    .is_some_and(|home| is_aio_owned_catalog_value(value, home))
        });
    revert_root_key(
        &mut lines,
        "model_catalog_json",
        backup_model_catalog.as_deref(),
    );

    // --- Remove the proxy-injected `[model_providers.aio]` section ---
    // If the backup had this section, we leave it; otherwise remove it.
    let backup_had_aio =
        !find_model_provider_base_table_indices(&backup_lines, CODEX_PROVIDER_KEY).is_empty();
    if !backup_had_aio {
        remove_model_provider_section(&mut lines, CODEX_PROVIDER_KEY);
    }

    // --- Revert `[windows] sandbox` ---
    // If the backup did not have `[windows]` sandbox, remove the one the proxy added.
    let backup_had_windows_sandbox = has_windows_sandbox(&backup_lines);
    if !backup_had_windows_sandbox {
        remove_windows_sandbox(&mut lines);
    }

    let mut out = lines.join("\n");
    out.push('\n');
    write_cli_proxy_file_atomic(target_path, out.as_bytes())?;
    Ok(())
}

// -- TOML helpers for merge-restore -----------------------------------------

/// Find the value of a root-level `key = "value"` line (before any `[table]` header).
pub(super) fn find_root_key_value(lines: &[String], key: &str) -> Option<String> {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());
    for line in &lines[..first_table] {
        let trimmed = line.trim_start();
        if let Some((candidate, value)) = trimmed.split_once('=') {
            if candidate.trim() == key {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Revert a root-level key to its backup value, or remove it if backup didn't have it.
pub(super) fn revert_root_key(lines: &mut Vec<String>, key: &str, backup_value: Option<&str>) {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());

    let pos = lines[..first_table].iter().position(|line| {
        line.trim_start()
            .split_once('=')
            .is_some_and(|(candidate, _)| candidate.trim() == key)
    });

    match (pos, backup_value) {
        (Some(idx), Some(val)) => {
            lines[idx] = format!("{key} = {val}");
        }
        (Some(idx), None) => {
            lines.remove(idx);
        }
        (None, Some(val)) => {
            // Backup had it but current doesn't -- shouldn't happen, but restore it
            lines.insert(0, format!("{key} = {val}"));
        }
        (None, None) => {} // Neither has it, nothing to do
    }
}

/// Remove `[model_providers.<provider_key>]` section and its nested tables.
pub(super) fn remove_model_provider_section(lines: &mut Vec<String>, provider_key: &str) {
    // Remove base tables
    loop {
        let indices = find_model_provider_base_table_indices(lines, provider_key);
        if indices.is_empty() {
            break;
        }
        let start = indices[0];
        let end = find_next_table_header(lines, start.saturating_add(1));
        lines.drain(start..end);
    }

    // Remove nested tables
    while let Some(start) = find_model_provider_nested_table_index(lines, provider_key) {
        let end = find_next_table_header(lines, start.saturating_add(1));
        lines.drain(start..end);
    }
}

/// Check if backup lines contain a `[windows]` section with `sandbox` key.
pub(super) fn has_windows_sandbox(lines: &[String]) -> bool {
    let Some(start) = lines.iter().position(|l| l.trim() == "[windows]") else {
        return false;
    };
    let end = find_next_table_header(lines, start.saturating_add(1));
    lines[start + 1..end]
        .iter()
        .any(|l| l.trim_start().starts_with("sandbox"))
}

/// Remove the `sandbox` key from the `[windows]` section; remove the section if empty.
pub(super) fn remove_windows_sandbox(lines: &mut Vec<String>) {
    let Some(start) = lines.iter().position(|l| l.trim() == "[windows]") else {
        return;
    };
    let end = find_next_table_header(lines, start.saturating_add(1));

    // Remove sandbox line
    let mut i = start + 1;
    while i < end && i < lines.len() {
        if lines[i].trim_start().starts_with("sandbox") {
            lines.remove(i);
            break;
        }
        i += 1;
    }

    // If only the header remains (with optional blank lines), remove the whole section
    let new_end = find_next_table_header(lines, start.saturating_add(1));
    let body_empty = lines[start + 1..new_end]
        .iter()
        .all(|l| l.trim().is_empty());
    if body_empty {
        lines.drain(start..new_end);
    }
}

pub(super) fn find_next_table_header(lines: &[String], from: usize) -> usize {
    lines[from..]
        .iter()
        .position(|line| line.trim().starts_with('['))
        .map(|offset| from + offset)
        .unwrap_or(lines.len())
}

fn insert_model_provider_section(
    lines: &mut Vec<String>,
    insert_at: usize,
    provider_key: &str,
    base_url: &str,
) {
    let header = format!("[model_providers.{provider_key}]");
    let section = [
        header,
        format!("name = \"{provider_key}\""),
        format!("base_url = \"{base_url}\""),
        "wire_api = \"responses\"".to_string(),
        "requires_openai_auth = true".to_string(),
    ];

    lines.splice(insert_at..insert_at, section);
}

pub(super) fn is_model_provider_base_header_line(trimmed: &str, provider_key: &str) -> bool {
    trimmed == format!("[model_providers.{provider_key}]")
        || trimmed == format!("[model_providers.\"{provider_key}\"]")
        || trimmed == format!("[model_providers.'{provider_key}']")
}

pub(super) fn find_model_provider_base_table_indices(
    lines: &[String],
    provider_key: &str,
) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            is_model_provider_base_header_line(line.trim(), provider_key).then_some(idx)
        })
        .collect()
}

pub(super) fn find_model_provider_nested_table_index(
    lines: &[String],
    provider_key: &str,
) -> Option<usize> {
    let prefix_unquoted = format!("[model_providers.{provider_key}.");
    let prefix_double = format!("[model_providers.\"{provider_key}\".");
    let prefix_single = format!("[model_providers.'{provider_key}'.");

    lines.iter().position(|line| {
        let trimmed = line.trim();
        trimmed.starts_with(&prefix_unquoted)
            || trimmed.starts_with(&prefix_double)
            || trimmed.starts_with(&prefix_single)
    })
}

fn patch_model_provider_base_table(
    lines: &mut Vec<String>,
    start: usize,
    provider_key: &str,
    base_url: &str,
) {
    let end = find_next_table_header(lines, start.saturating_add(1));

    let mut body: Vec<String> = Vec::new();
    for line in lines[start.saturating_add(1)..end].iter() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            body.push(line.clone());
            continue;
        }

        let Some((k, _)) = trimmed.split_once('=') else {
            body.push(line.clone());
            continue;
        };

        match k.trim() {
            "name" | "base_url" | "wire_api" | "requires_openai_auth" => {}
            _ => body.push(line.clone()),
        }
    }

    let managed = [
        format!("name = \"{provider_key}\""),
        format!("base_url = \"{base_url}\""),
        "wire_api = \"responses\"".to_string(),
        "requires_openai_auth = true".to_string(),
    ];

    let mut patched: Vec<String> = Vec::with_capacity(managed.len() + body.len());
    patched.extend(managed);
    if !body.is_empty()
        && !body.first().is_some_and(|l| l.trim().is_empty())
        && !patched.last().is_some_and(|l| l.trim().is_empty())
    {
        patched.push(String::new());
    }
    patched.extend(body);

    lines.splice(start.saturating_add(1)..end, patched);
}

pub(super) fn upsert_model_provider_base_table(
    lines: &mut Vec<String>,
    provider_key: &str,
    base_url: &str,
) {
    let mut bases = find_model_provider_base_table_indices(lines, provider_key);
    bases.sort();

    // Ensure there is exactly one base table, and keep nested tables intact.
    if let Some(&keep_start) = bases.first() {
        let nested_start = find_model_provider_nested_table_index(lines, provider_key);

        // Remove duplicates first (from bottom) to keep indices stable.
        for start in bases.into_iter().rev() {
            if start == keep_start {
                continue;
            }
            let end = find_next_table_header(lines, start.saturating_add(1));
            lines.drain(start..end);
        }

        patch_model_provider_base_table(lines, keep_start, provider_key, base_url);

        // TOML requires parent tables appear before nested child tables. If the base table
        // is currently after a nested table, move it before the first nested occurrence.
        if let Some(nested_start) = nested_start {
            if keep_start > nested_start {
                let end = find_next_table_header(lines, keep_start.saturating_add(1));
                let block: Vec<String> = lines.drain(keep_start..end).collect();
                lines.splice(nested_start..nested_start, block);
            }
        }
        return;
    }

    // No base table found: insert before the first nested table if it exists, otherwise append.
    let mut insert_at =
        find_model_provider_nested_table_index(lines, provider_key).unwrap_or(lines.len());
    if insert_at > 0 && !lines[insert_at.saturating_sub(1)].trim().is_empty() {
        lines.insert(insert_at, String::new());
        insert_at += 1;
    }

    insert_model_provider_section(lines, insert_at, provider_key, base_url);
}

/// Upsert a root-level `key = "value"` line before any `[table]` header.
/// If `trailing_blank` is true and the inserted line is followed by a non-blank
/// line, an empty separator line is added after it.
fn upsert_root_toml_key(lines: &mut Vec<String>, key: &str, value: &str, trailing_blank: bool) {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());

    if let Some(line) = lines.iter_mut().take(first_table).find(|line| {
        line.trim_start()
            .split_once('=')
            .is_some_and(|(candidate, _)| candidate.trim() == key)
    }) {
        *line = format!("{key} = \"{value}\"");
        return;
    }

    let mut insert_at = 0;
    while insert_at < first_table {
        let trimmed = lines[insert_at].trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            insert_at += 1;
            continue;
        }
        break;
    }

    lines.insert(insert_at, format!("{key} = \"{value}\""));
    if trailing_blank && insert_at + 1 < lines.len() && !lines[insert_at + 1].trim().is_empty() {
        lines.insert(insert_at + 1, String::new());
    }
}

pub(super) fn upsert_root_model_provider(lines: &mut Vec<String>, value: &str) {
    upsert_root_toml_key(lines, "model_provider", value, true);
}

pub(super) fn upsert_root_preferred_auth_method(lines: &mut Vec<String>, value: &str) {
    upsert_root_toml_key(lines, "preferred_auth_method", value, false);
}

pub(super) fn remove_root_preferred_auth_method_if_api_key(lines: &mut Vec<String>) {
    let first_table = lines
        .iter()
        .position(|l| l.trim().starts_with('['))
        .unwrap_or(lines.len());

    let Some(pos) = lines[..first_table]
        .iter()
        .position(|l| l.trim_start().starts_with("preferred_auth_method"))
    else {
        return;
    };

    let Some((_, value)) = lines[pos].trim_start().split_once('=') else {
        return;
    };

    let normalized = value.trim().trim_matches('"').trim_matches('\'');
    if normalized == "apikey" {
        lines.remove(pos);
    }
}

fn has_root_preferred_auth_method_api_key(config: &str) -> bool {
    let lines: Vec<String> = config.lines().map(|line| line.to_string()).collect();
    find_root_key_value(&lines, "preferred_auth_method")
        .as_deref()
        .map(|value| value.trim().trim_matches('"').trim_matches('\'') == "apikey")
        .unwrap_or(false)
}

pub(super) fn upsert_windows_sandbox(lines: &mut Vec<String>) {
    let header = "[windows]";
    if let Some(start) = lines.iter().position(|l| l.trim() == header) {
        let end = find_next_table_header(lines, start.saturating_add(1));
        let has_sandbox = lines[start + 1..end]
            .iter()
            .any(|l| l.trim_start().starts_with("sandbox"));
        if !has_sandbox {
            lines.insert(start + 1, "sandbox = \"elevated\"".to_string());
        }
    } else {
        if !lines.is_empty() && !lines.last().unwrap_or(&String::new()).trim().is_empty() {
            lines.push(String::new());
        }
        lines.push(header.to_string());
        lines.push("sandbox = \"elevated\"".to_string());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexConfigPlatform {
    Windows,
    Other,
}

impl CodexConfigPlatform {
    pub(super) fn current() -> Self {
        if std::env::consts::OS == "windows" {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

#[cfg(test)]
pub(super) fn build_codex_config_toml(
    current: Option<Vec<u8>>,
    base_url: &str,
    platform: CodexConfigPlatform,
) -> AppResult<Vec<u8>> {
    build_codex_config_toml_with_auth_strategy(current, base_url, platform, false, None)
}

#[cfg(test)]
pub(super) fn build_codex_config_toml_oauth_compatible(
    current: Option<Vec<u8>>,
    base_url: &str,
    platform: CodexConfigPlatform,
) -> AppResult<Vec<u8>> {
    build_codex_config_toml_with_auth_strategy(current, base_url, platform, true, None)
}

pub(super) fn build_codex_config_toml_for_proxy(
    current: Option<Vec<u8>>,
    base_url: &str,
    platform: CodexConfigPlatform,
    oauth_compatible: bool,
    model_catalog_value: Option<&str>,
) -> AppResult<Vec<u8>> {
    build_codex_config_toml_with_auth_strategy(
        current,
        base_url,
        platform,
        oauth_compatible,
        Some(model_catalog_value),
    )
}

fn build_codex_config_toml_with_auth_strategy(
    current: Option<Vec<u8>>,
    base_url: &str,
    platform: CodexConfigPlatform,
    oauth_compatible: bool,
    model_catalog_value: Option<Option<&str>>,
) -> AppResult<Vec<u8>> {
    let input = current
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();

    let mut lines: Vec<String> = if input.is_empty() {
        Vec::new()
    } else {
        input.lines().map(|l| l.to_string()).collect()
    };

    upsert_root_model_provider(&mut lines, CODEX_PROVIDER_KEY);
    if oauth_compatible {
        remove_root_preferred_auth_method_if_api_key(&mut lines);
    } else {
        upsert_root_preferred_auth_method(&mut lines, "apikey");
    }
    upsert_model_provider_base_table(&mut lines, CODEX_PROVIDER_KEY, base_url);
    if let Some(model_catalog_value) = model_catalog_value {
        revert_root_key(&mut lines, "model_catalog_json", model_catalog_value);
    }
    if platform == CodexConfigPlatform::Windows {
        upsert_windows_sandbox(&mut lines);
    }

    let mut out = lines.join("\n");
    out.push('\n');
    Ok(out.into_bytes())
}

pub(super) fn build_codex_auth_json(current: Option<Vec<u8>>) -> AppResult<Vec<u8>> {
    let mut value = match current {
        Some(bytes) if bytes.is_empty() => serde_json::json!({}),
        Some(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .map_err(|e| format!("CLI_PROXY_INVALID_AUTH_JSON: failed to parse auth.json: {e}"))?,
        None => serde_json::json!({}),
    };

    let obj = value.as_object_mut().ok_or_else(|| {
        crate::shared::error::AppError::from(
            "CLI_PROXY_INVALID_AUTH_JSON: auth.json root must be a JSON object",
        )
    })?;
    obj.insert(
        "OPENAI_API_KEY".to_string(),
        serde_json::Value::String(PLACEHOLDER_KEY.to_string()),
    );
    obj.insert(
        "auth_mode".to_string(),
        serde_json::Value::String("apikey".to_string()),
    );
    // Remove OAuth residuals that would confuse Codex CLI into chatgpt auth mode.
    obj.remove("tokens");
    obj.remove("last_refresh");

    let mut out = serde_json::to_vec_pretty(&value)
        .map_err(|e| format!("failed to serialize auth.json: {e}"))?;
    out.push(b'\n');
    Ok(out)
}

/// Provider key used when remote_compaction is enabled (Codex requires "OpenAI" for Remote Compact).
const CODEX_REMOTE_COMPACTION_PROVIDER_KEY: &str = "OpenAI";

/// Check whether Codex proxy config is currently applied.
/// Supports both normal mode (provider key = "aio") and remote_compaction mode (provider key = "OpenAI").
pub(super) fn is_proxy_config_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> bool {
    let config_path = match codex_config_path(app) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let config = match read_cli_proxy_file(&config_path) {
        Ok(v) => String::from_utf8_lossy(&v).to_string(),
        Err(_) => return false,
    };

    let expected_base = format!("base_url = \"{base_origin}/v1\"");

    // Check base_url first - this must always be present
    if !config.contains(&expected_base) {
        return false;
    }

    // Check for either normal mode ("aio") or remote_compaction mode ("OpenAI")
    let has_normal_provider = check_provider_config(&config, CODEX_PROVIDER_KEY);
    let has_remote_compaction_provider =
        check_provider_config(&config, CODEX_REMOTE_COMPACTION_PROVIDER_KEY);

    if !has_normal_provider && !has_remote_compaction_provider {
        return false;
    }

    if super::codex_oauth_compatible_proxy_mode(app) {
        return !has_root_preferred_auth_method_api_key(&config);
    }

    let auth_path = match codex_auth_path(app) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let auth_bytes = match read_cli_proxy_file(&auth_path) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let auth = match serde_json::from_slice::<serde_json::Value>(&auth_bytes) {
        Ok(v) => v,
        Err(_) => return false,
    };
    auth.get("OPENAI_API_KEY")
        .and_then(|v| v.as_str())
        .is_some()
}

/// Check if the config contains the expected model_provider and model_providers table for a given key.
fn check_provider_config(config: &str, provider_key: &str) -> bool {
    let expected_provider = format!("model_provider = \"{provider_key}\"");
    let expected_table_unquoted = format!("[model_providers.{provider_key}]");
    let expected_table_double = format!("[model_providers.\"{provider_key}\"]");
    let expected_table_single = format!("[model_providers.'{provider_key}']");

    if !config.contains(&expected_provider) {
        return false;
    }

    config.contains(&expected_table_unquoted)
        || config.contains(&expected_table_double)
        || config.contains(&expected_table_single)
}

/// Public entry point called from `sync_enabled` and `rebind_codex_home_after_change`.
pub(super) fn rebind_codex_home_after_change<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
    apply_live: bool,
) -> AppResult<CliProxyResult> {
    if !base_origin.starts_with("http://") && !base_origin.starts_with("https://") {
        return Err("SEC_INVALID_INPUT: base_origin must start with http:// or https://".into());
    }

    let trace_id = super::new_trace_id("cli-proxy-codex-home-rebind");
    let origin = Some(base_origin.to_string());
    let Some(manifest) = super::read_manifest(app, "codex")? else {
        return Ok(CliProxyResult::success(
            trace_id,
            "codex",
            false,
            "Codex 代理未启用，无需重绑".to_string(),
            origin,
        ));
    };

    if !manifest.enabled {
        return Ok(CliProxyResult::success(
            trace_id,
            "codex",
            false,
            "Codex 代理未启用，无需重绑".to_string(),
            origin,
        ));
    }

    if !super::manifest_target_paths_changed(app, &manifest)? {
        let msg = if apply_live {
            "Codex 目录未变化，无需重绑"
        } else {
            "Codex 目录未变化，待网关启动后按现有配置接管"
        };
        return Ok(CliProxyResult::success(
            trace_id,
            "codex",
            true,
            msg.to_string(),
            origin,
        ));
    }

    rebind_codex_manifest_after_home_change(app, manifest, base_origin, apply_live, trace_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_projection_degrades_failure_only_for_apply_paths() {
        let error = || -> AppResult<Option<crate::infra::codex_model_catalog::projection::CatalogProjection>> {
            Err("CLI_PROXY_CODEX_CATALOG_FAILED: Codex CLI not found".to_string().into())
        };

        // Enable/sync path: a missing/old Codex CLI degrades to "no projection".
        assert!(resolve_projection(error(), true)
            .expect("degraded")
            .is_none());

        // Refresh path stays strict so the UI can report the failure.
        let err = resolve_projection(error(), false).expect_err("strict");
        assert!(err.to_string().contains("Codex CLI not found"));

        // A successful projection passes through untouched.
        let projection = crate::infra::codex_model_catalog::projection::CatalogProjection {
            bytes: b"{}".to_vec(),
            affected_sources: vec!["gpt-test".to_string()],
        };
        let passed = resolve_projection(Ok(Some(projection.clone())), true).expect("ok");
        assert_eq!(passed, Some(projection));
    }

    #[test]
    fn catalog_pointer_patch_replaces_and_removes_only_the_root_key() {
        let input =
            b"model_catalog_json_backup = \"keep.json\"\nmodel_catalog_json = \"user.json\"\nmodel = \"gpt-test\"\n\n[other]\nvalue = 1\n".to_vec();

        let replaced = build_codex_catalog_pointer_config(
            Some(input.clone()),
            Some("\"aio-codex-model-catalog.json\""),
        );
        let replaced = String::from_utf8(replaced).expect("utf8");
        assert!(replaced.contains("model_catalog_json = \"aio-codex-model-catalog.json\""));
        assert!(replaced.contains("model_catalog_json_backup = \"keep.json\""));
        assert!(replaced.contains("model = \"gpt-test\""));
        assert!(replaced.contains("[other]\nvalue = 1"));

        let removed = build_codex_catalog_pointer_config(Some(input), None);
        let removed = String::from_utf8(removed).expect("utf8");
        assert!(!removed.contains("model_catalog_json ="));
        assert!(removed.contains("model_catalog_json_backup = \"keep.json\""));
        assert!(removed.contains("model = \"gpt-test\""));
        assert!(removed.contains("[other]\nvalue = 1"));
    }

    #[test]
    fn catalog_apply_rolls_back_catalog_when_config_write_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        let catalog_path = temp.path().join("catalog.json");
        let old_catalog = b"{\"models\":[{\"slug\":\"old\"}]}\n".to_vec();
        std::fs::write(&catalog_path, &old_catalog).expect("write old catalog");

        let invalid_parent = temp.path().join("not-a-directory");
        std::fs::write(&invalid_parent, b"blocker").expect("write blocker");
        let config_path = invalid_parent.join("config.toml");

        let error = apply_catalog_and_config(
            &config_path,
            None,
            b"model_catalog_json = \"catalog.json\"\n",
            &catalog_path,
            Some(old_catalog.clone()),
            Some(b"{\"models\":[{\"slug\":\"new\"}]}\n"),
            None,
        )
        .expect_err("config write should fail");

        assert!(error.to_string().contains("failed"));
        assert_eq!(
            std::fs::read(&catalog_path).expect("read restored catalog"),
            old_catalog
        );
        assert!(!config_path.exists());
    }
}
