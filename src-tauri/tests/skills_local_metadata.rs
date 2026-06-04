mod support;

use rusqlite::params;
use support::SkillTestFixture;

const SOURCE_METADATA_FILE: &str = ".aio-coding-hub.source.json";

#[test]
fn skills_local_list_skips_managed_ssot_links() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Managed Link List");

    aio_coding_hub_lib::test_support::skill_set_enabled_json(
        &handle,
        fix.workspace_id,
        fix.skill_id,
        true,
    )
    .expect("enable managed skill");

    let rows = support::json_array(
        aio_coding_hub_lib::test_support::skills_local_list_json(&handle, fix.workspace_id)
            .expect("list local skills"),
    );

    assert!(
        rows.iter()
            .all(|row| support::json_str(row, "dir_name") != fix.skill_key),
        "managed skill link should not be reported as a local skill: {rows:?}"
    );
}

#[test]
fn skills_local_list_accepts_case_insensitive_skill_md_file_name() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Mixed Case Skill Md");

    let dir_name = "mixed-case-md";
    let local_dir = fix.cli_skills_root.join(dir_name);
    std::fs::create_dir_all(&local_dir).expect("create local skill dir");
    std::fs::write(
        local_dir.join("Skill.md"),
        "---\nname: Mixed Case Skill Md\ndescription: Mixed case file name\n---\n",
    )
    .expect("write local skill md");

    let rows = support::json_array(
        aio_coding_hub_lib::test_support::skills_local_list_json(&handle, fix.workspace_id)
            .expect("list local skills"),
    );
    let row = rows
        .iter()
        .find(|row| support::json_str(row, "dir_name") == dir_name)
        .expect("mixed-case skill row");

    assert_eq!(support::json_str(row, "name"), "Mixed Case Skill Md");
}

#[test]
fn skills_local_list_fails_when_source_metadata_is_invalid() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Local Metadata List");

    let local_dir = fix.cli_skills_root.join("broken-metadata");
    std::fs::create_dir_all(&local_dir).expect("create local skill dir");
    std::fs::write(local_dir.join("SKILL.md"), "name: Broken Metadata Skill\n")
        .expect("write local skill md");
    std::fs::write(local_dir.join(SOURCE_METADATA_FILE), b"{invalid json")
        .expect("write invalid source metadata");

    let err = aio_coding_hub_lib::test_support::skills_local_list_json(&handle, fix.workspace_id)
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("failed to parse source metadata"),
        "unexpected error: {err}"
    );
}

#[test]
fn skill_import_local_fails_when_source_metadata_is_invalid() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Local Metadata Import");

    let local_dir = fix.cli_skills_root.join("broken-metadata");
    std::fs::create_dir_all(&local_dir).expect("create local skill dir");
    std::fs::write(local_dir.join("SKILL.md"), "name: Broken Metadata Skill\n")
        .expect("write local skill md");
    std::fs::write(local_dir.join(SOURCE_METADATA_FILE), b"{invalid json")
        .expect("write invalid source metadata");

    let err = aio_coding_hub_lib::test_support::skill_import_local_json(
        &handle,
        fix.workspace_id,
        "broken-metadata",
    )
    .unwrap_err()
    .to_string();

    assert!(
        err.contains("failed to parse source metadata"),
        "unexpected error: {err}"
    );

    let imported_count: i64 = fix
        .conn
        .query_row(
            "SELECT COUNT(1) FROM skills WHERE skill_key = ?1",
            params!["broken-metadata"],
            |row| row.get(0),
        )
        .expect("count imported skills");
    assert_eq!(
        imported_count, 0,
        "invalid metadata skill should not be imported"
    );
    assert!(
        !local_dir.join(".aio-coding-hub.managed").exists(),
        "import should not mark the local dir as managed"
    );
}

#[test]
fn skill_without_npx_lock_can_still_be_listed_and_imported_as_local() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex No Npx Lock");

    let dir_name = "plain-local-only-aio-test-skill";
    let local_dir = fix.cli_skills_root.join(dir_name);
    std::fs::create_dir_all(&local_dir).expect("create local skill dir");
    std::fs::write(
        local_dir.join("SKILL.md"),
        "---\nname: Plain Local Only AIO Test Skill\ndescription: Local only\n---\n",
    )
    .expect("write local skill md");

    let rows = support::json_array(
        aio_coding_hub_lib::test_support::skills_local_list_json(&handle, fix.workspace_id)
            .expect("list local skills"),
    );
    let row = rows
        .iter()
        .find(|row| support::json_str(row, "dir_name") == dir_name)
        .expect("local-only skill row");

    assert!(
        row.get("source_git_url")
            .map(serde_json::Value::is_null)
            .unwrap_or(true),
        "local-only skill should not require npx source metadata"
    );

    let imported = aio_coding_hub_lib::test_support::skill_import_local_json(
        &handle,
        fix.workspace_id,
        dir_name,
    )
    .expect("import local-only skill");

    assert_eq!(
        support::json_str(&imported, "source_git_url"),
        "local://codex"
    );
    assert_eq!(support::json_str(&imported, "source_branch"), "local");
    assert_eq!(support::json_str(&imported, "source_subdir"), dir_name);
}

#[test]
fn skills_local_list_infers_npx_skills_lock_source_metadata() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Npx Lock List");

    let agents_dir = app.home_dir().join(".agents");
    std::fs::create_dir_all(&agents_dir).expect("create agents dir");
    std::fs::write(
        agents_dir.join(".skill-lock.json"),
        serde_json::json!({
            "schemaVersion": 1,
            "skills": {
                "review-skill": {
                    "source": "github",
                    "sourceUrl": "https://github.com/acme/skills",
                    "ref": "main",
                    "skillPath": "packs/review-skill/SKILL.md",
                    "skillFolderHash": "sha256-test"
                }
            }
        })
        .to_string(),
    )
    .expect("write npx skills lock");

    let local_dir = fix.cli_skills_root.join("review-skill");
    std::fs::create_dir_all(&local_dir).expect("create local skill dir");
    std::fs::write(
        local_dir.join("SKILL.md"),
        "---\nname: Review Skill\ndescription: Review pull requests\n---\n",
    )
    .expect("write local skill md");

    let rows = support::json_array(
        aio_coding_hub_lib::test_support::skills_local_list_json(&handle, fix.workspace_id)
            .expect("list local skills"),
    );
    let row = rows
        .iter()
        .find(|row| support::json_str(row, "dir_name") == "review-skill")
        .expect("review skill row");

    assert_eq!(
        support::json_str(row, "source_git_url"),
        "https://github.com/acme/skills"
    );
    assert_eq!(support::json_str(row, "source_branch"), "main");
    assert_eq!(
        support::json_str(row, "source_subdir"),
        "packs/review-skill"
    );
}

#[test]
fn skills_local_list_continues_after_empty_xdg_npx_lock() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Npx Lock Fallback");

    let state_home = std::path::PathBuf::from(std::env::var_os("XDG_STATE_HOME").expect("xdg"));
    let xdg_agents_dir = state_home.join(".agents");
    std::fs::create_dir_all(&xdg_agents_dir).expect("create xdg agents dir");
    std::fs::write(
        xdg_agents_dir.join(".skill-lock.json"),
        serde_json::json!({ "skills": {} }).to_string(),
    )
    .expect("write empty xdg npx skills lock");

    let agents_dir = app.home_dir().join(".agents");
    std::fs::create_dir_all(&agents_dir).expect("create home agents dir");
    std::fs::write(
        agents_dir.join(".skill-lock.json"),
        serde_json::json!({
            "skills": {
                "review-skill": {
                    "source": "github",
                    "sourceUrl": "https://github.com/acme/skills",
                    "ref": "main",
                    "skillPath": "packs/review-skill/SKILL.md"
                }
            }
        })
        .to_string(),
    )
    .expect("write home npx skills lock");

    let local_dir = fix.cli_skills_root.join("review-skill");
    std::fs::create_dir_all(&local_dir).expect("create local skill dir");
    std::fs::write(
        local_dir.join("SKILL.md"),
        "---\nname: Review Skill\ndescription: Review pull requests\n---\n",
    )
    .expect("write local skill md");

    let rows = support::json_array(
        aio_coding_hub_lib::test_support::skills_local_list_json(&handle, fix.workspace_id)
            .expect("list local skills"),
    );
    let row = rows
        .iter()
        .find(|row| support::json_str(row, "dir_name") == "review-skill")
        .expect("review skill row");

    assert_eq!(
        support::json_str(row, "source_git_url"),
        "https://github.com/acme/skills"
    );
}

#[test]
fn skill_import_local_uses_npx_lock_source_and_records_content_hash() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Npx Lock Import");

    let agents_dir = app.home_dir().join(".agents");
    std::fs::create_dir_all(&agents_dir).expect("create agents dir");
    std::fs::write(
        agents_dir.join(".skill-lock.json"),
        serde_json::json!({
            "skills": {
                "review-skill": {
                    "sourceUrl": "https://github.com/acme/skills",
                    "ref": "main",
                    "skillPath": "packs/review-skill/SKILL.md"
                }
            }
        })
        .to_string(),
    )
    .expect("write npx skills lock");

    let local_dir = fix.cli_skills_root.join("review-skill");
    std::fs::create_dir_all(&local_dir).expect("create local skill dir");
    std::fs::write(
        local_dir.join("SKILL.md"),
        "---\nname: Review Skill\ndescription: Review pull requests\n---\n",
    )
    .expect("write local skill md");
    std::fs::write(local_dir.join("guide.md"), "review carefully\n").expect("write skill file");

    let imported = aio_coding_hub_lib::test_support::skill_import_local_json(
        &handle,
        fix.workspace_id,
        "review-skill",
    )
    .expect("import local skill");

    let skill_id = support::json_i64(&imported, "id");
    assert!(skill_id > 0);
    assert_eq!(
        support::json_str(&imported, "source_git_url"),
        "https://github.com/acme/skills"
    );
    assert_eq!(support::json_str(&imported, "source_branch"), "main");
    assert_eq!(
        support::json_str(&imported, "source_subdir"),
        "packs/review-skill"
    );

    let content_hash: Option<String> = fix
        .conn
        .query_row(
            "SELECT installed_content_hash FROM skills WHERE id = ?1",
            params![skill_id],
            |row| row.get(0),
        )
        .expect("query content hash");

    assert!(
        content_hash
            .as_deref()
            .is_some_and(|value| value.starts_with("sha256:")),
        "expected installed_content_hash, got {content_hash:?}"
    );
}
