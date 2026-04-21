#[cfg(unix)]
use super::skill_fs::export_skill_dir_files;
use super::skill_fs::{cli_skills_root, ssot_skills_root};
use super::*;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::MutexGuard;

static TEST_ENV_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
struct EnvRestore {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvRestore {
    fn save_once(&mut self, key: &'static str) {
        if self.saved.iter().any(|(saved_key, _)| *saved_key == key) {
            return;
        }
        self.saved.push((key, std::env::var_os(key)));
    }

    fn set_var(&mut self, key: &'static str, value: impl Into<OsString>) {
        self.save_once(key);
        std::env::set_var(key, value.into());
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..).rev() {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

struct ConfigMigrateTestApp {
    _lock: MutexGuard<'static, ()>,
    _env: EnvRestore,
    #[allow(dead_code)]
    home: tempfile::TempDir,
    app: tauri::App<tauri::test::MockRuntime>,
    db: crate::db::Db,
}

impl ConfigMigrateTestApp {
    fn new() -> Self {
        let lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().expect("tempdir");
        let seq = TEST_ENV_SEQ.fetch_add(1, Ordering::Relaxed);
        let mut env = EnvRestore::default();
        let home_os = home.path().as_os_str().to_os_string();
        env.set_var("AIO_CODING_HUB_HOME_DIR", home_os.clone());
        env.set_var(
            "AIO_CODING_HUB_DOTDIR_NAME",
            format!(".aio-coding-hub-config-migrate-test-{seq}"),
        );
        crate::test_support::clear_settings_cache();

        let app = tauri::test::mock_app();
        app.manage(crate::resident::ResidentState::default());
        let db = crate::db::init(app.handle()).expect("init db");

        Self {
            _lock: lock,
            _env: env,
            home,
            app,
            db,
        }
    }

    fn handle(&self) -> tauri::AppHandle<tauri::test::MockRuntime> {
        self.app.handle().clone()
    }
}

fn query_workspace(conn: &Connection, cli_key: &str) -> (i64, String) {
    conn.query_row(
        "SELECT id, name FROM workspaces WHERE cli_key = ?1 ORDER BY id ASC LIMIT 1",
        params![cli_key],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .expect("query workspace")
}

fn write_skill_md(dir: &Path, name: &str, description: &str) {
    std::fs::create_dir_all(dir).expect("create skill dir");
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n"),
    )
    .expect("write skill md");
}

fn make_test_bundle(schema_version: u32) -> ConfigBundle {
    ConfigBundle {
        schema_version,
        exported_at: "2026-03-29T00:00:00.000Z".to_string(),
        app_version: "0.0.0-test".to_string(),
        settings: serde_json::to_string(&settings::AppSettings::default()).expect("settings"),
        providers: Vec::new(),
        sort_modes: Vec::new(),
        sort_mode_active: HashMap::new(),
        workspaces: vec![WorkspaceExport {
            cli_key: "codex".to_string(),
            name: "Imported".to_string(),
            is_active: true,
            prompts: Vec::new(),
            prompt: None,
        }],
        mcp_servers: Vec::new(),
        skill_repos: Vec::new(),
        installed_skills: (schema_version >= CONFIG_BUNDLE_SCHEMA_VERSION).then(Vec::new),
        local_skills: (schema_version >= CONFIG_BUNDLE_SCHEMA_VERSION).then(Vec::new),
    }
}

#[cfg(unix)]
fn create_file_symlink(src: &Path, dst: &Path) {
    std::os::unix::fs::symlink(src, dst).expect("create symlink");
}

#[test]
fn validate_bundle_schema_version_accepts_current_version() {
    assert!(super::validate_bundle_schema_version(CONFIG_BUNDLE_SCHEMA_VERSION).is_ok());
    assert!(super::validate_bundle_schema_version(CONFIG_BUNDLE_SCHEMA_VERSION_V1).is_ok());
}

#[test]
fn validate_bundle_schema_version_rejects_mismatch() {
    let err = super::validate_bundle_schema_version(CONFIG_BUNDLE_SCHEMA_VERSION + 1)
        .expect_err("schema version should fail");
    assert!(err
        .to_string()
        .contains("SEC_INVALID_INPUT: unsupported config bundle schema_version"));
}

#[test]
fn config_export_includes_full_prompts_provider_and_skill_payload() {
    let test_app = ConfigMigrateTestApp::new();
    let app = test_app.handle();
    let conn = test_app.db.open_connection().expect("open db");
    let (codex_workspace_id, codex_workspace_name) = query_workspace(&conn, "codex");

    conn.execute(
        r#"
INSERT INTO providers(
  cli_key, name, base_url, base_urls_json, base_url_mode, auth_mode,
  claude_models_json, supported_models_json, model_mapping_json, api_key_plaintext,
  enabled, priority, sort_order, cost_multiplier, limit_5h_usd, limit_daily_usd,
  daily_reset_mode, daily_reset_time, limit_weekly_usd, limit_monthly_usd, limit_total_usd,
  tags_json, note, oauth_provider_type, oauth_access_token, oauth_refresh_token, oauth_id_token,
  oauth_token_uri, oauth_client_id, oauth_client_secret, oauth_expires_at, oauth_email,
  oauth_refresh_lead_s, oauth_last_refreshed_at, oauth_last_error, created_at, updated_at
) VALUES (
  'codex', 'oauth-provider', 'https://api.example.com', '["https://api.example.com","https://backup.example.com"]',
  'order', 'oauth', '{"main":"gpt-5.4"}', '{"gpt-5.4":true}', '{"gpt-5.4":"gpt-5.4"}', '',
  1, 100, 0, 1.25, 1.0, 2.0, 'fixed', '00:00:00', 3.0, 4.0, 5.0, '["team"]', 'note',
  'openai', 'access-token', 'refresh-token', 'id-token', 'https://auth.example.com/token',
  'client-id', 'client-secret', 2000000000, 'dev@example.com', 7200, 1999999999, 'last error',
  1, 1
)
"#,
        [],
    )
    .expect("insert provider");

    conn.execute(
        r#"
INSERT INTO prompts(workspace_id, name, content, enabled, created_at, updated_at)
VALUES (?1, 'default', 'prompt one', 1, 1, 1),
       (?1, 'review', 'prompt two', 0, 1, 1)
"#,
        params![codex_workspace_id],
    )
    .expect("insert prompts");

    conn.execute(
        r#"
INSERT INTO skill_repos(git_url, branch, enabled, created_at, updated_at)
VALUES ('https://example.com/repo.git', 'main', 1, 1, 1)
"#,
        [],
    )
    .expect("insert skill repo");

    conn.execute(
        r#"
INSERT INTO skills(
  skill_key, name, normalized_name, description, source_git_url, source_branch, source_subdir,
  created_at, updated_at
) VALUES (
  'review-skill', 'Review Skill', 'review-skill', 'Installed review skill',
  'https://example.com/repo.git', 'main', 'skills/review', 1, 1
)
"#,
        [],
    )
    .expect("insert skill");
    let skill_id = conn.last_insert_rowid();
    conn.execute(
        r#"
INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
VALUES (?1, ?2, 1, 1)
"#,
        params![codex_workspace_id, skill_id],
    )
    .expect("enable skill");

    let ssot_root = ssot_skills_root(&app).expect("ssot root");
    let installed_skill_dir = ssot_root.join("review-skill");
    write_skill_md(
        &installed_skill_dir,
        "Review Skill",
        "Installed review skill",
    );
    std::fs::write(installed_skill_dir.join("README.md"), "installed").expect("write readme");

    let local_root = cli_skills_root(&app, "codex").expect("local root");
    let local_skill_dir = local_root.join("local-review");
    write_skill_md(&local_skill_dir, "Local Review", "Local review skill");
    std::fs::write(local_skill_dir.join("notes.txt"), "local").expect("write local file");
    let source_metadata = skill_fs::SkillSourceMetadataFile {
        source_git_url: "https://example.com/local.git".to_string(),
        source_branch: "main".to_string(),
        source_subdir: "skills/local-review".to_string(),
    };
    std::fs::write(
        local_skill_dir.join(SKILL_SOURCE_MARKER_FILE),
        serde_json::to_vec_pretty(&source_metadata).expect("serialize source"),
    )
    .expect("write source metadata");

    let bundle = config_export(&app, &test_app.db).expect("config export");

    let provider = bundle
        .providers
        .iter()
        .find(|provider| provider.name == "oauth-provider")
        .expect("provider export");
    assert_eq!(
        provider.base_urls,
        vec![
            "https://api.example.com".to_string(),
            "https://backup.example.com".to_string()
        ]
    );
    assert_eq!(provider.oauth_id_token.as_deref(), Some("id-token"));
    assert_eq!(provider.oauth_refresh_lead_seconds, 7200);
    assert_eq!(provider.oauth_last_refreshed_at, Some(1999999999));
    assert_eq!(provider.oauth_last_error.as_deref(), Some("last error"));
    assert_eq!(provider.supported_models_json, "{\"gpt-5.4\":true}");
    assert_eq!(provider.model_mapping_json, "{\"gpt-5.4\":\"gpt-5.4\"}");

    let codex_workspace = bundle
        .workspaces
        .iter()
        .find(|workspace| workspace.cli_key == "codex" && workspace.name == codex_workspace_name)
        .expect("codex workspace export");
    assert_eq!(codex_workspace.prompts.len(), 2);
    assert!(codex_workspace.prompt.is_none());

    let installed_skill = bundle
        .installed_skills
        .as_ref()
        .expect("installed skills export")
        .iter()
        .find(|skill| skill.skill_key == "review-skill")
        .expect("installed skill export");
    assert_eq!(installed_skill.enabled_in_workspaces.len(), 1);
    assert_eq!(
        installed_skill.enabled_in_workspaces[0],
        ("codex".to_string(), codex_workspace_name.clone())
    );
    assert!(installed_skill
        .files
        .iter()
        .any(|file| file.relative_path == "SKILL.md"));

    let local_skill = bundle
        .local_skills
        .as_ref()
        .expect("local skills export")
        .iter()
        .find(|skill| skill.cli_key == "codex" && skill.dir_name == "local-review")
        .expect("local skill export");
    assert_eq!(
        local_skill.source_git_url.as_deref(),
        Some("https://example.com/local.git")
    );
    assert!(local_skill
        .files
        .iter()
        .any(|file| file.relative_path == "notes.txt"));
}

#[test]
fn config_import_v2_restores_full_prompt_and_skill_payload() {
    let test_app = ConfigMigrateTestApp::new();
    let app = test_app.handle();
    let bundle = ConfigBundle {
        providers: vec![ProviderExport {
            id: Some(1),
            cli_key: "codex".to_string(),
            name: "oauth-provider".to_string(),
            base_urls: vec![
                "https://api.example.com".to_string(),
                "https://backup.example.com".to_string(),
            ],
            base_url_mode: "order".to_string(),
            api_key_plaintext: String::new(),
            auth_mode: "oauth".to_string(),
            oauth_provider_type: Some("openai".to_string()),
            oauth_access_token: Some("access-token".to_string()),
            oauth_refresh_token: Some("refresh-token".to_string()),
            oauth_id_token: Some("id-token".to_string()),
            oauth_token_expiry: Some(2_000_000_000),
            oauth_scopes: None,
            oauth_token_uri: Some("https://auth.example.com/token".to_string()),
            oauth_client_id: Some("client-id".to_string()),
            oauth_client_secret: Some("client-secret".to_string()),
            oauth_email: Some("dev@example.com".to_string()),
            oauth_refresh_lead_seconds: 7200,
            oauth_last_refreshed_at: Some(1_999_999_999),
            oauth_last_error: Some("last error".to_string()),
            claude_models_json: "{\"main\":\"gpt-5.4\"}".to_string(),
            supported_models_json: "{\"gpt-5.4\":true}".to_string(),
            model_mapping_json: "{\"gpt-5.4\":\"gpt-5.4\"}".to_string(),
            enabled: true,
            priority: 100,
            cost_multiplier: 1.25,
            limit_5h_usd: Some(1.0),
            limit_daily_usd: Some(2.0),
            limit_weekly_usd: Some(3.0),
            limit_monthly_usd: Some(4.0),
            limit_total_usd: Some(5.0),
            daily_reset_mode: "fixed".to_string(),
            daily_reset_time: "00:00:00".to_string(),
            tags_json: "[\"team\"]".to_string(),
            note: "note".to_string(),
            source_provider_id: None,
            source_provider_cli_key: None,
            bridge_type: None,
        }],
        sort_modes: Vec::new(),
        sort_mode_active: HashMap::new(),
        workspaces: vec![WorkspaceExport {
            cli_key: "codex".to_string(),
            name: "Imported".to_string(),
            is_active: true,
            prompts: vec![
                PromptExport {
                    name: "default".to_string(),
                    content: "prompt one".to_string(),
                    enabled: true,
                },
                PromptExport {
                    name: "review".to_string(),
                    content: "prompt two".to_string(),
                    enabled: false,
                },
            ],
            prompt: None,
        }],
        skill_repos: vec![SkillRepoExport {
            git_url: "https://example.com/repo.git".to_string(),
            branch: "main".to_string(),
            enabled: true,
        }],
        installed_skills: Some(vec![InstalledSkillExport {
            skill_key: "review-skill".to_string(),
            name: "Review Skill".to_string(),
            description: "Installed review skill".to_string(),
            source_git_url: "https://example.com/repo.git".to_string(),
            source_branch: "main".to_string(),
            source_subdir: "skills/review".to_string(),
            enabled_in_workspaces: vec![("codex".to_string(), "Imported".to_string())],
            files: vec![
                SkillFileExport {
                    relative_path: "SKILL.md".to_string(),
                    content_base64: BASE64_STANDARD.encode(
                        b"---\nname: Review Skill\ndescription: Installed review skill\n---\n",
                    ),
                },
                SkillFileExport {
                    relative_path: "README.md".to_string(),
                    content_base64: BASE64_STANDARD.encode(b"installed"),
                },
            ],
        }]),
        local_skills: Some(vec![LocalSkillExport {
            cli_key: "codex".to_string(),
            dir_name: "local-review".to_string(),
            name: "Local Review".to_string(),
            description: "Local review skill".to_string(),
            source_git_url: Some("https://example.com/local.git".to_string()),
            source_branch: Some("main".to_string()),
            source_subdir: Some("skills/local-review".to_string()),
            files: vec![
                SkillFileExport {
                    relative_path: "SKILL.md".to_string(),
                    content_base64: BASE64_STANDARD
                        .encode(b"---\nname: Local Review\ndescription: Local review skill\n---\n"),
                },
                SkillFileExport {
                    relative_path: "notes.txt".to_string(),
                    content_base64: BASE64_STANDARD.encode(b"local"),
                },
            ],
        }]),
        ..make_test_bundle(CONFIG_BUNDLE_SCHEMA_VERSION)
    };

    let result = config_import(&app, &test_app.db, bundle).expect("config import");
    assert_eq!(result.providers_imported, 1);
    assert_eq!(result.prompts_imported, 2);
    assert_eq!(result.installed_skills_imported, 1);
    assert_eq!(result.local_skills_imported, 1);

    let conn = test_app.db.open_connection().expect("open db");
    let prompt_count: i64 = conn
        .query_row("SELECT COUNT(1) FROM prompts", [], |row| row.get(0))
        .expect("prompt count");
    assert_eq!(prompt_count, 2);

    let oauth_id_token: Option<String> = conn
        .query_row(
            "SELECT oauth_id_token FROM providers WHERE name = 'oauth-provider'",
            [],
            |row| row.get(0),
        )
        .expect("oauth id token");
    assert_eq!(oauth_id_token.as_deref(), Some("id-token"));

    let skill_enabled_count: i64 = conn
        .query_row("SELECT COUNT(1) FROM workspace_skill_enabled", [], |row| {
            row.get(0)
        })
        .expect("skill enabled count");
    assert_eq!(skill_enabled_count, 1);

    let ssot_root = ssot_skills_root(&app).expect("ssot root");
    assert!(ssot_root.join("review-skill").join("README.md").exists());

    let local_root = cli_skills_root(&app, "codex").expect("local root");
    assert!(local_root.join("local-review").join("notes.txt").exists());
    assert!(local_root.join("review-skill").join("SKILL.md").exists());

    let prompt_bytes = crate::prompt_sync::read_target_bytes(&app, "codex")
        .expect("read prompt target")
        .expect("prompt target exists");
    assert_eq!(
        String::from_utf8(prompt_bytes)
            .expect("utf8")
            .trim_end_matches('\n'),
        "prompt one"
    );
}

#[test]
fn config_import_v1_keeps_existing_skill_state() {
    let test_app = ConfigMigrateTestApp::new();
    let app = test_app.handle();
    let conn = test_app.db.open_connection().expect("open db");
    let (codex_workspace_id, _) = query_workspace(&conn, "codex");

    conn.execute(
        r#"
INSERT INTO skills(
  skill_key, name, normalized_name, description, source_git_url, source_branch, source_subdir,
  created_at, updated_at
) VALUES (
  'existing-skill', 'Existing Skill', 'existing-skill', 'Existing skill',
  'https://example.com/existing.git', 'main', 'skills/existing', 1, 1
)
"#,
        [],
    )
    .expect("insert existing skill");
    let existing_skill_id = conn.last_insert_rowid();
    conn.execute(
        r#"
INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
VALUES (?1, ?2, 1, 1)
"#,
        params![codex_workspace_id, existing_skill_id],
    )
    .expect("enable existing skill");

    let ssot_root = ssot_skills_root(&app).expect("ssot root");
    write_skill_md(
        &ssot_root.join("existing-skill"),
        "Existing Skill",
        "Existing skill",
    );

    let local_root = cli_skills_root(&app, "codex").expect("local root");
    write_skill_md(
        &local_root.join("existing-local"),
        "Existing Local",
        "Local skill",
    );

    let bundle = ConfigBundle {
        workspaces: vec![WorkspaceExport {
            cli_key: "codex".to_string(),
            name: "Imported".to_string(),
            is_active: true,
            prompts: Vec::new(),
            prompt: Some(PromptExport {
                name: "default".to_string(),
                content: "legacy prompt".to_string(),
                enabled: true,
            }),
        }],
        installed_skills: None,
        local_skills: None,
        ..make_test_bundle(CONFIG_BUNDLE_SCHEMA_VERSION_V1)
    };

    let result = config_import(&app, &test_app.db, bundle).expect("config import");
    assert_eq!(result.prompts_imported, 1);
    assert_eq!(result.installed_skills_imported, 0);
    assert_eq!(result.local_skills_imported, 0);

    let conn = test_app.db.open_connection().expect("open db");
    let skill_count: i64 = conn
        .query_row("SELECT COUNT(1) FROM skills", [], |row| row.get(0))
        .expect("skill count");
    assert_eq!(skill_count, 1);
    let restored_enabled_count: i64 = conn
        .query_row(
            r#"
SELECT COUNT(1)
FROM workspace_skill_enabled e
JOIN skills s ON s.id = e.skill_id
WHERE s.skill_key = 'existing-skill'
"#,
            [],
            |row| row.get(0),
        )
        .expect("restored enabled skill count");
    assert_eq!(restored_enabled_count, 1);
    assert!(ssot_root.join("existing-skill").join("SKILL.md").exists());
    assert!(local_root.join("existing-local").join("SKILL.md").exists());
    assert!(local_root.join("existing-skill").join("SKILL.md").exists());
}

#[test]
fn config_import_v2_rejects_missing_installed_skills_payload() {
    let test_app = ConfigMigrateTestApp::new();
    let app = test_app.handle();
    let mut bundle = make_test_bundle(CONFIG_BUNDLE_SCHEMA_VERSION);
    bundle.installed_skills = None;

    let err = config_import(&app, &test_app.db, bundle)
        .err()
        .expect("missing installed_skills");
    assert!(err
        .to_string()
        .contains("SEC_INVALID_INPUT: config bundle missing installed_skills"));
}

#[test]
fn config_import_v2_rejects_missing_local_skills_payload() {
    let test_app = ConfigMigrateTestApp::new();
    let app = test_app.handle();
    let mut bundle = make_test_bundle(CONFIG_BUNDLE_SCHEMA_VERSION);
    bundle.local_skills = None;

    let err = config_import(&app, &test_app.db, bundle)
        .err()
        .expect("missing local_skills");
    assert!(err
        .to_string()
        .contains("SEC_INVALID_INPUT: config bundle missing local_skills"));
}

#[test]
fn validate_local_skills_for_import_rejects_unknown_cli_key() {
    let err = import::validate_local_skills_for_import(&[LocalSkillExport {
        cli_key: "cursor".to_string(),
        dir_name: "local-review".to_string(),
        name: "Local Review".to_string(),
        description: "Local review skill".to_string(),
        source_git_url: None,
        source_branch: None,
        source_subdir: None,
        files: vec![SkillFileExport {
            relative_path: "SKILL.md".to_string(),
            content_base64: BASE64_STANDARD
                .encode(b"---\nname: Local Review\ndescription: Local review skill\n---\n"),
        }],
    }])
    .expect_err("unknown cli key should fail");
    assert!(err
        .to_string()
        .contains("SEC_INVALID_INPUT: unknown local skill cli_key=cursor"));
}

#[cfg(unix)]
#[test]
fn export_skill_dir_files_rejects_symlink_escape() {
    let temp = tempfile::tempdir().expect("tempdir");
    let skill_dir = temp.path().join("local-review");
    write_skill_md(&skill_dir, "Local Review", "Local review skill");

    let outside_file = temp.path().join("outside.txt");
    std::fs::write(&outside_file, "secret").expect("write outside file");
    create_file_symlink(&outside_file, &skill_dir.join("escape.txt"));

    let err = export_skill_dir_files(&skill_dir, true)
        .err()
        .expect("symlink escape should fail");
    assert!(err
        .to_string()
        .contains("SKILL_EXPORT_BLOCKED_SYMLINK_ESCAPE"));
}
