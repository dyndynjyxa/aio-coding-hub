use super::git_url::{normalize_repo_branch, parse_github_owner_repo};
use super::paths::repos_root;
use super::util::now_unix_nanos;
use std::io::{Cursor, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

const REPO_BRANCH_FILE: &str = ".aio-coding-hub.repo-branch";
const REPO_SNAPSHOT_MARKER_FILE: &str = ".aio-coding-hub.repo-snapshot";

fn fnv1a64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn repo_cache_dir(app: &tauri::AppHandle, git_url: &str, branch: &str) -> Result<PathBuf, String> {
    let root = repos_root(app)?;
    let key = format!("{}#{}", git_url.trim(), branch.trim());
    Ok(root.join(format!("{:016x}", fnv1a64(&key))))
}

struct RepoLockGuard {
    path: PathBuf,
    file: Option<std::fs::File>,
}

impl RepoLockGuard {
    fn acquire(path: PathBuf) -> Result<Self, String> {
        fn is_stale(lock_path: &Path, stale_after: Duration) -> bool {
            let Ok(meta) = std::fs::metadata(lock_path) else {
                return false;
            };
            let Ok(modified) = meta.modified() else {
                return false;
            };
            let Ok(age) = SystemTime::now().duration_since(modified) else {
                return false;
            };
            age > stale_after
        }

        let stale_after = Duration::from_secs(120);
        let deadline = SystemTime::now() + Duration::from_secs(30);

        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    let _ = writeln!(
                        file,
                        "pid={} ts_nanos={}",
                        std::process::id(),
                        now_unix_nanos()
                    );
                    return Ok(Self {
                        path,
                        file: Some(file),
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if is_stale(&path, stale_after) {
                        let _ = std::fs::remove_file(&path);
                        continue;
                    }
                    if SystemTime::now() > deadline {
                        return Err(format!(
                            "SKILL_REPO_LOCK_TIMEOUT: failed to acquire repo lock {}",
                            path.display()
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(err) => {
                    return Err(format!(
                        "SKILL_REPO_LOCK_ERROR: failed to create repo lock {}: {err}",
                        path.display()
                    ));
                }
            }
        }
    }
}

impl Drop for RepoLockGuard {
    fn drop(&mut self) {
        let _ = self.file.take();
        let _ = std::fs::remove_file(&self.path);
    }
}

fn lock_path_for_repo_dir(dir: &Path) -> PathBuf {
    dir.with_extension("lock")
}

fn remove_path_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
        return Ok(());
    }
    std::fs::remove_file(path).map_err(|e| format!("failed to remove {}: {e}", path.display()))
}

fn run_git(mut cmd: Command) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("SKILL_GIT_NOT_FOUND: failed to execute git: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let msg = if !stderr.is_empty() { stderr } else { stdout };
    Err(format!("SKILL_GIT_ERROR: {msg}"))
}

fn run_git_capture(mut cmd: Command) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("SKILL_GIT_NOT_FOUND: failed to execute git: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let msg = if !stderr.is_empty() { stderr } else { stdout };
    Err(format!("SKILL_GIT_ERROR: {msg}"))
}

fn is_remote_branch_not_found(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    (e.contains("remote branch") && e.contains("not found"))
        || e.contains("couldn't find remote ref")
        || e.contains("could not find remote ref")
}

fn read_repo_branch(dir: &Path) -> Option<String> {
    let path = dir.join(REPO_BRANCH_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let branch = text.trim().to_string();
    if branch.is_empty() {
        return None;
    }
    Some(branch)
}

fn write_repo_branch(dir: &Path, branch: &str) -> Result<(), String> {
    let path = dir.join(REPO_BRANCH_FILE);
    std::fs::write(&path, format!("{}\n", branch.trim()))
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    Ok(())
}

fn detect_checked_out_branch(dir: &Path) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD");
    let out = run_git_capture(cmd)?;
    let branch = out.trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        return Err("SKILL_GIT_ERROR: failed to detect current branch".to_string());
    }
    Ok(branch)
}

fn build_github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent(format!("aio-coding-hub/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("SKILL_HTTP_ERROR: failed to build http client: {e}"))
}

pub(super) fn github_api_url(segments: &[&str]) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse("https://api.github.com")
        .map_err(|e| format!("SKILL_GITHUB_URL_ERROR: {e}"))?;
    {
        let mut ps = url
            .path_segments_mut()
            .map_err(|_| "SKILL_GITHUB_URL_ERROR: invalid github api base url".to_string())?;
        for seg in segments {
            ps.push(seg);
        }
    }
    Ok(url)
}

fn github_default_branch(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
) -> Result<String, String> {
    let url = github_api_url(&["repos", owner, repo])?;
    let client = client.clone();
    tauri::async_runtime::block_on(async move {
        let resp = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("SKILL_HTTP_ERROR: github request failed: {e}"))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("SKILL_HTTP_ERROR: failed to read github response: {e}"))?;

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err("SKILL_GITHUB_REPO_NOT_FOUND: repository not found".to_string());
        }
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(
                "SKILL_GITHUB_FORBIDDEN: github request forbidden (rate limit?)".to_string(),
            );
        }
        if !status.is_success() {
            return Err(format!(
                "SKILL_GITHUB_HTTP_ERROR: github returned http status {}",
                status
            ));
        }

        let root: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("SKILL_GITHUB_PARSE_ERROR: github json parse failed: {e}"))?;
        let branch = root
            .get("default_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if branch.is_empty() {
            return Err("SKILL_GITHUB_PARSE_ERROR: missing default_branch".to_string());
        }
        Ok(branch.to_string())
    })
}

fn github_download_zipball(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    r#ref: &str,
) -> Result<Vec<u8>, String> {
    let url = github_api_url(&["repos", owner, repo, "zipball", r#ref])?;
    let client = client.clone();
    tauri::async_runtime::block_on(async move {
        let resp = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("SKILL_HTTP_ERROR: github zip download failed: {e}"))?;

        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err("SKILL_GITHUB_REF_NOT_FOUND: branch/ref not found".to_string());
        }
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(
                "SKILL_GITHUB_FORBIDDEN: github request forbidden (rate limit?)".to_string(),
            );
        }
        if !status.is_success() {
            return Err(format!(
                "SKILL_GITHUB_HTTP_ERROR: github returned http status {}",
                status
            ));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("SKILL_HTTP_ERROR: failed to read github zip body: {e}"))?;
        Ok(bytes.to_vec())
    })
}

pub(super) fn unzip_repo_zip(zip_bytes: &[u8], dst_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dst_dir)
        .map_err(|e| format!("failed to create {}: {e}", dst_dir.display()))?;

    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| format!("SKILL_ZIP_ERROR: failed to open zip archive: {e}"))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("SKILL_ZIP_ERROR: failed to read zip entry: {e}"))?;
        let name = file.name().replace('\\', "/");
        if name.is_empty() {
            continue;
        }

        let rel = Path::new(&name);
        if rel.is_absolute() {
            return Err("SKILL_ZIP_ERROR: invalid zip entry path (absolute)".to_string());
        }
        for comp in rel.components() {
            match comp {
                Component::CurDir | Component::Normal(_) => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err("SKILL_ZIP_ERROR: invalid zip entry path".to_string());
                }
            }
        }

        let out_path = dst_dir.join(rel);
        if file.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("failed to create {}: {e}", out_path.display()))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }

        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| format!("failed to create {}: {e}", out_path.display()))?;
        std::io::copy(&mut file, &mut out_file)
            .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
    }

    let mut top_dirs = Vec::new();
    let mut top_files = 0_usize;
    let entries = std::fs::read_dir(dst_dir)
        .map_err(|e| format!("failed to read dir {}: {e}", dst_dir.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", dst_dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            top_dirs.push(path);
        } else {
            top_files += 1;
        }
    }

    if top_dirs.len() != 1 || top_files != 0 {
        return Err(format!(
            "SKILL_ZIP_ERROR: expected single root directory in zip (dirs={}, files={})",
            top_dirs.len(),
            top_files
        ));
    }

    Ok(top_dirs.remove(0))
}

fn repo_snapshot_marker_path(dir: &Path) -> PathBuf {
    dir.join(REPO_SNAPSHOT_MARKER_FILE)
}

fn write_repo_snapshot_marker(dir: &Path, git_url: &str, branch: &str) -> Result<(), String> {
    let path = repo_snapshot_marker_path(dir);
    let content = format!(
        "aio-coding-hub\nmode=snapshot\ngit_url={}\nbranch={}\n",
        git_url.trim(),
        branch.trim()
    );
    std::fs::write(&path, content)
        .map_err(|e| format!("failed to write marker {}: {e}", path.display()))?;
    Ok(())
}

fn ensure_github_repo_snapshot(
    app: &tauri::AppHandle,
    git_url: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    refresh: bool,
) -> Result<PathBuf, String> {
    let dir = repo_cache_dir(app, git_url, branch)?;
    let snapshot_marker = repo_snapshot_marker_path(&dir);
    let git_dir = dir.join(".git");

    if !refresh && (snapshot_marker.exists() || git_dir.exists()) {
        return Ok(dir);
    }

    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    let _lock = RepoLockGuard::acquire(lock_path_for_repo_dir(&dir))?;

    let snapshot_marker = repo_snapshot_marker_path(&dir);
    let git_dir = dir.join(".git");
    if !refresh && (snapshot_marker.exists() || git_dir.exists()) {
        return Ok(dir);
    }

    // Self-heal: if the repo cache dir exists but isn't a git repo or a valid snapshot, remove it.
    if dir.exists() && !git_dir.exists() && !snapshot_marker.exists() {
        remove_path_if_exists(&dir)?;
    }

    let client = build_github_client()?;

    let mut effective_branch = String::new();
    let mut zip_bytes: Option<Vec<u8>> = None;
    let mut last_err: Option<String> = None;

    if branch == "auto" {
        // Common default branches: avoid GitHub API unless needed (rate limits).
        for candidate in ["main", "master"] {
            match github_download_zipball(&client, owner, repo, candidate) {
                Ok(bytes) => {
                    effective_branch = candidate.to_string();
                    zip_bytes = Some(bytes);
                    break;
                }
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }

        if zip_bytes.is_none() {
            match github_default_branch(&client, owner, repo) {
                Ok(default_branch) => {
                    match github_download_zipball(&client, owner, repo, &default_branch) {
                        Ok(bytes) => {
                            effective_branch = default_branch;
                            zip_bytes = Some(bytes);
                        }
                        Err(err) => {
                            last_err = Some(err);
                        }
                    }
                }
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }
    } else {
        match github_download_zipball(&client, owner, repo, branch) {
            Ok(bytes) => {
                effective_branch = branch.to_string();
                zip_bytes = Some(bytes);
            }
            Err(err) => {
                last_err = Some(err);
            }
        }
    }

    let Some(zip_bytes) = zip_bytes else {
        return Err(last_err.unwrap_or_else(|| {
            "SKILL_GITHUB_DOWNLOAD_FAILED: failed to download github zip".to_string()
        }));
    };
    if effective_branch.is_empty() {
        return Err("SKILL_GITHUB_BRANCH_ERROR: failed to resolve branch".to_string());
    }

    let parent = dir
        .parent()
        .ok_or_else(|| "SEC_INVALID_INPUT: invalid repo cache dir".to_string())?;
    let dir_name = dir
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("repo")
        .to_string();
    let nonce = now_unix_nanos();

    let staging = parent.join(format!(".{dir_name}.staging-{nonce}"));
    let _ = remove_path_if_exists(&staging);
    std::fs::create_dir_all(&staging)
        .map_err(|e| format!("failed to create {}: {e}", staging.display()))?;

    let extracted_root = match unzip_repo_zip(&zip_bytes, &staging) {
        Ok(v) => v,
        Err(err) => {
            let _ = remove_path_if_exists(&staging);
            return Err(err);
        }
    };

    write_repo_branch(&extracted_root, &effective_branch)?;
    write_repo_snapshot_marker(&extracted_root, git_url, &effective_branch)?;

    // Atomic-ish swap: move old dir away, then move new dir into place.
    let backup = parent.join(format!(".{dir_name}.old-{nonce}"));
    if dir.exists() && std::fs::rename(&dir, &backup).is_err() {
        if let Err(err) = remove_path_if_exists(&dir) {
            let _ = remove_path_if_exists(&staging);
            return Err(format!(
                "SKILL_REPO_BUSY: failed to replace {}: {err}",
                dir.display()
            ));
        }
    }

    if let Err(err) = std::fs::rename(&extracted_root, &dir) {
        let _ = remove_path_if_exists(&staging);
        if backup.exists() {
            let _ = std::fs::rename(&backup, &dir);
        }
        return Err(format!(
            "SKILL_REPO_UPDATE_FAILED: failed to activate repo snapshot {}: {err}",
            dir.display()
        ));
    }

    let _ = remove_path_if_exists(&backup);
    let _ = remove_path_if_exists(&staging);
    Ok(dir)
}

fn ensure_git_repo_cache(
    app: &tauri::AppHandle,
    git_url: &str,
    branch: &str,
    refresh: bool,
) -> Result<PathBuf, String> {
    let dir = repo_cache_dir(app, git_url, branch)?;
    let git_dir = dir.join(".git");

    if !refresh && git_dir.exists() {
        return Ok(dir);
    }

    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    let _lock = RepoLockGuard::acquire(lock_path_for_repo_dir(&dir))?;

    let git_dir = dir.join(".git");
    if !refresh && git_dir.exists() {
        return Ok(dir);
    }

    if !git_dir.exists() {
        // Self-heal: a previous failed clone can leave the dir behind without .git.
        if dir.exists() {
            remove_path_if_exists(&dir)?;
        }

        if branch == "auto" {
            let mut cmd = Command::new("git");
            cmd.arg("clone")
                .arg("--depth")
                .arg("1")
                .arg(git_url)
                .arg(&dir);
            run_git(cmd)?;

            if let Ok(actual_branch) = detect_checked_out_branch(&dir) {
                write_repo_branch(&dir, &actual_branch)?;
            } else {
                write_repo_branch(&dir, branch)?;
            }

            return Ok(dir);
        }

        let mut cmd = Command::new("git");
        cmd.arg("clone")
            .arg("--depth")
            .arg("1")
            .arg("--branch")
            .arg(branch)
            .arg(git_url)
            .arg(&dir);
        match run_git(cmd) {
            Ok(()) => {
                write_repo_branch(&dir, branch)?;
                return Ok(dir);
            }
            Err(err) => {
                if !is_remote_branch_not_found(&err) {
                    return Err(err);
                }

                remove_path_if_exists(&dir)?;

                let mut cmd = Command::new("git");
                cmd.arg("clone")
                    .arg("--depth")
                    .arg("1")
                    .arg(git_url)
                    .arg(&dir);
                run_git(cmd)?;

                if let Ok(actual_branch) = detect_checked_out_branch(&dir) {
                    write_repo_branch(&dir, &actual_branch)?;
                } else {
                    write_repo_branch(&dir, branch)?;
                }

                return Ok(dir);
            }
        }
    }

    if !refresh {
        return Ok(dir);
    }

    let mut effective_branch = read_repo_branch(&dir).unwrap_or_else(|| branch.to_string());
    if effective_branch == "auto" {
        if let Ok(actual_branch) = detect_checked_out_branch(&dir) {
            effective_branch = actual_branch;
            write_repo_branch(&dir, &effective_branch)?;
        }
    }

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(&dir)
        .arg("fetch")
        .arg("origin")
        .arg(&effective_branch)
        .arg("--depth")
        .arg("1");
    if let Err(err) = run_git(cmd) {
        if !is_remote_branch_not_found(&err) {
            return Err(err);
        }

        remove_path_if_exists(&dir)?;

        let mut cmd = Command::new("git");
        cmd.arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(git_url)
            .arg(&dir);
        run_git(cmd)?;

        if let Ok(actual_branch) = detect_checked_out_branch(&dir) {
            write_repo_branch(&dir, &actual_branch)?;
        } else {
            write_repo_branch(&dir, branch)?;
        }

        return Ok(dir);
    }

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(&dir)
        .arg("checkout")
        .arg("-B")
        .arg(&effective_branch)
        .arg(format!("origin/{effective_branch}"));
    run_git(cmd)?;

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(&dir)
        .arg("reset")
        .arg("--hard")
        .arg(format!("origin/{effective_branch}"));
    run_git(cmd)?;

    Ok(dir)
}

pub(super) fn ensure_repo_cache(
    app: &tauri::AppHandle,
    git_url: &str,
    branch: &str,
    refresh: bool,
) -> Result<PathBuf, String> {
    let git_url = git_url.trim();
    if git_url.is_empty() {
        return Err("SEC_INVALID_INPUT: git_url is required".to_string());
    }

    let branch = normalize_repo_branch(branch);

    if let Some((owner, repo)) = parse_github_owner_repo(git_url) {
        return ensure_github_repo_snapshot(app, git_url, &owner, &repo, &branch, refresh);
    }

    ensure_git_repo_cache(app, git_url, &branch, refresh)
}
