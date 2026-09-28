//! Usage: Workspace (profile) preview/apply orchestration.

use crate::claude_plugins;
use crate::db;
use crate::mcp_sync;
use crate::prompt_sync;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use crate::{mcp, prompts, skills, workspaces};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceEnabledPromptPreview {
    pub name: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspacePromptsPreview {
    pub from_enabled: Option<WorkspaceEnabledPromptPreview>,
    pub to_enabled: Option<WorkspaceEnabledPromptPreview>,
    pub will_change: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceItemsPreview {
    pub from_enabled: Vec<String>,
    pub to_enabled: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspacePreview {
    pub cli_key: String,
    pub from_workspace_id: Option<i64>,
    pub to_workspace_id: i64,
    pub prompts: WorkspacePromptsPreview,
    pub mcp: WorkspaceItemsPreview,
    pub skills: WorkspaceItemsPreview,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceApplyReport {
    pub cli_key: String,
    pub from_workspace_id: Option<i64>,
    pub to_workspace_id: i64,
    pub applied_at: i64,
}

fn excerpt(content: &str) -> String {
    const MAX_CHARS: usize = 160;
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut cutoff = normalized.len();
    for (idx, (byte_idx, _)) in normalized.char_indices().enumerate() {
        if idx == MAX_CHARS {
            cutoff = byte_idx;
            break;
        }
    }
    if cutoff == normalized.len() {
        return normalized;
    }
    format!("{}…", &normalized[..cutoff])
}

fn enabled_prompt_raw(
    conn: &Connection,
    workspace_id: i64,
) -> Result<Option<(String, String)>, String> {
    conn.query_row(
        r#"
SELECT name, content
FROM prompts
WHERE workspace_id = ?1 AND enabled = 1
ORDER BY updated_at DESC, id DESC
LIMIT 1
"#,
        params![workspace_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query enabled prompt: {e}"))
}

fn enabled_prompt_preview(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Option<WorkspaceEnabledPromptPreview>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(None);
    };
    let Some((name, content)) = enabled_prompt_raw(conn, workspace_id)? else {
        return Ok(None);
    };
    Ok(Some(WorkspaceEnabledPromptPreview {
        name,
        excerpt: excerpt(&content),
    }))
}

fn list_enabled_mcp_keys(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Vec<String>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT s.server_key
    FROM mcp_servers s
    JOIN workspace_mcp_enabled e
      ON e.server_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.server_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled mcp query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to query enabled mcp servers: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read enabled mcp row: {e}"))?);
    }
    Ok(out)
}

fn list_enabled_skill_keys(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Vec<String>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT s.skill_key
    FROM skills s
    JOIN workspace_skill_enabled e
      ON e.skill_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.skill_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled skills query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to query enabled skills: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read enabled skill row: {e}"))?);
    }
    Ok(out)
}

fn diff(from_enabled: &[String], to_enabled: &[String]) -> (Vec<String>, Vec<String>) {
    let from_set: HashSet<&str> = from_enabled.iter().map(String::as_str).collect();
    let to_set: HashSet<&str> = to_enabled.iter().map(String::as_str).collect();

    let mut added: Vec<String> = to_set
        .difference(&from_set)
        .map(|v| v.to_string())
        .collect();
    let mut removed: Vec<String> = from_set
        .difference(&to_set)
        .map(|v| v.to_string())
        .collect();

    added.sort();
    removed.sort();
    (added, removed)
}

pub fn preview(
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<WorkspacePreview> {
    let conn = db.open_connection()?;

    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    let from_workspace_id = workspaces::active_id_by_cli(&conn, &cli_key)?;

    let from_enabled_prompt = enabled_prompt_preview(&conn, from_workspace_id)?;
    let to_enabled_prompt = enabled_prompt_preview(&conn, Some(workspace_id))?;

    let will_change = match (from_workspace_id, Some(workspace_id)) {
        (None, _) => to_enabled_prompt.is_some(),
        (Some(from_id), Some(to_id)) => {
            let from_raw = enabled_prompt_raw(&conn, from_id)?;
            let to_raw = enabled_prompt_raw(&conn, to_id)?;
            from_raw.map(|v| v.1).unwrap_or_default() != to_raw.map(|v| v.1).unwrap_or_default()
        }
        _ => false,
    };

    let from_mcp = list_enabled_mcp_keys(&conn, from_workspace_id)?;
    let to_mcp = list_enabled_mcp_keys(&conn, Some(workspace_id))?;
    let (mcp_added, mcp_removed) = diff(&from_mcp, &to_mcp);

    let from_skills = list_enabled_skill_keys(&conn, from_workspace_id)?;
    let to_skills = list_enabled_skill_keys(&conn, Some(workspace_id))?;
    let (skills_added, skills_removed) = diff(&from_skills, &to_skills);

    Ok(WorkspacePreview {
        cli_key,
        from_workspace_id,
        to_workspace_id: workspace_id,
        prompts: WorkspacePromptsPreview {
            from_enabled: from_enabled_prompt,
            to_enabled: to_enabled_prompt,
            will_change,
        },
        mcp: WorkspaceItemsPreview {
            from_enabled: from_mcp,
            to_enabled: to_mcp,
            added: mcp_added,
            removed: mcp_removed,
        },
        skills: WorkspaceItemsPreview {
            from_enabled: from_skills,
            to_enabled: to_skills,
            added: skills_added,
            removed: skills_removed,
        },
    })
}

pub fn apply<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<WorkspaceApplyReport> {
    let conn = db.open_connection()?;

    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    let from_workspace_id = workspaces::active_id_by_cli(&conn, &cli_key)?;

    if from_workspace_id == Some(workspace_id) {
        return Ok(WorkspaceApplyReport {
            cli_key,
            from_workspace_id,
            to_workspace_id: workspace_id,
            applied_at: now_unix_seconds(),
        });
    }

    let prev_prompt_target = prompt_sync::read_target_bytes(app, &cli_key)?;
    let prev_prompt_manifest = prompt_sync::read_manifest_bytes(app, &cli_key)?;
    let prev_mcp_target = mcp_sync::read_target_bytes(app, &cli_key)?;
    let prev_mcp_manifest = mcp_sync::read_manifest_bytes(app, &cli_key)?;
    let managed_mcp_server_keys: HashSet<String> = list_enabled_mcp_keys(&conn, from_workspace_id)?
        .into_iter()
        .collect();

    if let Err(err) = prompts::sync_cli_for_workspace(app, &conn, workspace_id) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        return Err(err);
    }

    if let Err(err) = mcp::swap_local_mcp_servers_for_workspace_switch(
        app,
        &cli_key,
        &managed_mcp_server_keys,
        from_workspace_id,
        workspace_id,
    ) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
        let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);
        return Err(err);
    }

    if let Err(err) = mcp::sync_cli_for_workspace(app, &conn, workspace_id) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
        let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);
        return Err(err);
    }

    let mut local_plugins_swap = if cli_key == "claude" {
        match claude_plugins::swap_local_plugins_for_workspace_switch(
            app,
            &cli_key,
            from_workspace_id,
            workspace_id,
        ) {
            Ok(swap) => Some(swap),
            Err(err) => {
                let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
                let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
                let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
                let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);
                return Err(err);
            }
        }
    } else {
        None
    };

    if let Err(err) = skills::sync_cli_for_workspace(app, &conn, workspace_id) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
        let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);

        if let Some(swap) = local_plugins_swap.take() {
            swap.rollback();
        }

        if let Some(from_id) = from_workspace_id {
            let _ = skills::sync_cli_for_workspace(app, &conn, from_id);
        }

        return Err(err);
    }

    let local_skills_swap = match skills::swap_local_skills_for_workspace_switch(
        app,
        &conn,
        &cli_key,
        from_workspace_id,
        workspace_id,
    ) {
        Ok(swap) => swap,
        Err(err) => {
            let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
            let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
            let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
            let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);

            if let Some(swap) = local_plugins_swap.take() {
                swap.rollback();
            }

            if let Some(from_id) = from_workspace_id {
                let _ = skills::sync_cli_for_workspace(app, &conn, from_id);
            }

            return Err(err);
        }
    };

    if let Err(err) = workspaces::set_active(&conn, workspace_id) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
        let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);

        local_skills_swap.rollback();

        if let Some(swap) = local_plugins_swap.take() {
            swap.rollback();
        }

        if let Some(from_id) = from_workspace_id {
            let _ = skills::sync_cli_for_workspace(app, &conn, from_id);
        }

        return Err(err);
    }

    Ok(WorkspaceApplyReport {
        cli_key,
        from_workspace_id,
        to_workspace_id: workspace_id,
        applied_at: now_unix_seconds(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
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

    struct GrokWorkspaceTestApp {
        _lock: MutexGuard<'static, ()>,
        _env: EnvRestore,
        _home: tempfile::TempDir,
        _db_dir: tempfile::TempDir,
        app: tauri::App<tauri::test::MockRuntime>,
        db: db::Db,
        grok_home: std::path::PathBuf,
    }

    impl GrokWorkspaceTestApp {
        fn new() -> Self {
            let lock = crate::test_support::test_env_lock();
            let home = tempfile::tempdir().expect("home tempdir");
            let db_dir = tempfile::tempdir().expect("db tempdir");
            let grok_home = home.path().join("custom-grok");
            let mut env = EnvRestore::default();
            env.set(
                "AIO_CODING_HUB_HOME_DIR",
                home.path().as_os_str().to_os_string(),
            );
            env.set(
                "AIO_CODING_HUB_DOTDIR_NAME",
                format!(
                    ".aio-grok-workspace-test-{}",
                    TEST_SEQ.fetch_add(1, Ordering::Relaxed)
                ),
            );
            env.set("GROK_HOME", grok_home.as_os_str().to_os_string());
            crate::test_support::clear_settings_cache();
            let app = tauri::test::mock_app();
            let db =
                db::init_for_tests(&db_dir.path().join("workspace.sqlite")).expect("init test db");

            Self {
                _lock: lock,
                _env: env,
                _home: home,
                _db_dir: db_dir,
                app,
                db,
                grok_home,
            }
        }

        fn handle(&self) -> tauri::AppHandle<tauri::test::MockRuntime> {
            self.app.handle().clone()
        }

        fn default_workspace_id(&self) -> i64 {
            let list = workspaces::list_by_cli(&self.db, "grok").expect("list Grok workspaces");
            list.active_id.expect("default Grok workspace active")
        }

        fn target_workspace_id(&self) -> i64 {
            workspaces::create(&self.db, "grok", "Target", false)
                .expect("create target workspace")
                .id
        }

        fn set_prompt(&self, workspace_id: i64, content: &str) {
            self.db
                .open_connection()
                .expect("open db")
                .execute(
                    "UPDATE prompts SET content = ?1, enabled = 1 WHERE workspace_id = ?2",
                    params![content, workspace_id],
                )
                .expect("update target prompt");
        }

        fn add_mcp(&self, workspace_id: i64) {
            let conn = self.db.open_connection().expect("open db");
            conn.execute(
                r#"
INSERT INTO mcp_servers(
  server_key, name, normalized_name, transport, command, args_json, env_json,
  cwd, url, headers_json, created_at, updated_at
) VALUES ('managed', 'Managed', 'managed', 'stdio', 'npx', '["-y"]', '{}', NULL, NULL, '{}', 1, 1)
"#,
                [],
            )
            .expect("insert MCP server");
            let server_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at) VALUES (?1, ?2, 1, 1)",
                params![workspace_id, server_id],
            )
            .expect("enable MCP server");
        }

        fn add_skill(&self, workspace_id: i64, create_ssot: bool) {
            let conn = self.db.open_connection().expect("open db");
            conn.execute(
                r#"
INSERT INTO skills(
  skill_key, name, normalized_name, description, source_git_url, source_branch,
  source_subdir, installed_commit, installed_content_hash, created_at, updated_at
) VALUES ('demo', 'Demo', 'demo', '', 'https://example.test/skills.git', 'main', 'demo', NULL, NULL, 1, 1)
"#,
                [],
            )
            .expect("insert skill");
            let skill_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at) VALUES (?1, ?2, 1, 1)",
                params![workspace_id, skill_id],
            )
            .expect("enable skill");

            if create_ssot {
                let paths =
                    crate::skills::paths_get(&self.handle(), "grok").expect("resolve skill paths");
                let skill_dir = std::path::PathBuf::from(paths.ssot_dir).join("demo");
                std::fs::create_dir_all(&skill_dir).expect("create SSOT skill");
                std::fs::write(skill_dir.join("SKILL.md"), "---\nname: Demo\n---\n")
                    .expect("write SKILL.md");
            }
        }

        fn write_config(&self, bytes: &[u8]) {
            std::fs::create_dir_all(&self.grok_home).expect("create Grok home");
            std::fs::write(self.grok_home.join("config.toml"), bytes).expect("write config");
        }

        fn write_prompt(&self, content: &str) {
            std::fs::create_dir_all(&self.grok_home).expect("create Grok home");
            std::fs::write(self.grok_home.join("AGENTS.md"), content).expect("write prompt");
        }

        fn assert_active(&self, workspace_id: i64) {
            let conn = self.db.open_connection().expect("open db");
            assert_eq!(
                workspaces::active_id_by_cli(&conn, "grok").expect("active workspace"),
                Some(workspace_id)
            );
        }
    }

    const INITIAL_CONFIG: &str = r#"# keep
[model.aio]
model = "grok-build"
base_url = "http://127.0.0.1:37123/grok/v1"

[mcp_servers.local]
command = "local"
"#;

    #[test]
    fn grok_workspace_round_trip_applies_prompt_mcp_skills_and_local_stash() {
        let test = GrokWorkspaceTestApp::new();
        let default_id = test.default_workspace_id();
        let target_id = test.target_workspace_id();
        test.set_prompt(target_id, "target instructions");
        test.add_mcp(target_id);
        test.add_skill(target_id, true);
        test.write_config(INITIAL_CONFIG.as_bytes());
        test.write_prompt("original instructions");

        let report = apply(&test.handle(), &test.db, target_id).expect("apply target workspace");

        assert_eq!(report.cli_key, "grok");
        test.assert_active(target_id);
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "target instructions\n"
        );
        let config =
            std::fs::read_to_string(test.grok_home.join("config.toml")).expect("read config");
        let document = config
            .parse::<toml_edit::DocumentMut>()
            .expect("valid Grok TOML");
        assert_eq!(
            document["model"]["aio"]["model"].as_str(),
            Some("grok-build")
        );
        assert_eq!(
            document["mcp_servers"]["managed"]["command"].as_str(),
            Some("npx")
        );
        assert!(document["mcp_servers"].get("local").is_none());
        assert!(test.grok_home.join("skills").join("demo").exists());

        apply(&test.handle(), &test.db, default_id).expect("restore default workspace");

        test.assert_active(default_id);
        let restored = std::fs::read_to_string(test.grok_home.join("config.toml"))
            .expect("read restored config");
        let restored = restored
            .parse::<toml_edit::DocumentMut>()
            .expect("valid restored TOML");
        assert_eq!(
            restored["mcp_servers"]["local"]["command"].as_str(),
            Some("local")
        );
        assert!(restored["mcp_servers"].get("managed").is_none());
        assert_eq!(
            restored["model"]["aio"]["model"].as_str(),
            Some("grok-build")
        );
        assert!(!test.grok_home.join("skills").join("demo").exists());
    }

    #[test]
    fn grok_workspace_prompt_failure_restores_files_and_active_workspace() {
        let test = GrokWorkspaceTestApp::new();
        let default_id = test.default_workspace_id();
        let target_id = test.target_workspace_id();
        test.set_prompt(target_id, &"x".repeat(1024 * 1024 + 1));
        test.write_config(INITIAL_CONFIG.as_bytes());
        test.write_prompt("original instructions");

        let error =
            apply(&test.handle(), &test.db, target_id).expect_err("oversized prompt must fail");

        assert!(error.to_string().contains("too large"));
        test.assert_active(default_id);
        assert_eq!(
            std::fs::read(test.grok_home.join("config.toml")).expect("read config"),
            INITIAL_CONFIG.as_bytes()
        );
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "original instructions"
        );
        assert!(prompt_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read prompt manifest")
            .is_none());
    }

    #[test]
    fn grok_workspace_mcp_failure_rolls_back_prompt_config_and_manifests() {
        let test = GrokWorkspaceTestApp::new();
        let default_id = test.default_workspace_id();
        let target_id = test.target_workspace_id();
        test.set_prompt(target_id, "target instructions");
        test.add_mcp(target_id);
        let invalid = b"[mcp_servers\ninvalid = true\n";
        test.write_config(invalid);
        test.write_prompt("original instructions");

        let error =
            apply(&test.handle(), &test.db, target_id).expect_err("invalid Grok TOML must fail");

        assert!(error.to_string().contains("GROK_CONFIG_INVALID_TOML"));
        test.assert_active(default_id);
        assert_eq!(
            std::fs::read(test.grok_home.join("config.toml")).expect("read config"),
            invalid
        );
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "original instructions"
        );
        assert!(prompt_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read prompt manifest")
            .is_none());
        assert!(mcp_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read MCP manifest")
            .is_none());
    }

    #[test]
    fn grok_workspace_skills_failure_rolls_back_prompt_mcp_and_active_workspace() {
        let test = GrokWorkspaceTestApp::new();
        let default_id = test.default_workspace_id();
        let target_id = test.target_workspace_id();
        test.set_prompt(target_id, "target instructions");
        test.add_mcp(target_id);
        test.add_skill(target_id, false);
        test.write_config(INITIAL_CONFIG.as_bytes());
        test.write_prompt("original instructions");

        let error =
            apply(&test.handle(), &test.db, target_id).expect_err("missing SSOT skill must fail");

        assert!(error.to_string().contains("SKILL_SSOT_MISSING"));
        test.assert_active(default_id);
        assert_eq!(
            std::fs::read(test.grok_home.join("config.toml")).expect("read config"),
            INITIAL_CONFIG.as_bytes()
        );
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "original instructions"
        );
        assert!(prompt_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read prompt manifest")
            .is_none());
        assert!(mcp_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read MCP manifest")
            .is_none());
    }
}
