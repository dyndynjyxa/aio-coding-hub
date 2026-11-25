//! Antigravity reads the same persistent Enterprise Gateway config in CLI and editor sessions.

use super::detection::{resolve_wsl_home_unc, wsl_linux_path_to_windows_path};
use super::manifest::{read_wsl_manifest, WSL_CLIENT_CONFIG_MAX_BYTES};
use super::shell::{bash_single_quote, run_wsl_bash_script};
use super::types::{WslCliBackup, WslDistroManifest};
use crate::cli_proxy::agy_gateway;
use crate::cli_proxy::legacy_gemini_env::{legacy_env_assignment, legacy_env_owned_base};
use crate::shared::error::AppResult;
use crate::shared::fs::{read_file_with_max_len, regular_file_metadata};
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

const GATEWAY: &str = "antigravity.gateway";
const OWNER_HOME: &str = "antigravity.home";
const PREVIOUS_ORIGIN: &str = "antigravity.previousOrigin";
const INACTIVE: &str = "antigravity.inactive";
const KNOWN_ORIGIN: &str = "antigravity.origin:";
const CONFIG: &str = ".gemini/config/config.json";

#[derive(Debug)]
struct ConfigEdit {
    path: String,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
}

impl ConfigEdit {
    fn replace_script(&self, rollback: bool) -> String {
        let (before, after) = if rollback {
            (&self.after, &self.before)
        } else {
            (&self.before, &self.after)
        };
        let check = match before {
            Some(bytes) => format!(
                "test -f \"$file\" && test \"$(sha256sum < \"$file\" | cut -d' ' -f1)\" = '{:x}'",
                Sha256::digest(bytes)
            ),
            None => "test ! -e \"$file\"".into(),
        };
        let replace = match after {
            Some(bytes) => format!(
                r#"
mkdir -p -- "$(dirname -- "$file")"
tmp="$(mktemp "${{file}}.tmp.XXXXXX")"
trap 'rm -f -- "$tmp"' EXIT
if [ -f "$file" ]; then cp -p -- "$file" "$tmp"; fi
printf '%s' '{}' | base64 -d > "$tmp"
mv -f -- "$tmp" "$file"
"#,
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ),
            None => "rm -f -- \"$file\"".into(),
        };
        format!(
            r#"set -euo pipefail
file={}
if [ -L "$file" ] || ! ( {check} ); then
  echo 'WSL configuration changed or is a symbolic link; retry after checking the file' >&2
  exit 1
fi
{replace}"#,
            bash_single_quote(&self.path)
        )
    }
}

pub(super) struct WslAgyPlan {
    pub(super) pending: WslCliBackup,
    pub(super) retained: Option<WslCliBackup>,
    edits: Vec<ConfigEdit>,
}

impl WslAgyPlan {
    pub(super) fn apply(&self, distro: &str) -> AppResult<()> {
        self.apply_with(|script| run_wsl_bash_script(distro, script))
    }

    fn apply_with(&self, mut run: impl FnMut(&str) -> AppResult<()>) -> AppResult<()> {
        for (index, edit) in self.edits.iter().enumerate() {
            if let Err(error) = run(&edit.replace_script(false)) {
                for applied in self.edits[..index].iter().rev() {
                    run(&applied.replace_script(true)).map_err(|rollback| {
                        format!("{error}; restoring WSL configuration also failed: {rollback}")
                    })?;
                }
                return Err(error);
            }
        }
        Ok(())
    }
}

pub(super) fn has_legacy_env(backup: &WslCliBackup) -> bool {
    backup.cli_key == "gemini"
        && ["GOOGLE_GEMINI_BASE_URL", "GEMINI_API_KEY"]
            .iter()
            .any(|key| backup.original_values.contains_key(*key))
}

fn read_config(path: &Path) -> AppResult<Option<Vec<u8>>> {
    if regular_file_metadata(path)?.is_none() {
        return Ok(None);
    }
    read_file_with_max_len(path, WSL_CLIENT_CONFIG_MAX_BYTES).map(Some)
}

pub(super) fn agy_config_applied<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    distro: &str,
) -> AppResult<bool> {
    let Some(manifest) = read_wsl_manifest(app, distro)? else {
        return Ok(false);
    };
    let Some(backup) = manifest.cli_backups.iter().find(|b| b.cli_key == "gemini") else {
        return Ok(false);
    };
    if is_inactive(backup) {
        return Ok(false);
    }
    let Some(origin) = backup.injected_keys.get(GATEWAY) else {
        return Ok(false);
    };
    let home = resolve_wsl_home_unc(distro)?;
    if backup.injected_keys.get(OWNER_HOME) != Some(&linux_home(distro, &home)?) {
        return Ok(false);
    }
    Ok(read_config(&home.join(CONFIG))?
        .is_some_and(|value| agy_gateway::is_applied(&value, origin)))
}

fn linux_home(distro: &str, home: &Path) -> AppResult<String> {
    let unc = home.to_string_lossy();
    let rel = unc
        .strip_prefix(&format!(r"\\wsl$\{distro}\"))
        .ok_or("Unrecognized WSL home in proxy backup")?;
    Ok(format!("/{}", rel.replace('\\', "/")))
}

// Older Gemini records stored their owner only in the shared distro header.
// Pin it before another CLI can rewrite that header after a default-user change.
pub(super) fn pin_legacy_home(
    distro: &str,
    mut manifest: WslDistroManifest,
) -> AppResult<WslDistroManifest> {
    if let Some(home) = &manifest.wsl_home_unc {
        for backup in &mut manifest.cli_backups {
            if has_legacy_env(backup) && !backup.injected_keys.contains_key(OWNER_HOME) {
                backup
                    .injected_keys
                    .insert(OWNER_HOME.into(), linux_home(distro, Path::new(home))?);
            }
        }
    }
    Ok(manifest)
}

pub(super) fn plan_wsl_agy(
    distro: &str,
    home: &Path,
    previous: Option<&WslCliBackup>,
    origin: Option<&str>,
) -> AppResult<WslAgyPlan> {
    prepare_changes(&linux_home(distro, home)?, previous, origin, |path| {
        read_config(&wsl_linux_path_to_windows_path(distro, path))
    })
}

fn original_config(bytes: &[u8]) -> AppResult<String> {
    let value: Value = serde_json::from_slice(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes))
        .map_err(|_| "Invalid Antigravity config JSON")?;
    let root = value
        .as_object()
        .ok_or("Antigravity config must be a JSON object")?;
    let snapshot = match root.get("gateway") {
        Some(gateway) => json!({"gateway":gateway}),
        None => json!({}),
    };
    Ok(snapshot.to_string())
}

pub(super) fn is_inactive(backup: &WslCliBackup) -> bool {
    backup.cli_key == "gemini" && backup.injected_keys.contains_key(INACTIVE)
}

fn known_owned_origin(bytes: &[u8], backup: &WslCliBackup) -> Option<String> {
    // Unrecorded LAN endpoints must not receive credentials from an older baseline.
    let config: Value =
        serde_json::from_slice(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes)).ok()?;
    let origin = config
        .get("gateway")?
        .get("url")?
        .as_str()?
        .strip_suffix("/gemini")?;
    ([GATEWAY, PREVIOUS_ORIGIN]
        .iter()
        .any(|key| backup.injected_keys.get(*key).is_some_and(|o| o == origin))
        || backup
            .injected_keys
            .contains_key(&format!("{KNOWN_ORIGIN}{origin}")))
    .then(|| origin.to_owned())
    .filter(|o| agy_gateway::has_owned_endpoint(bytes, o))
}

fn prepare_changes(
    home: &str,
    previous: Option<&WslCliBackup>,
    origin: Option<&str>,
    mut read: impl FnMut(&str) -> AppResult<Option<Vec<u8>>>,
) -> AppResult<WslAgyPlan> {
    let previous = previous.filter(|b| {
        // After restoring the old user, a subsequent opt-in for a different
        // default user starts with that account's own baseline.
        !(origin.is_some()
            && is_inactive(b)
            && b.injected_keys
                .get(OWNER_HOME)
                .is_some_and(|user| user != home))
    });
    let mut pending = previous.cloned().unwrap_or_else(|| WslCliBackup {
        cli_key: "gemini".into(),
        original_values: Default::default(),
        injected_keys: Default::default(),
    });
    // The distribution's default user can change between configuration and exit.
    // Never restore one account's original gateway credentials into another home.
    let owned_home = pending
        .injected_keys
        .get(OWNER_HOME)
        .map(String::as_str)
        .unwrap_or(home);
    if origin.is_some() && owned_home != home {
        return Err("WSL 默认用户已变更，请先关闭 Antigravity 同步以恢复原用户配置".into());
    }
    let mut edits = Vec::new();
    if has_legacy_env(&pending) {
        let path = format!("{owned_home}/.gemini/.env");
        if let Some(before) = read(&path)? {
            let after = restore_legacy_env(&before, &pending)?;
            if after != before {
                edits.push(ConfigEdit {
                    path,
                    before: Some(before),
                    after: Some(after),
                });
            }
        }
    }

    let mut retained = None;
    if origin.is_some() || pending.original_values.contains_key(GATEWAY) {
        let path = format!("{owned_home}/{CONFIG}");
        let before = read(&path)?;
        let after = if let Some(origin) = origin {
            let next = agy_gateway::apply(before.as_deref(), origin)?;
            // A stale editor can write any earlier AIO endpoint after a port change or disable.
            // Without prior ownership, even an AIO-shaped gateway can be a user's original.
            let cached = before.as_deref().is_some_and(|bytes| {
                known_owned_origin(bytes, &pending).is_some()
                    || agy_gateway::has_aio_gateway(bytes)
                    || agy_gateway::has_owned_endpoint(bytes, origin)
            });
            if !pending.original_values.contains_key(GATEWAY) || !cached {
                let original = before.as_deref().map(original_config).transpose()?;
                pending.original_values.insert(GATEWAY.into(), original);
            }
            // Generic loopback detection handles old ports; journal exact nonlocal origins.
            for key in [GATEWAY, PREVIOUS_ORIGIN] {
                if let Some(old) = pending.injected_keys.remove(key) {
                    if old != origin && !agy_gateway::is_local_gateway_url(&format!("{old}/gemini"))
                    {
                        pending
                            .injected_keys
                            .insert(format!("{KNOWN_ORIGIN}{old}"), String::new());
                    }
                }
            }
            pending.injected_keys.remove(INACTIVE);
            pending.injected_keys.insert(GATEWAY.into(), origin.into());
            pending.injected_keys.insert(OWNER_HOME.into(), home.into());
            Some(next)
        } else {
            let original = pending.original_values[GATEWAY]
                .as_deref()
                .map(str::as_bytes);
            let managed = pending
                .injected_keys
                .get(GATEWAY)
                .ok_or("Missing Antigravity gateway origin in backup")?;
            let effective = before
                .as_deref()
                .and_then(|bytes| known_owned_origin(bytes, &pending));
            if effective.is_none() && !before.as_deref().is_some_and(agy_gateway::has_aio_gateway) {
                if !is_inactive(&pending) {
                    pending.original_values.insert(
                        GATEWAY.into(),
                        before.as_deref().map(original_config).transpose()?,
                    );
                }
                before.clone()
            } else {
                agy_gateway::restore(
                    before.as_deref(),
                    original,
                    effective.as_deref().unwrap_or(managed),
                )?
            }
        };
        let mut next_record = pending.clone();
        for key in ["GOOGLE_GEMINI_BASE_URL", "GEMINI_API_KEY"] {
            next_record.original_values.remove(key);
            next_record.injected_keys.remove(key);
        }
        if origin.is_none() {
            // Keep the baseline for a late editor write, without claiming current ownership.
            next_record
                .injected_keys
                .insert(INACTIVE.into(), String::new());
        }
        retained = Some(next_record);
        if after
            .as_ref()
            .is_some_and(|bytes| bytes.len() > WSL_CLIENT_CONFIG_MAX_BYTES)
        {
            return Err("WSL Antigravity config exceeds the supported size".into());
        }
        if before != after {
            edits.push(ConfigEdit {
                path,
                before,
                after,
            });
        }
    }
    Ok(WslAgyPlan {
        pending,
        retained,
        edits,
    })
}

fn restore_legacy_env(bytes: &[u8], backup: &WslCliBackup) -> AppResult<Vec<u8>> {
    let content = std::str::from_utf8(bytes).map_err(|_| "WSL Gemini .env must be UTF-8")?;
    let owns_base = legacy_env_owned_base(
        content,
        backup
            .injected_keys
            .get("GOOGLE_GEMINI_BASE_URL")
            .map(String::as_str),
        backup
            .injected_keys
            .get("GEMINI_API_KEY")
            .map(String::as_str),
    );
    let mut restored = String::new();
    for line in content.split_inclusive('\n') {
        if let Some((key, value)) = legacy_env_assignment(line) {
            if matches!(key, "GOOGLE_GEMINI_BASE_URL" | "GEMINI_API_KEY")
                && (backup.injected_keys.get(key).is_some_and(|v| v == value)
                    || (key == "GOOGLE_GEMINI_BASE_URL" && owns_base == Some(value)))
            {
                // A manually chosen URL must never receive the previous endpoint's API key.
                if owns_base.is_none() {
                    continue;
                }
                match backup.original_values.get(key) {
                    Some(Some(original)) => {
                        let (prefix, suffix) = line.rsplit_once(value).unwrap();
                        restored.push_str(prefix);
                        restored.push_str(original);
                        restored.push_str(suffix);
                        continue;
                    }
                    Some(None) => continue,
                    None => {}
                }
            }
        }
        restored.push_str(line);
    }
    Ok(restored.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn old_backup() -> WslCliBackup {
        serde_json::from_value(json!({
            "cli_key":"gemini",
            "injected_keys":{"GOOGLE_GEMINI_BASE_URL":"http://old/gemini","GEMINI_API_KEY":"placeholder"},
            "original_values":{"GOOGLE_GEMINI_BASE_URL":null,"GEMINI_API_KEY":"original"}
        })).unwrap()
    }

    #[test]
    fn upgrade_restores_legacy_keys_without_enrolling_the_shared_gateway() {
        let source = b"# keep\r\nGOOGLE_GEMINI_BASE_URL=http://old/gemini\r\nexport GEMINI_API_KEY=placeholder\r\n";
        let plan = prepare_changes("/home/u", Some(&old_backup()), None, |path| {
            assert_eq!(path, "/home/u/.gemini/.env");
            Ok(Some(source.to_vec()))
        })
        .unwrap();
        assert!(plan.retained.is_none());
        assert!(!plan.pending.original_values.contains_key(GATEWAY));
        assert_eq!(plan.edits.len(), 1);
        assert_eq!(
            plan.edits[0].after.as_deref(),
            Some(&b"# keep\r\nexport GEMINI_API_KEY=original\r\n"[..])
        );
        assert_eq!(
            restore_legacy_env(b"GEMINI_API_KEY=user-new", &old_backup()).unwrap(),
            b"GEMINI_API_KEY=user-new"
        );
    }

    #[test]
    fn wsl_legacy_new_destination_never_receives_the_previous_api_key() {
        for (url_lines, retained) in [
            ("", ""),
            ("GOOGLE_GEMINI_BASE_URL=https://new\n", "GOOGLE_GEMINI_BASE_URL=https://new\n"),
            ("GOOGLE_GEMINI_BASE_URL=http://old/gemini\nexport\tGOOGLE_GEMINI_BASE_URL = https://new\n", "export\tGOOGLE_GEMINI_BASE_URL = https://new\n"),
            ("GOOGLE_GEMINI_BASE_URL=http://old/gemini\nGOOGLE_GEMINI_BASE_URL: https://new\n", "GOOGLE_GEMINI_BASE_URL: https://new\n"),
        ] {
            let current = format!("{url_lines}GEMINI_API_KEY=placeholder\nOTHER=keep\n");
            assert_eq!(restore_legacy_env(current.as_bytes(), &old_backup()).unwrap(),
                       format!("{retained}OTHER=keep\n").as_bytes());
        }
    }

    #[test]
    fn wsl_legacy_cached_local_port_restores_the_original_pair() {
        let mut backup = old_backup();
        backup.original_values.insert(
            "GOOGLE_GEMINI_BASE_URL".into(),
            Some("https://prior".into()),
        );
        let current =
            b"GOOGLE_GEMINI_BASE_URL=http://127.0.0.1:37129/gemini\nGEMINI_API_KEY=placeholder\n";
        assert_eq!(
            restore_legacy_env(current, &backup).unwrap(),
            b"GOOGLE_GEMINI_BASE_URL=https://prior\nGEMINI_API_KEY=original\n"
        );
        let changed = b"GOOGLE_GEMINI_BASE_URL=http://192.168.1.90:37129/gemini\nGEMINI_API_KEY=placeholder\n";
        assert_eq!(
            restore_legacy_env(changed, &backup).unwrap(),
            b"GOOGLE_GEMINI_BASE_URL=http://192.168.1.90:37129/gemini\n"
        );
    }

    #[test]
    fn wsl_legacy_home_survives_other_cli_rewriting_the_distro_home() {
        let manifest: WslDistroManifest = serde_json::from_value(json!({
            "schema_version":1, "distro":"Ubuntu", "configured":true, "proxy_origin":"http://old",
            "configured_at":1, "wsl_home_unc":r"\\wsl$\Ubuntu\home\old", "cli_backups":[old_backup()]
        })).unwrap();
        let mut pinned = pin_legacy_home("Ubuntu", manifest).unwrap();
        pinned.wsl_home_unc = Some(r"\\wsl$\Ubuntu\home\new".into());
        let persisted = pin_legacy_home("Ubuntu", pinned).unwrap();
        let backup = &persisted.cli_backups[0];
        let from_new_home = |origin| {
            prepare_changes("/home/new", Some(backup), origin, |path| {
                assert_eq!(path, "/home/old/.gemini/.env");
                Ok(Some(
                    b"GOOGLE_GEMINI_BASE_URL=http://old/gemini\nGEMINI_API_KEY=placeholder\n"
                        .to_vec(),
                ))
            })
        };
        assert!(from_new_home(Some("http://new-port")).is_err());
        let disable = from_new_home(None).unwrap();
        assert_eq!(disable.edits[0].path, "/home/old/.gemini/.env");
        assert_eq!(
            disable.edits[0].after.as_deref(),
            Some(&b"GEMINI_API_KEY=original\n"[..])
        );
    }

    #[cfg(unix)]
    mod linux_io {
        use super::*;
        use std::{
            fs,
            os::unix::fs::{symlink, PermissionsExt},
            process::Command,
        };

        fn write_script(script: &str) -> AppResult<()> {
            // WSL coreutils provides sha256sum; older macOS test runners ship shasum.
            let script = format!(
                "command -v sha256sum >/dev/null || sha256sum() {{ shasum -a 256; }}\n{script}"
            );
            let out = Command::new("bash").arg("-c").arg(script).output().unwrap();
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).into_owned().into())
            }
        }

        fn plan(
            dir: &Path,
            backup: Option<&WslCliBackup>,
            origin: Option<&str>,
        ) -> AppResult<WslAgyPlan> {
            prepare_changes(dir.to_str().unwrap(), backup, origin, |path| {
                read_config(Path::new(path))
            })
        }

        #[test]
        fn wsl_agy_gateway_preserves_shared_settings_and_original_gateway_across_rebind() {
            let temp = tempfile::tempdir().unwrap();
            let file = temp.path().join(CONFIG);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            let original = json!({"gateway":{"url":"https://prior.example","apiKey":"prior","enabled":false,"extension":"keep","headers":{"X-Auth":"prior-token"}},"plugins":{"x":1},"userSettings":{"mode":"old"}});
            fs::write(&file, original.to_string()).unwrap();
            fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
            let origin1 = "http://127.0.0.1:10001";
            let origin2 = "http://127.0.0.1:10002";
            let first = plan(temp.path(), None, Some(origin1)).unwrap();
            let saved = first.retained.as_ref().unwrap().original_values[GATEWAY]
                .as_ref()
                .unwrap();
            assert!(!saved.contains("plugins") && !saved.contains("userSettings"));
            first.apply_with(write_script).unwrap();
            assert!(agy_gateway::is_applied(&fs::read(&file).unwrap(), origin1));
            assert_eq!(
                fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600
            );

            let mut current: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
            assert_eq!(current["gateway"]["extension"], "keep");
            assert!(current["gateway"].get("headers").is_none());
            current["userSettings"]["mode"] = json!("new");
            current["gateway"]["newOption"] = json!(true);
            // Toggling the gateway in the editor leaves our URL and placeholder in place.
            current["gateway"]["enabled"] = json!(false);
            fs::write(&file, current.to_string()).unwrap();
            let second = plan(temp.path(), first.retained.as_ref(), Some(origin2)).unwrap();
            // A crash before committing a port change still leaves a restorable old URL.
            let interrupted = plan(temp.path(), Some(&second.pending), None).unwrap();
            let restored: Value =
                serde_json::from_slice(interrupted.edits[0].after.as_ref().unwrap()).unwrap();
            assert_eq!(restored["gateway"]["apiKey"], "prior");
            second.apply_with(write_script).unwrap();
            assert!(agy_gateway::is_applied(&fs::read(&file).unwrap(), origin2));
            plan(temp.path(), second.retained.as_ref(), None)
                .unwrap()
                .apply_with(write_script)
                .unwrap();
            let restored: Value = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
            let mut expected = original["gateway"].clone();
            expected["newOption"] = json!(true);
            assert_eq!(restored["gateway"], expected);
            assert_eq!(restored["plugins"], original["plugins"]);
            assert_eq!(restored["userSettings"]["mode"], "new");
        }

        #[test]
        fn wsl_agy_gateway_removes_only_empty_owned_config_and_preserves_other_data() {
            let temp = tempfile::tempdir().unwrap();
            let file = temp.path().join(CONFIG);
            for add_plugin in [false, true] {
                let enable = plan(temp.path(), None, Some("http://127.0.0.1:42")).unwrap();
                enable.apply_with(write_script).unwrap();
                if add_plugin {
                    let mut value: Value =
                        serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
                    value["plugins"] = json!({"keep":true});
                    fs::write(&file, value.to_string()).unwrap();
                }
                plan(temp.path(), enable.retained.as_ref(), None)
                    .unwrap()
                    .apply_with(write_script)
                    .unwrap();
                if add_plugin {
                    assert_eq!(
                        serde_json::from_slice::<Value>(&fs::read(&file).unwrap()).unwrap(),
                        json!({"plugins":{"keep":true}})
                    );
                } else {
                    assert!(!file.exists());
                }
            }
        }

        #[test]
        fn wsl_sync_after_manual_gateway_change_restores_the_new_user_choice() {
            let temp = tempfile::tempdir().unwrap();
            let file = temp.path().join(CONFIG);
            let first = plan(temp.path(), None, Some("http://local:1")).unwrap();
            first.apply_with(write_script).unwrap();
            let manual = json!({"gateway":{"url":"https://new.example","apiKey":"new-key","headers":{"X-Auth":"new"}}});
            fs::write(&file, manual.to_string()).unwrap();
            let release = plan(temp.path(), first.retained.as_ref(), None).unwrap();
            assert!(release.edits.is_empty());
            let sync = plan(temp.path(), first.retained.as_ref(), Some("http://local:2")).unwrap();
            sync.apply_with(write_script).unwrap();
            plan(temp.path(), sync.retained.as_ref(), None)
                .unwrap()
                .apply_with(write_script)
                .unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&fs::read(file).unwrap()).unwrap(),
                manual
            );
        }

        #[test]
        fn wsl_disabled_journal_restores_original_after_cached_ports_and_reenable() {
            let temp = tempfile::tempdir().unwrap();
            let file = temp.path().join(CONFIG);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            let original =
                json!({"gateway":{"url":"https://prior","apiKey":"original"},"plugins":["keep"]});
            fs::write(
                &file,
                [b"\xef\xbb\xbf".as_slice(), original.to_string().as_bytes()].concat(),
            )
            .unwrap();
            let mut journal = None;
            let ports = [
                "http://172.20.1.2:37123",
                "http://172.20.1.2:37124",
                "http://172.20.1.2:37125",
            ];
            for origin in ports {
                let p = plan(temp.path(), journal.as_ref(), Some(origin)).unwrap();
                p.apply_with(write_script).unwrap();
                journal = p.retained;
            }
            let off = plan(temp.path(), journal.as_ref(), None).unwrap();
            off.apply_with(write_script).unwrap();
            journal = off.retained;
            assert!(is_inactive(journal.as_ref().unwrap()));
            for cached in [ports[0], "http://127.0.0.1:29123"] {
                let stale =
                    agy_gateway::apply(Some(original.to_string().as_bytes()), cached).unwrap();
                fs::write(&file, stale).unwrap();
                let on = plan(temp.path(), journal.as_ref(), Some(ports[2])).unwrap();
                on.apply_with(write_script).unwrap();
                let off = plan(temp.path(), on.retained.as_ref(), None).unwrap();
                off.apply_with(write_script).unwrap();
                journal = off.retained;
                assert_eq!(
                    serde_json::from_slice::<Value>(&fs::read(&file).unwrap()).unwrap(),
                    original
                );
            }
            // An unknown LAN destination plus placeholder is not a recorded WSL gateway.
            let manual = agy_gateway::apply(None, "http://192.168.8.9:37123").unwrap();
            fs::write(&file, &manual).unwrap();
            let off = plan(temp.path(), journal.as_ref(), None).unwrap();
            assert!(off.edits.is_empty());
            assert_eq!(off.retained.as_ref(), journal.as_ref());
            assert_eq!(fs::read(&file).unwrap(), manual);
        }

        #[test]
        fn wsl_first_enrollment_preserves_an_existing_aio_shaped_gateway() {
            let temp = tempfile::tempdir().unwrap();
            let file = temp.path().join(CONFIG);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            let original = agy_gateway::apply(None, "http://127.0.0.1:3111").unwrap();
            fs::write(&file, &original).unwrap();
            let on = plan(temp.path(), None, Some("http://127.0.0.1:3222")).unwrap();
            on.apply_with(write_script).unwrap();
            let off = plan(temp.path(), on.retained.as_ref(), None).unwrap();
            off.apply_with(write_script).unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&fs::read(file).unwrap()).unwrap(),
                serde_json::from_slice::<Value>(&original).unwrap()
            );
        }

        #[test]
        fn wsl_default_user_change_restores_only_the_original_home() {
            let first = tempfile::tempdir().unwrap();
            let second = tempfile::tempdir().unwrap();
            let old = plan(first.path(), None, Some("http://127.0.0.1:1")).unwrap();
            old.apply_with(write_script).unwrap();
            assert!(plan(
                second.path(),
                old.retained.as_ref(),
                Some("http://127.0.0.1:1")
            )
            .is_err());
            let second_config = second.path().join(CONFIG);
            fs::create_dir_all(second_config.parent().unwrap()).unwrap();
            fs::write(&second_config, b"second user's settings").unwrap();
            let off = plan(second.path(), old.retained.as_ref(), None).unwrap();
            off.apply_with(write_script).unwrap();
            assert!(!first.path().join(CONFIG).exists());
            assert_eq!(fs::read(second_config).unwrap(), b"second user's settings");
            let user_choice = br#"{"gateway":{"url":"https://manual","apiKey":"new-private-key"}}"#;
            fs::write(first.path().join(CONFIG), user_choice).unwrap();
            let startup = prepare_changes(
                second.path().to_str().unwrap(),
                off.retained.as_ref(),
                None,
                |path| {
                    assert_eq!(Path::new(path), first.path().join(CONFIG));
                    read_config(Path::new(path))
                },
            )
            .unwrap();
            assert!(startup.edits.is_empty());
            assert_eq!(startup.retained, off.retained);
        }

        #[test]
        fn wsl_agy_failed_config_edit_rolls_back_legacy_env_and_respects_symlinks() {
            let temp = tempfile::tempdir().unwrap();
            let file = temp.path().join(CONFIG);
            let env = temp.path().join(".gemini/.env");
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            let old = b"GOOGLE_GEMINI_BASE_URL=http://old/gemini\nGEMINI_API_KEY=placeholder\n";
            fs::write(&env, old).unwrap();
            fs::write(&file, b"not json").unwrap();
            assert!(plan(temp.path(), Some(&old_backup()), Some("http://127.0.0.1:1")).is_err());
            assert_eq!(fs::read(&env).unwrap(), old);
            fs::remove_file(&file).unwrap();
            let enable =
                plan(temp.path(), Some(&old_backup()), Some("http://127.0.0.1:1")).unwrap();
            symlink(&env, &file).unwrap();
            assert!(enable.apply_with(write_script).is_err());
            assert!(fs::symlink_metadata(&file)
                .unwrap()
                .file_type()
                .is_symlink());
            assert_eq!(fs::read(&env).unwrap(), old);
        }
    }
}
