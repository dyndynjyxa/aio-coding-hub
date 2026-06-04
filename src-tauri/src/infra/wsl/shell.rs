//! Low-level WSL shell execution and file I/O helpers.

use crate::shared::error::AppResult;
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::thread::JoinHandle;

const WSL_SCRIPT_OUTPUT_STREAM_LIMIT: usize = 32 * 1024;
const WSL_CAPTURE_STDOUT_STREAM_LIMIT: usize = 16 * 1024 * 1024;
const WSL_OUTPUT_READ_CHUNK_SIZE: usize = 8 * 1024;

#[cfg(windows)]
pub(super) fn hide_window_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
pub(super) fn hide_window_cmd(program: &str) -> Command {
    Command::new(program)
}

pub(super) fn decode_utf16_le(mut bytes: &[u8]) -> String {
    // BOM (FF FE)
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        bytes = &bytes[2..];
    }

    let len = bytes.len() - (bytes.len() % 2);
    let mut u16s = Vec::with_capacity(len / 2);
    for chunk in bytes[..len].chunks_exact(2) {
        u16s.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    String::from_utf16_lossy(&u16s)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LimitedWslShellOutput {
    bytes: Vec<u8>,
    truncated: bool,
    limit: usize,
}

impl LimitedWslShellOutput {
    fn empty(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            truncated: false,
            limit,
        }
    }
}

#[derive(Debug)]
struct LimitedWslShellProcessOutput {
    status: ExitStatus,
    stdout: LimitedWslShellOutput,
    stderr: LimitedWslShellOutput,
}

fn read_limited_wsl_shell_output<R: Read>(
    mut reader: R,
    limit: usize,
) -> std::io::Result<LimitedWslShellOutput> {
    let mut bytes = Vec::with_capacity(limit.min(WSL_OUTPUT_READ_CHUNK_SIZE));
    let mut truncated = false;
    let mut chunk = [0_u8; WSL_OUTPUT_READ_CHUNK_SIZE];

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

    Ok(LimitedWslShellOutput {
        bytes,
        truncated,
        limit,
    })
}

fn spawn_limited_wsl_shell_output_reader<R>(
    reader: R,
    limit: usize,
) -> JoinHandle<std::io::Result<LimitedWslShellOutput>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || read_limited_wsl_shell_output(reader, limit))
}

fn collect_wsl_shell_output_reader(
    task: Option<JoinHandle<std::io::Result<LimitedWslShellOutput>>>,
    limit: usize,
    stream_name: &str,
) -> AppResult<LimitedWslShellOutput> {
    let Some(task) = task else {
        return Ok(LimitedWslShellOutput::empty(limit));
    };

    match task.join() {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(format!("failed to read wsl {stream_name}: {error}").into()),
        Err(_) => Err(format!("failed to join wsl {stream_name} reader").into()),
    }
}

fn drain_wsl_shell_output_readers(
    stdout_task: Option<JoinHandle<std::io::Result<LimitedWslShellOutput>>>,
    stderr_task: Option<JoinHandle<std::io::Result<LimitedWslShellOutput>>>,
    stdout_limit: usize,
    stderr_limit: usize,
) {
    let _ = collect_wsl_shell_output_reader(stdout_task, stdout_limit, "stdout");
    let _ = collect_wsl_shell_output_reader(stderr_task, stderr_limit, "stderr");
}

fn render_limited_wsl_shell_output(
    output: &LimitedWslShellOutput,
    stream_name: &str,
    decode_utf16_when_better: bool,
) -> String {
    let utf8 = String::from_utf8_lossy(&output.bytes).trim().to_string();
    let mut rendered = if decode_utf16_when_better
        && (utf8.contains('\0') || utf8.contains('\u{FFFD}'))
    {
        let decoded = decode_utf16_le(&output.bytes);
        let trimmed = decoded.trim().to_string();
        let utf8_replacements = utf8.chars().filter(|c| *c == '\u{FFFD}').count();
        let decoded_replacements = trimmed.chars().filter(|c| *c == '\u{FFFD}').count();
        if !trimmed.is_empty() && (utf8.contains('\0') || decoded_replacements < utf8_replacements)
        {
            trimmed
        } else {
            utf8
        }
    } else {
        utf8
    };
    if output.truncated {
        if !rendered.is_empty() {
            rendered.push('\n');
        }
        rendered.push_str(&format!(
            "[wsl {stream_name} truncated after {} bytes]",
            output.limit
        ));
    }
    rendered
}

fn wsl_error_from_output(output: &LimitedWslShellProcessOutput) -> crate::shared::error::AppError {
    let stdout = render_limited_wsl_shell_output(&output.stdout, "stdout", false);
    // wsl.exe on non-English Windows may emit UTF-16LE warnings on stderr;
    // bash/python errors inside the distro are usually UTF-8. Use UTF-16LE
    // when it clearly renders better.
    let stderr = render_limited_wsl_shell_output(&output.stderr, "stderr", true);
    let msg = if !stderr.is_empty() { stderr } else { stdout };
    format!(
        "WSL_ERROR: {}",
        if msg.is_empty() {
            "unknown error"
        } else {
            &msg
        }
    )
    .into()
}

fn run_wsl_bash_script_with_limits(
    distro: &str,
    script: &str,
    stdout_limit: usize,
    stderr_limit: usize,
) -> AppResult<LimitedWslShellProcessOutput> {
    let mut cmd = hide_window_cmd("wsl");
    cmd.args(["-d", distro, "bash"]);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn wsl: {e}"))?;
    let stdout_task = child
        .stdout
        .take()
        .map(|stdout| spawn_limited_wsl_shell_output_reader(stdout, stdout_limit));
    let stderr_task = child
        .stderr
        .take()
        .map(|stderr| spawn_limited_wsl_shell_output_reader(stderr, stderr_limit));

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        if let Err(error) = stdin.write_all(script.as_bytes()) {
            let _ = child.kill();
            let _ = child.wait();
            drain_wsl_shell_output_readers(stdout_task, stderr_task, stdout_limit, stderr_limit);
            return Err(format!("failed to write wsl stdin: {error}").into());
        }
    }

    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            drain_wsl_shell_output_readers(stdout_task, stderr_task, stdout_limit, stderr_limit);
            return Err(format!("failed to wait for wsl: {error}").into());
        }
    };

    let stdout = collect_wsl_shell_output_reader(stdout_task, stdout_limit, "stdout")?;
    let stderr = collect_wsl_shell_output_reader(stderr_task, stderr_limit, "stderr")?;
    Ok(LimitedWslShellProcessOutput {
        status,
        stdout,
        stderr,
    })
}

pub(super) fn bash_single_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

pub(super) fn wsl_resolve_codex_home_script(var_name: &str) -> String {
    format!(
        r#"
codex_home_raw="${{CODEX_HOME:-$HOME/.codex}}"
{var_name}="$codex_home_raw"
if [ "$codex_home_raw" = "~" ]; then
  {var_name}="$HOME"
elif [ "${{codex_home_raw#~/}}" != "$codex_home_raw" ]; then
  {var_name}="$HOME/${{codex_home_raw#~/}}"
elif [ "${{codex_home_raw#~\\}}" != "$codex_home_raw" ]; then
  {var_name}="$HOME/${{codex_home_raw#~\\}}"
else
  case "$codex_home_raw" in
    [A-Za-z]:[\\/]*)
      if command -v wslpath >/dev/null 2>&1; then
        {var_name}="$(wslpath -u "$codex_home_raw")"
      else
        drive="$(printf '%s' "$codex_home_raw" | cut -c1 | tr '[:upper:]' '[:lower:]')"
        rest="$(printf '%s' "$codex_home_raw" | cut -c3- | sed 's#\\\\#/#g; s#^/##')"
        {var_name}="/mnt/$drive/$rest"
      fi
      ;;
    *)
      if [ "${{codex_home_raw#/}}" = "$codex_home_raw" ]; then
        {var_name}="$HOME/$codex_home_raw"
      fi
      ;;
  esac
fi
if [ "$(basename -- "${{{var_name}}}")" = "config.toml" ]; then
  {var_name}="$(dirname "${{{var_name}}}")"
fi
"#,
        var_name = var_name
    )
}

pub(super) fn run_wsl_bash_script(distro: &str, script: &str) -> AppResult<()> {
    let output = run_wsl_bash_script_with_limits(
        distro,
        script,
        WSL_SCRIPT_OUTPUT_STREAM_LIMIT,
        WSL_SCRIPT_OUTPUT_STREAM_LIMIT,
    )?;
    if output.status.success() {
        return Ok(());
    }

    Err(wsl_error_from_output(&output))
}

/// Execute a bash script inside a WSL distro and capture its stdout.
pub(super) fn run_wsl_bash_script_capture(distro: &str, script: &str) -> AppResult<String> {
    let output = run_wsl_bash_script_with_limits(
        distro,
        script,
        WSL_CAPTURE_STDOUT_STREAM_LIMIT,
        WSL_SCRIPT_OUTPUT_STREAM_LIMIT,
    )?;
    if output.status.success() {
        if output.stdout.truncated {
            return Err(format!("WSL_ERROR: stdout exceeded {} bytes", output.stdout.limit).into());
        }
        return Ok(String::from_utf8_lossy(&output.stdout.bytes).to_string());
    }

    Err(wsl_error_from_output(&output))
}

/// Read a file from WSL using base64 encoding. Returns None if file does not exist.
pub(super) fn read_wsl_file(distro: &str, path_expr: &str) -> AppResult<Option<Vec<u8>>> {
    read_wsl_file_inner(distro, path_expr, None)
}

pub(super) fn read_wsl_file_with_max_len(
    distro: &str,
    path_expr: &str,
    max_len: usize,
) -> AppResult<Option<Vec<u8>>> {
    read_wsl_file_inner(distro, path_expr, Some(max_len))
}

fn read_wsl_file_inner(
    distro: &str,
    path_expr: &str,
    max_len: Option<usize>,
) -> AppResult<Option<Vec<u8>>> {
    use base64::Engine;

    let path_escaped = bash_single_quote(path_expr);
    let max_len_check = max_len
        .map(|limit| {
            format!(
                r#"
size="$(wc -c < "$target" | tr -d '[:space:]')"
if [ "$size" -gt {limit} ]; then
  echo "AIO_WSL_FILE_TOO_LARGE:$size"
  exit 0
fi
"#
            )
        })
        .unwrap_or_default();
    let script = format!(
        r#"
set -euo pipefail
target={path_escaped}
if [ ! -f "$target" ]; then
  echo "AIO_WSL_FILE_NOT_FOUND"
  exit 0
fi
{max_len_check}
base64 -w0 "$target"
echo ""
"#
    );
    let stdout = run_wsl_bash_script_capture(distro, &script)?;
    let trimmed = stdout.trim();
    if trimmed == "AIO_WSL_FILE_NOT_FOUND" {
        return Ok(None);
    }
    if let Some(size) = trimmed.strip_prefix("AIO_WSL_FILE_TOO_LARGE:") {
        let limit = max_len.unwrap_or(0);
        return Err(format!(
            "WSL_ERROR: file {path_expr} too large (max {limit} bytes, got {size})"
        )
        .into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|e| format!("WSL_ERROR: base64 decode failed: {e}"))?;
    Ok(Some(bytes))
}

/// Atomically write a file into WSL (backup + tmp + mv).
pub(super) fn write_wsl_file(distro: &str, path_expr: &str, content: &[u8]) -> AppResult<()> {
    use base64::Engine;

    let b64 = base64::engine::general_purpose::STANDARD.encode(content);
    let path_escaped = bash_single_quote(path_expr);
    let b64_escaped = bash_single_quote(&b64);

    let script = format!(
        r#"
set -euo pipefail
HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME

target={path_escaped}
dir="$(dirname "$target")"
mkdir -p "$dir"

ts="$(date +%s)"
if [ -f "$target" ]; then
  cp -a "$target" "$target.bak.$ts"
fi

tmp_path="$(mktemp "${{target}}.tmp.XXXXXX")"
cleanup() {{ rm -f "$tmp_path"; }}
trap cleanup EXIT

echo {b64_escaped} | base64 -d > "$tmp_path"

if [ -f "$target" ]; then
  chmod --reference="$target" "$tmp_path" 2>/dev/null || true
fi

mv -f "$tmp_path" "$target"
trap - EXIT
"#
    );
    run_wsl_bash_script(distro, &script)
}

pub(super) fn remove_wsl_file(distro: &str, path_expr: &str) -> AppResult<()> {
    let path_escaped = bash_single_quote(path_expr);
    let script = format!(
        r#"
set -euo pipefail
target={path_escaped}
rm -f -- "$target"
"#
    );
    run_wsl_bash_script(distro, &script)
}

pub(super) fn wsl_path_exists(distro: &str, path_expr: &str) -> AppResult<bool> {
    let path_escaped = bash_single_quote(path_expr);
    let script = format!(
        r#"
set -euo pipefail
target={path_escaped}
if [ -e "$target" ]; then
  echo "1"
else
  echo "0"
fi
"#
    );
    Ok(run_wsl_bash_script_capture(distro, &script)?.trim() == "1")
}

pub(super) fn remove_wsl_dir(distro: &str, path_expr: &str) -> AppResult<()> {
    if !path_expr.starts_with('/') {
        return Err(format!("refusing to remove non-absolute WSL path: {path_expr}").into());
    }
    let path_escaped = bash_single_quote(path_expr);
    let script = format!(
        r#"
set -euo pipefail
target={path_escaped}
rm -rf -- "$target"
"#
    );
    run_wsl_bash_script(distro, &script)
}

pub(super) fn wsl_has_managed_skill_dir(distro: &str, path_expr: &str) -> AppResult<bool> {
    let path_escaped = bash_single_quote(path_expr);
    let script = format!(
        r#"
set -euo pipefail
target={path_escaped}
if [ -f "$target/{marker}" ]; then
  echo "1"
else
  echo "0"
fi
"#,
        marker = super::skills_sync::WSL_SKILL_MANAGED_MARKER_FILE
    );
    Ok(run_wsl_bash_script_capture(distro, &script)?.trim() == "1")
}

/// Write file atomically and force sync to disk (critical for UNC/9P paths during exit).
///
/// Writes to a `.aio-tmp` sibling first, syncs, then renames over the target.
/// This prevents config corruption if the process is killed mid-write.
pub(super) fn write_file_synced(path: &std::path::Path, data: &[u8]) -> AppResult<()> {
    use std::io::Write;
    let file_name = path.file_name().and_then(|v| v.to_str()).unwrap_or("file");
    let tmp_path = path.with_file_name(format!("{file_name}.aio-tmp"));

    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("failed to create {}: {e}", tmp_path.display()))?;
    file.write_all(data)
        .map_err(|e| format!("failed to write {}: {e}", tmp_path.display()))?;
    file.sync_all()
        .map_err(|e| format!("failed to sync {}: {e}", tmp_path.display()))?;
    drop(file);

    // Windows rename requires target not to exist.
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&tmp_path, path)
        .map_err(|e| format!("failed to finalize {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_limited_wsl_shell_output_keeps_bounded_prefix() {
        let output = read_limited_wsl_shell_output(Cursor::new(b"abcdefghijklmnop".to_vec()), 8)
            .expect("read");

        assert_eq!(output.bytes, b"abcdefgh");
        assert!(output.truncated);
        assert_eq!(output.limit, 8);
    }

    #[test]
    fn render_limited_wsl_shell_output_marks_truncated_stream() {
        let rendered = render_limited_wsl_shell_output(
            &LimitedWslShellOutput {
                bytes: b"script warning".to_vec(),
                truncated: true,
                limit: 14,
            },
            "stderr",
            false,
        );

        assert_eq!(
            rendered,
            "script warning\n[wsl stderr truncated after 14 bytes]"
        );
    }

    #[test]
    fn render_limited_wsl_shell_output_decodes_utf16_when_better() {
        let mut bytes = Vec::new();
        for unit in "错误".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }

        let rendered = render_limited_wsl_shell_output(
            &LimitedWslShellOutput {
                bytes,
                truncated: false,
                limit: 16,
            },
            "stderr",
            true,
        );

        assert_eq!(rendered, "错误");
    }
}
