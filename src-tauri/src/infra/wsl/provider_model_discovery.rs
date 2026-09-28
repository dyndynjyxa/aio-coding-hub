//! Read-only Codex discovery identity selection from applied WSL configuration.

use super::constants::{
    WSL_CODEX_API_KEY, WSL_CODEX_PREFERRED_AUTH_METHOD, WSL_CODEX_PROVIDER_KEY,
};
use super::manifest::{
    WSL_CLIENT_CONFIG_MAX_BYTES, WSL_MANIFEST_FILE_COUNT_MAX, WSL_MANIFEST_MAX_BYTES,
};
use super::types::WslDistroManifest;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const VERSION_SCRIPT: &str = "set -euo pipefail\nHOME=\"$(getent passwd \"$(whoami)\" | cut -d: -f6)\"\nexport HOME\n[ -n \"$HOME\" ]\nexec codex --version";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CodexVersionSource {
    Native,
    Wsl(String),
    Fallback,
}

#[cfg(windows)]
pub(crate) fn version_source<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    deadline: Instant,
) -> CodexVersionSource {
    let manifests = super::manifest::wsl_manifests_dir(app)
        .map_err(|_| ())
        .and_then(|dir| read_manifests(&dir, deadline));
    select_version_source(manifests, deadline, |distro, script, limit, remaining| {
        let command = probe_command(distro, script, remaining)?;
        crate::cli_manager::run_discovery_command(command, remaining, limit).map_err(|_| ())
    })
}

// Unlike restore's best-effort reader, discovery cannot select from a partial scan.
fn read_manifests(dir: &Path, deadline: Instant) -> Result<Vec<WslDistroManifest>, ()> {
    if Instant::now() >= deadline {
        return Err(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(()),
    };
    let mut manifests = Vec::new();
    for entry in entries {
        if Instant::now() >= deadline {
            return Err(());
        }
        let path = entry.map_err(|_| ())?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        if manifests.len() >= WSL_MANIFEST_FILE_COUNT_MAX {
            return Err(());
        }
        let bytes = crate::shared::fs::read_file_with_max_len(&path, WSL_MANIFEST_MAX_BYTES)
            .map_err(|_| ())?;
        let manifest: WslDistroManifest = serde_json::from_slice(&bytes).map_err(|_| ())?;
        if path.file_stem().and_then(|value| value.to_str()) != Some(&manifest.distro) {
            return Err(());
        }
        manifests.push(manifest);
    }
    manifests.sort_by(|left, right| left.distro.cmp(&right.distro));
    Ok(manifests)
}

fn is_candidate(manifest: &WslDistroManifest) -> Result<bool, ()> {
    if manifest.schema_version != 1 {
        return Err(());
    }
    if !manifest.configured {
        return Ok(false);
    }
    let mut backups = manifest
        .cli_backups
        .iter()
        .filter(|backup| backup.cli_key == "codex");
    let Some(backup) = backups.next() else {
        return Ok(false);
    };
    if backups.next().is_some() || manifest.configured_at <= 0 {
        return Err(());
    }
    validate_distro(&manifest.distro)?;
    let origin = reqwest::Url::parse(&manifest.proxy_origin).map_err(|_| ())?;
    if !matches!(origin.scheme(), "http" | "https")
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
        || origin.port() == Some(0)
    {
        return Err(());
    }
    for (key, expected) in [
        ("preferred_auth_method", WSL_CODEX_PREFERRED_AUTH_METHOD),
        ("model_provider", WSL_CODEX_PROVIDER_KEY),
        ("OPENAI_API_KEY", WSL_CODEX_API_KEY),
    ] {
        if backup.injected_keys.get(key).map(String::as_str) != Some(expected) {
            return Err(());
        }
    }
    Ok(true)
}

fn validate_distro(distro: &str) -> Result<(), ()> {
    if distro.trim().is_empty()
        || distro.trim() != distro
        || distro.starts_with('-')
        || distro.chars().count() > super::detection::WSL_DISTRO_MAX_CHARS
        || distro
            .chars()
            .any(|value| value.is_control() || matches!(value, '/' | '\\'))
    {
        Err(())
    } else {
        Ok(())
    }
}

// Verify the root config written by AIO, not arbitrary Codex profile/CLI overrides.
fn root_config_is_applied(config: &str, origin: &str) -> Result<bool, ()> {
    let root: toml::Value = toml::from_str(config).map_err(|_| ())?;
    for (key, expected) in [
        ("model_provider", WSL_CODEX_PROVIDER_KEY),
        ("preferred_auth_method", WSL_CODEX_PREFERRED_AUTH_METHOD),
    ] {
        let Some(value) = root.get(key) else {
            return Ok(false);
        };
        if value.as_str().ok_or(())? != expected {
            return Ok(false);
        }
    }
    let Some(providers) = root.get("model_providers") else {
        return Ok(false);
    };
    let Some(provider) = providers.as_table().ok_or(())?.get(WSL_CODEX_PROVIDER_KEY) else {
        return Ok(false);
    };
    let Some(base_url) = provider.as_table().ok_or(())?.get("base_url") else {
        return Ok(false);
    };
    Ok(base_url.as_str().ok_or(())? == format!("{origin}/v1"))
}

fn config_script() -> String {
    format!(
        "set -euo pipefail\nHOME=\"$(getent passwd \"$(whoami)\" | cut -d: -f6)\"\nexport HOME\n[ -n \"$HOME\" ]\n{}\n[ -d \"$codex_home\" ] && [ -x \"$codex_home\" ] || exit 1\nconfig=\"$codex_home/config.toml\"\nif [ -L \"$config\" ] && [ ! -e \"$config\" ]; then exit 1; fi\nif [ -e \"$config\" ]; then\n  [ -f \"$config\" ] && [ -r \"$config\" ] || exit 1\n  head -c {} -- \"$config\"\nfi\n",
        super::shell::wsl_resolve_codex_home_script("codex_home"),
        WSL_CLIENT_CONFIG_MAX_BYTES + 1,
    )
}

fn probe_command(distro: &str, script: &str, remaining: Duration) -> Result<Command, ()> {
    validate_distro(distro)?;
    let inner = remaining.saturating_sub(Duration::from_millis(500));
    if inner.as_millis() == 0 {
        return Err(());
    }
    let mut command = super::shell::hide_window_cmd("wsl.exe");
    // Linux timeout bounds the Linux process group even if the Windows wsl.exe
    // wrapper exits/is killed. Missing coreutils timeout fails closed; never install it.
    command.args([
        "--distribution",
        distro,
        "--exec",
        "timeout",
        "--kill-after=0.2s",
    ]);
    command.arg(format!("{}.{:03}s", inner.as_secs(), inner.subsec_millis()));
    command.args(["bash", "-lc", script]);
    Ok(command)
}

fn select_version_source(
    manifests: Result<Vec<WslDistroManifest>, ()>,
    deadline: Instant,
    mut probe: impl FnMut(&str, &str, usize, Duration) -> Result<String, ()>,
) -> CodexVersionSource {
    let Ok(manifests) = manifests else {
        return CodexVersionSource::Fallback;
    };
    let mut applied = Vec::new();
    for manifest in &manifests {
        match is_candidate(manifest) {
            Ok(false) => continue,
            Err(()) => return CodexVersionSource::Fallback,
            Ok(true) => {}
        }
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_secs(5));
        if remaining <= Duration::from_millis(500) {
            return CodexVersionSource::Fallback;
        }
        let config = match probe(
            &manifest.distro,
            &config_script(),
            WSL_CLIENT_CONFIG_MAX_BYTES,
            remaining,
        ) {
            Ok(config) => config,
            Err(()) => return CodexVersionSource::Fallback,
        };
        match root_config_is_applied(&config, &manifest.proxy_origin) {
            Ok(true) => applied.push(manifest.distro.as_str()),
            Ok(false) => {}
            Err(()) => return CodexVersionSource::Fallback,
        }
    }
    match applied.as_slice() {
        [] => CodexVersionSource::Native,
        [distro] => {
            let remaining = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(5));
            if remaining <= Duration::from_millis(500) {
                return CodexVersionSource::Fallback;
            }
            match probe(distro, VERSION_SCRIPT, 16 * 1024, remaining) {
                Ok(version) => CodexVersionSource::Wsl(version),
                Err(()) => CodexVersionSource::Fallback,
            }
        }
        _ => CodexVersionSource::Fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture_manifests(dir: &Path) -> Result<Vec<WslDistroManifest>, ()> {
        read_manifests(dir, Instant::now() + Duration::from_secs(5))
    }

    fn manifest(distro: &str) -> WslDistroManifest {
        serde_json::from_value(serde_json::json!({
            "schema_version": 1, "distro": distro, "configured": true,
            "configured_at": 123, "proxy_origin": "http://172.20.0.1:12345",
            "cli_backups": [{"cli_key": "codex", "original_values": {}, "injected_keys": {
                "preferred_auth_method": "apikey", "model_provider": "aio", "OPENAI_API_KEY": "aio-coding-hub"
            }}]
        })).unwrap()
    }

    fn applied_config() -> &'static str {
        "model_provider = 'aio'\npreferred_auth_method = 'apikey'\n[model_providers.aio]\nbase_url = 'http://172.20.0.1:12345/v1'\n"
    }

    fn select(manifests: Vec<WslDistroManifest>, config: &str) -> CodexVersionSource {
        select_version_source(
            Ok(manifests),
            Instant::now() + Duration::from_secs(1),
            |_, script, _, _| {
                Ok(if script == VERSION_SCRIPT {
                    "codex-cli 0.150.2".to_string()
                } else {
                    config.to_string()
                })
            },
        )
    }

    #[test]
    fn manual_applied_codex_selects_wsl_without_auto_config_flag() {
        // Selection takes applied state, not settings: manual apply is valid with auto=false.
        assert_eq!(
            select(vec![manifest("Ubuntu")], applied_config()),
            CodexVersionSource::Wsl("codex-cli 0.150.2".to_string())
        );
        assert_eq!(
            crate::gateway::oauth::adapters::codex::codex_model_discovery_version(Some(
                "codex-cli 0.150.2"
            )),
            "0.150.2"
        );
    }

    #[test]
    fn missing_failed_restored_stale_or_non_codex_state_does_not_activate_wsl() {
        assert_eq!(
            select(Vec::new(), applied_config()),
            CodexVersionSource::Native
        );
        let mut failed = manifest("Ubuntu");
        failed.configured = false;
        assert_eq!(
            select(vec![failed], applied_config()),
            CodexVersionSource::Native
        );
        let mut non_codex = manifest("Ubuntu");
        non_codex.cli_backups[0].cli_key = "claude".to_string();
        assert_eq!(
            select(vec![non_codex], applied_config()),
            CodexVersionSource::Native
        );
        for config in [
            "",
            "model_provider = 'other'",
            "[profiles.aio]\nmodel_provider = 'aio'",
        ] {
            assert_eq!(
                select(vec![manifest("Ubuntu")], config),
                CodexVersionSource::Native
            );
        }
        let stale = applied_config().replace("12345", "54321");
        assert_eq!(
            select(vec![manifest("Ubuntu")], &stale),
            CodexVersionSource::Native
        );
    }

    #[test]
    fn malformed_or_unknown_state_uses_fallback_not_native_or_partial_selection() {
        let mut malformed = manifest("Ubuntu");
        malformed.cli_backups[0]
            .injected_keys
            .remove("model_provider");
        assert_eq!(
            select(vec![malformed], applied_config()),
            CodexVersionSource::Fallback
        );
        for malformed_config in [
            "not toml",
            "model_provider = 42",
            "model_provider = 'aio'\npreferred_auth_method = 42",
        ] {
            assert_eq!(
                select(vec![manifest("Ubuntu")], malformed_config),
                CodexVersionSource::Fallback
            );
        }
        let deadline = Instant::now() + Duration::from_secs(1);
        assert_eq!(
            select_version_source(Err(()), deadline, |_, _, _, _| panic!("must not probe")),
            CodexVersionSource::Fallback
        );
        let mut calls = 0;
        let result = select_version_source(
            Ok(vec![manifest("Ubuntu"), manifest("Debian")]),
            deadline,
            |distro, script, _, _| {
                calls += 1;
                assert_ne!(script, VERSION_SCRIPT);
                if distro == "Ubuntu" {
                    Ok(applied_config().to_string())
                } else {
                    Err(())
                }
            },
        );
        assert_eq!(result, CodexVersionSource::Fallback);
        assert_eq!(calls, 2);
        assert_eq!(
            select_version_source(Ok(vec![manifest("Ubuntu")]), deadline, |_, script, _, _| {
                if script == VERSION_SCRIPT {
                    Err(())
                } else {
                    Ok(applied_config().to_string())
                }
            }),
            CodexVersionSource::Fallback
        );
    }

    #[test]
    fn multiple_applied_distros_never_choose_first_or_default() {
        let deadline = Instant::now() + Duration::from_secs(1);
        for distros in [["Ubuntu", "Debian"], ["Debian", "Ubuntu"]] {
            let mut calls = 0;
            assert_eq!(
                select_version_source(
                    Ok(distros.into_iter().map(manifest).collect()),
                    deadline,
                    |_, script, _, _| {
                        calls += 1;
                        assert_ne!(script, VERSION_SCRIPT);
                        Ok(applied_config().to_string())
                    }
                ),
                CodexVersionSource::Fallback
            );
            assert_eq!(calls, 2);
        }
        assert_eq!(
            select_version_source(
                Ok(vec![manifest("Ubuntu"), manifest("Debian")]),
                deadline,
                |distro, script, _, _| {
                    if distro == "Debian" {
                        Ok(String::new())
                    } else if script == VERSION_SCRIPT {
                        Ok("codex-cli 0.150.2".to_string())
                    } else {
                        Ok(applied_config().to_string())
                    }
                }
            ),
            CodexVersionSource::Wsl("codex-cli 0.150.2".to_string())
        );
    }

    #[test]
    fn expired_aggregate_deadline_stops_launching_probes() {
        let deadline = Instant::now();
        assert_eq!(
            select_version_source(Ok(vec![manifest("Ubuntu")]), deadline, |_, _, _, _| panic!(
                "expired probe"
            )),
            CodexVersionSource::Fallback
        );
        let mut calls = 0;
        let deadline = Instant::now() + Duration::from_millis(510);
        assert_eq!(
            select_version_source(
                Ok(vec![manifest("Ubuntu"), manifest("Debian")]),
                deadline,
                |_, _, _, remaining| {
                    calls += 1;
                    assert!(remaining <= Duration::from_millis(510));
                    std::thread::sleep(Duration::from_millis(20));
                    Ok(applied_config().to_string())
                }
            ),
            CodexVersionSource::Fallback
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn manifest_scan_fails_closed_for_malformed_oversized_or_excess_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_fixture_manifests(&dir.path().join("missing"))
            .unwrap()
            .is_empty());
        let path = dir.path().join("Ubuntu.json");
        std::fs::write(&path, serde_json::to_vec(&manifest("Ubuntu")).unwrap()).unwrap();
        assert_eq!(read_fixture_manifests(dir.path()).unwrap().len(), 1);
        std::fs::write(&path, b"not json").unwrap();
        assert!(read_fixture_manifests(dir.path()).is_err());
        std::fs::write(&path, vec![b' '; WSL_MANIFEST_MAX_BYTES + 1]).unwrap();
        assert!(read_fixture_manifests(dir.path()).is_err());
        std::fs::remove_file(path).unwrap();
        for i in 0..=WSL_MANIFEST_FILE_COUNT_MAX {
            let name = format!("Distro-{i}");
            std::fs::write(
                dir.path().join(format!("{name}.json")),
                serde_json::to_vec(&manifest(&name)).unwrap(),
            )
            .unwrap();
        }
        assert!(read_fixture_manifests(dir.path()).is_err());
    }

    #[test]
    fn probe_arguments_keep_distro_and_fixed_script_separate_and_bounded() {
        let distro = "Ubuntu '; echo injected";
        let script = config_script();
        let command = probe_command(distro, &script, Duration::from_secs(5)).unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|value| value.to_str().unwrap())
            .collect();
        assert_eq!(command.get_program(), "wsl.exe");
        assert_eq!(
            args,
            [
                "--distribution",
                distro,
                "--exec",
                "timeout",
                "--kill-after=0.2s",
                "4.500s",
                "bash",
                "-lc",
                &script
            ]
        );
        assert!(!script.contains(distro));
        assert!(script.contains("head -c 1048577"));
        for bad in ["", "--help", "Ubuntu\n", "../Ubuntu", "a\\b"] {
            assert!(probe_command(bad, &script, Duration::from_secs(5)).is_err());
        }
        assert!(probe_command("Ubuntu", &script, Duration::from_millis(100)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn discovery_command_reuses_timeout_and_rejects_oversized_output() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 10"]);
        let start = Instant::now();
        assert!(
            crate::cli_manager::run_discovery_command(command, Duration::from_millis(50), 32)
                .is_err()
        );
        assert!(start.elapsed() < Duration::from_secs(3));
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "printf '12345'"]);
        assert!(
            crate::cli_manager::run_discovery_command(command, Duration::from_secs(1), 4).is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn read_only_script_distinguishes_missing_file_from_unresolved_home() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("fixture home");
        let codex_home = home.join(".codex");
        std::fs::create_dir_all(&codex_home).unwrap();
        let script = format!(
            "getent() {{ printf 'fixture:x:1:1::%s:/bin/bash\\n' {}; }}\n{}",
            super::super::shell::bash_single_quote(home.to_str().unwrap()),
            config_script(),
        );
        let run = || {
            let mut command = Command::new("/bin/bash");
            command.args(["--noprofile", "--norc", "-c", &script]);
            command
                .env_remove("BASH_ENV")
                .env("CODEX_HOME", &codex_home);
            crate::cli_manager::run_discovery_command(
                command,
                Duration::from_secs(2),
                WSL_CLIENT_CONFIG_MAX_BYTES,
            )
        };
        assert_eq!(run().unwrap(), "");
        std::fs::write(codex_home.join("config.toml"), applied_config()).unwrap();
        assert_eq!(run().unwrap(), applied_config());
        std::fs::write(
            codex_home.join("config.toml"),
            vec![b'x'; WSL_CLIENT_CONFIG_MAX_BYTES + 1],
        )
        .unwrap();
        assert!(run().is_err());
        std::fs::remove_dir_all(&codex_home).unwrap();
        assert!(run().is_err());
    }

    #[cfg(windows)]
    #[test]
    fn discovery_command_reads_spaced_cmd_fixture_and_applied_crlf_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake wsl probe.cmd");
        let body = concat!(
            "@echo off\r\n",
            "echo model_provider = 'aio'\r\n",
            "echo preferred_auth_method = 'apikey'\r\n",
            "echo [model_providers.aio]\r\n",
            "echo base_url = 'http://172.20.0.1:12345/v1'\r\n"
        );
        std::fs::write(&path, body).unwrap();
        let output = crate::cli_manager::run_discovery_command(
            Command::new(path),
            Duration::from_secs(2),
            WSL_CLIENT_CONFIG_MAX_BYTES,
        )
        .unwrap();
        assert!(output.contains("\r\n"));
        assert!(root_config_is_applied(&output, "http://172.20.0.1:12345").unwrap());
    }
}
