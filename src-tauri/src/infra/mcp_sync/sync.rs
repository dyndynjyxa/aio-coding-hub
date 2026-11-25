//! Usage: Apply managed MCP server config to supported CLIs.

use crate::shared::time::now_unix_seconds;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::claude_json::build_claude_config_json;
use super::codex_toml::build_codex_config_toml;
use super::fs::{read_optional_file_with_max_len, write_file_atomic, write_file_atomic_if_changed};
use super::gemini_json::{merge_antigravity_mcp, release_legacy_entries};
use super::grok_toml::apply_grok_mcp_servers;
use super::manifest::{backup_for_enable, read_manifest, write_manifest};
use super::paths::{
    backup_file_name, mcp_sync_files_dir, mcp_sync_manifest_path, mcp_sync_root_dir,
    mcp_target_path, validate_cli_key,
};
use super::McpServerForSync;

static GROK_MCP_SYNC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn grok_mcp_sync_lock() -> Result<MutexGuard<'static, ()>, String> {
    GROK_MCP_SYNC_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "GROK_MCP_SYNC_LOCK_POISONED".to_string())
}

fn restore_optional_file(path: &Path, bytes: Option<&[u8]>) -> Result<(), String> {
    match bytes {
        Some(bytes) => write_file_atomic(path, bytes).map_err(Into::into),
        None => {
            if path.exists() {
                std::fs::remove_file(path)
                    .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
            }
            Ok(())
        }
    }
}

fn read_config_bytes(cli_key: &str, path: &Path) -> Result<Option<Vec<u8>>, String> {
    if cli_key == "grok" {
        return Ok(crate::grok_config::read_bytes_path(path)?);
    }
    Ok(read_optional_file_with_max_len(
        path,
        super::MCP_SYNC_TARGET_MAX_BYTES,
    )?)
}

fn restore_config_bytes(cli_key: &str, path: &Path, bytes: Option<Vec<u8>>) -> Result<(), String> {
    if cli_key == "grok" {
        return Ok(crate::grok_config::restore_bytes_path(path, bytes)?);
    }
    restore_optional_file(path, bytes.as_deref())
}

struct McpRebindSnapshots {
    cli_key: &'static str,
    old_path: PathBuf,
    old_bytes: Option<Vec<u8>>,
    new_path: PathBuf,
    new_bytes: Option<Vec<u8>>,
    backup_path: PathBuf,
    backup_bytes: Option<Vec<u8>>,
    manifest_path: PathBuf,
    manifest_bytes: Option<Vec<u8>>,
}

impl McpRebindSnapshots {
    fn capture<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        cli_key: &'static str,
        old_path: &Path,
        new_path: &Path,
    ) -> Result<Self, String> {
        let root = mcp_sync_root_dir(app, cli_key)?;
        let backup_path = mcp_sync_files_dir(&root).join(backup_file_name(cli_key));
        let manifest_path = mcp_sync_manifest_path(&root);
        Ok(Self {
            cli_key,
            old_bytes: read_config_bytes(cli_key, old_path)?,
            new_bytes: read_config_bytes(cli_key, new_path)?,
            backup_bytes: read_optional_file_with_max_len(
                &backup_path,
                super::MCP_SYNC_TARGET_MAX_BYTES,
            )?,
            manifest_bytes: read_optional_file_with_max_len(
                &manifest_path,
                super::MCP_SYNC_MANIFEST_MAX_BYTES,
            )?,
            old_path: old_path.to_owned(),
            new_path: new_path.to_path_buf(),
            backup_path,
            manifest_path,
        })
    }

    fn rollback(&self, original_error: String) -> Result<(), String> {
        let mut rollback_errors = Vec::new();
        for result in [
            restore_config_bytes(self.cli_key, &self.old_path, self.old_bytes.clone()),
            restore_config_bytes(self.cli_key, &self.new_path, self.new_bytes.clone()),
            restore_optional_file(&self.backup_path, self.backup_bytes.as_deref()),
            restore_optional_file(&self.manifest_path, self.manifest_bytes.as_deref()),
        ] {
            if let Err(error) = result {
                rollback_errors.push(error);
            }
        }

        if rollback_errors.is_empty() {
            return Err(original_error);
        }
        Err(format!(
            "MCP_SYNC_REBIND_ROLLBACK_FAILED: {original_error}; {}",
            rollback_errors.join("; ")
        ))
    }
}

pub(crate) fn build_next_bytes(
    cli_key: &str,
    current: Option<Vec<u8>>,
    managed_keys: &[String],
    servers: &[McpServerForSync],
) -> Result<Vec<u8>, String> {
    match cli_key {
        "claude" => build_claude_config_json(current, managed_keys, servers),
        "claude_desktop" => {
            if let Some(bytes) = current.as_ref() {
                let value: serde_json::Value = serde_json::from_slice(bytes)
                    .map_err(|error| format!("CLAUDE_DESKTOP_INVALID_CONFIG: {error}"))?;
                if !value.is_object()
                    || value
                        .get("mcpServers")
                        .is_some_and(|value| !value.is_object())
                {
                    return Err(
                        "CLAUDE_DESKTOP_INVALID_CONFIG: config and mcpServers must be objects"
                            .into(),
                    );
                }
            }
            build_claude_config_json(current, managed_keys, servers)
        }
        "codex" => build_codex_config_toml(current, managed_keys, servers),
        "gemini" => Ok(merge_antigravity_mcp(current, managed_keys, servers)?.bytes),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

fn normalized_keys(servers: &[McpServerForSync]) -> Vec<String> {
    let mut keys: Vec<String> = servers.iter().map(|s| s.server_key.to_string()).collect();
    keys.sort();
    keys.dedup();
    keys
}

fn rebind_grok_mcp<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    previous_manifest: super::manifest::McpSyncManifest,
    target_path: &Path,
    desired_keys: Vec<String>,
    servers: &[McpServerForSync],
) -> Result<(), String> {
    let snapshots = McpRebindSnapshots::capture(
        app,
        "grok",
        Path::new(&previous_manifest.file.path),
        target_path,
    )?;
    let apply = || -> Result<(), String> {
        if snapshots.old_bytes.is_some() {
            crate::grok_config::mutate_path(&snapshots.old_path, |document| {
                apply_grok_mcp_servers(document, &previous_manifest.managed_keys, &[])
                    .map_err(crate::shared::error::AppError::from)
            })?;
        }

        let mut manifest = backup_for_enable(app, "grok", Some(previous_manifest.clone()))?;
        manifest.enabled = false;
        manifest.managed_keys.clear();
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, "grok", &manifest)?;

        crate::grok_config::mutate_path(target_path, |document| {
            apply_grok_mcp_servers(document, &[], servers)
                .map_err(crate::shared::error::AppError::from)
        })?;
        manifest.enabled = true;
        manifest.managed_keys = desired_keys;
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, "grok", &manifest)?;
        Ok(())
    };

    match apply() {
        Ok(()) => Ok(()),
        Err(error) => snapshots.rollback(error),
    }
}

fn rebind_desktop_mcp<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    previous_manifest: super::manifest::McpSyncManifest,
    target_path: &Path,
    desired_keys: Vec<String>,
    servers: &[McpServerForSync],
) -> Result<(), String> {
    let snapshots = McpRebindSnapshots::capture(
        app,
        "claude_desktop",
        Path::new(&previous_manifest.file.path),
        target_path,
    )?;
    let apply = || -> Result<(), String> {
        if let Some(old_bytes) = snapshots.old_bytes.clone() {
            let next_bytes = build_next_bytes(
                "claude_desktop",
                Some(old_bytes),
                &previous_manifest.managed_keys,
                &[],
            )?;
            write_file_atomic_if_changed(&snapshots.old_path, &next_bytes)?;
        }

        let mut manifest =
            backup_for_enable(app, "claude_desktop", Some(previous_manifest.clone()))?;
        manifest.enabled = false;
        manifest.managed_keys.clear();
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, "claude_desktop", &manifest)?;

        let next_bytes =
            build_next_bytes("claude_desktop", snapshots.new_bytes.clone(), &[], servers)?;
        write_file_atomic_if_changed(target_path, &next_bytes)?;
        manifest.enabled = true;
        manifest.managed_keys = desired_keys;
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, "claude_desktop", &manifest)?;
        Ok(())
    };

    match apply() {
        Ok(()) => Ok(()),
        Err(error) => snapshots.rollback(error),
    }
}

pub(crate) fn warn_antigravity_mcp<R: tauri::Runtime>(app: &tauri::AppHandle<R>, message: String) {
    tracing::warn!("{message}");
    if let Ok(notice) = crate::notice::build(
        crate::notice::NoticeLevel::Warning,
        Some("Antigravity MCP".into()),
        message,
    ) {
        let _ = crate::notice::emit(app, notice);
    }
}

fn retire_legacy_mcp<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    path: &Path,
    ownership: &super::manifest::LegacyGeminiOwnership,
) -> Result<(), String> {
    if ownership.managed_keys.is_empty()
        || crate::shared::fs::regular_file_metadata(path)?.is_none()
    {
        return Ok(());
    }
    let old = super::fs::read_file_with_max_len(path, super::MCP_SYNC_TARGET_MAX_BYTES)?;
    let backup = if ownership.existed {
        let path = mcp_sync_files_dir(&mcp_sync_root_dir(app, "gemini")?).join("settings.json");
        crate::shared::fs::regular_file_metadata(&path)?;
        Some(super::fs::read_file_with_max_len(
            &path,
            super::MCP_SYNC_TARGET_MAX_BYTES,
        )?)
    } else {
        None
    };
    if let Some(next) = release_legacy_entries(&old, backup.as_deref(), &ownership.managed_keys)? {
        super::fs::write_config_file("gemini", path, &next)?;
    }
    Ok(())
}

fn sync_antigravity_mcp<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    previous: Option<super::manifest::McpSyncManifest>,
    target: &Path,
    servers: &[McpServerForSync],
) -> Result<(), String> {
    crate::shared::fs::regular_file_metadata(target)?;
    let legacy = super::paths::home_dir(app)?.join(".gemini/settings.json");
    let current_manifest = previous
        .as_ref()
        .filter(|m| m.enabled && Path::new(&m.file.path) == target);
    let owned = current_manifest.map_or(&[][..], |m| m.managed_keys.as_slice());
    let current = read_config_bytes("gemini", target)?;
    let merged = merge_antigravity_mcp(current, owned, servers)?;
    let rebind = previous
        .as_ref()
        .filter(|m| m.enabled && Path::new(&m.file.path) != target);
    let retiring = match rebind {
        Some(m) if Path::new(&m.file.path) == legacy => {
            Some(super::manifest::LegacyGeminiOwnership {
                existed: m.file.existed,
                managed_keys: m.managed_keys.clone(),
            })
        }
        Some(_) => {
            warn_antigravity_mcp(
                app,
                "旧 MCP 清单记录的路径无法识别，已保留原文件。请检查旧 Gemini 配置中的 MCP 服务。"
                    .into(),
            );
            None
        }
        None => current_manifest.and_then(|m| m.pending_legacy_gemini.clone()),
    };

    let should_write = !merged.managed_keys.is_empty() || !owned.is_empty();
    let mut manifest = match current_manifest {
        Some(m) => m.clone(),
        None => backup_for_enable(app, "gemini", previous)?,
    };
    // Journal the new ownership before installing it. A retry must never take
    // the already-installed file as the user's original backup.
    manifest.pending_legacy_gemini = retiring;
    let installed_keys = manifest.managed_keys.clone();
    manifest
        .managed_keys
        .extend(merged.managed_keys.iter().cloned());
    manifest.managed_keys.sort();
    manifest.managed_keys.dedup();
    write_manifest(app, "gemini", &manifest)?;
    if should_write {
        if let Err(error) = super::fs::write_config_file("gemini", target, &merged.bytes) {
            // The atomic install failed. Keep the original backup and pending
            // migration, but never claim names the user could now add locally.
            manifest.managed_keys = installed_keys;
            write_manifest(app, "gemini", &manifest)?;
            return Err(error.into());
        }
    }
    if let Some(old) = manifest.pending_legacy_gemini.take() {
        if let Err(error) = retire_legacy_mcp(app, &legacy, &old) {
            warn_antigravity_mcp(app, format!("已保留旧 Gemini MCP 设置。请手动检查 ~/.gemini/settings.json 中原由 AIO 管理的条目。{error}"));
        }
    }
    manifest.managed_keys = merged.managed_keys;
    manifest.updated_at = now_unix_seconds();
    write_manifest(app, "gemini", &manifest)?;
    Ok(())
}

pub(crate) fn antigravity_rebind_needed<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<bool, String> {
    let target = mcp_target_path(app, "gemini")?;
    Ok(read_manifest(app, "gemini")?.is_some_and(|m| {
        m.enabled && (m.pending_legacy_gemini.is_some() || Path::new(&m.file.path) != target)
    }))
}

pub(crate) fn legacy_antigravity_snapshot<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Option<(PathBuf, Vec<u8>)>, String> {
    let Some(m) = read_manifest(app, "gemini")? else {
        return Ok(None);
    };
    let path = super::paths::home_dir(app)?.join(".gemini/settings.json");
    if !m.enabled || (Path::new(&m.file.path) != path && m.pending_legacy_gemini.is_none()) {
        return Ok(None);
    }
    // Retirement will also leave a symlink, oversized or unreadable file alone.
    let old = (|| -> Result<_, String> {
        crate::shared::fs::regular_file_metadata(&path)?;
        read_config_bytes("gemini", &path)
    })()
    .ok()
    .flatten();
    Ok(old.map(|b| (path, b)))
}

pub fn sync_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    servers: &[McpServerForSync],
) -> Result<(), String> {
    validate_cli_key(cli_key)?;

    // Claude Desktop only loads stdio servers (`command`) from this file.
    if cli_key == "claude_desktop" {
        if let Some(server) = servers.iter().find(|server| server.transport != "stdio") {
            return Err(format!(
                "SEC_INVALID_INPUT: unsupported transport={} for claude_desktop: Claude Desktop 只能运行本地 stdio MCP 服务器，请在 Desktop 工作区停用 {}",
                server.transport, server.server_key
            ));
        }
    }

    static ANTIGRAVITY_SYNC: Mutex<()> = Mutex::new(());
    let _antigravity_guard = if cli_key == "gemini" {
        Some(
            ANTIGRAVITY_SYNC
                .lock()
                .map_err(|_| "ANTIGRAVITY_MCP_SYNC_LOCK_POISONED")?,
        )
    } else {
        None
    };
    let _grok_guard = if cli_key == "grok" {
        Some(grok_mcp_sync_lock()?)
    } else {
        None
    };
    let target_path = mcp_target_path(app, cli_key)?;

    let existing = read_manifest(app, cli_key)?;
    if matches!(cli_key, "claude_desktop" | "gemini") && existing.is_none() && servers.is_empty() {
        // A global refresh should only create these client files when needed.
        return Ok(());
    }
    if cli_key == "gemini" {
        return sync_antigravity_mcp(app, existing, &target_path, servers);
    }
    let desired_keys = normalized_keys(servers);
    if let Some(manifest) = existing.as_ref().filter(|manifest| manifest.enabled) {
        if cli_key == "grok"
            && !crate::grok_config::paths_equivalent(Path::new(&manifest.file.path), &target_path)?
        {
            return rebind_grok_mcp(app, manifest.clone(), &target_path, desired_keys, servers);
        }
        if cli_key == "claude_desktop" && Path::new(&manifest.file.path) != target_path.as_path() {
            return rebind_desktop_mcp(app, manifest.clone(), &target_path, desired_keys, servers);
        }
    }
    let should_backup = existing.as_ref().map(|m| !m.enabled).unwrap_or(true);

    let mut manifest = match if should_backup {
        backup_for_enable(app, cli_key, existing.clone())
    } else {
        Ok(existing.unwrap())
    } {
        Ok(m) => m,
        Err(err) => return Err(format!("MCP_SYNC_BACKUP_FAILED: {err}")),
    };

    if should_backup {
        // Persist snapshot before applying changes so we can restore on failure.
        manifest.enabled = false;
        manifest.managed_keys.clear();
        manifest.updated_at = now_unix_seconds();
        write_manifest(app, cli_key, &manifest)?;
    }

    manifest.file.path = target_path.to_string_lossy().to_string();

    let managed_keys = manifest.managed_keys.clone();
    if cli_key == "grok" {
        crate::grok_config::mutate_path(&target_path, |document| {
            apply_grok_mcp_servers(document, &managed_keys, servers)
                .map_err(crate::shared::error::AppError::from)
        })?;
    } else {
        let current =
            read_optional_file_with_max_len(&target_path, super::MCP_SYNC_TARGET_MAX_BYTES)?;
        let next_bytes = build_next_bytes(cli_key, current, &managed_keys, servers)?;
        super::fs::write_config_file(cli_key, &target_path, &next_bytes)?;
    }

    manifest.enabled = true;
    manifest.managed_keys = desired_keys;
    manifest.updated_at = now_unix_seconds();
    write_manifest(app, cli_key, &manifest)?;

    // Best-effort: sanity check to avoid duplicated keys in manifest.
    let set: HashSet<String> = manifest.managed_keys.iter().cloned().collect();
    if set.len() != manifest.managed_keys.len() {
        tracing::warn!(cli_key = %cli_key, "MCP sync: duplicate entries in managed_keys");
    }

    Ok(())
}

pub(crate) fn swap_grok_local_servers_for_workspace<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    managed_keys: &HashSet<String>,
    from_stash_path: &Path,
    target_stash: Option<Vec<u8>>,
) -> crate::shared::error::AppResult<()> {
    if target_stash
        .as_ref()
        .is_some_and(|bytes| bytes.len() > super::MCP_SYNC_TARGET_MAX_BYTES)
    {
        return Err(format!(
            "SEC_INVALID_INPUT: MCP local stash too large (max {} bytes)",
            super::MCP_SYNC_TARGET_MAX_BYTES
        )
        .into());
    }

    let config_path = crate::grok_config::config_path(app)?;
    crate::grok_config::mutate_path(&config_path, |document| {
        let current_stash = super::grok_toml::swap_grok_local_servers(
            document,
            managed_keys,
            target_stash.as_deref(),
        )
        .map_err(crate::shared::error::AppError::from)?;
        if current_stash.len() > super::MCP_SYNC_TARGET_MAX_BYTES {
            return Err(format!(
                "SEC_INVALID_INPUT: MCP local stash too large (max {} bytes)",
                super::MCP_SYNC_TARGET_MAX_BYTES
            )
            .into());
        }
        super::fs::write_file_atomic(from_stash_path, &current_stash)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::MutexGuard;

    static TEST_SEQ: AtomicU64 = AtomicU64::new(1);

    #[derive(Default)]
    struct EnvRestore(Vec<(&'static str, Option<OsString>)>);

    impl EnvRestore {
        fn set(&mut self, key: &'static str, value: impl Into<OsString>) {
            if !self.0.iter().any(|(saved, _)| *saved == key) {
                self.0.push((key, std::env::var_os(key)));
            }
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
            crate::test_support::clear_settings_cache();
        }
    }

    struct GrokMcpTestApp {
        // 字段按声明顺序 drop：必须先恢复环境变量再释放锁，否则下一个测试会把本测试的 env 当原值保存。
        _env: EnvRestore,
        _lock: MutexGuard<'static, ()>,
        _home: tempfile::TempDir,
        app: tauri::App<tauri::test::MockRuntime>,
    }

    impl GrokMcpTestApp {
        fn new() -> Self {
            let lock = crate::test_support::test_env_lock();
            let home = tempfile::tempdir().expect("tempdir");
            let mut env = EnvRestore::default();
            env.set(
                "AIO_CODING_HUB_HOME_DIR",
                home.path().as_os_str().to_os_string(),
            );
            env.set(
                "AIO_CODING_HUB_DOTDIR_NAME",
                format!(
                    ".aio-grok-mcp-test-{}",
                    TEST_SEQ.fetch_add(1, Ordering::Relaxed)
                ),
            );
            env.set(
                "GROK_HOME",
                home.path().join("custom-grok").into_os_string(),
            );
            crate::test_support::clear_settings_cache();
            Self {
                _lock: lock,
                _env: env,
                _home: home,
                app: tauri::test::mock_app(),
            }
        }

        fn handle(&self) -> tauri::AppHandle<tauri::test::MockRuntime> {
            self.app.handle().clone()
        }
    }

    fn stdio_server() -> McpServerForSync {
        McpServerForSync {
            server_key: "managed".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec!["-y".to_string(), "@example/mcp".to_string()],
            env: std::collections::BTreeMap::new(),
            cwd: None,
            url: None,
            headers: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn antigravity_migrates_owned_keys_only_and_preserves_original_backups() {
        let test = GrokMcpTestApp::new();
        let app = test.handle();
        let legacy = test._home.path().join(".gemini/settings.json");
        let target = test._home.path().join(".gemini/config/mcp_config.json");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let initial = br#"{"mcpServers":{"managed":{"command":"preexisting"}}}"#;
        let old = br#"{"other":true,"mcpServers":{"managed":{"command":"aio-old"},"unmanaged":{"httpUrl":"old-local"}}}"#;
        std::fs::write(&legacy, old).unwrap();
        let root = mcp_sync_root_dir(&app, "gemini").unwrap();
        let backups = mcp_sync_files_dir(&root);
        std::fs::create_dir_all(&backups).unwrap();
        std::fs::write(backups.join("settings.json"), initial).unwrap();
        let original_manifest = serde_json::json!({
            "schema_version":1,"managed_by":"aio-coding-hub","cli_key":"gemini","enabled":true,"created_at":0,"updated_at":0,
            "file":{"path":legacy,"existed":true,"backup_rel":"settings.json"},"managed_keys":["managed"]
        });
        std::fs::write(mcp_sync_manifest_path(&root), original_manifest.to_string()).unwrap();

        let agy_original = br#"{"mcpServers":{"user-server":{"command":"keep","disabled":true}}}"#;
        std::fs::write(&target, agy_original).unwrap();
        let before = crate::mcp::SingleCliBackup::capture(&app, "gemini").unwrap();
        let mut server = stdio_server();
        server.transport = "http".into();
        server.url = Some("https://example.test/mcp".into());
        server.headers.insert("X-Test".into(), "value".into());
        sync_cli(&app, "gemini", &[server.clone()]).unwrap();
        let old_json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&legacy).unwrap()).unwrap();
        assert_eq!(old_json["mcpServers"]["managed"]["command"], "preexisting");
        assert_eq!(old_json["mcpServers"]["unmanaged"]["httpUrl"], "old-local");
        assert_eq!(old_json["other"], true);
        let agy: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(
            agy["mcpServers"]["managed"],
            serde_json::json!({"serverUrl":"https://example.test/mcp", "headers":{"X-Test":"value"}})
        );
        assert_eq!(agy["mcpServers"]["user-server"]["disabled"], true);
        assert_eq!(
            std::fs::read(backups.join("settings.json")).unwrap(),
            initial
        );
        assert_eq!(
            std::fs::read(backups.join("mcp_config.json")).unwrap(),
            agy_original
        );
        // Resume after config installation but before the migration commit. Keep
        // the original new-target backup even when the requested keys have changed.
        let mut pending = read_manifest(&app, "gemini").unwrap().unwrap();
        pending.pending_legacy_gemini = Some(super::super::manifest::LegacyGeminiOwnership {
            existed: true,
            managed_keys: vec!["managed".into()],
        });
        write_manifest(&app, "gemini", &pending).unwrap();
        std::fs::write(&legacy, old).unwrap();
        assert!(antigravity_rebind_needed(&app).unwrap());
        sync_cli(&app, "gemini", &[]).unwrap();
        assert!(!antigravity_rebind_needed(&app).unwrap());
        assert_eq!(
            std::fs::read(backups.join("mcp_config.json")).unwrap(),
            agy_original
        );
        sync_cli(&app, "gemini", &[server]).unwrap();
        sync_cli(&app, "gemini", &[]).unwrap();
        let removed: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(
            removed,
            serde_json::from_slice::<serde_json::Value>(agy_original).unwrap()
        );
        // A workspace/config DB transaction can still fail after the file migration succeeds.
        before.restore(&app, "gemini");
        assert_eq!(std::fs::read(&legacy).unwrap(), old);
        assert_eq!(std::fs::read(&target).unwrap(), agy_original);
        assert!(antigravity_rebind_needed(&app).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(backups.join("mcp_config.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
        }
    }

    #[test]
    fn antigravity_takes_over_same_name_entries_and_removes_them_when_disabled() {
        let test = GrokMcpTestApp::new();
        let app = test.handle();
        let target = mcp_target_path(&app, "gemini").unwrap();
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let local = serde_json::json!({"mcpServers":{
            "conflict":{"command":"user-only"},
            "off":{"url":"https://old.test","disabled":true,"oauth":{"native":true}},
            "keep":{"command":"mine"}}});
        std::fs::write(&target, format!("\u{feff}{local}")).unwrap();
        let named = |key: &str| McpServerForSync {
            server_key: key.into(),
            ..stdio_server()
        };
        let read = || {
            super::super::parse_antigravity_mcp_root(Some(&std::fs::read(&target).unwrap()))
                .unwrap()
        };
        let aio = serde_json::json!({"command":"npx","args":["-y","@example/mcp"]});

        sync_cli(&app, "gemini", &[named("conflict"), named("off")]).unwrap();
        let mut off = aio.clone();
        off["oauth"] = serde_json::json!({"native":true});
        assert_eq!(
            read(),
            serde_json::json!({"mcpServers":{"conflict":aio,"off":off,"keep":{"command":"mine"}}})
        );
        assert_eq!(
            read_manifest(&app, "gemini").unwrap().unwrap().managed_keys,
            ["conflict", "off"]
        );

        // Disabling one server in AIO removes it from the file.
        sync_cli(&app, "gemini", &[named("off")]).unwrap();
        assert!(read()["mcpServers"].get("conflict").is_none());
        assert_eq!(read()["mcpServers"]["off"], off);

        sync_cli(&app, "gemini", &[]).unwrap();
        assert_eq!(
            read(),
            serde_json::json!({"mcpServers":{"keep":{"command":"mine"}}})
        );
        assert!(read_manifest(&app, "gemini")
            .unwrap()
            .unwrap()
            .managed_keys
            .is_empty());
    }

    #[test]
    fn antigravity_workspace_switch_keeps_taken_over_same_name_entry() {
        let test = GrokMcpTestApp::new();
        let app = test.handle();
        let db = crate::db::init_for_tests(&test._home.path().join("workspace.sqlite")).unwrap();
        let first = crate::workspaces::create(&db, "gemini", "First", false)
            .unwrap()
            .id;
        let second = crate::workspaces::create(&db, "gemini", "Second", false)
            .unwrap()
            .id;
        crate::workspaces::set_active(&db.open_connection().unwrap(), first).unwrap();
        let target = mcp_target_path(&app, "gemini").unwrap();
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let local =
            serde_json::json!({"mcpServers":{"managed":{"command":"user","nativeOption":true}}});
        std::fs::write(&target, local.to_string()).unwrap();
        // The first workspace requests the same name as an existing local entry.
        let server = stdio_server();
        sync_cli(&app, "gemini", std::slice::from_ref(&server)).unwrap();
        let conn = db.open_connection().unwrap();
        conn.execute(
            "INSERT INTO mcp_servers(server_key,name,normalized_name,transport,command,args_json,env_json,created_at,updated_at) VALUES ('managed','managed','managed','stdio','npx','[]','{}',1,1)",
            [],
        ).unwrap();
        let id = conn.last_insert_rowid();
        for ws in [first, second] {
            conn.execute(
                "INSERT INTO workspace_mcp_enabled(workspace_id,server_id,created_at,updated_at) VALUES (?1,?2,1,1)",
                rusqlite::params![ws, id],
            ).unwrap();
        }
        drop(conn);
        crate::workspace_switch::apply(&app, &db, second).unwrap();
        let read = || {
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(&target).unwrap()).unwrap()
        };
        assert_eq!(read()["mcpServers"]["managed"]["command"], "npx");
        crate::workspace_switch::apply(&app, &db, first).unwrap();
        assert_eq!(read()["mcpServers"]["managed"]["command"], "npx");
        assert_eq!(read()["mcpServers"]["managed"]["nativeOption"], true);
        assert_eq!(
            read_manifest(&app, "gemini").unwrap().unwrap().managed_keys,
            ["managed"]
        );
    }

    #[test]
    fn antigravity_legacy_jsonc_retirement_preserves_bom_comments_and_unknown_values() {
        let current = "\u{feff}{ // unrelated settings\r\n\"mcpServers\":{\"managed\":{\"command\":\"old\"},\"keep\":{\"url\":\"https://test//path\"},}, /* retained */ \"unrelated\":true,}";
        let original = r#"{/* local */"command":"mine","large":1e400}"#;
        let backup = format!(r#"{{"mcpServers":{{"managed":{original},}},}}"#);
        let keys = ["managed".into(), "managed".into()];
        let result = release_legacy_entries(current.as_bytes(), Some(backup.as_bytes()), &keys)
            .unwrap()
            .unwrap();
        let text = String::from_utf8(result).unwrap();
        assert_eq!(text, current.replace(r#"{"command":"old"}"#, original));
        assert!(
            release_legacy_entries(text.as_bytes(), Some(backup.as_bytes()), &keys)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn antigravity_retirement_skips_unprovable_legacy_files_and_clears_pending_ownership() {
        let test = GrokMcpTestApp::new();
        let app = test.handle();
        let root = mcp_sync_root_dir(&app, "gemini").unwrap();
        let target = mcp_target_path(&app, "gemini").unwrap();
        let legacy = test._home.path().join(".gemini/settings.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        for case in [
            "missing-backup",
            "invalid-jsonc",
            "unknown-target",
            "symlink",
        ] {
            let _ = std::fs::remove_file(&legacy);
            let _ = std::fs::remove_file(&target);
            let original = if case == "invalid-jsonc" {
                b"{invalid".as_slice()
            } else {
                br#"{"mcpServers":{"managed":{"command":"keep-legacy"}}}"#
            };
            let other = test._home.path().join("unowned.json");
            std::fs::write(&other, original).unwrap();
            if case == "symlink" {
                #[cfg(unix)]
                std::os::unix::fs::symlink(&other, &legacy).unwrap();
                #[cfg(not(unix))]
                continue;
            } else {
                std::fs::write(&legacy, original).unwrap();
            }
            let path = if case == "unknown-target" {
                &other
            } else {
                &legacy
            };
            let record = serde_json::json!({"schema_version":1,"managed_by":"aio-coding-hub","cli_key":"gemini","enabled":true,"created_at":0,"updated_at":0,
                 "file":{"path":path,"existed":case == "missing-backup","backup_rel":"settings.json"},"managed_keys":["managed"]});
            std::fs::write(mcp_sync_manifest_path(&root), record.to_string()).unwrap();
            sync_cli(&app, "gemini", &[stdio_server()]).unwrap();
            assert!(!antigravity_rebind_needed(&app).unwrap(), "{case}");
            assert_eq!(std::fs::read(&legacy).unwrap(), original, "{case}");
            assert_eq!(std::fs::read(&other).unwrap(), original, "{case}");
            assert_eq!(
                read_manifest(&app, "gemini").unwrap().unwrap().managed_keys,
                ["managed"]
            );
            assert!(target.is_file());
        }
    }

    #[test]
    fn antigravity_unwritable_target_does_not_roll_back_shared_mcp_changes_or_other_routes() {
        let mut test = GrokMcpTestApp::new();
        let home = test._home.path().to_owned();
        test._env.set("AIO_CODING_HUB_TEST_HOME", home.clone());
        test._env.set("CODEX_HOME", home.join(".codex"));
        test._env.set("CLAUDE_USER_DATA_DIR", home.join("desktop"));
        let app = test.handle();
        let db = crate::db::init_for_tests(&home.join("shared.sqlite")).unwrap();
        let claude = crate::workspaces::create(&db, "claude", "MCP test", false)
            .unwrap()
            .id;
        let agy = crate::workspaces::create(&db, "gemini", "MCP test", false)
            .unwrap()
            .id;
        {
            let conn = db.open_connection().unwrap();
            crate::workspaces::set_active(&conn, claude).unwrap();
            crate::workspaces::set_active(&conn, agy).unwrap();
        }
        let target = mcp_target_path(&app, "gemini").unwrap();
        let claude_file = mcp_target_path(&app, "claude").unwrap();
        let server = crate::mcp::parse_json(
            "\u{feff}{\"mcpServers\":{\"shared\":{\"command\":\"before\"}}}",
        )
        .unwrap()
        .servers;
        crate::mcp::import_servers(&app, &db, claude, server).unwrap();
        let id = crate::mcp::list_for_workspace(&db, claude)
            .unwrap()
            .into_iter()
            .find(|s| s.server_key == "shared")
            .unwrap()
            .id;
        crate::mcp::set_enabled(&app, &db, agy, id, true).unwrap();
        let update = |command: &str| {
            crate::mcp::upsert(
                &app,
                &db,
                Some(id),
                "shared",
                "shared",
                "stdio",
                Some(command),
                vec![],
                vec![],
                Default::default(),
                None,
                None,
                vec![],
                Default::default(),
            )
        };
        std::fs::write(&target, b"{malformed").unwrap();
        update("after").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"{malformed");
        let read_claude = || {
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(&claude_file).unwrap())
                .unwrap()
        };
        assert_eq!(read_claude()["mcpServers"]["shared"]["command"], "after");
        // Direct Gemini changes surface the error and roll back the link.
        assert!(crate::mcp::set_enabled(&app, &db, agy, id, false).is_err());
        assert!(
            crate::mcp::list_for_workspace(&db, agy)
                .unwrap()
                .iter()
                .find(|s| s.id == id)
                .unwrap()
                .enabled
        );
        std::fs::remove_file(&target).unwrap();
        std::fs::create_dir(&target).unwrap();
        update("still-writable").unwrap();
        assert!(target.is_dir());
        assert_eq!(
            read_claude()["mcpServers"]["shared"]["command"],
            "still-writable"
        );
        crate::mcp::delete(&app, &db, id).unwrap();
        assert!(read_claude()["mcpServers"].get("shared").is_none());
        assert!(target.is_dir());
    }

    #[test]
    fn antigravity_rejects_sse_and_malformed_json_without_modifying_user_config() {
        let test = GrokMcpTestApp::new();
        let app = test.handle();
        let target = mcp_target_path(&app, "gemini").unwrap();
        sync_cli(&app, "gemini", &[]).unwrap();
        assert!(!target.exists());
        let mut server = stdio_server();
        server.transport = "sse".into();
        assert!(sync_cli(&app, "gemini", &[server])
            .unwrap_err()
            .contains("SSE"));
        assert!(!target.exists());

        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        for raw in [b"{broken".as_slice(), br#"{"mcpServers":[]}"#] {
            std::fs::write(&target, raw).unwrap();
            assert!(sync_cli(&app, "gemini", &[stdio_server()])
                .unwrap_err()
                .contains("INVALID_CONFIG"));
            assert_eq!(std::fs::read(&target).unwrap(), raw);
        }
    }

    #[test]
    fn desktop_empty_global_refresh_does_not_touch_unmanaged_config() {
        let mut test = GrokMcpTestApp::new();
        let desktop_dir = test._home.path().join("desktop-3p");
        test._env
            .set("CLAUDE_USER_DATA_DIR", desktop_dir.into_os_string());
        let app = test.handle();
        let path = crate::infra::cli_proxy::claude_desktop_mcp_config_path(&app).unwrap();
        assert!(!path.exists());

        sync_cli(&app, "claude_desktop", &[]).unwrap();
        assert!(!path.exists());
        assert!(read_manifest(&app, "claude_desktop").unwrap().is_none());
    }

    #[test]
    fn desktop_mcp_sync_preserves_mode_preferences_and_unmanaged_servers() {
        let mut test = GrokMcpTestApp::new();
        let desktop_dir = test._home.path().join("desktop-3p");
        test._env
            .set("CLAUDE_USER_DATA_DIR", desktop_dir.into_os_string());
        let app = test.handle();
        let path = crate::infra::cli_proxy::claude_desktop_mcp_config_path(&app).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, br#"{"deploymentMode":"3p","preferences":{"theme":"dark"},"mcpServers":{"local":{"command":"node"}}}"#).unwrap();

        sync_cli(&app, "claude_desktop", &[stdio_server()]).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["deploymentMode"], "3p");
        assert_eq!(value["preferences"]["theme"], "dark");
        assert_eq!(value["mcpServers"]["local"]["command"], "node");
        assert_eq!(value["mcpServers"]["managed"]["command"], "npx");
    }

    #[test]
    fn desktop_mcp_sync_rejects_remote_servers() {
        let mut test = GrokMcpTestApp::new();
        let desktop_dir = test._home.path().join("desktop-3p");
        test._env
            .set("CLAUDE_USER_DATA_DIR", desktop_dir.into_os_string());
        let app = test.handle();
        let path = crate::infra::cli_proxy::claude_desktop_mcp_config_path(&app).unwrap();
        let remote = McpServerForSync {
            server_key: "remote".to_string(),
            transport: "http".to_string(),
            command: None,
            url: Some("https://example.com/mcp".to_string()),
            ..stdio_server()
        };

        let error = sync_cli(&app, "claude_desktop", &[stdio_server(), remote]).unwrap_err();
        assert!(error.starts_with("SEC_INVALID_INPUT: unsupported transport=http"));
        assert!(!path.exists());
        assert!(read_manifest(&app, "claude_desktop").unwrap().is_none());
    }

    #[test]
    fn desktop_mcp_sync_rebinds_after_3p_dir_change() {
        let mut test = GrokMcpTestApp::new();
        let old_dir = test._home.path().join("desktop-old");
        let new_dir = test._home.path().join("desktop-new");
        test._env
            .set("CLAUDE_USER_DATA_DIR", old_dir.as_os_str().to_os_string());
        let app = test.handle();
        let old_path = crate::infra::cli_proxy::claude_desktop_mcp_config_path(&app).unwrap();
        std::fs::create_dir_all(&old_dir).unwrap();
        std::fs::write(
            &old_path,
            br#"{"mcpServers":{"local_old":{"command":"keep-old"}}}"#,
        )
        .unwrap();
        sync_cli(&app, "claude_desktop", &[stdio_server()]).unwrap();

        test._env
            .set("CLAUDE_USER_DATA_DIR", new_dir.as_os_str().to_os_string());
        let new_path = crate::infra::cli_proxy::claude_desktop_mcp_config_path(&app).unwrap();
        std::fs::create_dir_all(&new_dir).unwrap();
        std::fs::write(
            &new_path,
            br#"{"mcpServers":{"local_new":{"command":"keep-new"}}}"#,
        )
        .unwrap();
        sync_cli(&app, "claude_desktop", &[stdio_server()]).unwrap();

        let old: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&old_path).unwrap()).unwrap();
        assert!(old["mcpServers"].get("managed").is_none());
        assert_eq!(old["mcpServers"]["local_old"]["command"], "keep-old");
        let new: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&new_path).unwrap()).unwrap();
        assert_eq!(new["mcpServers"]["managed"]["command"], "npx");
        assert_eq!(new["mcpServers"]["local_new"]["command"], "keep-new");
        let manifest = read_manifest(&app, "claude_desktop").unwrap().unwrap();
        assert_eq!(Path::new(&manifest.file.path), new_path.as_path());
        assert_eq!(manifest.managed_keys, vec!["managed".to_string()]);
    }

    #[test]
    fn grok_sync_cli_shares_config_with_proxy_and_tracks_managed_keys() {
        let test = GrokMcpTestApp::new();
        let app = test.handle();
        let path = crate::grok_config::config_path(&app).expect("Grok config path");
        std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config dir");
        std::fs::write(&path, "# keep\n[mcp_servers.local]\ncommand = \"local\"\n")
            .expect("write fixture");

        crate::grok_config::apply_proxy_profile(
            &app,
            "http://127.0.0.1:37123",
            &crate::grok_config::GrokProxyPreferences::default(),
            "placeholder",
        )
        .expect("apply proxy");
        sync_cli(&app, "grok", &[stdio_server()]).expect("sync Grok MCP");

        let first = std::fs::read_to_string(&path).expect("read synced config");
        let first_doc = first.parse::<toml_edit::DocumentMut>().expect("valid TOML");
        assert_eq!(first_doc["models"]["default"].as_str(), Some("aio"));
        assert_eq!(
            first_doc["mcp_servers"]["managed"]["command"].as_str(),
            Some("npx")
        );
        assert_eq!(
            first_doc["mcp_servers"]["local"]["command"].as_str(),
            Some("local")
        );

        crate::grok_config::apply_proxy_profile(
            &app,
            "http://127.0.0.1:39999",
            &crate::grok_config::GrokProxyPreferences::default(),
            "placeholder",
        )
        .expect("reapply proxy");
        let after_proxy = std::fs::read_to_string(&path).expect("read config after proxy");
        assert!(after_proxy.contains("[mcp_servers.managed]"));

        let manifest: serde_json::Value = serde_json::from_slice(
            &super::super::read_manifest_bytes(&app, "grok")
                .expect("read manifest")
                .expect("manifest bytes"),
        )
        .expect("manifest JSON");
        assert_eq!(manifest["enabled"].as_bool(), Some(true));
        assert_eq!(manifest["managed_keys"], serde_json::json!(["managed"]));

        sync_cli(&app, "grok", &[]).expect("remove managed Grok MCP");
        let removed = std::fs::read_to_string(&path).expect("read config after removal");
        let removed_doc = removed
            .parse::<toml_edit::DocumentMut>()
            .expect("valid TOML after removal");
        assert!(removed_doc["mcp_servers"].get("managed").is_none());
        assert_eq!(
            removed_doc["mcp_servers"]["local"]["command"].as_str(),
            Some("local")
        );
        assert_eq!(removed_doc["models"]["default"].as_str(), Some("aio"));
    }

    #[test]
    fn grok_sync_cli_rebinds_after_home_change_without_claiming_new_local_keys() {
        let mut test = GrokMcpTestApp::new();
        let app = test.handle();
        let old_home = test._home.path().join("grok-old");
        let new_home = test._home.path().join("grok-new");
        test._env
            .set("GROK_HOME", old_home.as_os_str().to_os_string());
        std::fs::create_dir_all(&old_home).expect("create old Grok home");
        let old_config = old_home.join("config.toml");
        std::fs::write(
            &old_config,
            "[mcp_servers.local_old]\ncommand = \"keep-old\"\n",
        )
        .expect("write old config");
        sync_cli(&app, "grok", &[stdio_server()]).expect("sync old Grok MCP");

        std::fs::create_dir_all(&new_home).expect("create new Grok home");
        let new_config = new_home.join("config.toml");
        std::fs::write(
            &new_config,
            "[mcp_servers.managed]\ncommand = \"new-local\"\n\n[mcp_servers.local_new]\ncommand = \"keep-new\"\n",
        )
        .expect("write new config");
        test._env
            .set("GROK_HOME", new_home.as_os_str().to_os_string());

        sync_cli(&app, "grok", &[]).expect("rebind Grok MCP");

        let old_document = std::fs::read_to_string(&old_config)
            .expect("read old config")
            .parse::<toml_edit::DocumentMut>()
            .expect("valid old config");
        assert!(old_document["mcp_servers"].get("managed").is_none());
        assert_eq!(
            old_document["mcp_servers"]["local_old"]["command"].as_str(),
            Some("keep-old")
        );

        let new_document = std::fs::read_to_string(&new_config)
            .expect("read new config")
            .parse::<toml_edit::DocumentMut>()
            .expect("valid new config");
        assert_eq!(
            new_document["mcp_servers"]["managed"]["command"].as_str(),
            Some("new-local")
        );
        assert_eq!(
            new_document["mcp_servers"]["local_new"]["command"].as_str(),
            Some("keep-new")
        );

        let manifest = read_manifest(&app, "grok")
            .expect("read manifest")
            .expect("Grok manifest");
        assert!(
            crate::grok_config::paths_equivalent(Path::new(&manifest.file.path), &new_config)
                .expect("compare paths")
        );
        assert!(manifest.managed_keys.is_empty());
    }

    #[test]
    fn grok_sync_cli_rebind_rolls_back_when_new_config_is_invalid() {
        let mut test = GrokMcpTestApp::new();
        let app = test.handle();
        let old_home = test._home.path().join("grok-old");
        let new_home = test._home.path().join("grok-new");
        test._env
            .set("GROK_HOME", old_home.as_os_str().to_os_string());
        std::fs::create_dir_all(&old_home).expect("create old Grok home");
        let old_config = old_home.join("config.toml");
        std::fs::write(&old_config, "[mcp_servers.local]\ncommand = \"keep\"\n")
            .expect("write old config");
        sync_cli(&app, "grok", &[stdio_server()]).expect("sync old Grok MCP");
        let old_before = std::fs::read(&old_config).expect("read managed old config");
        let manifest_before = super::super::read_manifest_bytes(&app, "grok")
            .expect("read old manifest")
            .expect("old manifest bytes");

        std::fs::create_dir_all(&new_home).expect("create new Grok home");
        let new_config = new_home.join("config.toml");
        let invalid_new = b"[mcp_servers\ninvalid = true\n";
        std::fs::write(&new_config, invalid_new).expect("write invalid new config");
        test._env
            .set("GROK_HOME", new_home.as_os_str().to_os_string());

        let error = sync_cli(&app, "grok", &[]).expect_err("invalid new config must fail rebind");

        assert!(error.contains("GROK_CONFIG_INVALID_TOML"));
        assert_eq!(
            std::fs::read(&old_config).expect("read rolled back old"),
            old_before
        );
        assert_eq!(
            std::fs::read(&new_config).expect("read unchanged invalid new"),
            invalid_new
        );
        assert_eq!(
            super::super::read_manifest_bytes(&app, "grok")
                .expect("read rolled back manifest")
                .expect("rolled back manifest bytes"),
            manifest_before
        );
    }

    #[test]
    fn grok_sync_cli_rejects_invalid_toml_without_enabling_manifest() {
        let test = GrokMcpTestApp::new();
        let app = test.handle();
        let path = crate::grok_config::config_path(&app).expect("Grok config path");
        std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config dir");
        let invalid = b"[mcp_servers\ninvalid = true\n";
        std::fs::write(&path, invalid).expect("write invalid fixture");

        let error =
            sync_cli(&app, "grok", &[stdio_server()]).expect_err("invalid Grok TOML must fail");

        assert!(error.contains("GROK_CONFIG_INVALID_TOML"));
        assert_eq!(std::fs::read(&path).expect("read original"), invalid);
        let manifest: serde_json::Value = serde_json::from_slice(
            &super::super::read_manifest_bytes(&app, "grok")
                .expect("read manifest")
                .expect("manifest bytes"),
        )
        .expect("manifest JSON");
        assert_eq!(manifest["enabled"].as_bool(), Some(false));
        assert_eq!(manifest["managed_keys"], serde_json::json!([]));
    }
}
