//! Usage: Grok CLI proxy lifecycle adapter backed by the shared Grok TOML store.

use crate::shared::error::AppResult;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

static GROK_PROXY_TRANSACTION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(super) fn transaction_lock() -> AppResult<MutexGuard<'static, ()>> {
    GROK_PROXY_TRANSACTION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "GROK_PROXY_TRANSACTION_LOCK_POISONED".into())
}

fn is_invalid_config_error(error: &crate::shared::error::AppError) -> bool {
    error.to_string().contains("GROK_CONFIG_INVALID_")
}

fn preserve_invalid_config<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let result = (|| -> AppResult<PathBuf> {
        let path = grok_config_path(app)?;
        let Some(bytes) = super::read_optional_cli_proxy_file(&path)? else {
            return Ok(path);
        };
        let safety_path = path.with_extension("toml.invalid-backup");
        super::write_cli_proxy_file_atomic(&safety_path, &bytes)?;
        Ok(safety_path)
    })();

    match result {
        Ok(path) => tracing::warn!(
            path = %path.display(),
            "cli_proxy: preserved invalid Grok config safety copy"
        ),
        Err(error) => tracing::warn!(
            error = %error,
            "cli_proxy: failed to preserve invalid Grok config safety copy"
        ),
    }
}

pub(super) fn grok_config_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    crate::grok_config::config_path(app)
}

pub(super) fn apply_proxy_config<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> AppResult<()> {
    apply_proxy_config_locked(app, base_origin)
}

fn apply_proxy_config_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> AppResult<()> {
    let previous_settings = crate::settings::read(app)?;
    let mut next_settings = previous_settings.clone();
    let preferences = match previous_settings.grok_proxy_preferences.clone() {
        Some(preferences) => crate::grok_config::validate_preferences(preferences)?,
        None => {
            let candidate = match crate::grok_config::inspect(app) {
                Ok(state) => state.preferences,
                Err(error) => {
                    if is_invalid_config_error(&error) {
                        preserve_invalid_config(app);
                    }
                    return Err(error);
                }
            };
            let candidate = crate::grok_config::validate_preferences(candidate)?;
            next_settings.grok_proxy_preferences = Some(candidate.clone());
            crate::settings::write(app, &next_settings)?;
            candidate
        }
    };

    if let Err(error) = crate::grok_config::apply_proxy_profile(
        app,
        base_origin,
        &preferences,
        super::PLACEHOLDER_KEY,
    ) {
        if is_invalid_config_error(&error) {
            preserve_invalid_config(app);
        }
        if previous_settings.grok_proxy_preferences.is_none() {
            crate::settings::write(app, &previous_settings)?;
        }
        return Err(error);
    }

    Ok(())
}

pub(super) fn set_preferences<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    preferences: crate::grok_config::GrokProxyPreferences,
) -> AppResult<crate::grok_config::GrokConfigState> {
    let preferences = crate::grok_config::validate_preferences(preferences)?;
    let _guard = transaction_lock()?;
    let previous_settings = crate::settings::read(app)?;
    let manifest = super::read_manifest(app, "grok")?;

    let Some(base_origin) = manifest
        .as_ref()
        .filter(|manifest| manifest.enabled)
        .and_then(|manifest| manifest.base_origin.clone())
    else {
        return crate::grok_config::set(app, preferences);
    };

    let mut next_settings = previous_settings.clone();
    next_settings.grok_proxy_preferences = Some(preferences.clone());
    crate::settings::write(app, &next_settings)?;

    if let Err(error) = crate::grok_config::apply_proxy_profile(
        app,
        &base_origin,
        &preferences,
        super::PLACEHOLDER_KEY,
    ) {
        if let Err(rollback_error) = crate::settings::write(app, &previous_settings) {
            return Err(format!(
                "GROK_PREFERENCES_TRANSACTION_ROLLBACK_FAILED: {error}; settings rollback failed: {rollback_error}"
            )
            .into());
        }
        return Err(error);
    }

    crate::grok_config::get(app)
}

pub(super) fn is_proxy_config_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    base_origin: &str,
) -> bool {
    let preferences = match crate::settings::read(app)
        .ok()
        .and_then(|settings| settings.grok_proxy_preferences)
    {
        Some(preferences) => preferences,
        None => return false,
    };

    crate::grok_config::is_proxy_profile_applied(
        app,
        base_origin,
        &preferences,
        super::PLACEHOLDER_KEY,
    )
    .unwrap_or(false)
}

pub(super) fn merge_restore_grok_config(
    target_path: &std::path::Path,
    backup_path: Option<&std::path::Path>,
) -> AppResult<()> {
    crate::grok_config::restore_proxy_fields_path(target_path, backup_path)
}

fn rollback_rebind<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    previous_manifest: &super::CliProxyManifest,
    old_target_snapshots: &[super::FileSnapshot],
    new_target_snapshots: &[super::FileSnapshot],
    backup_snapshots: &[super::FileSnapshot],
) {
    let _ = super::restore_file_snapshots(backup_snapshots);
    let _ = super::restore_file_snapshots(old_target_snapshots);
    let _ = super::restore_file_snapshots(new_target_snapshots);
    let _ = super::write_manifest(app, "grok", previous_manifest);
}

pub(super) fn rebind_grok_manifest_after_home_change<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: super::CliProxyManifest,
    base_origin: &str,
    apply_live: bool,
    trace_id: String,
) -> AppResult<super::CliProxyResult> {
    let previous_manifest = manifest.clone();
    let captured = super::capture_current_target_state(app, "grok")?;
    let old_target_snapshots = previous_manifest
        .files
        .iter()
        .map(|entry| super::snapshot_file(std::path::Path::new(&entry.path)))
        .collect::<AppResult<Vec<_>>>()?;
    let new_target_snapshots = super::snapshot_target_files(&captured)?;
    let backup_snapshots = super::snapshot_backup_files(app, "grok", &captured)?;
    let origin = Some(base_origin.to_string());

    if let Err(error) = super::restore_from_manifest(app, &previous_manifest) {
        rollback_rebind(
            app,
            &previous_manifest,
            &old_target_snapshots,
            &new_target_snapshots,
            &backup_snapshots,
        );
        return Ok(super::CliProxyResult::failure(
            trace_id,
            "grok",
            true,
            "CLI_PROXY_REBIND_RESTORE_FAILED",
            error.to_string(),
            origin,
        ));
    }

    if let Err(error) = super::write_captured_backups(app, "grok", &captured) {
        rollback_rebind(
            app,
            &previous_manifest,
            &old_target_snapshots,
            &new_target_snapshots,
            &backup_snapshots,
        );
        return Ok(super::CliProxyResult::failure(
            trace_id,
            "grok",
            true,
            "CLI_PROXY_REBIND_BACKUP_FAILED",
            error.to_string(),
            origin,
        ));
    }

    let next_manifest = super::build_manifest_from_captured(&manifest, base_origin, captured);
    if let Err(error) = super::write_manifest(app, "grok", &next_manifest) {
        rollback_rebind(
            app,
            &previous_manifest,
            &old_target_snapshots,
            &new_target_snapshots,
            &backup_snapshots,
        );
        return Ok(super::CliProxyResult::failure(
            trace_id,
            "grok",
            true,
            "CLI_PROXY_REBIND_MANIFEST_WRITE_FAILED",
            error.to_string(),
            origin,
        ));
    }

    if apply_live {
        if let Err(error) = apply_proxy_config_locked(app, base_origin) {
            rollback_rebind(
                app,
                &previous_manifest,
                &old_target_snapshots,
                &new_target_snapshots,
                &backup_snapshots,
            );
            return Ok(super::CliProxyResult::failure(
                trace_id,
                "grok",
                true,
                "CLI_PROXY_REBIND_APPLY_FAILED",
                error.to_string(),
                origin,
            ));
        }
    }

    Ok(super::CliProxyResult::success(
        trace_id,
        "grok",
        true,
        if apply_live {
            "已重绑 Grok 目录并写入当前网关配置".to_string()
        } else {
            "已重绑 Grok 目录基线，待网关启动后接管".to_string()
        },
        origin,
    ))
}
