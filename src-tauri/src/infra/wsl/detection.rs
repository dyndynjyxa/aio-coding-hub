//! WSL detection, path conversion, and host resolution.

use super::shell::{decode_utf16_le, hide_window_cmd, run_wsl_bash_script_capture};
use super::types::WslDetection;
use crate::settings;
use crate::shared::error::{AppError, AppResult};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::thread::JoinHandle;

const WSL_DETECTION_OUTPUT_STREAM_LIMIT: usize = 128 * 1024;
const WSL_DETECTION_OUTPUT_READ_CHUNK_SIZE: usize = 8 * 1024;
const WSL_DISTRO_MAX_CHARS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LimitedWslDetectionOutput {
    bytes: Vec<u8>,
    truncated: bool,
    limit: usize,
}

impl LimitedWslDetectionOutput {
    fn empty(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            truncated: false,
            limit,
        }
    }
}

#[derive(Debug)]
struct LimitedWslDetectionProcessOutput {
    status: ExitStatus,
    stdout: LimitedWslDetectionOutput,
}

fn read_limited_wsl_detection_output<R: Read>(
    mut reader: R,
    limit: usize,
) -> std::io::Result<LimitedWslDetectionOutput> {
    let mut bytes = Vec::with_capacity(limit.min(WSL_DETECTION_OUTPUT_READ_CHUNK_SIZE));
    let mut truncated = false;
    let mut chunk = [0_u8; WSL_DETECTION_OUTPUT_READ_CHUNK_SIZE];

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

    Ok(LimitedWslDetectionOutput {
        bytes,
        truncated,
        limit,
    })
}

fn spawn_limited_wsl_detection_output_reader<R>(
    reader: R,
) -> JoinHandle<std::io::Result<LimitedWslDetectionOutput>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        read_limited_wsl_detection_output(reader, WSL_DETECTION_OUTPUT_STREAM_LIMIT)
    })
}

fn collect_wsl_detection_output_reader(
    task: Option<JoinHandle<std::io::Result<LimitedWslDetectionOutput>>>,
) -> std::io::Result<LimitedWslDetectionOutput> {
    let Some(task) = task else {
        return Ok(LimitedWslDetectionOutput::empty(
            WSL_DETECTION_OUTPUT_STREAM_LIMIT,
        ));
    };

    match task.join() {
        Ok(result) => result,
        Err(_) => Err(std::io::Error::other(
            "failed to join WSL detection output reader",
        )),
    }
}

fn drain_wsl_detection_output_readers(
    stdout_task: Option<JoinHandle<std::io::Result<LimitedWslDetectionOutput>>>,
    stderr_task: Option<JoinHandle<std::io::Result<LimitedWslDetectionOutput>>>,
) {
    let _ = collect_wsl_detection_output_reader(stdout_task);
    let _ = collect_wsl_detection_output_reader(stderr_task);
}

fn run_limited_wsl_detection_command(
    mut cmd: Command,
) -> std::io::Result<LimitedWslDetectionProcessOutput> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stdout_task = child
        .stdout
        .take()
        .map(spawn_limited_wsl_detection_output_reader);
    let stderr_task = child
        .stderr
        .take()
        .map(spawn_limited_wsl_detection_output_reader);

    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            drain_wsl_detection_output_readers(stdout_task, stderr_task);
            return Err(error);
        }
    };

    let stdout = collect_wsl_detection_output_reader(stdout_task)?;
    let _stderr = collect_wsl_detection_output_reader(stderr_task)?;
    Ok(LimitedWslDetectionProcessOutput { status, stdout })
}

fn decode_utf16_le_if_better(bytes: &[u8]) -> String {
    let utf8 = String::from_utf8_lossy(bytes).to_string();
    if utf8.contains('\0') || utf8.contains('\u{FFFD}') {
        let decoded = decode_utf16_le(bytes);
        let trimmed = decoded.trim().to_string();
        let utf8_replacements = utf8.chars().filter(|c| *c == '\u{FFFD}').count();
        let decoded_replacements = trimmed.chars().filter(|c| *c == '\u{FFFD}').count();
        if !trimmed.is_empty() && (utf8.contains('\0') || decoded_replacements < utf8_replacements)
        {
            return trimmed;
        }
    }
    utf8
}

/// Resolve the Codex home path inside WSL, returned as a Windows path.
pub(super) fn resolve_wsl_codex_home_host_path(distro: &str) -> AppResult<PathBuf> {
    let script = format!(
        r#"
set -euo pipefail
HOME="$(getent passwd "$(whoami)" | cut -d: -f6)"
export HOME
{resolver}
printf '%s\n' "$codex_home"
"#,
        resolver = super::shell::wsl_resolve_codex_home_script("codex_home")
    );
    let resolved = run_wsl_bash_script_capture(distro, &script)?;
    let resolved = resolved.trim();
    if resolved.is_empty() || !resolved.starts_with('/') {
        return Err(format!("failed to resolve CODEX_HOME in {distro}: {resolved}").into());
    }
    Ok(wsl_linux_path_to_windows_path(distro, resolved))
}

/// Resolve the user HOME directory inside a WSL distro, returned as a Windows UNC path.
///
/// Example: distro `"Ubuntu"` -> `\\wsl$\Ubuntu\home\diao`
pub fn resolve_wsl_home_unc(distro: &str) -> AppResult<PathBuf> {
    if !cfg!(windows) {
        return Err(AppError::new(
            "WSL_ERROR",
            "WSL is only available on Windows",
        ));
    }

    let mut cmd = hide_window_cmd("wsl");
    cmd.args([
        "-d",
        distro,
        "--",
        "bash",
        "-lc",
        r#"getent passwd "$(whoami)" | cut -d: -f6"#,
    ]);
    let output = run_limited_wsl_detection_command(cmd)
        .map_err(|e| AppError::new("WSL_ERROR", format!("failed to run wsl.exe: {e}")))?;

    if !output.status.success() {
        return Err(AppError::new(
            "WSL_ERROR",
            format!("wsl command failed for distro: {distro}"),
        ));
    }
    if output.stdout.truncated {
        return Err(AppError::new(
            "WSL_ERROR",
            format!(
                "wsl HOME output for distro {distro} exceeded {} bytes",
                output.stdout.limit
            ),
        ));
    }

    let home = String::from_utf8_lossy(&output.stdout.bytes)
        .trim()
        .to_string();
    if home.is_empty() || !home.starts_with('/') {
        return Err(AppError::new(
            "WSL_ERROR",
            format!("invalid HOME for distro {distro}: {home}"),
        ));
    }

    // Build UNC path: \\wsl$\<distro><home_path_with_backslashes>
    let unc = format!(r"\\wsl$\{}{}", distro, home.replace('/', "\\"));
    Ok(PathBuf::from(unc))
}

fn normalize_detected_distro(distro: &str, detected_distros: &[String]) -> AppResult<String> {
    let trimmed = distro.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("SEC_INVALID_INPUT", "distro is required"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            "WSL distro name contains control characters",
        ));
    }
    if trimmed.chars().count() > WSL_DISTRO_MAX_CHARS {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("WSL distro name is too long (max {WSL_DISTRO_MAX_CHARS} chars)"),
        ));
    }
    if !detected_distros.iter().any(|d| d == trimmed) {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            format!("unknown WSL distro: {trimmed}"),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validate and normalize a distro name against the detected WSL distros list.
pub fn normalize_distro(distro: &str) -> AppResult<String> {
    let detection = detect();
    normalize_detected_distro(distro, &detection.distros)
}

/// Validate that a distro name is in the detected WSL distros list.
pub fn validate_distro(distro: &str) -> AppResult<()> {
    normalize_distro(distro)?;
    Ok(())
}

pub fn detect() -> WslDetection {
    let mut out = WslDetection {
        detected: false,
        distros: Vec::new(),
    };

    if !cfg!(windows) {
        return out;
    }

    let mut cmd = hide_window_cmd("wsl");
    cmd.args(["--list", "--quiet"]);
    let output = run_limited_wsl_detection_command(cmd);
    let Ok(output) = output else {
        return out;
    };
    if !output.status.success() || output.stdout.truncated {
        return out;
    }

    let decoded = decode_utf16_le(&output.stdout.bytes);
    for line in decoded.lines() {
        let mut distro = line.trim().to_string();
        distro = distro.trim_matches(&['\0', '\r'][..]).trim().to_string();
        if distro.is_empty() {
            continue;
        }
        if distro.starts_with("Windows") {
            continue;
        }
        out.distros.push(distro);
    }

    out.detected = !out.distros.is_empty();
    out
}

/// Resolve the host address that WSL distros should use to reach the gateway.
///
/// This is used by:
/// - Gateway listen mode `wsl_auto` (bind host)
/// - WSL client configuration (base origin host)
pub fn resolve_wsl_host(cfg: &settings::AppSettings) -> String {
    match cfg.wsl_host_address_mode {
        settings::WslHostAddressMode::Custom => {
            let addr = cfg.wsl_custom_host_address.trim();
            if addr.is_empty() {
                "127.0.0.1".to_string()
            } else {
                addr.to_string()
            }
        }
        settings::WslHostAddressMode::Auto => {
            host_ipv4_best_effort().unwrap_or_else(|| "127.0.0.1".to_string())
        }
    }
}

pub fn host_ipv4_best_effort() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }

    let output = run_limited_wsl_detection_command(hide_window_cmd("ipconfig")).ok()?;
    let stdout = { decode_utf16_le_if_better(&output.stdout.bytes) };
    use std::net::Ipv4Addr;

    let mut in_wsl_adapter = false;
    for raw_line in stdout.lines() {
        let line = raw_line.trim().trim_matches('\0');

        if line.contains("vEthernet (WSL)")
            || line.contains("vEthernet(WSL)")
            || line.contains("Ethernet adapter vEthernet (WSL)")
        {
            in_wsl_adapter = true;
            continue;
        }

        // Adapter section boundary (English + Chinese output). If localized, we keep scanning until we see IPv4.
        if in_wsl_adapter
            && line.ends_with(':')
            && (line.contains("adapter") || line.contains("适配器"))
            && !line.contains("WSL")
        {
            break;
        }

        if !in_wsl_adapter {
            continue;
        }

        if line.contains("IPv4") || line.contains("IP Address") {
            let Some((_, tail)) = line.rsplit_once(':').or_else(|| line.rsplit_once('：')) else {
                continue;
            };
            let ip = tail.trim();
            if ip.is_empty() || ip.contains(':') {
                continue;
            }
            if ip.parse::<Ipv4Addr>().is_ok() {
                return Some(ip.to_string());
            }
        }
    }

    None
}

pub(super) fn wsl_linux_path_to_windows_path(distro: &str, linux_path: &str) -> PathBuf {
    if let Some(rest) = linux_path.strip_prefix("/mnt/") {
        let mut parts = rest.splitn(2, '/');
        if let Some(drive) = parts.next() {
            if drive.len() == 1 && drive.chars().all(|value| value.is_ascii_alphabetic()) {
                let mut path = format!("{}:\\", drive.to_ascii_uppercase());
                if let Some(tail) = parts.next().filter(|value| !value.is_empty()) {
                    path.push_str(&tail.replace('/', "\\"));
                }
                return PathBuf::from(path);
            }
        }
    }

    PathBuf::from(format!(
        r"\\wsl$\{}{}",
        distro,
        linux_path.replace('/', "\\")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_limited_wsl_detection_output_keeps_bounded_prefix() {
        let output =
            read_limited_wsl_detection_output(Cursor::new(b"abcdefghijklmnop".to_vec()), 8)
                .expect("read");

        assert_eq!(output.bytes, b"abcdefgh");
        assert!(output.truncated);
        assert_eq!(output.limit, 8);
    }

    #[test]
    fn decode_utf16_le_if_better_preserves_utf8_text() {
        assert_eq!(
            decode_utf16_le_if_better("plain text".as_bytes()),
            "plain text"
        );
    }

    #[test]
    fn decode_utf16_le_if_better_decodes_localized_utf16_text() {
        let mut bytes = Vec::new();
        for unit in "错误".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }

        assert_eq!(decode_utf16_le_if_better(&bytes), "错误");
    }

    #[test]
    fn normalize_detected_distro_returns_trimmed_detected_name() {
        let detected = vec!["Ubuntu".to_string(), "Debian".to_string()];

        assert_eq!(
            normalize_detected_distro("  Ubuntu  ", &detected).expect("valid distro"),
            "Ubuntu"
        );
    }

    #[test]
    fn normalize_detected_distro_rejects_invalid_names_before_lookup() {
        let detected = vec!["Ubuntu".to_string()];

        assert_eq!(
            normalize_detected_distro("   ", &detected)
                .expect_err("blank distro")
                .to_string(),
            "SEC_INVALID_INPUT: distro is required"
        );
        assert_eq!(
            normalize_detected_distro("Ubu\nntu", &detected)
                .expect_err("control character")
                .to_string(),
            "SEC_INVALID_INPUT: WSL distro name contains control characters"
        );
        assert_eq!(
            normalize_detected_distro(&"x".repeat(WSL_DISTRO_MAX_CHARS + 1), &detected)
                .expect_err("oversized distro")
                .to_string(),
            "SEC_INVALID_INPUT: WSL distro name is too long (max 128 chars)"
        );
    }

    #[test]
    fn normalize_detected_distro_reports_unknown_trimmed_name() {
        let detected = vec!["Ubuntu".to_string()];

        assert_eq!(
            normalize_detected_distro("  Debian  ", &detected)
                .expect_err("unknown distro")
                .to_string(),
            "SEC_INVALID_INPUT: unknown WSL distro: Debian"
        );
    }
}
