//! Usage: Experimental JSON-RPC-over-stdio process plugin runtime PoC.
#![allow(dead_code)]

use crate::shared::error::{AppError, AppResult};
use serde_json::{json, Value};
use std::io::ErrorKind;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

#[derive(Debug, Clone)]
pub(crate) struct ProcessRuntimeConfig {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) start_timeout: Duration,
    pub(crate) hook_timeout: Duration,
    pub(crate) idle_recycle: Duration,
    pub(crate) max_line_bytes: usize,
}

impl Default for ProcessRuntimeConfig {
    fn default() -> Self {
        Self {
            program: String::new(),
            args: vec![],
            start_timeout: Duration::from_millis(500),
            hook_timeout: Duration::from_millis(300),
            idle_recycle: Duration::from_secs(30),
            max_line_bytes: 256 * 1024,
        }
    }
}

#[derive(Debug)]
pub(crate) struct JsonRpcProcessRuntime {
    config: ProcessRuntimeConfig,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    next_id: u64,
    last_used: Instant,
}

impl JsonRpcProcessRuntime {
    pub(crate) async fn start(config: ProcessRuntimeConfig) -> AppResult<Self> {
        if config.program.trim().is_empty() {
            return Err(AppError::new(
                "PLUGIN_PROCESS_INVALID_CONFIG",
                "process runtime program must not be empty",
            ));
        }
        let mut command = Command::new(&config.program);
        command.args(&config.args);
        command.env_clear();
        if let Some(path) = std::env::var_os("PATH") {
            command.env("PATH", path);
        }
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        #[cfg(windows)]
        {
            command.creation_flags(0x08000000);
        }

        let mut child = command.spawn().map_err(|err| {
            AppError::new(
                "PLUGIN_PROCESS_SPAWN_FAILED",
                format!("failed to start process plugin: {err}"),
            )
        })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            AppError::new(
                "PLUGIN_PROCESS_STDIN_UNAVAILABLE",
                "process plugin stdin was unavailable",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AppError::new(
                "PLUGIN_PROCESS_STDOUT_UNAVAILABLE",
                "process plugin stdout was unavailable",
            )
        })?;
        let mut runtime = Self {
            config,
            child: Some(child),
            stdin: Some(stdin),
            stdout: Some(BufReader::new(stdout)),
            next_id: 1,
            last_used: Instant::now(),
        };

        let ready = tokio::time::timeout(runtime.config.start_timeout, runtime.read_json_line())
            .await
            .map_err(|_| {
                AppError::new(
                    "PLUGIN_PROCESS_START_TIMEOUT",
                    format!(
                        "process plugin did not send ready message before start timeout: program={}, timeout_ms={}",
                        runtime.config.program,
                        runtime.config.start_timeout.as_millis()
                    ),
                )
            });
        let ready = match ready {
            Ok(Ok(value)) => value,
            Ok(Err(err)) | Err(err) => {
                runtime.kill_child().await;
                return Err(err);
            }
        };
        if ready.get("method").and_then(Value::as_str) != Some("plugin.ready") {
            runtime.kill_child().await;
            return Err(AppError::new(
                "PLUGIN_PROCESS_PROTOCOL_ERROR",
                "process plugin first message must be plugin.ready",
            ));
        }

        Ok(runtime)
    }

    pub(crate) async fn call_hook(&mut self, params: Value) -> AppResult<Value> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "plugin.handleHook",
            "params": params,
        });
        let line = serde_json::to_vec(&request).map_err(|err| {
            AppError::new(
                "PLUGIN_PROCESS_ENCODE_FAILED",
                format!("failed to encode JSON-RPC request: {err}"),
            )
        })?;
        if line.len() + 1 > self.config.max_line_bytes {
            return Err(AppError::new(
                "PLUGIN_PROCESS_REQUEST_TOO_LARGE",
                format!(
                    "process plugin request exceeded {} bytes",
                    self.config.max_line_bytes
                ),
            ));
        }

        let hook_timeout = self.config.hook_timeout;
        let result = tokio::time::timeout(hook_timeout, async {
            self.write_line(&line).await?;
            let response = self.read_json_line().await?;
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
                    "PLUGIN_PROCESS_HOOK_TIMEOUT",
                    "process plugin did not respond before hook timeout",
                ))
            }
        }
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
                "PLUGIN_PROCESS_CRASHED",
                "process plugin stdin is closed",
            ));
        };
        stdin
            .write_all(bytes)
            .await
            .map_err(|err| process_write_error("write process plugin request", err))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|err| process_write_error("terminate process plugin request line", err))?;
        stdin
            .flush()
            .await
            .map_err(|err| process_write_error("flush process plugin request", err))
    }

    async fn read_json_line(&mut self) -> AppResult<Value> {
        let max_line_bytes = self.config.max_line_bytes;
        let Some(stdout) = self.stdout.as_mut() else {
            return Err(AppError::new(
                "PLUGIN_PROCESS_CRASHED",
                "process plugin stdout is closed",
            ));
        };
        let mut line = String::new();
        let bytes = stdout.read_line(&mut line).await.map_err(|err| {
            AppError::new(
                "PLUGIN_PROCESS_READ_FAILED",
                format!("failed to read process plugin response: {err}"),
            )
        })?;
        if bytes == 0 {
            return Err(AppError::new(
                "PLUGIN_PROCESS_CRASHED",
                "process plugin exited before sending a response",
            ));
        }
        if bytes > max_line_bytes || line.len() > max_line_bytes {
            return Err(AppError::new(
                "PLUGIN_PROCESS_RESPONSE_TOO_LARGE",
                format!("process plugin response exceeded {max_line_bytes} bytes"),
            ));
        }
        serde_json::from_str(&line).map_err(|err| {
            AppError::new(
                "PLUGIN_PROCESS_PROTOCOL_ERROR",
                format!("process plugin response was not valid JSON: {err}"),
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

fn process_write_error(operation: &str, err: std::io::Error) -> AppError {
    if matches!(
        err.kind(),
        ErrorKind::BrokenPipe | ErrorKind::ConnectionReset
    ) {
        return AppError::new(
            "PLUGIN_PROCESS_CRASHED",
            format!("process plugin pipe closed while attempting to {operation}: {err}"),
        );
    }
    AppError::new(
        "PLUGIN_PROCESS_WRITE_FAILED",
        format!("failed to {operation}: {err}"),
    )
}

fn validate_json_rpc_response(expected_id: u64, response: Value) -> AppResult<Value> {
    if response.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(AppError::new(
            "PLUGIN_PROCESS_PROTOCOL_ERROR",
            "process plugin response must use JSON-RPC 2.0",
        ));
    }
    if response.get("id").and_then(Value::as_u64) != Some(expected_id) {
        return Err(AppError::new(
            "PLUGIN_PROCESS_PROTOCOL_ERROR",
            "process plugin response id did not match request id",
        ));
    }
    if let Some(error) = response.get("error") {
        return Err(AppError::new(
            "PLUGIN_PROCESS_REMOTE_ERROR",
            format!("process plugin returned JSON-RPC error: {error}"),
        ));
    }
    response.get("result").cloned().ok_or_else(|| {
        AppError::new(
            "PLUGIN_PROCESS_PROTOCOL_ERROR",
            "process plugin response was missing result",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    fn write_node_plugin(script: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("plugin.js");
        std::fs::write(&path, script).expect("write plugin script");
        (dir, path)
    }

    fn node_config(script_path: &std::path::Path) -> ProcessRuntimeConfig {
        ProcessRuntimeConfig {
            program: "node".to_string(),
            args: vec![script_path.display().to_string()],
            start_timeout: Duration::from_secs(5),
            hook_timeout: Duration::from_secs(5),
            idle_recycle: Duration::from_millis(50),
            max_line_bytes: 256 * 1024,
        }
    }

    #[tokio::test]
    async fn plugin_process_runtime_poc_handles_valid_json_rpc_hook() {
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
        let mut runtime = JsonRpcProcessRuntime::start(node_config(&script))
            .await
            .expect("start process runtime");

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
    async fn plugin_process_runtime_poc_does_not_inherit_host_environment() {
        std::env::set_var("AIO_PLUGIN_RUNTIME_SECRET_FOR_TEST", "host-secret");
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({
              jsonrpc: "2.0",
              method: "plugin.ready",
              leaked: process.env.AIO_PLUGIN_RUNTIME_SECRET_FOR_TEST || null
            }));
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
                  result: {
                    leaked: process.env.AIO_PLUGIN_RUNTIME_SECRET_FOR_TEST || null
                  }
                }));
              }
            });
            "#,
        );
        let mut runtime = JsonRpcProcessRuntime::start(node_config(&script))
            .await
            .expect("start process runtime");

        let response = runtime
            .call_hook(json!({"hook": "gateway.request.afterBodyRead", "context": {}}))
            .await
            .expect("hook response");

        std::env::remove_var("AIO_PLUGIN_RUNTIME_SECRET_FOR_TEST");
        runtime.shutdown().await;
        assert_eq!(response["leaked"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn plugin_process_runtime_poc_reports_start_timeout() {
        let (_dir, script) = write_node_plugin(
            r#"
            setTimeout(() => {
              console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            }, 1000);
            "#,
        );
        let mut config = node_config(&script);
        config.start_timeout = Duration::from_millis(50);

        let err = JsonRpcProcessRuntime::start(config)
            .await
            .expect_err("start timeout fails");

        assert!(err.to_string().contains("PLUGIN_PROCESS_START_TIMEOUT"));
    }

    #[tokio::test]
    async fn plugin_process_runtime_poc_reports_hook_timeout_and_kills_child() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.stdin.resume();
            "#,
        );
        let mut config = node_config(&script);
        config.hook_timeout = Duration::from_millis(50);
        let mut runtime = JsonRpcProcessRuntime::start(config)
            .await
            .expect("start process runtime");

        let err = runtime
            .call_hook(json!({"hook": "gateway.request.afterBodyRead", "context": {}}))
            .await
            .expect_err("hook timeout fails");

        assert!(err.to_string().contains("PLUGIN_PROCESS_HOOK_TIMEOUT"));
        assert!(!runtime.is_running());
    }

    #[tokio::test]
    async fn plugin_process_runtime_poc_reports_crash_without_host_panic() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.exit(7);
            "#,
        );
        let mut runtime = JsonRpcProcessRuntime::start(node_config(&script))
            .await
            .expect("start process runtime");

        let err = runtime
            .call_hook(json!({"hook": "gateway.request.afterBodyRead", "context": {}}))
            .await
            .expect_err("crashed child fails hook");

        assert!(err.to_string().contains("PLUGIN_PROCESS_CRASHED"));
        assert!(!runtime.is_running());
    }

    #[tokio::test]
    async fn plugin_process_runtime_poc_recycles_idle_child() {
        let (_dir, script) = write_node_plugin(
            r#"
            console.log(JSON.stringify({jsonrpc:"2.0", method:"plugin.ready"}));
            process.stdin.resume();
            "#,
        );
        let mut config = node_config(&script);
        config.idle_recycle = Duration::from_millis(10);
        let mut runtime = JsonRpcProcessRuntime::start(config)
            .await
            .expect("start process runtime");

        tokio::time::sleep(Duration::from_millis(30)).await;
        let recycled = runtime.recycle_if_idle().await.expect("idle recycle");

        assert!(recycled);
        assert!(!runtime.is_running());
    }

    #[test]
    fn plugin_process_runtime_maps_broken_pipe_write_to_crash() {
        let err = process_write_error(
            "write process plugin request",
            std::io::Error::new(ErrorKind::BrokenPipe, "closed pipe"),
        );

        assert!(err.to_string().contains("PLUGIN_PROCESS_CRASHED"));
    }
}
