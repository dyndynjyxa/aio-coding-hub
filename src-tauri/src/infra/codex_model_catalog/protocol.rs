use super::{CodexModelCapability, CodexReasoningEffortOption};
use crate::cli_manager::CodexLaunchSpec;
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const APP_SERVER_TIMEOUT: Duration = Duration::from_secs(20);
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(300);
const MAX_JSON_LINE_BYTES: usize = 4 * 1024 * 1024;
const STDOUT_EVENT_CHANNEL_CAPACITY: usize = 32;
const MAX_STDOUT_EVENT_COUNT: usize = 256;
const MAX_STDOUT_TOTAL_BYTES: usize = 16 * 1024 * 1024;
const MAX_STDERR_BYTES: usize = 16 * 1024;
const MODEL_PAGE_LIMIT: u64 = 200;
const MAX_MODEL_COUNT: usize = 1_000;
const MAX_PAGE_COUNT: usize = 32;
#[cfg(windows)]
const WINDOWS_CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProtocolError {
    Spawn,
    Timeout,
    Malformed,
    JsonRpc,
}

enum StdoutEvent {
    Line(String),
    Failed,
}

struct StdoutEvents {
    receiver: Receiver<StdoutEvent>,
    failed: Arc<AtomicBool>,
}

impl StdoutEvents {
    fn recv_line(&self, timeout: Duration) -> Result<String, ProtocolError> {
        if self.failed.load(Ordering::Acquire) {
            return Err(ProtocolError::Malformed);
        }
        let event = self
            .receiver
            .recv_timeout(timeout)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => ProtocolError::Timeout,
                mpsc::RecvTimeoutError::Disconnected => ProtocolError::Malformed,
            })?;
        if self.failed.load(Ordering::Acquire) {
            return Err(ProtocolError::Malformed);
        }
        match event {
            StdoutEvent::Line(line) => Ok(line),
            StdoutEvent::Failed => Err(ProtocolError::Malformed),
        }
    }
}

struct StdoutBudget {
    event_count: usize,
    total_bytes: usize,
    max_event_count: usize,
    max_total_bytes: usize,
}

impl StdoutBudget {
    fn new(max_event_count: usize, max_total_bytes: usize) -> Self {
        Self {
            event_count: 0,
            total_bytes: 0,
            max_event_count,
            max_total_bytes,
        }
    }

    fn record(&mut self, bytes: usize) -> bool {
        if self.event_count >= self.max_event_count
            || bytes > self.max_total_bytes.saturating_sub(self.total_bytes)
        {
            return false;
        }
        self.event_count += 1;
        self.total_bytes += bytes;
        true
    }
}

pub(crate) fn fetch_model_catalog(
    launch: &CodexLaunchSpec,
    codex_home: &Path,
) -> Result<Vec<CodexModelCapability>, ProtocolError> {
    let mut child = ManagedChild::spawn(launch, codex_home)?;
    let result = run_protocol(&mut child);
    child.shutdown(result.is_err());
    result
}

fn run_protocol(child: &mut ManagedChild) -> Result<Vec<CodexModelCapability>, ProtocolError> {
    run_protocol_with_timeout(child, APP_SERVER_TIMEOUT)
}

fn run_protocol_with_timeout(
    child: &mut ManagedChild,
    timeout: Duration,
) -> Result<Vec<CodexModelCapability>, ProtocolError> {
    let deadline = Instant::now() + timeout;
    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "aio-coding-hub",
                "title": "AIO Coding Hub",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": { "experimentalApi": true }
        }
    });
    let _ = child.request_result(1, initialize, deadline)?;

    child.send(json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }))?;

    let mut cursor: Option<String> = None;
    let mut seen_cursors = std::collections::HashSet::new();
    let mut models = Vec::new();

    for page_index in 0..MAX_PAGE_COUNT {
        let request_id = 2 + page_index as u64;
        let mut params = json!({
            "limit": MODEL_PAGE_LIMIT,
            "includeHidden": true
        });
        if let Some(value) = cursor.as_deref() {
            params["cursor"] = Value::String(value.to_string());
        }

        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "model/list",
            "params": params
        });
        let result = child.request_result(request_id, request, deadline)?;
        let data = result
            .get("data")
            .and_then(Value::as_array)
            .ok_or(ProtocolError::Malformed)?;

        for raw_model in data {
            models.push(parse_model(raw_model)?);
            if models.len() > MAX_MODEL_COUNT {
                return Err(ProtocolError::Malformed);
            }
        }

        let next_cursor = parse_next_cursor(&result)?;
        let Some(next_cursor) = next_cursor else {
            return Ok(models);
        };
        if !seen_cursors.insert(next_cursor.clone()) {
            return Err(ProtocolError::Malformed);
        }
        cursor = Some(next_cursor);
    }

    Err(ProtocolError::Malformed)
}

fn parse_next_cursor(result: &Value) -> Result<Option<String>, ProtocolError> {
    match result.get("nextCursor") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if value.trim().is_empty() => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.to_owned())),
        Some(_) => Err(ProtocolError::Malformed),
    }
}

#[derive(Debug, serde::Deserialize)]
struct RawCodexModel {
    id: String,
    model: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    hidden: Option<bool>,
    #[serde(rename = "isDefault")]
    is_default: Option<bool>,
    #[serde(rename = "supportedReasoningEfforts")]
    supported_reasoning_efforts: Option<Vec<RawReasoningEffortOption>>,
    #[serde(rename = "defaultReasoningEffort")]
    default_reasoning_effort: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct RawReasoningEffortOption {
    #[serde(rename = "reasoningEffort")]
    reasoning_effort: String,
    description: Option<String>,
}

fn parse_model(raw: &Value) -> Result<CodexModelCapability, ProtocolError> {
    let parsed: RawCodexModel =
        serde_json::from_value(raw.clone()).map_err(|_| ProtocolError::Malformed)?;
    let id = parsed.id.trim().to_string();
    if id.is_empty() {
        return Err(ProtocolError::Malformed);
    }
    let model = parsed
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&id)
        .to_string();
    let display_name = parsed
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&model)
        .to_string();
    let supported_reasoning_efforts = parsed
        .supported_reasoning_efforts
        .map(|options| {
            options
                .into_iter()
                .map(|option| {
                    let reasoning_effort = option.reasoning_effort.trim().to_string();
                    if reasoning_effort.is_empty() {
                        return Err(ProtocolError::Malformed);
                    }
                    Ok(CodexReasoningEffortOption {
                        reasoning_effort,
                        description: option
                            .description
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned),
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let default_reasoning_effort = parsed
        .default_reasoning_effort
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Ok(CodexModelCapability {
        id,
        model,
        display_name,
        hidden: parsed.hidden.unwrap_or(false),
        is_default: parsed.is_default.unwrap_or(false),
        supported_reasoning_efforts,
        default_reasoning_effort,
    })
}

struct ManagedChild {
    child: Child,
    stdout_events: Option<StdoutEvents>,
    stdout_task: Option<JoinHandle<()>>,
    stderr_task: Option<JoinHandle<()>>,
    #[cfg(unix)]
    process_id: u32,
    #[cfg(windows)]
    job: WindowsJob,
}

impl ManagedChild {
    fn spawn(launch: &CodexLaunchSpec, codex_home: &Path) -> Result<Self, ProtocolError> {
        let mut command = build_command(launch);
        command
            .env("CODEX_HOME", codex_home)
            .env("PATH", &launch.runtime_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_command(&mut command);

        let mut child = command.spawn().map_err(|_| ProtocolError::Spawn)?;
        #[cfg(unix)]
        let process_id = child.id();
        #[cfg(windows)]
        let job = match WindowsJob::attach(child.id()) {
            Ok(job) => job,
            Err(_) => {
                terminate_windows_process_tree(child.id());
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProtocolError::Spawn);
            }
        };

        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProtocolError::Spawn);
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProtocolError::Spawn);
        };
        let (stdout_events, stdout_task) = spawn_stdout_reader(stdout);
        let stderr_task = Some(thread::spawn(move || drain_stderr(stderr)));

        Ok(Self {
            child,
            stdout_events: Some(stdout_events),
            stdout_task: Some(stdout_task),
            stderr_task,
            #[cfg(unix)]
            process_id,
            #[cfg(windows)]
            job,
        })
    }

    fn send(&mut self, message: Value) -> Result<(), ProtocolError> {
        let Some(stdin) = self.child.stdin.as_mut() else {
            return Err(ProtocolError::Malformed);
        };
        let payload = serde_json::to_vec(&message).map_err(|_| ProtocolError::Malformed)?;
        stdin
            .write_all(&payload)
            .and_then(|_| stdin.write_all(b"\n"))
            .and_then(|_| stdin.flush())
            .map_err(|_| ProtocolError::Malformed)
    }

    fn request_result(
        &mut self,
        request_id: u64,
        request: Value,
        deadline: Instant,
    ) -> Result<Value, ProtocolError> {
        self.send(request)?;
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or(ProtocolError::Timeout)?;
            let line = self
                .stdout_events
                .as_ref()
                .ok_or(ProtocolError::Malformed)?
                .recv_line(remaining)?;
            let message: Value =
                serde_json::from_str(&line).map_err(|_| ProtocolError::Malformed)?;
            if let Some(id) = message.get("id") {
                if message.get("method").is_some() || message.get("params").is_some() {
                    return Err(ProtocolError::Malformed);
                }
                if id != &json!(request_id) {
                    continue;
                }
            }
            if let Some(result) = parse_jsonrpc_response(&message, request_id)? {
                return Ok(result);
            }
        }
    }

    fn shutdown(&mut self, force: bool) {
        let _ = self.child.stdin.take();
        self.stdout_events.take();
        let graceful_deadline = Instant::now() + GRACEFUL_SHUTDOWN_TIMEOUT;

        if !force {
            while Instant::now() < graceful_deadline {
                match self.child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => thread::sleep(Duration::from_millis(10)),
                    Err(_) => break,
                }
            }
        }

        self.terminate_tree();
        let _ = self.child.wait();

        if let Some(task) = self.stdout_task.take() {
            let _ = task.join();
        }
        if let Some(task) = self.stderr_task.take() {
            let _ = task.join();
        }
    }

    fn terminate_tree(&mut self) {
        #[cfg(unix)]
        terminate_unix_process_group(self.process_id);
        #[cfg(windows)]
        {
            // A .cmd/.bat wrapper may create its Node child before the wrapper
            // is attached to the Job Object. taskkill /T covers that race;
            // closing the Job Object still handles descendants created later.
            terminate_windows_process_tree(self.child.id());
        }
        let _ = self.child.kill();
        #[cfg(windows)]
        self.job.close();
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.shutdown(true);
        }
    }
}

fn build_command(launch: &CodexLaunchSpec) -> Command {
    #[cfg(windows)]
    {
        let is_script = launch
            .executable
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("cmd") || value.eq_ignore_ascii_case("bat"))
            .unwrap_or(false);
        if is_script {
            let mut command = Command::new("cmd.exe");
            command.args(["/D", "/S", "/C"]);
            command.arg(format!(
                "\"{}\" app-server --stdio",
                launch.executable.to_string_lossy().replace('"', "\\\"")
            ));
            return command;
        }
    }

    let mut command = Command::new(&launch.executable);
    command.args(["app-server", "--stdio"]);
    command
}

fn configure_command(command: &mut Command) {
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            if setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(WINDOWS_CREATE_NO_WINDOW);
    }
}

fn spawn_stdout_reader(stdout: ChildStdout) -> (StdoutEvents, JoinHandle<()>) {
    let (sender, receiver) = mpsc::sync_channel(STDOUT_EVENT_CHANNEL_CAPACITY);
    let failed = Arc::new(AtomicBool::new(false));
    let reader_failed = Arc::clone(&failed);
    let task = thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut buffer = Vec::new();
        let mut budget = StdoutBudget::new(MAX_STDOUT_EVENT_COUNT, MAX_STDOUT_TOTAL_BYTES);
        loop {
            match read_bounded_line(&mut reader, &mut buffer) {
                Ok(Some(line)) => {
                    if !budget.record(line.len()) {
                        fail_stdout_reader(&sender, &reader_failed);
                        return;
                    }
                    if sender.send(StdoutEvent::Line(line)).is_err() {
                        return;
                    }
                }
                Ok(None) => return,
                Err(_) => {
                    fail_stdout_reader(&sender, &reader_failed);
                    return;
                }
            }
        }
    });
    (StdoutEvents { receiver, failed }, task)
}

fn fail_stdout_reader(sender: &SyncSender<StdoutEvent>, failed: &AtomicBool) {
    failed.store(true, Ordering::Release);
    let _ = sender.send(StdoutEvent::Failed);
}

fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    buffer: &mut Vec<u8>,
) -> io::Result<Option<String>> {
    buffer.clear();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if buffer.is_empty() {
                return Ok(None);
            }
            break;
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |index| index + 1);
        if buffer.len().saturating_add(take) > MAX_JSON_LINE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "App Server JSONL line exceeds the size limit",
            ));
        }
        buffer.extend_from_slice(&available[..take]);
        reader.consume(take);
        if newline.is_some() {
            break;
        }
    }
    String::from_utf8(std::mem::take(buffer))
        .map(Some)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "App Server output is not UTF-8"))
}

fn drain_stderr(mut stderr: ChildStderr) {
    let mut buffer = [0_u8; 4096];
    let mut retained = 0usize;
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(read) => retained = retained.saturating_add(read).min(MAX_STDERR_BYTES),
        }
    }
}

fn parse_jsonrpc_response(
    message: &Value,
    request_id: u64,
) -> Result<Option<Value>, ProtocolError> {
    let object = message.as_object().ok_or(ProtocolError::Malformed)?;
    if let Some(version) = object.get("jsonrpc") {
        if version.as_str() != Some("2.0") {
            return Err(ProtocolError::Malformed);
        }
    }

    let Some(id) = object.get("id") else {
        if object.get("method").and_then(Value::as_str).is_some() {
            return Ok(None);
        }
        return Err(ProtocolError::Malformed);
    };
    if id != &json!(request_id) {
        return Ok(None);
    }
    if object.contains_key("method") || object.contains_key("params") {
        return Err(ProtocolError::Malformed);
    }

    match (object.get("error"), object.get("result")) {
        (Some(error), None) if error.is_object() => Err(ProtocolError::JsonRpc),
        (None, Some(result)) => Ok(Some(result.clone())),
        _ => Err(ProtocolError::Malformed),
    }
}

#[cfg(unix)]
extern "C" {
    fn setsid() -> i32;
    fn kill(pid: i32, signal: i32) -> i32;
}

#[cfg(unix)]
fn terminate_unix_process_group(process_id: u32) {
    let Ok(process_id) = i32::try_from(process_id) else {
        return;
    };
    unsafe {
        const SIGTERM: i32 = 15;
        const SIGKILL: i32 = 9;
        let _ = kill(-process_id, SIGTERM);
        thread::sleep(Duration::from_millis(30));
        let _ = kill(-process_id, SIGKILL);
    }
}

#[cfg(windows)]
struct WindowsJob {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
fn terminate_windows_process_tree(process_id: u32) {
    use std::os::windows::process::CommandExt;

    let taskkill_path = std::env::var_os("SystemRoot")
        .map(|root| Path::new(&root).join("System32").join("taskkill.exe"))
        .filter(|path| path.is_file());
    let mut command = match taskkill_path {
        Some(path) => Command::new(path),
        None => Command::new("taskkill"),
    };
    command
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .creation_flags(WINDOWS_CREATE_NO_WINDOW);
    let _ = command.status();
}

#[cfg(windows)]
impl WindowsJob {
    fn attach(process_id: u32) -> Result<Self, ()> {
        use std::ffi::c_void;
        use std::mem::size_of;
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
        };

        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &mut info as *mut _ as *mut c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) != 0;
            let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, process_id);
            let assigned =
                configured && !process.is_null() && AssignProcessToJobObject(handle, process) != 0;
            if !process.is_null() {
                CloseHandle(process);
            }
            if !assigned {
                CloseHandle(handle);
                return Err(());
            }
            Ok(Self { handle })
        }
    }

    fn close(&mut self) {
        if !self.handle.is_null() {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        fetch_model_catalog, parse_model, parse_next_cursor, read_bounded_line,
        run_protocol_with_timeout, StdoutBudget,
    };
    use crate::cli_manager::CodexLaunchSpec;
    use serde_json::json;
    use std::io::{BufReader, Cursor};

    #[cfg(unix)]
    use std::ffi::OsString;
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::path::PathBuf;
    #[cfg(unix)]
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    fn fixture(script: &str) -> (PathBuf, PathBuf, CodexLaunchSpec) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "aio codex catalog fixture {}-{}",
            std::process::id(),
            unique
        ));
        let home = root.join("codex home");
        let script_path = root.join("codex fixture.sh");
        fs::create_dir_all(&home).unwrap();
        fs::write(&script_path, script).unwrap();
        let mut permissions = fs::metadata(&script_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).unwrap();
        let launch = CodexLaunchSpec {
            executable: script_path.clone(),
            runtime_path: OsString::from("/usr/bin:/bin"),
            version: Some("fixture".to_string()),
        };
        (root, home, launch)
    }

    #[test]
    fn parse_model_preserves_missing_empty_and_non_empty_efforts() {
        assert_eq!(
            parse_model(&json!({"id": "missing"}))
                .unwrap()
                .supported_reasoning_efforts,
            None
        );
        assert_eq!(
            parse_model(&json!({"id": "empty", "supportedReasoningEfforts": []}))
                .unwrap()
                .supported_reasoning_efforts,
            Some(Vec::new())
        );
        assert_eq!(
            parse_model(&json!({
                "id": "known",
                "supportedReasoningEfforts": [{"reasoningEffort": "max"}]
            }))
            .unwrap()
            .supported_reasoning_efforts
            .unwrap()[0]
                .reasoning_effort,
            "max"
        );
    }

    #[test]
    fn parse_model_uses_id_when_model_or_display_name_is_missing() {
        let model = parse_model(&json!({"id": "gpt-5.6-sol"})).unwrap();
        assert_eq!(model.model, "gpt-5.6-sol");
        assert_eq!(model.display_name, "gpt-5.6-sol");
    }

    #[test]
    fn parse_model_rejects_malformed_declared_fields() {
        assert!(parse_model(&json!({"id": ""})).is_err());
        assert!(parse_model(&json!({
            "id": "broken",
            "supportedReasoningEfforts": "max"
        }))
        .is_err());
        assert!(parse_model(&json!({
            "id": "broken",
            "supportedReasoningEfforts": [{}]
        }))
        .is_err());
    }

    #[test]
    fn parse_next_cursor_rejects_invalid_types_instead_of_returning_partial_catalog() {
        assert_eq!(parse_next_cursor(&json!({})).unwrap(), None);
        assert_eq!(
            parse_next_cursor(&json!({"nextCursor": null})).unwrap(),
            None
        );
        assert_eq!(parse_next_cursor(&json!({"nextCursor": ""})).unwrap(), None);
        assert_eq!(
            parse_next_cursor(&json!({"nextCursor": "page-2"})).unwrap(),
            Some("page-2".to_string())
        );
        assert!(matches!(
            parse_next_cursor(&json!({"nextCursor": 2})),
            Err(super::ProtocolError::Malformed)
        ));
    }

    #[test]
    fn bounded_reader_rejects_oversized_jsonl_lines() {
        let bytes = vec![b'x'; super::MAX_JSON_LINE_BYTES + 1];
        let mut reader = BufReader::new(Cursor::new(bytes));
        let mut buffer = Vec::new();
        assert!(read_bounded_line(&mut reader, &mut buffer).is_err());
    }

    #[test]
    fn stdout_budget_enforces_event_and_total_byte_limits() {
        let mut event_budget = StdoutBudget::new(2, 10);
        assert!(event_budget.record(4));
        assert!(event_budget.record(6));
        assert!(!event_budget.record(0));

        let mut byte_budget = StdoutBudget::new(3, 10);
        assert!(byte_budget.record(10));
        assert!(!byte_budget.record(1));
    }

    #[cfg(unix)]
    #[test]
    fn fixture_covers_handshake_notifications_pagination_and_response_ids() {
        let (root, home, launch) = fixture(
            r##"#!/bin/sh
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$CODEX_HOME/requests.log"
  case "$line" in
    *'"method":"initialize","params"'*)
      printf '%s\n' '{"method":"notice","params":{}}'
      printf '%s\n' '{"id":99,"result":{}}'
      printf '%s\n' '{"id":1,"result":{}}'
      ;;
    *'"method":"model/list"'*)
      case "$line" in
        *'"cursor":"page-2"'*)
          printf '%s\n' '{"id":3,"result":{"data":[{"id":"gpt-5.6-luna","model":"gpt-5.6-luna","hidden":true,"supportedReasoningEfforts":[{"reasoningEffort":"low"},{"reasoningEffort":"max"}]}]}}'
          ;;
        *)
          printf '%s\n' '{"id":77,"result":{"data":[]}}'
          printf '%s\n' '{"id":2,"result":{"data":[{"id":"gpt-5.6-sol","displayName":"Sol","isDefault":true,"supportedReasoningEfforts":[{"reasoningEffort":"max","description":"deep"},{"reasoningEffort":"ultra","description":"delegate"},{"reasoningEffort":"future-effort"}],"defaultReasoningEffort":"max"}],"nextCursor":"page-2"}}'
          ;;
      esac
      ;;
  esac
done
"##,
        );

        let models = fetch_model_catalog(&launch, &home).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model, "gpt-5.6-sol");
        assert_eq!(
            models[0].supported_reasoning_efforts.as_ref().unwrap()[1].reasoning_effort,
            "ultra"
        );
        assert!(models[1].hidden);
        assert_eq!(models[1].model, "gpt-5.6-luna");

        let requests = fs::read_to_string(home.join("requests.log")).unwrap();
        assert!(requests.contains("\"method\":\"initialized\""));
        assert!(requests.contains("\"includeHidden\":true"));
        assert!(requests.contains("\"cursor\":\"page-2\""));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn fixture_maps_json_rpc_errors_and_timeouts_without_partial_results() {
        let (root, home, launch) = fixture(
            r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize","params"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{}}'
      ;;
    *'"method":"model/list"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"error":{"code":-1,"message":"fixture error"}}'
      ;;
  esac
done
"##,
        );
        assert!(matches!(
            fetch_model_catalog(&launch, &home),
            Err(super::ProtocolError::JsonRpc)
        ));
        let _ = fs::remove_dir_all(&root);

        let (root, home, launch) = fixture(
            r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize","params"'*) : ;;
  esac
done
"##,
        );
        let mut child = super::ManagedChild::spawn(&launch, &home).unwrap();
        assert!(matches!(
            run_protocol_with_timeout(&mut child, Duration::from_millis(20)),
            Err(super::ProtocolError::Timeout)
        ));
        child.shutdown(true);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn fixture_fails_closed_when_stdout_event_budget_is_exceeded() {
        let (root, home, launch) = fixture(
            r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize","params"'*)
      index=0
      while [ "$index" -le 256 ]; do
        printf '%s\n' '{"method":"notice","params":{}}'
        index=$((index + 1))
      done
      printf '%s\n' '{"id":1,"result":{}}'
      ;;
  esac
done
"##,
        );

        assert!(matches!(
            fetch_model_catalog(&launch, &home),
            Err(super::ProtocolError::Malformed)
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn fixture_rejects_invalid_json_rpc_envelopes_and_server_requests() {
        let (root, home, launch) = fixture(
            r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize","params"'*)
      printf '%s\n' '{"jsonrpc":"1.0","id":1,"result":{}}'
      ;;
  esac
done
"##,
        );
        assert!(matches!(
            fetch_model_catalog(&launch, &home),
            Err(super::ProtocolError::Malformed)
        ));
        let _ = fs::remove_dir_all(&root);

        let (root, home, launch) = fixture(
            r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize","params"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"error":{},"result":{}}'
      ;;
  esac
done
"##,
        );
        assert!(matches!(
            fetch_model_catalog(&launch, &home),
            Err(super::ProtocolError::Malformed)
        ));
        let _ = fs::remove_dir_all(&root);

        let (root, home, launch) = fixture(
            r##"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize","params"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"server/request","params":{}}'
      ;;
  esac
done
"##,
        );
        assert!(matches!(
            fetch_model_catalog(&launch, &home),
            Err(super::ProtocolError::Malformed)
        ));
        let _ = fs::remove_dir_all(root);
    }
}
