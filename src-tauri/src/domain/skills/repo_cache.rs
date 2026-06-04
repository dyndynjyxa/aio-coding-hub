use super::git_url::{normalize_repo_branch, parse_github_owner_repo};
use super::paths::repos_root;
use super::util::now_unix_nanos;
use crate::shared::fs::{read_file_with_max_len, write_file_atomic};
use crate::shared::http_body::read_text_with_limit;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

const REPO_BRANCH_FILE: &str = ".aio-coding-hub.repo-branch";
const REPO_SNAPSHOT_MARKER_FILE: &str = ".aio-coding-hub.repo-snapshot";
const REPO_BRANCH_FILE_MAX_BYTES: usize = 1024;
const GIT_OUTPUT_STREAM_LIMIT: usize = 32 * 1024;
const GIT_OUTPUT_READ_CHUNK_SIZE: usize = 8 * 1024;
const GITHUB_JSON_RESPONSE_BODY_LIMIT: usize = 2 * 1024 * 1024;
const GITHUB_ZIP_DOWNLOAD_LIMIT: usize = 256 * 1024 * 1024;
const REPO_ZIP_MAX_ENTRIES: usize = 100_000;
const REPO_ZIP_EXTRACTED_BYTES_LIMIT: u64 = 512 * 1024 * 1024;

fn fnv1a64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn repo_cache_dir(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    git_url: &str,
    branch: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    let root = repos_root(app)?;
    let key = format!("{}#{}", git_url.trim(), branch.trim());
    Ok(root.join(format!("{:016x}", fnv1a64(&key))))
}

struct RepoLockGuard {
    path: PathBuf,
    file: Option<std::fs::File>,
}

impl RepoLockGuard {
    fn acquire(path: PathBuf) -> crate::shared::error::AppResult<Self> {
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
                        )
                        .into());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(err) => {
                    return Err(format!(
                        "SKILL_REPO_LOCK_ERROR: failed to create repo lock {}: {err}",
                        path.display()
                    )
                    .into());
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

fn remove_path_if_exists(path: &Path) -> crate::shared::error::AppResult<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
        return Ok(());
    }
    std::fs::remove_file(path)
        .map_err(|e| format!("failed to remove {}: {e}", path.display()).into())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LimitedGitOutput {
    bytes: Vec<u8>,
    truncated: bool,
    limit: usize,
}

impl LimitedGitOutput {
    fn empty(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            truncated: false,
            limit,
        }
    }
}

#[derive(Debug)]
struct LimitedGitProcessOutput {
    status: std::process::ExitStatus,
    stdout: LimitedGitOutput,
    stderr: LimitedGitOutput,
}

fn read_limited_git_output<R: Read>(
    mut reader: R,
    limit: usize,
) -> std::io::Result<LimitedGitOutput> {
    let mut bytes = Vec::with_capacity(limit.min(GIT_OUTPUT_READ_CHUNK_SIZE));
    let mut truncated = false;
    let mut chunk = [0_u8; GIT_OUTPUT_READ_CHUNK_SIZE];

    loop {
        let read = reader.read(&mut chunk)?;
        if read == 0 {
            break;
        }

        let remaining = limit.saturating_sub(bytes.len());
        if remaining > 0 {
            let keep = read.min(remaining);
            bytes.extend_from_slice(&chunk[..keep]);
            if keep < read {
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }

    Ok(LimitedGitOutput {
        bytes,
        truncated,
        limit,
    })
}

fn spawn_limited_git_output_reader<R>(reader: R) -> JoinHandle<std::io::Result<LimitedGitOutput>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || read_limited_git_output(reader, GIT_OUTPUT_STREAM_LIMIT))
}

fn collect_git_output_reader(
    task: Option<JoinHandle<std::io::Result<LimitedGitOutput>>>,
    stream_name: &str,
) -> crate::shared::error::AppResult<LimitedGitOutput> {
    let Some(task) = task else {
        return Ok(LimitedGitOutput::empty(GIT_OUTPUT_STREAM_LIMIT));
    };

    match task.join() {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => {
            Err(format!("SKILL_GIT_ERROR: failed to read git {stream_name}: {error}").into())
        }
        Err(_) => Err(format!("SKILL_GIT_ERROR: failed to join git {stream_name} reader").into()),
    }
}

fn collect_limited_git_process_output(
    status: std::process::ExitStatus,
    stdout_task: Option<JoinHandle<std::io::Result<LimitedGitOutput>>>,
    stderr_task: Option<JoinHandle<std::io::Result<LimitedGitOutput>>>,
) -> crate::shared::error::AppResult<LimitedGitProcessOutput> {
    let stdout = collect_git_output_reader(stdout_task, "stdout")?;
    let stderr = collect_git_output_reader(stderr_task, "stderr")?;
    Ok(LimitedGitProcessOutput {
        status,
        stdout,
        stderr,
    })
}

fn drain_git_output_readers(
    stdout_task: Option<JoinHandle<std::io::Result<LimitedGitOutput>>>,
    stderr_task: Option<JoinHandle<std::io::Result<LimitedGitOutput>>>,
) {
    let _ = collect_git_output_reader(stdout_task, "stdout");
    let _ = collect_git_output_reader(stderr_task, "stderr");
}

fn limited_git_output_to_string(output: &LimitedGitOutput, stream_name: &str) -> String {
    let mut rendered = String::from_utf8_lossy(&output.bytes).trim().to_string();
    if output.truncated {
        if !rendered.is_empty() {
            rendered.push('\n');
        }
        rendered.push_str(&format!(
            "[git {stream_name} truncated after {} bytes]",
            output.limit
        ));
    }
    rendered
}

fn command_limited_git_output(
    mut cmd: Command,
) -> crate::shared::error::AppResult<LimitedGitProcessOutput> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("SKILL_GIT_NOT_FOUND: failed to execute git: {e}"))?;
    let stdout_task = child.stdout.take().map(spawn_limited_git_output_reader);
    let stderr_task = child.stderr.take().map(spawn_limited_git_output_reader);

    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            drain_git_output_readers(stdout_task, stderr_task);
            return Err(format!("SKILL_GIT_ERROR: failed to wait for git: {error}").into());
        }
    };

    collect_limited_git_process_output(status, stdout_task, stderr_task)
}

fn git_error_message(out: &LimitedGitProcessOutput) -> String {
    let stderr = limited_git_output_to_string(&out.stderr, "stderr");
    let stdout = limited_git_output_to_string(&out.stdout, "stdout");
    if !stderr.is_empty() {
        return stderr;
    }
    if !stdout.is_empty() {
        return stdout;
    }
    format!("git exited with status {}", out.status)
}

fn run_git(cmd: Command) -> crate::shared::error::AppResult<()> {
    let out = command_limited_git_output(cmd)?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!("SKILL_GIT_ERROR: {}", git_error_message(&out)).into())
}

fn run_git_capture(cmd: Command) -> crate::shared::error::AppResult<String> {
    let out = command_limited_git_output(cmd)?;
    if out.status.success() {
        return Ok(limited_git_output_to_string(&out.stdout, "stdout"));
    }
    Err(format!("SKILL_GIT_ERROR: {}", git_error_message(&out)).into())
}

fn is_remote_branch_not_found(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    (e.contains("remote branch") && e.contains("not found"))
        || e.contains("couldn't find remote ref")
        || e.contains("could not find remote ref")
}

fn read_repo_branch(dir: &Path) -> Option<String> {
    let path = dir.join(REPO_BRANCH_FILE);
    let bytes = read_file_with_max_len(&path, REPO_BRANCH_FILE_MAX_BYTES).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let branch = text.trim().to_string();
    if branch.is_empty() {
        return None;
    }
    Some(branch)
}

fn write_repo_branch(dir: &Path, branch: &str) -> crate::shared::error::AppResult<()> {
    let path = dir.join(REPO_BRANCH_FILE);
    write_file_atomic(&path, format!("{}\n", branch.trim()).as_bytes())?;
    Ok(())
}

fn detect_checked_out_branch(dir: &Path) -> crate::shared::error::AppResult<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD");
    let out = run_git_capture(cmd)?;
    let branch = out.trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        return Err("SKILL_GIT_ERROR: failed to detect current branch".into());
    }
    Ok(branch)
}

/// Get the HEAD commit SHA of a git repository or snapshot directory.
/// For snapshot directories (GitHub zip downloads), reads from the marker file.
/// For git repos, uses `git rev-parse HEAD`.
pub(super) fn get_repo_head_commit(repo_dir: &Path) -> crate::shared::error::AppResult<String> {
    // For GitHub snapshot directories, there's no .git folder.
    // In this case, we cannot get a commit hash directly.
    // We'll try git rev-parse first, and if that fails, return an error.
    let git_dir = repo_dir.join(".git");
    if !git_dir.exists() {
        // Snapshot mode: no commit hash available from the zip download.
        // The GitHub API doesn't include commit info in zipball downloads.
        // Return an error indicating no commit info.
        return Err("SKILL_NO_COMMIT_INFO: snapshot mode, no git history available".into());
    }

    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo_dir).arg("rev-parse").arg("HEAD");
    let out = run_git_capture(cmd)?;
    let commit = out.trim().to_string();
    if commit.is_empty() {
        return Err("SKILL_GIT_ERROR: failed to get HEAD commit".into());
    }
    Ok(commit)
}

fn build_github_client() -> crate::shared::error::AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent(format!("aio-coding-hub/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("SKILL_HTTP_ERROR: failed to build http client: {e}").into())
}

fn github_zip_too_large_error(limit: usize) -> String {
    format!("SKILL_HTTP_ERROR: github zip body exceeds {limit} bytes")
}

fn append_github_zip_body_chunk(
    bytes: &mut Vec<u8>,
    chunk: &[u8],
    limit: usize,
) -> Result<(), String> {
    if chunk.len() > limit.saturating_sub(bytes.len()) {
        return Err(github_zip_too_large_error(limit));
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}

async fn read_github_zip_body_with_limit(
    mut resp: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let content_length = resp.content_length();
    if content_length.is_some_and(|len| len > limit as u64) {
        return Err(github_zip_too_large_error(limit));
    }

    let capacity = content_length
        .and_then(|len| usize::try_from(len).ok())
        .unwrap_or_default()
        .min(limit);
    let mut bytes = Vec::with_capacity(capacity);
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("SKILL_HTTP_ERROR: failed to read github zip body: {e}"))?
    {
        append_github_zip_body_chunk(&mut bytes, chunk.as_ref(), limit)?;
    }
    Ok(bytes)
}

pub(super) fn github_api_url(segments: &[&str]) -> crate::shared::error::AppResult<reqwest::Url> {
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
) -> crate::shared::error::AppResult<String> {
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

        let body = read_text_with_limit(
            resp,
            GITHUB_JSON_RESPONSE_BODY_LIMIT,
            "github repo response",
        )
        .await
        .map_err(|e| format!("SKILL_HTTP_ERROR: {e}"))?;
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
    .map_err(Into::into)
}

fn github_download_zipball(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    r#ref: &str,
) -> crate::shared::error::AppResult<Vec<u8>> {
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

        read_github_zip_body_with_limit(resp, GITHUB_ZIP_DOWNLOAD_LIMIT).await
    })
    .map_err(Into::into)
}

/// Get the latest commit SHA for a branch from GitHub API.
pub(super) fn github_get_branch_commit(
    owner: &str,
    repo: &str,
    branch: &str,
) -> crate::shared::error::AppResult<String> {
    let client = build_github_client()?;
    let url = github_api_url(&["repos", owner, repo, "commits", branch])?;
    tauri::async_runtime::block_on(async move {
        let resp = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("SKILL_HTTP_ERROR: github request failed: {e}"))?;

        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err("SKILL_GITHUB_REF_NOT_FOUND: branch/commit not found".to_string());
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

        let body = read_text_with_limit(
            resp,
            GITHUB_JSON_RESPONSE_BODY_LIMIT,
            "github commit response",
        )
        .await
        .map_err(|e| format!("SKILL_HTTP_ERROR: {e}"))?;
        let root: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("SKILL_GITHUB_PARSE_ERROR: github json parse failed: {e}"))?;
        let sha = root
            .get("sha")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if sha.is_empty() {
            return Err("SKILL_GITHUB_PARSE_ERROR: missing commit sha".to_string());
        }
        Ok(sha.to_string())
    })
    .map_err(Into::into)
}

fn zip_too_many_entries_error(max_entries: usize) -> String {
    format!("SKILL_ZIP_ERROR: zip has too many entries (max {max_entries})")
}

fn zip_extracted_too_large_error(max_extracted_bytes: u64) -> String {
    format!("SKILL_ZIP_ERROR: extracted zip content exceeds {max_extracted_bytes} bytes")
}

fn unzip_repo_zip_with_limits(
    zip_bytes: &[u8],
    dst_dir: &Path,
    max_entries: usize,
    max_extracted_bytes: u64,
) -> crate::shared::error::AppResult<PathBuf> {
    std::fs::create_dir_all(dst_dir)
        .map_err(|e| format!("failed to create {}: {e}", dst_dir.display()))?;

    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| format!("SKILL_ZIP_ERROR: failed to open zip archive: {e}"))?;
    if archive.len() > max_entries {
        return Err(zip_too_many_entries_error(max_entries).into());
    }

    let mut extracted_bytes = 0_u64;
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("SKILL_ZIP_ERROR: failed to read zip entry: {e}"))?;
        let name = file.name().replace('\\', "/");
        if name.is_empty() {
            continue;
        }

        let rel = Path::new(&name);
        if rel.is_absolute() {
            return Err("SKILL_ZIP_ERROR: invalid zip entry path (absolute)".into());
        }
        for comp in rel.components() {
            match comp {
                Component::CurDir | Component::Normal(_) => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err("SKILL_ZIP_ERROR: invalid zip entry path".into());
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

        let remaining = max_extracted_bytes.saturating_sub(extracted_bytes);
        if file.size() > remaining {
            return Err(zip_extracted_too_large_error(max_extracted_bytes).into());
        }

        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| format!("failed to create {}: {e}", out_path.display()))?;
        let copied = {
            let mut limited_file = file.take(remaining + 1);
            std::io::copy(&mut limited_file, &mut out_file)
        }
        .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
        if copied > remaining {
            return Err(zip_extracted_too_large_error(max_extracted_bytes).into());
        }
        extracted_bytes = extracted_bytes.saturating_add(copied);
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
        )
        .into());
    }

    Ok(top_dirs.remove(0))
}

pub(super) fn unzip_repo_zip(
    zip_bytes: &[u8],
    dst_dir: &Path,
) -> crate::shared::error::AppResult<PathBuf> {
    unzip_repo_zip_with_limits(
        zip_bytes,
        dst_dir,
        REPO_ZIP_MAX_ENTRIES,
        REPO_ZIP_EXTRACTED_BYTES_LIMIT,
    )
}

fn repo_snapshot_marker_path(dir: &Path) -> PathBuf {
    dir.join(REPO_SNAPSHOT_MARKER_FILE)
}

fn write_repo_snapshot_marker(
    dir: &Path,
    git_url: &str,
    branch: &str,
) -> crate::shared::error::AppResult<()> {
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
    app: &tauri::AppHandle<impl tauri::Runtime>,
    git_url: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    refresh: bool,
) -> crate::shared::error::AppResult<PathBuf> {
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
                    last_err = Some(err.to_string());
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
                            last_err = Some(err.to_string());
                        }
                    }
                }
                Err(err) => {
                    last_err = Some(err.to_string());
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
                last_err = Some(err.to_string());
            }
        }
    }

    let Some(zip_bytes) = zip_bytes else {
        return Err(last_err
            .unwrap_or_else(|| {
                "SKILL_GITHUB_DOWNLOAD_FAILED: failed to download github zip".to_string()
            })
            .into());
    };
    if effective_branch.is_empty() {
        return Err("SKILL_GITHUB_BRANCH_ERROR: failed to resolve branch".into());
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
            )
            .into());
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
        )
        .into());
    }

    let _ = remove_path_if_exists(&backup);
    let _ = remove_path_if_exists(&staging);
    Ok(dir)
}

fn ensure_git_repo_cache(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    git_url: &str,
    branch: &str,
    refresh: bool,
) -> crate::shared::error::AppResult<PathBuf> {
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
                let err_text = err.to_string();
                if !is_remote_branch_not_found(&err_text) {
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
        let err_text = err.to_string();
        if !is_remote_branch_not_found(&err_text) {
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
    app: &tauri::AppHandle<impl tauri::Runtime>,
    git_url: &str,
    branch: &str,
    refresh: bool,
) -> crate::shared::error::AppResult<PathBuf> {
    let git_url = git_url.trim();
    if git_url.is_empty() {
        return Err("SEC_INVALID_INPUT: git_url is required".into());
    }

    let branch = normalize_repo_branch(branch);

    if let Some((owner, repo)) = parse_github_owner_repo(git_url) {
        return ensure_github_repo_snapshot(app, git_url, &owner, &repo, &branch, refresh);
    }

    ensure_git_repo_cache(app, git_url, &branch, refresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", now_unix_nanos()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn read_limited_git_output_keeps_bounded_prefix() {
        let output =
            read_limited_git_output(Cursor::new(b"abcdefghijklmnop".to_vec()), 8).expect("read");

        assert_eq!(output.bytes, b"abcdefgh");
        assert!(output.truncated);
        assert_eq!(output.limit, 8);
    }

    #[test]
    fn limited_git_output_to_string_marks_truncated_stream() {
        let rendered = limited_git_output_to_string(
            &LimitedGitOutput {
                bytes: b"first lines\n".to_vec(),
                truncated: true,
                limit: 11,
            },
            "stderr",
        );

        assert_eq!(
            rendered,
            "first lines\n[git stderr truncated after 11 bytes]"
        );
    }

    #[test]
    fn append_github_zip_body_chunk_accepts_exact_limit() {
        let mut bytes = b"abcd".to_vec();

        append_github_zip_body_chunk(&mut bytes, b"ef", 6).expect("append");

        assert_eq!(bytes, b"abcdef");
    }

    #[test]
    fn append_github_zip_body_chunk_rejects_limit_overflow_without_mutating() {
        let mut bytes = b"abcd".to_vec();
        let err = append_github_zip_body_chunk(&mut bytes, b"ef", 5).expect_err("reject");

        assert_eq!(err, "SKILL_HTTP_ERROR: github zip body exceeds 5 bytes");
        assert_eq!(bytes, b"abcd");
    }

    #[test]
    fn read_repo_branch_rejects_oversized_marker() {
        let dir = make_temp_dir("aio-repo-branch-large");
        std::fs::write(
            dir.join(REPO_BRANCH_FILE),
            vec![b'x'; REPO_BRANCH_FILE_MAX_BYTES + 1],
        )
        .expect("write branch marker");

        let branch = read_repo_branch(&dir);

        assert!(branch.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unzip_repo_zip_with_limits_rejects_too_many_entries() {
        let mut buf = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::<()>::default();
        zip.add_directory("repo/", opts).expect("add root");
        zip.add_directory("repo/nested/", opts).expect("add nested");
        zip.finish().expect("finish zip");

        let out_dir = make_temp_dir("aio-unzip-entry-limit");
        let err = unzip_repo_zip_with_limits(&buf.into_inner(), &out_dir, 1, 1024)
            .expect_err("entry limit")
            .to_string();

        assert_eq!(err, "SKILL_ZIP_ERROR: zip has too many entries (max 1)");

        let _ = std::fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn unzip_repo_zip_with_limits_rejects_extracted_size_overflow() {
        let mut buf = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::<()>::default();
        zip.add_directory("repo/", opts).expect("add root");
        zip.start_file("repo/SKILL.md", opts).expect("start file");
        zip.write_all(b"abcdef").expect("write file");
        zip.finish().expect("finish zip");

        let out_dir = make_temp_dir("aio-unzip-byte-limit");
        let err = unzip_repo_zip_with_limits(&buf.into_inner(), &out_dir, 4, 5)
            .expect_err("byte limit")
            .to_string();

        assert_eq!(
            err,
            "SKILL_ZIP_ERROR: extracted zip content exceeds 5 bytes"
        );

        let _ = std::fs::remove_dir_all(&out_dir);
    }
}
