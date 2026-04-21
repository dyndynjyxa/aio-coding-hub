#[cfg(test)]
mod tests {
    use crate::mcp_sync::McpServerForSync;
    use crate::wsl::data_gathering::gather_skills_sync_data;
    use crate::wsl::mcp_adapt::adapt_mcp_servers_for_wsl;
    use crate::wsl::types::{WslCliBackup, WslDistroManifest};
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::path::PathBuf;

    struct ScopedEnvVar {
        key: &'static str,
        value: Option<OsString>,
    }

    impl ScopedEnvVar {
        fn set(key: &'static str, value: impl Into<OsString>) -> Self {
            let prev_value = std::env::var_os(key);
            std::env::set_var(key, value.into());
            Self {
                key,
                value: prev_value,
            }
        }
    }

    impl Drop for ScopedEnvVar {
        fn drop(&mut self) {
            match self.value.take() {
                Some(prev) => std::env::set_var(self.key, prev),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn make_server(
        command: Option<&str>,
        args: Vec<&str>,
        cwd: Option<&str>,
        env: Vec<(&str, &str)>,
    ) -> McpServerForSync {
        McpServerForSync {
            server_key: "test".to_string(),
            transport: "stdio".to_string(),
            command: command.map(|s| s.to_string()),
            args: args.into_iter().map(|s| s.to_string()).collect(),
            env: env
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            cwd: cwd.map(|s| s.to_string()),
            url: None,
            headers: BTreeMap::new(),
        }
    }

    #[test]
    fn test_win_path_to_wsl_mount_drive_letter() {
        assert_eq!(
            crate::wsl::mcp_adapt::win_path_to_wsl_mount(r"C:\Users\foo\bar"),
            "/mnt/c/Users/foo/bar"
        );
        assert_eq!(
            crate::wsl::mcp_adapt::win_path_to_wsl_mount(r"D:\tools\cli"),
            "/mnt/d/tools/cli"
        );
    }

    #[test]
    fn test_win_path_to_wsl_mount_non_absolute() {
        assert_eq!(
            crate::wsl::mcp_adapt::win_path_to_wsl_mount(r".\relative\path"),
            "./relative/path"
        );
    }

    #[test]
    fn test_wsl_linux_path_to_windows_path_for_mount_drive() {
        assert_eq!(
            crate::wsl::detection::wsl_linux_path_to_windows_path(
                "Ubuntu",
                "/mnt/c/Users/test/.codex"
            ),
            PathBuf::from(r"C:\Users\test\.codex")
        );
    }

    #[test]
    fn test_wsl_linux_path_to_windows_path_for_unc_path() {
        assert_eq!(
            crate::wsl::detection::wsl_linux_path_to_windows_path("Ubuntu", "/home/test/.codex"),
            PathBuf::from(r"\\wsl$\Ubuntu\home\test\.codex")
        );
    }

    #[test]
    fn test_strip_win_exe_ext() {
        assert_eq!(crate::wsl::mcp_adapt::strip_win_exe_ext("npx.cmd"), "npx");
        assert_eq!(
            crate::wsl::mcp_adapt::strip_win_exe_ext("server.exe"),
            "server"
        );
        assert_eq!(crate::wsl::mcp_adapt::strip_win_exe_ext("run.bat"), "run");
        assert_eq!(crate::wsl::mcp_adapt::strip_win_exe_ext("npx"), "npx");
    }

    #[test]
    fn test_adapt_converts_args_with_windows_paths() {
        let servers = vec![make_server(
            Some("npx.cmd"),
            vec!["-y", "@mcp/server-fs", r"C:\Users\diao\Documents"],
            Some(r"C:\Users\diao\project"),
            vec![],
        )];

        let adapted = adapt_mcp_servers_for_wsl(&servers);

        assert_eq!(adapted[0].command.as_deref(), Some("npx"));
        assert_eq!(adapted[0].args[0], "-y");
        assert_eq!(adapted[0].args[1], "@mcp/server-fs");
        assert_eq!(adapted[0].args[2], "/mnt/c/Users/diao/Documents");
        assert_eq!(adapted[0].cwd.as_deref(), Some("/mnt/c/Users/diao/project"));
    }

    #[test]
    fn test_adapt_converts_env_with_windows_paths() {
        let servers = vec![make_server(
            Some("node"),
            vec![],
            None,
            vec![
                ("NODE_PATH", r"C:\Users\diao\node_modules"),
                ("API_KEY", "sk-abc123"),
            ],
        )];

        let adapted = adapt_mcp_servers_for_wsl(&servers);

        assert_eq!(
            adapted[0].env.get("NODE_PATH").unwrap(),
            "/mnt/c/Users/diao/node_modules"
        );
        // Non-path values should remain unchanged
        assert_eq!(adapted[0].env.get("API_KEY").unwrap(), "sk-abc123");
    }

    #[test]
    fn test_adapt_leaves_non_windows_args_unchanged() {
        let servers = vec![make_server(
            Some("node"),
            vec!["--port", "3000", "/home/user/script.js"],
            None,
            vec![],
        )];

        let adapted = adapt_mcp_servers_for_wsl(&servers);

        assert_eq!(
            adapted[0].args,
            vec!["--port", "3000", "/home/user/script.js"]
        );
    }

    #[test]
    fn test_adapt_command_windows_absolute_path_uses_basename() {
        let servers = vec![make_server(
            Some(r"C:\Program Files\tool\server.exe"),
            vec![],
            None,
            vec![],
        )];

        let adapted = adapt_mcp_servers_for_wsl(&servers);

        assert_eq!(adapted[0].command.as_deref(), Some("server"));
    }

    #[test]
    fn test_wsl_manifest_roundtrip() {
        let manifest = WslDistroManifest {
            schema_version: 1,
            distro: "Ubuntu".to_string(),
            configured: true,
            proxy_origin: "http://172.20.0.1:12345".to_string(),
            configured_at: 1700000000,
            wsl_home_unc: Some(r"\\wsl$\Ubuntu\home\testuser".to_string()),
            cli_backups: vec![
                WslCliBackup {
                    cli_key: "claude".to_string(),
                    injected_keys: [
                        (
                            "ANTHROPIC_BASE_URL".to_string(),
                            "http://172.20.0.1:12345/claude".to_string(),
                        ),
                        (
                            "ANTHROPIC_AUTH_TOKEN".to_string(),
                            "aio-coding-hub".to_string(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    original_values: [
                        (
                            "ANTHROPIC_BASE_URL".to_string(),
                            Some("https://api.anthropic.com".to_string()),
                        ),
                        (
                            "ANTHROPIC_AUTH_TOKEN".to_string(),
                            Some("sk-old-token".to_string()),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
                WslCliBackup {
                    cli_key: "gemini".to_string(),
                    injected_keys: [("GEMINI_API_KEY".to_string(), "aio-coding-hub".to_string())]
                        .into_iter()
                        .collect(),
                    original_values: [("GEMINI_API_KEY".to_string(), Some("old-key".to_string()))]
                        .into_iter()
                        .collect(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&manifest).expect("serialize");
        let deserialized: WslDistroManifest = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.schema_version, 1);
        assert_eq!(deserialized.distro, "Ubuntu");
        assert_eq!(deserialized.proxy_origin, "http://172.20.0.1:12345");
        assert_eq!(deserialized.cli_backups.len(), 2);
        assert_eq!(deserialized.cli_backups[0].cli_key, "claude");
        assert_eq!(
            deserialized.cli_backups[0]
                .original_values
                .get("ANTHROPIC_BASE_URL"),
            Some(&Some("https://api.anthropic.com".to_string()))
        );
    }

    #[test]
    fn test_wsl_manifest_with_null_originals() {
        let manifest = WslDistroManifest {
            schema_version: 1,
            distro: "Debian".to_string(),
            configured: true,
            proxy_origin: "http://172.20.0.1:9999".to_string(),
            configured_at: 1700000000,
            wsl_home_unc: None,
            cli_backups: vec![WslCliBackup {
                cli_key: "claude".to_string(),
                injected_keys: [
                    (
                        "ANTHROPIC_BASE_URL".to_string(),
                        "http://172.20.0.1:9999/claude".to_string(),
                    ),
                    (
                        "ANTHROPIC_AUTH_TOKEN".to_string(),
                        "aio-coding-hub".to_string(),
                    ),
                ]
                .into_iter()
                .collect(),
                original_values: [
                    ("ANTHROPIC_BASE_URL".to_string(), None),
                    ("ANTHROPIC_AUTH_TOKEN".to_string(), None),
                ]
                .into_iter()
                .collect(),
            }],
        };

        let json = serde_json::to_string_pretty(&manifest).expect("serialize");
        // Verify null is present in JSON
        assert!(
            json.contains("null"),
            "JSON should contain null for missing original values"
        );

        let deserialized: WslDistroManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            deserialized.cli_backups[0]
                .original_values
                .get("ANTHROPIC_BASE_URL"),
            Some(&None)
        );
        assert_eq!(
            deserialized.cli_backups[0]
                .original_values
                .get("ANTHROPIC_AUTH_TOKEN"),
            Some(&None)
        );
    }

    #[test]
    fn test_extract_toml_value() {
        let content = r#"
preferred_auth_method = "api-key"
model_provider = "openai"
model = "o3"
"#;
        assert_eq!(
            crate::wsl::manifest::extract_toml_value(content, "preferred_auth_method"),
            Some("api-key".to_string())
        );
        assert_eq!(
            crate::wsl::manifest::extract_toml_value(content, "model_provider"),
            Some("openai".to_string())
        );
        assert_eq!(
            crate::wsl::manifest::extract_toml_value(content, "nonexistent"),
            None
        );
    }

    #[test]
    fn test_extract_env_value() {
        let content = r#"
# comment line
GOOGLE_GEMINI_BASE_URL=http://localhost:1234/gemini
GEMINI_API_KEY=aio-coding-hub
OTHER_VAR=keep
"#;
        assert_eq!(
            crate::wsl::manifest::extract_env_value(content, "GOOGLE_GEMINI_BASE_URL"),
            Some("http://localhost:1234/gemini".to_string())
        );
        assert_eq!(
            crate::wsl::manifest::extract_env_value(content, "GEMINI_API_KEY"),
            Some("aio-coding-hub".to_string())
        );
        assert_eq!(
            crate::wsl::manifest::extract_env_value(content, "MISSING"),
            None
        );
    }

    #[test]
    fn test_extract_env_value_with_export() {
        let content = "export GEMINI_API_KEY=my-key\n";
        assert_eq!(
            crate::wsl::manifest::extract_env_value(content, "GEMINI_API_KEY"),
            Some("my-key".to_string())
        );
    }

    #[test]
    fn restore_codex_config_toml_restores_root_keys_and_removes_injected_provider_section() {
        let backup = WslCliBackup {
            cli_key: "codex".to_string(),
            injected_keys: std::collections::HashMap::new(),
            original_values: [
                (
                    "preferred_auth_method".to_string(),
                    Some("device_code".to_string()),
                ),
                ("model_provider".to_string(), Some("openai".to_string())),
            ]
            .into_iter()
            .collect(),
        };
        let content = r#"
preferred_auth_method = "apikey"
model_provider = "aio"
model = "gpt-5"

[model_providers.aio]
name = "aio"
base_url = "http://127.0.0.1:37123/v1"
wire_api = "responses"
requires_openai_auth = true

[model_providers.openai]
name = "openai"
base_url = "https://api.openai.com/v1"
"#;

        let restored = crate::wsl::manifest::restore_codex_config_toml(content, &backup)
            .expect("restore codex config");

        assert_eq!(
            crate::wsl::manifest::extract_toml_value(&restored, "preferred_auth_method"),
            Some("device_code".to_string())
        );
        assert_eq!(
            crate::wsl::manifest::extract_toml_value(&restored, "model_provider"),
            Some("openai".to_string())
        );
        assert!(!restored.contains("[model_providers.aio]"));
        assert!(restored.contains("[model_providers.openai]"));
        assert!(restored.contains("model = \"gpt-5\""));
    }

    #[test]
    fn restore_codex_config_toml_removes_injected_root_keys_when_original_values_missing() {
        let backup = WslCliBackup {
            cli_key: "codex".to_string(),
            injected_keys: std::collections::HashMap::new(),
            original_values: [
                ("preferred_auth_method".to_string(), None),
                ("model_provider".to_string(), None),
            ]
            .into_iter()
            .collect(),
        };
        let content = r#"
preferred_auth_method = "apikey"
model_provider = "aio"

[model_providers.aio]
name = "aio"
base_url = "http://127.0.0.1:37123/v1"

[model_providers.custom]
name = "custom"
base_url = "https://example.com/v1"
"#;

        let restored = crate::wsl::manifest::restore_codex_config_toml(content, &backup)
            .expect("restore codex config");

        assert_eq!(
            crate::wsl::manifest::extract_toml_value(&restored, "preferred_auth_method"),
            None
        );
        assert_eq!(
            crate::wsl::manifest::extract_toml_value(&restored, "model_provider"),
            None
        );
        assert!(!restored.contains("[model_providers.aio]"));
        assert!(restored.contains("[model_providers.custom]"));
    }

    #[test]
    fn gather_skills_sync_data_collects_enabled_skill_files_for_active_workspace() {
        use crate::shared::error::AppResult;

        let result = (|| -> AppResult<()> {
            let _guard = crate::test_support::test_env_lock();
            let temp = tempfile::tempdir().expect("tempdir");
            let _test_home = ScopedEnvVar::set(
                "AIO_CODING_HUB_TEST_HOME",
                temp.path().as_os_str().to_os_string(),
            );

            let app = tauri::test::mock_app();
            let app_handle = app.handle().clone();
            let conn = rusqlite::Connection::open_in_memory().expect("open in-memory sqlite");

            conn.execute_batch(
                r#"
CREATE TABLE workspace_active (
  cli_key TEXT PRIMARY KEY,
  workspace_id INTEGER
);
CREATE TABLE skills (
  id INTEGER PRIMARY KEY,
  skill_key TEXT NOT NULL
);
CREATE TABLE workspace_skill_enabled (
  workspace_id INTEGER NOT NULL,
  skill_id INTEGER NOT NULL
);
"#,
            )
            .expect("create schema");
            conn.execute(
                "INSERT INTO workspace_active(cli_key, workspace_id) VALUES ('codex', 101)",
                [],
            )
            .expect("insert active workspace");
            conn.execute(
                "INSERT INTO skills(id, skill_key) VALUES (1, 'review-skill')",
                [],
            )
            .expect("insert skill");
            conn.execute(
                "INSERT INTO workspace_skill_enabled(workspace_id, skill_id) VALUES (101, 1)",
                [],
            )
            .expect("enable skill");

            let ssot_root = crate::app_paths::app_data_dir(&app_handle)?
                .join("skills")
                .join("review-skill");
            std::fs::create_dir_all(ssot_root.join("nested")).expect("create skill dirs");
            std::fs::write(ssot_root.join("SKILL.md"), "---\nname: Review\n---\n")
                .expect("write skill md");
            std::fs::write(ssot_root.join("nested").join("notes.txt"), "checklist")
                .expect("write nested file");
            std::fs::write(
                ssot_root.join(crate::wsl::skills_sync::WSL_SKILL_SOURCE_MARKER_FILE),
                "{\"source\":\"ignored\"}",
            )
            .expect("write source marker");

            let data = gather_skills_sync_data(&app_handle, &conn)?;

            assert!(data.claude.is_empty());
            assert_eq!(data.codex.len(), 1);
            assert!(data.gemini.is_empty());
            assert_eq!(data.codex[0].skill_key, "review-skill");

            let relative_paths: Vec<&str> = data.codex[0]
                .files
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect();
            assert!(relative_paths.contains(&"SKILL.md"));
            assert!(relative_paths.contains(&"nested/notes.txt"));
            assert!(
                !relative_paths.contains(&crate::wsl::skills_sync::WSL_SKILL_SOURCE_MARKER_FILE)
            );

            let nested = data.codex[0]
                .files
                .iter()
                .find(|file| file.relative_path == "nested/notes.txt")
                .expect("nested file");
            assert_eq!(nested.content, b"checklist");
            Ok(())
        })();

        result.expect("gather skills sync data");
    }
}
