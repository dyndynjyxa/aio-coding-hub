mod support;

use rusqlite::params;
use std::path::Path;
use std::process::Command;

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_repo_skill(repo: &Path, version: &str) {
    let skill_dir = repo.join("skills").join("context7");
    std::fs::create_dir_all(&skill_dir).expect("create repo skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: Context7 {version}\ndescription: Context skill {version}\n---\n"),
    )
    .expect("write repo skill md");
    std::fs::write(skill_dir.join("guide.md"), format!("guide {version}\n"))
        .expect("write repo guide");
}

fn commit_repo(repo: &Path, message: &str) {
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", message]);
}

fn create_skill_repo(root: &Path) {
    std::fs::create_dir_all(root).expect("create repo");
    run_git(root, &["init"]);
    run_git(root, &["checkout", "-B", "main"]);
    run_git(root, &["config", "user.email", "skill-test@example.com"]);
    run_git(root, &["config", "user.name", "Skill Test"]);
    write_repo_skill(root, "v1");
    commit_repo(root, "skill v1");
}

#[test]
fn skill_update_replaces_content_in_place_and_preserves_workspace_enablements() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let repo_dir = app.home_dir().join("remote-skills-repo");
    create_skill_repo(&repo_dir);

    let workspace1 = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "codex",
        "Codex One",
        false,
    )
    .expect("create workspace 1");
    let workspace1_id = support::json_i64(&workspace1, "id");
    assert!(workspace1_id > 0);

    let db_path = aio_coding_hub_lib::test_support::db_path(&handle).expect("db path");
    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    conn.execute(
        r#"
INSERT INTO workspace_active(cli_key, workspace_id, updated_at)
VALUES ('codex', ?1, 1)
ON CONFLICT(cli_key) DO UPDATE SET
  workspace_id = excluded.workspace_id,
  updated_at = excluded.updated_at
"#,
        params![workspace1_id],
    )
    .expect("set workspace 1 active");

    let installed = aio_coding_hub_lib::test_support::skill_install_json(
        &handle,
        workspace1_id,
        &repo_dir.to_string_lossy(),
        "main",
        "skills/context7",
        true,
    )
    .expect("install skill");
    let skill_id = support::json_i64(&installed, "id");
    assert!(skill_id > 0);

    let workspace2 = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "codex",
        "Codex Two",
        true,
    )
    .expect("create workspace 2");
    let workspace2_id = support::json_i64(&workspace2, "id");
    assert!(workspace2_id > 0);

    let before_enabled: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_skill_enabled WHERE skill_id = ?1",
            params![skill_id],
            |row| row.get(0),
        )
        .expect("count enablements before update");
    assert_eq!(before_enabled, 2);

    write_repo_skill(&repo_dir, "v2");
    commit_repo(&repo_dir, "skill v2");

    let update_rows = support::json_array(
        aio_coding_hub_lib::test_support::skill_check_updates_json(&handle, workspace1_id)
            .expect("check updates"),
    );
    let update_info = update_rows
        .iter()
        .find(|row| support::json_i64(row, "skill_id") == skill_id)
        .expect("update info for local git source");
    assert!(support::json_bool(update_info, "has_update"));

    let updated =
        aio_coding_hub_lib::test_support::skill_update_json(&handle, workspace1_id, skill_id)
            .expect("update skill");

    assert_eq!(support::json_i64(&updated, "id"), skill_id);
    assert_eq!(support::json_str(&updated, "name"), "Context7 v2");

    let after_enabled: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_skill_enabled WHERE skill_id = ?1",
            params![skill_id],
            |row| row.get(0),
        )
        .expect("count enablements after update");
    assert_eq!(after_enabled, 2);

    let workspace2_enabled: i64 = conn
        .query_row(
            r#"
SELECT COUNT(1)
FROM workspace_skill_enabled
WHERE skill_id = ?1 AND workspace_id = ?2
"#,
            params![skill_id, workspace2_id],
            |row| row.get(0),
        )
        .expect("count workspace 2 enablement");
    assert_eq!(workspace2_enabled, 1);

    let app_data_dir =
        aio_coding_hub_lib::test_support::app_data_dir(&handle).expect("app data dir");
    let guide_path = app_data_dir
        .join("skills")
        .join("context7-v1")
        .join("guide.md");
    assert_eq!(
        std::fs::read_to_string(&guide_path).expect("read updated guide"),
        "guide v2\n"
    );
}

#[test]
fn skill_update_sync_failure_restores_previous_content_and_metadata() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let repo_dir = app.home_dir().join("remote-skills-repo");
    create_skill_repo(&repo_dir);

    let workspace = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "codex",
        "Codex Sync Rollback",
        false,
    )
    .expect("create codex workspace");
    let workspace_id = support::json_i64(&workspace, "id");

    let db_path = aio_coding_hub_lib::test_support::db_path(&handle).expect("db path");
    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    conn.execute(
        r#"
INSERT INTO workspace_active(cli_key, workspace_id, updated_at)
VALUES ('codex', ?1, 1)
ON CONFLICT(cli_key) DO UPDATE SET
  workspace_id = excluded.workspace_id,
  updated_at = excluded.updated_at
"#,
        params![workspace_id],
    )
    .expect("set codex workspace active");

    let installed = aio_coding_hub_lib::test_support::skill_install_json(
        &handle,
        workspace_id,
        &repo_dir.to_string_lossy(),
        "main",
        "skills/context7",
        true,
    )
    .expect("install skill");
    let skill_id = support::json_i64(&installed, "id");
    let skill_key = support::json_str(&installed, "skill_key");

    let old_content_hash: Option<String> = conn
        .query_row(
            "SELECT installed_content_hash FROM skills WHERE id = ?1",
            params![skill_id],
            |row| row.get(0),
        )
        .expect("query old content hash");

    let gemini_workspace = aio_coding_hub_lib::test_support::workspace_create_json(
        &handle,
        "gemini",
        "Gemini Sync Rollback",
        false,
    )
    .expect("create gemini workspace");
    let gemini_workspace_id = support::json_i64(&gemini_workspace, "id");
    conn.execute(
        r#"
INSERT INTO workspace_active(cli_key, workspace_id, updated_at)
VALUES ('gemini', ?1, 1)
ON CONFLICT(cli_key) DO UPDATE SET
  workspace_id = excluded.workspace_id,
  updated_at = excluded.updated_at
"#,
        params![gemini_workspace_id],
    )
    .expect("set gemini workspace active");
    conn.execute(
        r#"
INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
VALUES (?1, ?2, 1, 1)
"#,
        params![gemini_workspace_id, skill_id],
    )
    .expect("enable skill in gemini workspace");

    let gemini_skills_root = app.home_dir().join(".gemini").join("skills");
    std::fs::create_dir_all(&gemini_skills_root).expect("create gemini skills root");
    std::fs::write(gemini_skills_root.join(&skill_key), "unmanaged blocker\n")
        .expect("write unmanaged blocking target");

    write_repo_skill(&repo_dir, "v2");
    commit_repo(&repo_dir, "skill v2");

    let err = aio_coding_hub_lib::test_support::skill_update_json(&handle, workspace_id, skill_id)
        .expect_err("update should fail when sync fails")
        .to_string();
    assert!(
        err.contains("SKILL_UPDATE_SYNC_FAILED"),
        "unexpected error: {err}"
    );

    let app_data_dir =
        aio_coding_hub_lib::test_support::app_data_dir(&handle).expect("app data dir");
    let guide_path = app_data_dir
        .join("skills")
        .join(&skill_key)
        .join("guide.md");
    assert_eq!(
        std::fs::read_to_string(&guide_path).expect("read restored guide"),
        "guide v1\n"
    );

    let (name, content_hash): (String, Option<String>) = conn
        .query_row(
            "SELECT name, installed_content_hash FROM skills WHERE id = ?1",
            params![skill_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query restored metadata");
    assert_eq!(name, "Context7 v1");
    assert_eq!(content_hash, old_content_hash);
}
