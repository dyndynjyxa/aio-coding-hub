use super::git_url::parse_github_owner_repo;
use super::limits::{
    SKILL_FILE_COUNT_MAX, SKILL_FILE_MAX_BYTES, SKILL_MD_MAX_BYTES, SKILL_SOURCE_METADATA_MAX_BYTES,
};
use super::repo_cache::{github_api_url, unzip_repo_zip};
use super::util::now_unix_nanos;
use std::io::{Cursor, Write};
use std::path::PathBuf;

fn make_temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{prefix}-{}", now_unix_nanos()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn parse_github_owner_repo_handles_common_urls() {
    assert_eq!(
        parse_github_owner_repo("https://github.com/owner/repo.git"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_github_owner_repo("git@github.com:Owner/Repo.git"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_github_owner_repo("https://github.com/owner/repo/tree/main/skills"),
        Some(("owner".to_string(), "repo".to_string()))
    );
    assert_eq!(
        parse_github_owner_repo("https://gitlab.com/owner/repo"),
        None
    );
}

#[test]
fn github_api_url_encodes_branch_path_segments() {
    let url = github_api_url(&["repos", "owner", "repo", "zipball", "feature/x"]).expect("url");
    let s = url.to_string();
    assert!(
        s.contains("feature%2Fx"),
        "expected encoded branch in url, got: {s}"
    );
}

#[test]
fn unzip_repo_zip_rejects_path_traversal_entries() {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buf);
    let opts = zip::write::FileOptions::<()>::default();

    zip.add_directory("repo/", opts).expect("add dir");
    zip.start_file("..\\evil.txt", opts).expect("start file");
    zip.write_all(b"evil").expect("write");
    zip.finish().expect("finish zip");

    let bytes = buf.into_inner();
    let out_dir = make_temp_dir("aio-unzip-test");
    let err = unzip_repo_zip(&bytes, &out_dir).unwrap_err().to_string();

    assert!(
        err.starts_with("SKILL_ZIP_ERROR:"),
        "unexpected error: {err}"
    );

    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn unzip_repo_zip_accepts_backslash_paths_inside_repo() {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buf);
    let opts = zip::write::FileOptions::<()>::default();

    zip.add_directory("repo\\", opts).expect("add dir");
    zip.add_directory("repo\\nested\\", opts)
        .expect("add nested dir");
    zip.start_file("repo\\nested\\SKILL.md", opts)
        .expect("start file");
    zip.write_all(b"---\nname: Test\n---\n").expect("write");
    zip.finish().expect("finish zip");

    let bytes = buf.into_inner();
    let out_dir = make_temp_dir("aio-unzip-test-ok");
    let repo_root = unzip_repo_zip(&bytes, &out_dir).expect("unzip");

    assert!(repo_root.join("nested").join("SKILL.md").exists());

    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn parse_skill_md_rejects_oversized_file() {
    let dir = make_temp_dir("aio-skill-md-large");
    let path = dir.join("SKILL.md");
    std::fs::write(&path, vec![b'x'; SKILL_MD_MAX_BYTES + 1]).expect("write large skill md");

    let err = super::skill_md::parse_skill_md(&path).expect_err("oversized SKILL.md should fail");

    assert!(err.contains("too large"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn copy_dir_recursive_rejects_oversized_file() {
    let src = make_temp_dir("aio-skill-copy-large-src");
    let dst = make_temp_dir("aio-skill-copy-large-dst");
    let dst = dst.join("copy");
    std::fs::write(src.join("SKILL.md"), "---\nname: Large\n---\n").expect("write skill md");
    std::fs::write(
        src.join("large.bin"),
        vec![b'x'; SKILL_FILE_MAX_BYTES as usize + 1],
    )
    .expect("write large file");

    let err = super::fs_ops::copy_dir_recursive(&src, &dst)
        .expect_err("oversized skill copy should fail");

    assert!(err.to_string().contains("too large"));
    let _ = std::fs::remove_dir_all(&src);
    let _ = std::fs::remove_dir_all(dst.parent().unwrap_or(&dst));
}

#[test]
fn copy_dir_recursive_rejects_too_many_files() {
    let src = make_temp_dir("aio-skill-copy-many-src");
    let dst = make_temp_dir("aio-skill-copy-many-dst");
    let dst = dst.join("copy");
    std::fs::write(src.join("SKILL.md"), "---\nname: Many\n---\n").expect("write skill md");
    for index in 0..SKILL_FILE_COUNT_MAX {
        std::fs::write(src.join(format!("{index}.txt")), b"x").expect("write file");
    }

    let err = super::fs_ops::copy_dir_recursive(&src, &dst)
        .expect_err("too many skill files should fail");

    assert!(err.to_string().contains("too many skill files"));
    let _ = std::fs::remove_dir_all(&src);
    let _ = std::fs::remove_dir_all(dst.parent().unwrap_or(&dst));
}

#[test]
fn read_source_metadata_rejects_oversized_file() {
    let dir = make_temp_dir("aio-skill-source-large");
    std::fs::write(
        dir.join(".aio-coding-hub.source.json"),
        vec![b'x'; SKILL_SOURCE_METADATA_MAX_BYTES + 1],
    )
    .expect("write large source metadata");

    let err = super::fs_ops::read_source_metadata(&dir)
        .expect_err("oversized source metadata should fail");

    assert!(err.to_string().contains("too large"));
    let _ = std::fs::remove_dir_all(&dir);
}
