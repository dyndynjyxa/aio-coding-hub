//! Usage: Sync/backup/restore prompt instruction files for supported CLIs (infra adapter).

use crate::app_paths;
use crate::codex_paths;
use crate::shared::fs::{
    copy_dir_recursive_if_missing, copy_file_if_missing, read_file_with_max_len,
    read_optional_file_with_max_len, write_file_atomic, write_file_atomic_if_changed,
};
use crate::shared::time::now_unix_seconds;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MANIFEST_SCHEMA_VERSION: u32 = 1;
const MANAGED_BY: &str = "aio-coding-hub";
const LEGACY_APP_DOTDIR_NAMES: &[&str] = &[".aio-gateway", ".aio_gateway"];
const PROMPT_SYNC_TARGET_MAX_BYTES: usize = 1024 * 1024;
const PROMPT_SYNC_MANIFEST_MAX_BYTES: usize = 256 * 1024;

fn ensure_prompt_sync_bytes_within_limit(
    bytes: &[u8],
    max_len: usize,
    label: &str,
) -> crate::shared::error::AppResult<()> {
    if bytes.len() > max_len {
        return Err(format!("SEC_INVALID_INPUT: {label} too large (max {max_len} bytes)").into());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptSyncFileEntry {
    path: String,
    existed: bool,
    backup_rel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptSyncManifest {
    schema_version: u32,
    managed_by: String,
    cli_key: String,
    enabled: bool,
    applied_prompt_id: Option<i64>,
    created_at: i64,
    updated_at: i64,
    file: PromptSyncFileEntry,
}

#[derive(Debug)]
struct PromptFileSnapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key).map_err(Into::into)
}

fn home_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::app_paths::home_dir(app)
}

fn prompt_target_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    validate_cli_key(cli_key)?;
    let home = home_dir(app)?;

    match cli_key {
        "claude" => Ok(home.join(".claude").join("CLAUDE.md")),
        "codex" => codex_paths::codex_agents_md_path(app),
        "gemini" => Ok(home.join(".gemini").join("GEMINI.md")),
        "grok" => crate::grok_config::agents_md_path(app),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}").into()),
    }
}

fn prompt_sync_root_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?
        .join("prompt-sync")
        .join(cli_key))
}

fn prompt_sync_files_dir(root: &Path) -> PathBuf {
    root.join("files")
}

fn prompt_sync_safety_dir(root: &Path) -> PathBuf {
    root.join("restore-safety")
}

fn prompt_sync_manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.json")
}

fn legacy_prompt_sync_roots<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<PathBuf>> {
    let home = home_dir(app)?;
    Ok(LEGACY_APP_DOTDIR_NAMES
        .iter()
        .map(|dir_name| home.join(dir_name).join("prompt-sync").join(cli_key))
        .collect())
}

fn try_migrate_legacy_prompt_sync_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<bool> {
    let new_root = prompt_sync_root_dir(app, cli_key)?;
    let new_manifest_path = prompt_sync_manifest_path(&new_root);
    if new_manifest_path.exists() {
        return Ok(false);
    }

    for legacy_root in legacy_prompt_sync_roots(app, cli_key)? {
        let legacy_manifest_path = prompt_sync_manifest_path(&legacy_root);
        if !legacy_manifest_path.exists() {
            continue;
        }

        std::fs::create_dir_all(&new_root)
            .map_err(|e| format!("failed to create {}: {e}", new_root.display()))?;

        let _ = copy_file_if_missing(&legacy_manifest_path, &new_manifest_path)?;

        let legacy_files_dir = prompt_sync_files_dir(&legacy_root);
        if legacy_files_dir.exists() {
            let new_files_dir = prompt_sync_files_dir(&new_root);
            copy_dir_recursive_if_missing(&legacy_files_dir, &new_files_dir)?;
        }

        return Ok(true);
    }

    Ok(false)
}

pub fn read_target_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    let path = prompt_target_path(app, cli_key)?;
    read_optional_file_with_max_len(&path, PROMPT_SYNC_TARGET_MAX_BYTES)
}

pub fn restore_target_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> crate::shared::error::AppResult<()> {
    let path = prompt_target_path(app, cli_key)?;
    match bytes {
        Some(content) => {
            ensure_prompt_sync_bytes_within_limit(
                &content,
                PROMPT_SYNC_TARGET_MAX_BYTES,
                "prompt sync target restore",
            )?;
            write_file_atomic(&path, &content)
        }
        None => {
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
            }
            Ok(())
        }
    }
}

pub fn read_manifest_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    let path = prompt_sync_manifest_path(&root);
    read_optional_file_with_max_len(&path, PROMPT_SYNC_MANIFEST_MAX_BYTES)
}

pub fn restore_manifest_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> crate::shared::error::AppResult<()> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    let path = prompt_sync_manifest_path(&root);
    match bytes {
        Some(content) => {
            ensure_prompt_sync_bytes_within_limit(
                &content,
                PROMPT_SYNC_MANIFEST_MAX_BYTES,
                "prompt sync manifest restore",
            )?;
            write_file_atomic(&path, &content)
        }
        None => {
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
            }
            Ok(())
        }
    }
}

fn read_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Option<PromptSyncManifest>> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    let path = prompt_sync_manifest_path(&root);

    if !path.exists() {
        if let Err(err) = try_migrate_legacy_prompt_sync_dir(app, cli_key) {
            tracing::warn!("prompt sync migration failed: {}", err);
        }
    }

    let Some(content) = read_optional_file_with_max_len(&path, PROMPT_SYNC_MANIFEST_MAX_BYTES)?
    else {
        return Ok(None);
    };

    let manifest: PromptSyncManifest = serde_json::from_slice(&content)
        .map_err(|e| format!("failed to parse prompt manifest.json: {e}"))?;

    if manifest.managed_by != MANAGED_BY {
        return Err(format!(
            "prompt manifest managed_by mismatch: expected {MANAGED_BY}, got {}",
            manifest.managed_by
        )
        .into());
    }

    Ok(Some(manifest))
}

fn write_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    manifest: &PromptSyncManifest,
) -> crate::shared::error::AppResult<()> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create {}: {e}", root.display()))?;
    let path = prompt_sync_manifest_path(&root);

    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("failed to serialize prompt manifest.json: {e}"))?;
    write_file_atomic(&path, &bytes)?;
    Ok(())
}

fn capture_prompt_file_snapshot(
    path: &Path,
    max_len: usize,
) -> crate::shared::error::AppResult<PromptFileSnapshot> {
    Ok(PromptFileSnapshot {
        path: path.to_path_buf(),
        bytes: read_optional_file_with_max_len(path, max_len)?,
    })
}

fn restore_prompt_file_snapshot(
    snapshot: &PromptFileSnapshot,
) -> crate::shared::error::AppResult<()> {
    match snapshot.bytes.as_ref() {
        Some(bytes) => write_file_atomic(&snapshot.path, bytes),
        None => {
            if snapshot.path.exists() {
                std::fs::remove_file(&snapshot.path)
                    .map_err(|e| format!("failed to remove {}: {e}", snapshot.path.display()))?;
            }
            Ok(())
        }
    }
}

fn prompt_manifest_target_changed(
    manifest: &PromptSyncManifest,
    target_path: &Path,
) -> crate::shared::error::AppResult<bool> {
    let manifest_path = Path::new(&manifest.file.path);
    if manifest.cli_key == "grok" {
        return Ok(!crate::grok_config::paths_equivalent(
            manifest_path,
            target_path,
        )?);
    }
    Ok(manifest_path != target_path)
}

fn rollback_prompt_rebind(
    snapshots: &[PromptFileSnapshot],
    original_error: crate::shared::error::AppError,
) -> crate::shared::error::AppResult<()> {
    let mut rollback_errors = Vec::new();
    for snapshot in snapshots.iter().rev() {
        if let Err(error) = restore_prompt_file_snapshot(snapshot) {
            rollback_errors.push(error.to_string());
        }
    }

    if rollback_errors.is_empty() {
        return Err(original_error);
    }

    Err(format!(
        "PROMPT_SYNC_REBIND_ROLLBACK_FAILED: {original_error}; {}",
        rollback_errors.join("; ")
    )
    .into())
}

fn backup_for_enable<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    existing: Option<PromptSyncManifest>,
) -> crate::shared::error::AppResult<PromptSyncManifest> {
    let root = prompt_sync_root_dir(app, cli_key)?;
    let files_dir = prompt_sync_files_dir(&root);
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("failed to create {}: {e}", files_dir.display()))?;

    let target_path = prompt_target_path(app, cli_key)?;
    let now = now_unix_seconds();

    let existed = target_path.exists();
    let backup_rel = if existed {
        let bytes = read_file_with_max_len(&target_path, PROMPT_SYNC_TARGET_MAX_BYTES)?;
        let backup_name = target_path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("prompt.md")
            .to_string();
        let backup_path = files_dir.join(&backup_name);
        write_file_atomic(&backup_path, &bytes)?;
        Some(backup_name)
    } else {
        None
    };

    let created_at = existing.as_ref().map(|m| m.created_at).unwrap_or(now);

    Ok(PromptSyncManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        managed_by: MANAGED_BY.to_string(),
        cli_key: cli_key.to_string(),
        enabled: true,
        applied_prompt_id: None,
        created_at,
        updated_at: now,
        file: PromptSyncFileEntry {
            path: target_path.to_string_lossy().to_string(),
            existed,
            backup_rel,
        },
    })
}

fn prompt_backup_candidates(manifest: &PromptSyncManifest) -> Vec<String> {
    let mut candidates = manifest.file.backup_rel.iter().cloned().collect::<Vec<_>>();
    if let Some(file_name) = Path::new(&manifest.file.path)
        .file_name()
        .and_then(|value| value.to_str())
    {
        let file_name = file_name.to_string();
        if !candidates.contains(&file_name) {
            candidates.push(file_name);
        }
    }
    candidates
}

fn ensure_prompt_rebind_restore_available<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: &PromptSyncManifest,
) -> crate::shared::error::AppResult<()> {
    if !manifest.file.existed {
        return Ok(());
    }
    let files_dir = prompt_sync_files_dir(&prompt_sync_root_dir(app, &manifest.cli_key)?);
    if prompt_backup_candidates(manifest)
        .iter()
        .any(|relative| files_dir.join(relative).is_file())
    {
        return Ok(());
    }
    Err(format!(
        "PROMPT_SYNC_REBIND_BACKUP_MISSING: cannot restore {} before rebinding",
        manifest.file.path
    )
    .into())
}

pub(crate) fn prompt_content_to_bytes(content: &str) -> Vec<u8> {
    let trimmed = content.trim_matches('\u{feff}').trim_end();
    let mut out = trimmed.as_bytes().to_vec();
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    out
}

fn restore_from_manifest<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    manifest: &PromptSyncManifest,
) -> crate::shared::error::AppResult<()> {
    let cli_key = manifest.cli_key.as_str();
    validate_cli_key(cli_key)?;

    let root = prompt_sync_root_dir(app, cli_key)?;
    let files_dir = prompt_sync_files_dir(&root);
    let safety_dir = prompt_sync_safety_dir(&root);
    std::fs::create_dir_all(&safety_dir)
        .map_err(|e| format!("failed to create {}: {e}", safety_dir.display()))?;

    let target_path = PathBuf::from(&manifest.file.path);
    let ts = now_unix_seconds();

    if manifest.file.existed {
        for rel in prompt_backup_candidates(manifest) {
            let backup_path = files_dir.join(&rel);
            if !backup_path.exists() {
                continue;
            }
            let bytes = read_file_with_max_len(&backup_path, PROMPT_SYNC_TARGET_MAX_BYTES)?;
            write_file_atomic(&target_path, &bytes)?;
            return Ok(());
        }

        // No backup available. Keep current file content as-is (best-effort),
        // but store a safety snapshot to help users recover manually.
        if target_path.exists() {
            if let Ok(Some(bytes)) =
                read_optional_file_with_max_len(&target_path, PROMPT_SYNC_TARGET_MAX_BYTES)
            {
                let safe_name = format!("{ts}_prompt_keep_current_no_backup");
                let safe_path = safety_dir.join(safe_name);
                let _ = write_file_atomic(&safe_path, &bytes);
            }
        }

        tracing::warn!(cli_key = %cli_key, "prompt sync: backup not found");
        return Ok(());
    }

    if !target_path.exists() {
        return Ok(());
    }

    // If the file did not exist before enabling prompt sync, restore to "absent".
    // Safety copy current content before removal.
    if let Ok(Some(bytes)) =
        read_optional_file_with_max_len(&target_path, PROMPT_SYNC_TARGET_MAX_BYTES)
    {
        let safe_name = format!("{ts}_prompt_before_remove");
        let safe_path = safety_dir.join(safe_name);
        let _ = write_file_atomic(&safe_path, &bytes);
    }

    std::fs::remove_file(&target_path)
        .map_err(|e| format!("failed to remove {}: {e}", target_path.display()))?;

    Ok(())
}

fn rebind_enabled_prompt<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    previous_manifest: PromptSyncManifest,
    prompt_id: i64,
    target_path: &Path,
    bytes: &[u8],
) -> crate::shared::error::AppResult<()> {
    ensure_prompt_rebind_restore_available(app, &previous_manifest)?;
    let root = prompt_sync_root_dir(app, &previous_manifest.cli_key)?;
    let files_dir = prompt_sync_files_dir(&root);
    let backup_name = target_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("prompt.md");
    let snapshots = vec![
        capture_prompt_file_snapshot(
            &prompt_sync_manifest_path(&root),
            PROMPT_SYNC_MANIFEST_MAX_BYTES,
        )?,
        capture_prompt_file_snapshot(&files_dir.join(backup_name), PROMPT_SYNC_TARGET_MAX_BYTES)?,
        capture_prompt_file_snapshot(target_path, PROMPT_SYNC_TARGET_MAX_BYTES)?,
        capture_prompt_file_snapshot(
            &PathBuf::from(&previous_manifest.file.path),
            PROMPT_SYNC_TARGET_MAX_BYTES,
        )?,
    ];

    let apply = || -> crate::shared::error::AppResult<()> {
        restore_from_manifest(app, &previous_manifest)?;

        let mut manifest = backup_for_enable(
            app,
            &previous_manifest.cli_key,
            Some(previous_manifest.clone()),
        )?;
        manifest.enabled = false;
        manifest.applied_prompt_id = None;
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, &previous_manifest.cli_key, &manifest)?;

        write_file_atomic_if_changed(target_path, bytes)?;
        manifest.enabled = true;
        manifest.applied_prompt_id = Some(prompt_id);
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, &previous_manifest.cli_key, &manifest)
    };

    match apply() {
        Ok(()) => Ok(()),
        Err(error) => rollback_prompt_rebind(&snapshots, error),
    }
}

pub fn apply_enabled_prompt<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    prompt_id: i64,
    content: &str,
) -> crate::shared::error::AppResult<()> {
    validate_cli_key(cli_key)?;

    let bytes = prompt_content_to_bytes(content);
    ensure_prompt_sync_bytes_within_limit(
        &bytes,
        PROMPT_SYNC_TARGET_MAX_BYTES,
        "prompt sync target content",
    )?;
    let target_path = prompt_target_path(app, cli_key)?;

    let existing = read_manifest(app, cli_key)?;
    if let Some(manifest) = existing.as_ref().filter(|manifest| manifest.enabled) {
        if prompt_manifest_target_changed(manifest, &target_path)? {
            return rebind_enabled_prompt(app, manifest.clone(), prompt_id, &target_path, &bytes);
        }
    }
    let should_backup = existing.as_ref().map(|m| !m.enabled).unwrap_or(true);

    let mut manifest = match if should_backup {
        backup_for_enable(app, cli_key, existing.clone())
    } else {
        Ok(existing.unwrap())
    } {
        Ok(m) => m,
        Err(err) => return Err(format!("PROMPT_SYNC_BACKUP_FAILED: {err}").into()),
    };

    if should_backup {
        // Persist snapshot before applying changes so we can restore on failure.
        manifest.enabled = false;
        manifest.applied_prompt_id = None;
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, cli_key, &manifest)?;
    }

    manifest.file.path = target_path.to_string_lossy().to_string();

    write_file_atomic_if_changed(&target_path, &bytes)?;

    manifest.enabled = true;
    manifest.applied_prompt_id = Some(prompt_id);
    manifest.updated_at = now_unix_seconds();
    write_manifest(app, cli_key, &manifest)?;

    Ok(())
}

pub fn restore_disabled_prompt<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<()> {
    validate_cli_key(cli_key)?;

    let Some(mut manifest) = read_manifest(app, cli_key)? else {
        let root = prompt_sync_root_dir(app, cli_key)?;
        let files_dir = prompt_sync_files_dir(&root);
        let safety_dir = prompt_sync_safety_dir(&root);
        std::fs::create_dir_all(&safety_dir)
            .map_err(|e| format!("failed to create {}: {e}", safety_dir.display()))?;

        let target_path = prompt_target_path(app, cli_key)?;
        let ts = now_unix_seconds();

        let backup_rel = target_path
            .file_name()
            .and_then(|v| v.to_str())
            .and_then(|file_name| {
                let name = file_name.to_string();
                let backup_path = files_dir.join(&name);
                if !backup_path.exists() {
                    return None;
                }

                let bytes =
                    read_file_with_max_len(&backup_path, PROMPT_SYNC_TARGET_MAX_BYTES).ok()?;
                write_file_atomic(&target_path, &bytes).ok()?;
                Some(name)
            });

        if backup_rel.is_none() && target_path.exists() {
            if let Ok(Some(bytes)) =
                read_optional_file_with_max_len(&target_path, PROMPT_SYNC_TARGET_MAX_BYTES)
            {
                let safe_name = format!("{ts}_prompt_keep_current_no_manifest");
                let safe_path = safety_dir.join(safe_name);
                let _ = write_file_atomic(&safe_path, &bytes);
            }
            tracing::warn!(cli_key = %cli_key, "prompt sync: manifest missing, keeping current files");
        }

        let now = now_unix_seconds();
        let manifest = PromptSyncManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            managed_by: MANAGED_BY.to_string(),
            cli_key: cli_key.to_string(),
            enabled: false,
            applied_prompt_id: None,
            created_at: now,
            updated_at: now,
            file: PromptSyncFileEntry {
                path: target_path.to_string_lossy().to_string(),
                existed: target_path.exists(),
                backup_rel,
            },
        };
        write_manifest(app, cli_key, &manifest)?;
        return Ok(());
    };

    restore_from_manifest(app, &manifest)?;

    manifest.enabled = false;
    manifest.applied_prompt_id = None;
    manifest.updated_at = now_unix_seconds();
    write_manifest(app, cli_key, &manifest)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[derive(Default)]
    struct EnvRestore(Vec<(&'static str, Option<OsString>)>);

    impl EnvRestore {
        fn set(&mut self, key: &'static str, value: impl Into<OsString>) {
            self.0.push((key, std::env::var_os(key)));
            std::env::set_var(key, value.into());
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, value) in self.0.drain(..).rev() {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn prompt_sync_bytes_limit_rejects_oversized_content() {
        let bytes = vec![b'x'; PROMPT_SYNC_TARGET_MAX_BYTES + 1];

        let err = ensure_prompt_sync_bytes_within_limit(
            &bytes,
            PROMPT_SYNC_TARGET_MAX_BYTES,
            "prompt sync target content",
        )
        .expect_err("oversized prompt sync content should fail");

        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn grok_prompt_target_uses_grok_home_agents_md() {
        let _lock = crate::test_support::test_env_lock();
        let mut env = EnvRestore::default();
        let temp = tempfile::tempdir().expect("tempdir");
        let grok_home = temp.path().join("custom-grok");
        env.set("GROK_HOME", grok_home.as_os_str().to_os_string());
        let app = tauri::test::mock_app();

        let path = prompt_target_path(app.handle(), "grok").expect("Grok prompt target");

        assert_eq!(path, grok_home.join("AGENTS.md"));
    }

    #[test]
    fn grok_prompt_rebinds_after_home_change() {
        let _lock = crate::test_support::test_env_lock();
        let temp = tempfile::tempdir().expect("tempdir");
        let old_home = temp.path().join("grok-old");
        let new_home = temp.path().join("grok-new");
        let mut env = EnvRestore::default();
        env.set(
            "AIO_CODING_HUB_HOME_DIR",
            temp.path().as_os_str().to_os_string(),
        );
        env.set("AIO_CODING_HUB_DOTDIR_NAME", ".aio-prompt-rebind-test");
        env.set("GROK_HOME", old_home.as_os_str().to_os_string());
        let app = tauri::test::mock_app();

        std::fs::create_dir_all(&old_home).expect("create old Grok home");
        let old_target = old_home.join("AGENTS.md");
        std::fs::write(&old_target, "old local\n").expect("write old prompt");
        apply_enabled_prompt(app.handle(), "grok", 1, "managed old")
            .expect("apply old managed prompt");

        std::fs::create_dir_all(&new_home).expect("create new Grok home");
        let new_target = new_home.join("AGENTS.md");
        std::fs::write(&new_target, "new local\n").expect("write new prompt");
        std::env::set_var("GROK_HOME", &new_home);

        apply_enabled_prompt(app.handle(), "grok", 2, "managed new")
            .expect("rebind managed prompt");

        assert_eq!(
            std::fs::read_to_string(&old_target).expect("read restored old prompt"),
            "old local\n"
        );
        assert_eq!(
            std::fs::read_to_string(&new_target).expect("read managed new prompt"),
            "managed new\n"
        );

        restore_disabled_prompt(app.handle(), "grok").expect("disable rebound prompt");
        assert_eq!(
            std::fs::read_to_string(&new_target).expect("read restored new prompt"),
            "new local\n"
        );
    }

    #[test]
    fn grok_prompt_rebind_fails_closed_when_old_backup_is_missing() {
        let _lock = crate::test_support::test_env_lock();
        let temp = tempfile::tempdir().expect("tempdir");
        let old_home = temp.path().join("grok-old");
        let new_home = temp.path().join("grok-new");
        let mut env = EnvRestore::default();
        env.set(
            "AIO_CODING_HUB_HOME_DIR",
            temp.path().as_os_str().to_os_string(),
        );
        env.set(
            "AIO_CODING_HUB_DOTDIR_NAME",
            ".aio-prompt-rebind-missing-test",
        );
        env.set("GROK_HOME", old_home.as_os_str().to_os_string());
        let app = tauri::test::mock_app();

        std::fs::create_dir_all(&old_home).expect("create old Grok home");
        let old_target = old_home.join("AGENTS.md");
        std::fs::write(&old_target, "old local\n").expect("write old prompt");
        apply_enabled_prompt(app.handle(), "grok", 1, "managed old")
            .expect("apply old managed prompt");
        let backup = prompt_sync_files_dir(
            &prompt_sync_root_dir(app.handle(), "grok").expect("prompt sync root"),
        )
        .join("AGENTS.md");
        std::fs::remove_file(backup).expect("remove old backup");

        std::fs::create_dir_all(&new_home).expect("create new Grok home");
        let new_target = new_home.join("AGENTS.md");
        std::fs::write(&new_target, "new local\n").expect("write new prompt");
        std::env::set_var("GROK_HOME", &new_home);

        let error = apply_enabled_prompt(app.handle(), "grok", 2, "managed new")
            .expect_err("missing old backup must block rebind");

        assert!(error
            .to_string()
            .contains("PROMPT_SYNC_REBIND_BACKUP_MISSING"));
        assert_eq!(
            std::fs::read_to_string(&old_target).expect("read unchanged old prompt"),
            "managed old\n"
        );
        assert_eq!(
            std::fs::read_to_string(&new_target).expect("read unchanged new prompt"),
            "new local\n"
        );
        let manifest = read_manifest(app.handle(), "grok")
            .expect("read manifest")
            .expect("Grok prompt manifest");
        assert_eq!(Path::new(&manifest.file.path), old_target);
    }
}
