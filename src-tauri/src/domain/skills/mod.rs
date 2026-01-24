//! Usage: Skills domain (repositories, installed skills, local import, and CLI integration).

mod discover;
mod fs_ops;
mod git_url;
mod installed;
mod local;
mod ops;
mod paths;
mod repo_cache;
mod repos;
mod skill_md;
mod types;
mod util;

pub use discover::discover_available;
pub use installed::installed_list;
pub use local::{import_local, local_list};
pub use ops::{install, set_enabled, uninstall};
pub use paths::paths_get;
pub use repos::{repo_delete, repo_upsert, repos_list};
pub use types::{
    AvailableSkillSummary, InstalledSkillSummary, LocalSkillSummary, SkillRepoSummary, SkillsPaths,
};

// Keep unit tests stable while internals move into submodules.
#[cfg(test)]
use git_url::parse_github_owner_repo;
#[cfg(test)]
use repo_cache::{github_api_url, unzip_repo_zip};
#[cfg(test)]
use util::now_unix_nanos;

#[cfg(test)]
mod tests {
    use super::{github_api_url, now_unix_nanos, parse_github_owner_repo, unzip_repo_zip};
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
        let err = unzip_repo_zip(&bytes, &out_dir).unwrap_err();

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
}
