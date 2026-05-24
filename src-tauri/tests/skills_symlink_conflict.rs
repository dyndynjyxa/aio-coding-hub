mod support;

use rusqlite::params;
use support::{json_bool, SkillTestFixture};

#[cfg(unix)]
fn symlink_dir(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn symlink_dir(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

#[test]
fn skills_enable_and_uninstall_do_not_conflict_with_unmanaged_symlink_dir() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Link Workspace");

    std::fs::create_dir_all(&fix.cli_skills_root).expect("create codex skills root");

    let external_link_src = app.home_dir().join(".external-links").join(&fix.skill_key);
    std::fs::create_dir_all(&external_link_src).expect("create external link src");
    std::fs::write(
        external_link_src.join("SKILL.md"),
        "name: Context7 External\n",
    )
    .expect("write external skill");

    let linked_skill_dir = fix.cli_skills_root.join(&fix.skill_key);
    symlink_dir(&external_link_src, &linked_skill_dir).expect("create unmanaged symlink dir");
    assert!(
        std::fs::symlink_metadata(&linked_skill_dir)
            .expect("symlink metadata")
            .file_type()
            .is_symlink(),
        "expected codex skills entry to be symlink"
    );
    assert!(
        !linked_skill_dir.join(".aio-coding-hub.managed").exists(),
        "symlink dir should be unmanaged"
    );

    let enabled = aio_coding_hub_lib::test_support::skill_set_enabled_json(
        &handle,
        fix.workspace_id,
        fix.skill_id,
        true,
    )
    .expect("enable skill with unmanaged symlink present");
    assert!(
        json_bool(&enabled, "enabled"),
        "skill should be enabled even when unmanaged symlink exists"
    );

    assert!(
        std::fs::symlink_metadata(&linked_skill_dir)
            .expect("symlink metadata after enable")
            .file_type()
            .is_symlink(),
        "unmanaged symlink should stay untouched after enable"
    );

    aio_coding_hub_lib::test_support::skill_uninstall(&handle, fix.skill_id)
        .expect("uninstall should not be blocked by unmanaged symlink");

    let remaining: i64 = fix
        .conn
        .query_row(
            "SELECT COUNT(1) FROM skills WHERE id = ?1",
            params![fix.skill_id],
            |row| row.get(0),
        )
        .expect("count skills");
    assert_eq!(remaining, 0, "skill row should be deleted");

    assert!(
        std::fs::symlink_metadata(&linked_skill_dir)
            .expect("symlink metadata after uninstall")
            .file_type()
            .is_symlink(),
        "unmanaged symlink should remain after uninstall"
    );
}

#[test]
fn skills_enable_recovers_missing_ssot_from_local_source() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Recover SSOT");

    std::fs::remove_dir_all(&fix.ssot_skill_dir).expect("remove ssot dir to simulate drift");

    fix.conn
        .execute(
            r#"
UPDATE skills
SET source_git_url = ?1, source_branch = 'local', source_subdir = ?2
WHERE id = ?3
"#,
            params!["local://codex", &fix.skill_key, fix.skill_id],
        )
        .expect("update local source");

    let local_skill_dir = fix.cli_skills_root.join(&fix.skill_key);
    std::fs::create_dir_all(&local_skill_dir).expect("create local skill dir");
    std::fs::write(local_skill_dir.join("SKILL.md"), "name: Context7 Local\n")
        .expect("write local skill md");

    let enabled = aio_coding_hub_lib::test_support::skill_set_enabled_json(
        &handle,
        fix.workspace_id,
        fix.skill_id,
        true,
    )
    .expect("enable should recover missing ssot from local source");
    assert!(json_bool(&enabled, "enabled"), "skill should be enabled");

    assert!(fix.ssot_skill_dir.exists(), "ssot dir should be recreated");
    assert!(
        fix.ssot_skill_dir.join("SKILL.md").exists(),
        "ssot dir should contain SKILL.md"
    );
    assert!(
        local_skill_dir.exists(),
        "local skill dir should stay untouched after enable"
    );
}

#[test]
fn skill_uninstall_does_not_block_on_unmanaged_plain_skill_dir() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Uninstall Plain Dir");

    let claude_skill_dir = app
        .home_dir()
        .join(".claude")
        .join("skills")
        .join(&fix.skill_key);
    std::fs::create_dir_all(&claude_skill_dir).expect("create unmanaged claude skill dir");
    std::fs::write(claude_skill_dir.join("SKILL.md"), "name: Context7 Claude\n")
        .expect("write unmanaged claude skill");

    aio_coding_hub_lib::test_support::skill_uninstall(&handle, fix.skill_id)
        .expect("uninstall should not be blocked by unmanaged plain skill dir");

    let remaining: i64 = fix
        .conn
        .query_row(
            "SELECT COUNT(1) FROM skills WHERE id = ?1",
            params![fix.skill_id],
            |row| row.get(0),
        )
        .expect("count skills");
    assert_eq!(remaining, 0, "skill row should be deleted");

    assert!(
        claude_skill_dir.exists(),
        "unmanaged plain skill dir should remain after uninstall"
    );
}

#[test]
fn skill_uninstall_removes_managed_ssot_link_without_deleting_ssot_target_first() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Managed Link");

    let enabled = aio_coding_hub_lib::test_support::skill_set_enabled_json(
        &handle,
        fix.workspace_id,
        fix.skill_id,
        true,
    )
    .expect("enable skill");
    assert!(json_bool(&enabled, "enabled"), "skill should be enabled");

    let linked_skill_dir = fix.cli_skills_root.join(&fix.skill_key);
    assert!(
        std::fs::symlink_metadata(&linked_skill_dir)
            .expect("managed link metadata")
            .file_type()
            .is_symlink(),
        "active codex skill target should be a managed symlink"
    );
    assert!(
        fix.ssot_skill_dir.join("SKILL.md").exists(),
        "ssot target should exist before uninstall"
    );

    aio_coding_hub_lib::test_support::skill_uninstall(&handle, fix.skill_id)
        .expect("uninstall managed link");

    assert!(
        std::fs::symlink_metadata(&linked_skill_dir).is_err(),
        "managed symlink should be removed"
    );
    assert!(
        !fix.ssot_skill_dir.exists(),
        "ssot target should be deleted by uninstall after links are removed"
    );
}

#[cfg(unix)]
#[test]
fn skill_uninstall_removes_broken_managed_ssot_symlink() {
    let app = support::TestApp::new();
    let handle = app.handle();

    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
    let fix = SkillTestFixture::new(&app, &handle, "codex", "Codex Broken Managed Link");

    let enabled = aio_coding_hub_lib::test_support::skill_set_enabled_json(
        &handle,
        fix.workspace_id,
        fix.skill_id,
        true,
    )
    .expect("enable skill");
    assert!(json_bool(&enabled, "enabled"), "skill should be enabled");

    let linked_skill_dir = fix.cli_skills_root.join(&fix.skill_key);
    assert!(
        std::fs::symlink_metadata(&linked_skill_dir)
            .expect("managed link metadata")
            .file_type()
            .is_symlink(),
        "active codex skill target should be a managed symlink"
    );

    std::fs::remove_dir_all(&fix.ssot_skill_dir).expect("remove ssot target");
    assert!(
        !linked_skill_dir.exists(),
        "broken symlink should be false through Path::exists"
    );
    assert!(
        std::fs::symlink_metadata(&linked_skill_dir).is_ok(),
        "broken symlink should still exist at the directory entry level"
    );

    aio_coding_hub_lib::test_support::skill_uninstall(&handle, fix.skill_id)
        .expect("uninstall should clean broken managed link");

    assert!(
        std::fs::symlink_metadata(&linked_skill_dir).is_err(),
        "broken managed symlink should be removed"
    );
}
