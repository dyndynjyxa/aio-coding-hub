//! Usage: Extension Host JSON-RPC-over-stdio child process transport.
#![allow(dead_code)]

use crate::shared::error::{AppError, AppResult};
use serde_json::{json, Value as JsonValue};
use std::fmt;
use std::io::ErrorKind;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub(crate) struct ExtensionHostProcessConfig {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) start_timeout: Duration,
    pub(crate) hook_timeout: Duration,
    pub(crate) idle_recycle: Duration,
    pub(crate) max_line_bytes: usize,
    pub(crate) ready_method: String,
    pub(crate) allow_startup_noise: bool,
    pub(crate) host_handler: Option<Arc<dyn ExtensionHostMethodHandler>>,
}

impl fmt::Debug for ExtensionHostProcessConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtensionHostProcessConfig")
            .field("program", &self.program)
            .field("args", &self.args)
            .field("start_timeout", &self.start_timeout)
            .field("hook_timeout", &self.hook_timeout)
            .field("idle_recycle", &self.idle_recycle)
            .field("max_line_bytes", &self.max_line_bytes)
            .field("ready_method", &self.ready_method)
            .field("allow_startup_noise", &self.allow_startup_noise)
            .field("host_handler", &self.host_handler.is_some())
            .finish()
    }
}

impl Default for ExtensionHostProcessConfig {
    fn default() -> Self {
        Self {
            program: String::new(),
            args: vec![],
            start_timeout: Duration::from_millis(500),
            hook_timeout: Duration::from_millis(300),
            idle_recycle: Duration::from_secs(30),
            max_line_bytes: 256 * 1024,
            ready_method: "plugin.ready".to_string(),
            allow_startup_noise: false,
            host_handler: None,
        }
    }
}

pub(crate) trait ExtensionHostMethodHandler: Send + Sync + 'static {
    fn handle_host_method(&self, method: &str, params: JsonValue) -> AppResult<JsonValue>;
}

#[derive(Debug)]
pub(crate) struct ExtensionHostChildProcess {
    config: ExtensionHostProcessConfig,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    next_id: u64,
    last_used: Instant,
}

impl ExtensionHostChildProcess {
    pub(crate) async fn start(config: ExtensionHostProcessConfig) -> AppResult<Self> {
        if config.program.trim().is_empty() {
            return Err(AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_INVALID_CONFIG",
                "extension host process program must not be empty",
            ));
        }
        let mut command = Command::new(&config.program);
        command.args(&config.args);
        command.env_clear();
        preserve_runtime_environment(&mut command);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        #[cfg(windows)]
        {
            command.creation_flags(0x08000000);
        }

        let mut child = command.spawn().map_err(|err| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_SPAWN_FAILED",
                format!("failed to start extension host process: {err}"),
            )
        })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_STDIN_UNAVAILABLE",
                "extension host process stdin was unavailable",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_STDOUT_UNAVAILABLE",
                "extension host process stdout was unavailable",
            )
        })?;
        if let Some(mut stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut sink = tokio::io::sink();
                let _ = tokio::io::copy(&mut stderr, &mut sink).await;
            });
        }
        let mut runtime = Self {
            config,
            child: Some(child),
            stdin: Some(stdin),
            stdout: Some(BufReader::new(stdout)),
            next_id: 1,
            last_used: Instant::now(),
        };

        let ready = tokio::time::timeout(runtime.config.start_timeout, runtime.read_ready_message())
            .await
            .map_err(|_| {
                AppError::new(
                    "PLUGIN_EXTENSION_HOST_PROCESS_START_TIMEOUT",
                    format!(
                        "extension host process did not send ready message before start timeout: program={}, timeout_ms={}",
                        runtime.config.program,
                        runtime.config.start_timeout.as_millis()
                    ),
                )
            });
        let _ready = match ready {
            Ok(Ok(value)) => value,
            Ok(Err(err)) | Err(err) => {
                runtime.kill_child().await;
                return Err(err);
            }
        };

        Ok(runtime)
    }

    pub(crate) async fn call_method(
        &mut self,
        method: &str,
        params: JsonValue,
    ) -> AppResult<JsonValue> {
        self.call_method_with_timeout(method, params, self.config.hook_timeout)
            .await
    }

    pub(crate) async fn call_method_with_timeout(
        &mut self,
        method: &str,
        params: JsonValue,
        timeout: Duration,
    ) -> AppResult<JsonValue> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = serde_json::to_vec(&request).map_err(|err| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_ENCODE_FAILED",
                format!("failed to encode JSON-RPC request: {err}"),
            )
        })?;
        if line.len() + 1 > self.config.max_line_bytes {
            return Err(AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_REQUEST_TOO_LARGE",
                format!(
                    "extension host process request exceeded {} bytes",
                    self.config.max_line_bytes
                ),
            ));
        }

        let result = tokio::time::timeout(timeout, async {
            self.write_line(&line).await?;
            let response = self.read_response_for_request(id).await?;
            validate_json_rpc_response(id, response)
        })
        .await;

        match result {
            Ok(Ok(value)) => {
                self.last_used = Instant::now();
                Ok(value)
            }
            Ok(Err(err)) => {
                self.kill_child().await;
                Err(err)
            }
            Err(_) => {
                self.kill_child().await;
                Err(AppError::new(
                    "PLUGIN_EXTENSION_HOST_PROCESS_HOOK_TIMEOUT",
                    "extension host process did not respond before hook timeout",
                ))
            }
        }
    }

    pub(crate) async fn call_hook(&mut self, params: JsonValue) -> AppResult<JsonValue> {
        self.call_method("plugin.handleHook", params).await
    }

    pub(crate) fn is_running(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            return false;
        };
        matches!(child.try_wait(), Ok(None))
    }

    pub(crate) async fn recycle_if_idle(&mut self) -> AppResult<bool> {
        if self.child.is_some() && self.last_used.elapsed() >= self.config.idle_recycle {
            self.kill_child().await;
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) async fn shutdown(&mut self) {
        self.kill_child().await;
    }

    async fn write_line(&mut self, bytes: &[u8]) -> AppResult<()> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_CRASHED",
                "extension host process stdin is closed",
            ));
        };
        stdin.write_all(bytes).await.map_err(|err| {
            extension_host_process_write_error("write extension host process request", err)
        })?;
        stdin.write_all(b"\n").await.map_err(|err| {
            extension_host_process_write_error("terminate extension host process request line", err)
        })?;
        stdin.flush().await.map_err(|err| {
            extension_host_process_write_error("flush extension host process request", err)
        })
    }

    async fn read_json_line(&mut self) -> AppResult<JsonValue> {
        let line = self.read_line_string().await?;
        serde_json::from_str(&line).map_err(|err| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_PROTOCOL_ERROR",
                format!("extension host process response was not valid JSON: {err}"),
            )
        })
    }

    async fn read_ready_message(&mut self) -> AppResult<JsonValue> {
        loop {
            let line = self.read_line_string().await?;
            let value: JsonValue = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(err) if self.config.allow_startup_noise => {
                    tracing::debug!(
                        "ignoring extension host process startup line that was not JSON: {err}"
                    );
                    continue;
                }
                Err(err) => {
                    return Err(AppError::new(
                        "PLUGIN_EXTENSION_HOST_PROCESS_PROTOCOL_ERROR",
                        format!("extension host process response was not valid JSON: {err}"),
                    ));
                }
            };
            if value.get("method").and_then(|method| method.as_str())
                == Some(self.config.ready_method.as_str())
            {
                return Ok(value);
            }
            if self.config.allow_startup_noise {
                tracing::debug!(
                    expected = self.config.ready_method.as_str(),
                    actual = value
                        .get("method")
                        .and_then(|method| method.as_str())
                        .unwrap_or("<missing>"),
                    "ignoring extension host process startup message before ready"
                );
                continue;
            }
            return Err(AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_PROTOCOL_ERROR",
                format!(
                    "extension host process first message must be {}",
                    self.config.ready_method
                ),
            ));
        }
    }

    async fn read_response_for_request(&mut self, expected_id: u64) -> AppResult<JsonValue> {
        loop {
            let response = self.read_json_line().await?;
            if response.get("id").and_then(|value| value.as_u64()) == Some(expected_id) {
                return Ok(response);
            }
            if response.get("method").and_then(|value| value.as_str()) == Some("host.call") {
                let id = response.get("id").cloned().unwrap_or(JsonValue::Null);
                let result = self.handle_host_call(response);
                let message = match result {
                    Ok(value) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": value,
                    }),
                    Err(error) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32000,
                            "message": error.to_string(),
                            "data": { "code": error.code() },
                        },
                    }),
                };
                let bytes = serde_json::to_vec(&message).map_err(|err| {
                    AppError::new(
                        "PLUGIN_EXTENSION_HOST_PROCESS_ENCODE_FAILED",
                        format!("failed to encode host JSON-RPC response: {err}"),
                    )
                })?;
                if bytes.len() + 1 > self.config.max_line_bytes {
                    return Err(AppError::new(
                        "PLUGIN_EXTENSION_HOST_PROCESS_RESPONSE_TOO_LARGE",
                        format!(
                            "host JSON-RPC response exceeded {} bytes",
                            self.config.max_line_bytes
                        ),
                    ));
                }
                self.write_line(&bytes).await?;
                continue;
            }
            return Err(AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_PROTOCOL_ERROR",
                "extension host process sent an unexpected JSON-RPC message",
            ));
        }
    }

    fn handle_host_call(&self, request: JsonValue) -> AppResult<JsonValue> {
        let method = request
            .get("params")
            .and_then(|params| params.get("method"))
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                AppError::new(
                    "PLUGIN_EXTENSION_HOST_PROCESS_INVALID_HOST_CALL",
                    "host.call requires params.method",
                )
            })?;
        let params = request
            .get("params")
            .and_then(|params| params.get("params"))
            .cloned()
            .unwrap_or(JsonValue::Null);
        let handler = self.config.host_handler.as_ref().ok_or_else(|| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_HOST_CALL_UNAVAILABLE",
                format!("host method is not available: {method}"),
            )
        })?;
        handler.handle_host_method(method, params)
    }

    async fn read_line_string(&mut self) -> AppResult<String> {
        let max_line_bytes = self.config.max_line_bytes;
        let Some(stdout) = self.stdout.as_mut() else {
            return Err(AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_CRASHED",
                "extension host process stdout is closed",
            ));
        };
        let bytes = read_bounded_line(stdout, max_line_bytes).await?;
        String::from_utf8(bytes).map_err(|err| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_PROTOCOL_ERROR",
                format!("extension host process response was not valid UTF-8: {err}"),
            )
        })
    }

    async fn kill_child(&mut self) {
        self.stdin.take();
        self.stdout.take();
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(Some(_)) => {}
                Ok(None) => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
                Err(_) => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
        }
    }
}

impl Drop for ExtensionHostChildProcess {
    fn drop(&mut self) {
        self.stdin.take();
        self.stdout.take();
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let _ = child.wait().await;
                });
            }
        }
    }
}

fn preserve_runtime_environment(command: &mut Command) {
    const ENV_ALLOWLIST: &[&str] = &[
        "PATH",
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "TMPDIR",
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "APPDIR",
        "APPIMAGE",
        "ARGV0",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
    ];

    for key in ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}

async fn read_bounded_line(
    stdout: &mut BufReader<ChildStdout>,
    max_line_bytes: usize,
) -> AppResult<Vec<u8>> {
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        let read = stdout.read(&mut byte).await.map_err(|err| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_READ_FAILED",
                format!("failed to read extension host process response: {err}"),
            )
        })?;
        if read == 0 {
            if line.is_empty() {
                return Err(AppError::new(
                    "PLUGIN_EXTENSION_HOST_PROCESS_CRASHED",
                    "extension host process exited before sending a response",
                ));
            }
            return Ok(line);
        }
        line.push(byte[0]);
        if line.len() > max_line_bytes {
            return Err(AppError::new(
                "PLUGIN_EXTENSION_HOST_PROCESS_RESPONSE_TOO_LARGE",
                format!("extension host process response exceeded {max_line_bytes} bytes"),
            ));
        }
        if byte[0] == b'\n' {
            return Ok(line);
        }
    }
}

fn extension_host_process_write_error(operation: &str, err: std::io::Error) -> AppError {
    if matches!(
        err.kind(),
        ErrorKind::BrokenPipe | ErrorKind::ConnectionReset
    ) {
        return AppError::new(
            "PLUGIN_EXTENSION_HOST_PROCESS_CRASHED",
            format!("extension host process pipe closed while attempting to {operation}: {err}"),
        );
    }
    AppError::new(
        "PLUGIN_EXTENSION_HOST_PROCESS_WRITE_FAILED",
        format!("failed to {operation}: {err}"),
    )
}

fn validate_json_rpc_response(expected_id: u64, response: JsonValue) -> AppResult<JsonValue> {
    if response.get("jsonrpc").and_then(|value| value.as_str()) != Some("2.0") {
        return Err(AppError::new(
            "PLUGIN_EXTENSION_HOST_PROCESS_PROTOCOL_ERROR",
            "extension host process response must use JSON-RPC 2.0",
        ));
    }
    if response.get("id").and_then(|value| value.as_u64()) != Some(expected_id) {
        return Err(AppError::new(
            "PLUGIN_EXTENSION_HOST_PROCESS_PROTOCOL_ERROR",
            "extension host process response id did not match request id",
        ));
    }
    if let Some(error) = response.get("error") {
        if let Some(code) = error
            .get("data")
            .and_then(|data| data.get("code"))
            .and_then(|code| code.as_str())
        {
            let message = error
                .get("message")
                .and_then(|message| message.as_str())
                .unwrap_or("extension host process returned JSON-RPC error");
            return Err(AppError::new(code, message));
        }
        return Err(AppError::new(
            "PLUGIN_EXTENSION_HOST_PROCESS_REMOTE_ERROR",
            format!("extension host process returned JSON-RPC error: {error}"),
        ));
    }
    response.get("result").cloned().ok_or_else(|| {
        AppError::new(
            "PLUGIN_EXTENSION_HOST_PROCESS_PROTOCOL_ERROR",
            "extension host process response was missing result",
        )
    })
}

#[cfg(test)]
#[test]
fn extension_host_process_env_probe_for_tests() {
    if !std::env::args().any(|arg| arg == "--extension-host-process-env-probe") {
        return;
    }

    use std::io::{BufRead, Write};

    let leaked_env = || {
        std::env::var_os("AIO_PLUGIN_RUNTIME_SECRET_FOR_TEST")
            .map(|value| JsonValue::String(value.to_string_lossy().into_owned()))
            .unwrap_or(JsonValue::Null)
    };

    let mut stdout = std::io::stdout();
    writeln!(
        stdout,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "method": "plugin.ready",
            "leaked": leaked_env(),
        })
    )
    .expect("write ready");
    stdout.flush().expect("flush ready");

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.expect("read request");
        if line.trim().is_empty() {
            continue;
        }
        let request: JsonValue = serde_json::from_str(&line).expect("parse request");
        writeln!(
            stdout,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(JsonValue::Null),
                "result": {
                    "leaked": leaked_env(),
                },
            })
        )
        .expect("write response");
        stdout.flush().expect("flush response");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    struct EchoHostHandler;

    impl ExtensionHostMethodHandler for EchoHostHandler {
        fn handle_host_method(&self, method: &str, params: JsonValue) -> AppResult<JsonValue> {
            Ok(json!({
                "method": method,
                "params": params,
            }))
        }
    }

    fn write_node_plugin(script: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("plugin.js");
        std::fs::write(&path, script).expect("write plugin script");
        (dir, path)
    }

    const NODE_PROCESS_TEST_START_TIMEOUT: Duration = Duration::from_secs(30);

    fn node_config(script_path: &std::path::Path) -> ExtensionHostProcessConfig {
        ExtensionHostProcessConfig {
            program: "node".to_string(),
            args: vec![script_path.display().to_string()],
            start_timeout: NODE_PROCESS_TEST_START_TIMEOUT,
            hook_timeout: Duration::from_secs(5),
            idle_recycle: Duration::from_millis(50),
            max_line_bytes: 256 * 1024,
            ready_method: "plugin.ready".to_string(),
            allow_startup_noise: false,
            host_handler: None,
        }
    }

    #[test]
    fn node_process_test_config_uses_ci_tolerant_start_timeout() {
        let path = std::path::Path::new("plugin.js");
        let config = node_config(path);

        assert_eq!(config.start_timeout, NODE_PROCESS_TEST_START_TIMEOUT);
    }

    #[tokio::test]
    async fn extension_host_process_handles_valid_json_rpc_hook() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.stdin.setEncoding("utf8");
            let buffer = "";
            process.stdin.on("data", chunk => {
              buffer += chunk;
              const lines = buffer.split("\n");
              buffer = lines.pop();
              for (const line of lines) {
                if (!line.trim()) continue;
                const req = JSON.parse(line);
                console.log(JSON.stringify({
                  jsonrpc: "2.0",
                  id: req.id,
                  result: { action: "pass", hook: req.params.hook }
                }));
              }
            });
            "#,
        );
        let mut runtime = ExtensionHostChildProcess::start(node_config(&script))
            .await
            .expect("start extension host process");

        let response = runtime
            .call_hook(json!({"hook": "gateway.request.afterBodyRead", "context": {}}))
            .await
            .expect("hook response");

        assert_eq!(
            response,
            json!({"action": "pass", "hook": "gateway.request.afterBodyRead"})
        );
        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn extension_host_process_handles_host_call_before_final_response() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.stdin.setEncoding("utf8");
            let buffer = "";
            let pendingRequest = null;
            process.stdin.on("data", chunk => {
              buffer += chunk;
              const lines = buffer.split("\n");
              buffer = lines.pop();
              for (const line of lines) {
                if (!line.trim()) continue;
                const req = JSON.parse(line);
                if (pendingRequest && req.id === 99) {
                  console.log(JSON.stringify({
                    jsonrpc: "2.0",
                    id: pendingRequest.id,
                    result: { host: req.result }
                  }));
                  pendingRequest = null;
                  continue;
                }
                pendingRequest = req;
                console.log(JSON.stringify({
                  jsonrpc: "2.0",
                  id: 99,
                  method: "host.call",
                  params: { method: "echo", params: req.params }
                }));
              }
            });
            "#,
        );
        let mut config = node_config(&script);
        config.host_handler = Some(Arc::new(EchoHostHandler));
        let mut runtime = ExtensionHostChildProcess::start(config)
            .await
            .expect("start extension host process");

        let response = runtime
            .call_method(
                "plugin.handleHook",
                json!({ "hook": "gateway.request.afterBodyRead" }),
            )
            .await
            .expect("host call response");

        assert_eq!(
            response,
            json!({
                "host": {
                    "method": "echo",
                    "params": { "hook": "gateway.request.afterBodyRead" }
                }
            })
        );
        runtime.shutdown().await;
    }

    #[tokio::test]
    async fn extension_host_process_does_not_inherit_host_environment() {
        std::env::set_var("AIO_PLUGIN_RUNTIME_SECRET_FOR_TEST", "host-secret");
        let program = std::env::current_exe().expect("current test executable");
        let mut runtime = ExtensionHostChildProcess::start(ExtensionHostProcessConfig {
            program: program.display().to_string(),
            args: vec![
                "--exact".to_string(),
                "app::plugins::extension_host_process::extension_host_process_env_probe_for_tests"
                    .to_string(),
                "--nocapture".to_string(),
                "--".to_string(),
                "--extension-host-process-env-probe".to_string(),
            ],
            start_timeout: NODE_PROCESS_TEST_START_TIMEOUT,
            hook_timeout: Duration::from_secs(5),
            idle_recycle: Duration::from_millis(50),
            max_line_bytes: 256 * 1024,
            ready_method: "plugin.ready".to_string(),
            allow_startup_noise: true,
            host_handler: None,
        })
        .await
        .expect("start extension host process");

        let response = runtime
            .call_hook(json!({"hook": "gateway.request.afterBodyRead", "context": {}}))
            .await
            .expect("hook response");

        std::env::remove_var("AIO_PLUGIN_RUNTIME_SECRET_FOR_TEST");
        runtime.shutdown().await;
        assert_eq!(response["leaked"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn extension_host_process_reports_start_timeout() {
        let (_dir, script) = write_node_plugin(
            r#"
            setTimeout(() => {
              console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            }, 1000);
            "#,
        );
        let mut config = node_config(&script);
        config.start_timeout = Duration::from_millis(50);

        let err = ExtensionHostChildProcess::start(config)
            .await
            .expect_err("start timeout fails");

        assert!(err
            .to_string()
            .contains("PLUGIN_EXTENSION_HOST_PROCESS_START_TIMEOUT"));
    }

    #[tokio::test]
    async fn extension_host_process_reports_hook_timeout_and_kills_child() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.stdin.resume();
            "#,
        );
        let mut config = node_config(&script);
        config.hook_timeout = Duration::from_millis(50);
        let mut runtime = ExtensionHostChildProcess::start(config)
            .await
            .expect("start extension host process");

        let err = runtime
            .call_hook(json!({"hook": "gateway.request.afterBodyRead", "context": {}}))
            .await
            .expect_err("hook timeout fails");

        assert!(err
            .to_string()
            .contains("PLUGIN_EXTENSION_HOST_PROCESS_HOOK_TIMEOUT"));
        assert!(!runtime.is_running());
    }

    #[tokio::test]
    async fn extension_host_process_reports_crash_without_host_panic() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.exit(7);
            "#,
        );
        let mut runtime = ExtensionHostChildProcess::start(node_config(&script))
            .await
            .expect("start extension host process");

        let err = runtime
            .call_hook(json!({"hook": "gateway.request.afterBodyRead", "context": {}}))
            .await
            .expect_err("crashed child fails hook");

        assert!(err
            .to_string()
            .contains("PLUGIN_EXTENSION_HOST_PROCESS_CRASHED"));
        assert!(!runtime.is_running());
    }

    #[tokio::test]
    async fn extension_host_process_rejects_oversized_response_without_full_line_buffering() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.stdin.setEncoding("utf8");
            process.stdin.on("data", () => {
              process.stdout.write("x".repeat(2048));
            });
            "#,
        );
        let mut config = node_config(&script);
        config.max_line_bytes = 512;
        let mut runtime = ExtensionHostChildProcess::start(config)
            .await
            .expect("start extension host process");

        let err = runtime
            .call_method("plugin.handleHook", json!({}))
            .await
            .expect_err("oversized response fails");

        assert_eq!(
            err.code(),
            "PLUGIN_EXTENSION_HOST_PROCESS_RESPONSE_TOO_LARGE"
        );
        assert!(!runtime.is_running());
    }

    #[tokio::test]
    async fn extension_host_process_recycles_idle_child() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.stdin.resume();
            "#,
        );
        let mut config = node_config(&script);
        config.idle_recycle = Duration::from_millis(10);
        let mut runtime = ExtensionHostChildProcess::start(config)
            .await
            .expect("start extension host process");

        tokio::time::sleep(Duration::from_millis(30)).await;
        let recycled = runtime.recycle_if_idle().await.expect("idle recycle");

        assert!(recycled);
        assert!(!runtime.is_running());
    }

    #[test]
    fn extension_host_process_maps_broken_pipe_write_to_crash() {
        let err = extension_host_process_write_error(
            "write extension host process request",
            std::io::Error::new(ErrorKind::BrokenPipe, "closed pipe"),
        );

        assert!(err
            .to_string()
            .contains("PLUGIN_EXTENSION_HOST_PROCESS_CRASHED"));
    }
}
