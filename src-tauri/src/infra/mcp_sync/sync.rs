//! Usage: Apply managed MCP server config to supported CLIs.

use crate::shared::time::now_unix_seconds;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::claude_json::build_claude_config_json;
use super::codex_toml::build_codex_config_toml;
use super::fs::{read_optional_file_with_max_len, write_file_atomic, write_file_atomic_if_changed};
use super::gemini_json::build_gemini_settings_json;
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

struct GrokMcpRebindSnapshots {
    old_path: PathBuf,
    old_bytes: Option<Vec<u8>>,
    new_path: PathBuf,
    new_bytes: Option<Vec<u8>>,
    backup_path: PathBuf,
    backup_bytes: Option<Vec<u8>>,
    manifest_path: PathBuf,
    manifest_bytes: Option<Vec<u8>>,
}

impl GrokMcpRebindSnapshots {
    fn capture<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        previous_manifest: &super::manifest::McpSyncManifest,
        new_path: &Path,
    ) -> Result<Self, String> {
        let root = mcp_sync_root_dir(app, "grok")?;
        let backup_path = mcp_sync_files_dir(&root).join(backup_file_name("grok"));
        let manifest_path = mcp_sync_manifest_path(&root);
        let old_path = PathBuf::from(&previous_manifest.file.path);
        Ok(Self {
            old_bytes: crate::grok_config::read_bytes_path(&old_path)?,
            new_bytes: crate::grok_config::read_bytes_path(new_path)?,
            backup_bytes: read_optional_file_with_max_len(
                &backup_path,
                super::MCP_SYNC_TARGET_MAX_BYTES,
            )?,
            manifest_bytes: read_optional_file_with_max_len(
                &manifest_path,
                super::MCP_SYNC_MANIFEST_MAX_BYTES,
            )?,
            old_path,
            new_path: new_path.to_path_buf(),
            backup_path,
            manifest_path,
        })
    }

    fn rollback(&self, original_error: String) -> Result<(), String> {
        let mut rollback_errors = Vec::new();
        for result in [
            crate::grok_config::restore_bytes_path(&self.old_path, self.old_bytes.clone())
                .map_err(String::from),
            crate::grok_config::restore_bytes_path(&self.new_path, self.new_bytes.clone())
                .map_err(String::from),
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
        "codex" => build_codex_config_toml(current, managed_keys, servers),
        "gemini" => build_gemini_settings_json(current, managed_keys, servers),
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
    let snapshots = GrokMcpRebindSnapshots::capture(app, &previous_manifest, target_path)?;
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

pub fn sync_cli<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    servers: &[McpServerForSync],
) -> Result<(), String> {
    validate_cli_key(cli_key)?;

    let _grok_guard = if cli_key == "grok" {
        Some(grok_mcp_sync_lock()?)
    } else {
        None
    };
    let target_path = mcp_target_path(app, cli_key)?;

    let existing = read_manifest(app, cli_key)?;
    let desired_keys = normalized_keys(servers);
    if let Some(manifest) = existing.as_ref().filter(|manifest| manifest.enabled) {
        if cli_key == "grok"
            && !crate::grok_config::paths_equivalent(Path::new(&manifest.file.path), &target_path)?
        {
            return rebind_grok_mcp(app, manifest.clone(), &target_path, desired_keys, servers);
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
        write_file_atomic_if_changed(&target_path, &next_bytes)?;
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
        _lock: MutexGuard<'static, ()>,
        _env: EnvRestore,
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
