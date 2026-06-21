//! Usage: Sandboxed WASM plugin runtime foundation for community code plugins.
#![allow(dead_code)]

use crate::gateway::plugins::context::{GatewayHookAction, GatewayHookResult};
use crate::shared::error::{AppError, AppResult};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use wasmtime::{Config, Engine, Linker, Module, ResourceLimiter, Store};

const DEFAULT_MAX_JSON_BYTES: usize = 256 * 1024;
const DEFAULT_MEMORY_LIMIT_BYTES: usize = 16 * 1024 * 1024;
const DEFAULT_FUEL: u64 = 5_000_000;
const WASM_PAGE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct WasmRuntimeLimits {
    pub(crate) max_input_bytes: usize,
    pub(crate) max_output_bytes: usize,
    pub(crate) memory_limit_bytes: usize,
    pub(crate) fuel: u64,
}

impl Default for WasmRuntimeLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: DEFAULT_MAX_JSON_BYTES,
            max_output_bytes: DEFAULT_MAX_JSON_BYTES,
            memory_limit_bytes: DEFAULT_MEMORY_LIMIT_BYTES,
            fuel: DEFAULT_FUEL,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WasmHookInvocation {
    pub(crate) plugin_id: String,
    pub(crate) hook: String,
    pub(crate) trace_id: Option<String>,
    pub(crate) config: Value,
    pub(crate) context: Value,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WasmPluginExecutor {
    pub(crate) limits: WasmRuntimeLimits,
}

#[derive(Debug)]
struct StoreState {
    memory_limit_bytes: usize,
}

impl ResourceLimiter for StoreState {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(desired <= self.memory_limit_bytes)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(desired <= 1024)
    }

    fn instances(&self) -> usize {
        1
    }

    fn memories(&self) -> usize {
        1
    }

    fn tables(&self) -> usize {
        4
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WasmRequestEnvelope<'a> {
    abi_version: &'static str,
    plugin_id: &'a str,
    hook: &'a str,
    trace_id: Option<&'a str>,
    config: &'a Value,
    context: &'a Value,
}

impl WasmPluginExecutor {
    pub(crate) fn execute_module_bytes(
        &self,
        module_bytes: &[u8],
        invocation: WasmHookInvocation,
    ) -> AppResult<Value> {
        if self.limits.memory_limit_bytes < WASM_PAGE_BYTES {
            return Err(AppError::new(
                "PLUGIN_WASM_INVALID_LIMIT",
                "WASM memory limit must be at least one WebAssembly page",
            ));
        }

        let request = WasmRequestEnvelope {
            abi_version: "1.0.0",
            plugin_id: &invocation.plugin_id,
            hook: &invocation.hook,
            trace_id: invocation.trace_id.as_deref(),
            config: &invocation.config,
            context: &invocation.context,
        };
        let input = serde_json::to_vec(&request).map_err(|err| {
            AppError::new(
                "PLUGIN_WASM_INPUT_ENCODE_FAILED",
                format!("failed to encode WASM plugin input: {err}"),
            )
        })?;
        if input.len() > self.limits.max_input_bytes {
            return Err(AppError::new(
                "PLUGIN_WASM_INPUT_TOO_LARGE",
                format!(
                    "WASM plugin input exceeded {} bytes",
                    self.limits.max_input_bytes
                ),
            ));
        }

        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).map_err(|err| {
            AppError::new(
                "PLUGIN_WASM_ENGINE_FAILED",
                format!("failed to create WASM engine: {err}"),
            )
        })?;
        let module = Module::new(&engine, module_bytes).map_err(|err| {
            AppError::new(
                "PLUGIN_WASM_INVALID_MODULE",
                format!("failed to compile WASM module: {err}"),
            )
        })?;
        let mut store = Store::new(
            &engine,
            StoreState {
                memory_limit_bytes: self.limits.memory_limit_bytes,
            },
        );
        store.limiter(|state| state);
        store.set_fuel(self.limits.fuel).map_err(|err| {
            AppError::new(
                "PLUGIN_WASM_FUEL_SETUP_FAILED",
                format!("failed to configure WASM fuel: {err}"),
            )
        })?;

        let linker: Linker<StoreState> = Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module).map_err(|err| {
            AppError::new(
                "PLUGIN_WASM_IMPORT_DENIED",
                format!("failed to instantiate WASM module without host imports: {err}"),
            )
        })?;
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            AppError::new(
                "PLUGIN_WASM_MISSING_MEMORY",
                "WASM plugin must export memory named memory",
            )
        })?;
        memory.write(&mut store, 0, &input).map_err(|_| {
            AppError::new(
                "PLUGIN_WASM_MEMORY_WRITE_FAILED",
                "failed to write WASM plugin input into guest memory",
            )
        })?;

        let handle = instance
            .get_typed_func::<(i32, i32), i64>(&mut store, "aio_plugin_handle")
            .map_err(|err| {
                AppError::new(
                    "PLUGIN_WASM_MISSING_ENTRYPOINT",
                    format!("missing WASM guest entrypoint aio_plugin_handle: {err}"),
                )
            })?;
        let packed = handle
            .call(&mut store, (0, input.len() as i32))
            .map_err(|err| {
                let fuel_exhausted = store.get_fuel().map(|fuel| fuel == 0).unwrap_or(false);
                map_wasm_call_error("WASM plugin execution failed", err, fuel_exhausted)
            })?;
        let ptr = (packed >> 32) as usize;
        let len = (packed & 0xffff_ffff) as usize;
        if len > self.limits.max_output_bytes {
            return Err(AppError::new(
                "PLUGIN_WASM_OUTPUT_TOO_LARGE",
                format!(
                    "WASM plugin output exceeded {} bytes",
                    self.limits.max_output_bytes
                ),
            ));
        }
        let mut output = vec![0_u8; len];
        memory.read(&store, ptr, &mut output).map_err(|_| {
            AppError::new(
                "PLUGIN_WASM_MEMORY_READ_FAILED",
                "failed to read WASM plugin output from guest memory",
            )
        })?;
        serde_json::from_slice(&output).map_err(|err| {
            AppError::new(
                "PLUGIN_WASM_INVALID_OUTPUT",
                format!("WASM plugin output was not valid JSON: {err}"),
            )
        })
    }
}

fn map_wasm_call_error(prefix: &str, err: wasmtime::Error, fuel_exhausted: bool) -> AppError {
    let message = err.to_string();
    if fuel_exhausted || message.to_ascii_lowercase().contains("fuel") {
        return AppError::new(
            "PLUGIN_WASM_FUEL_EXHAUSTED",
            format!("{prefix}: fuel exhausted"),
        );
    }
    AppError::new(
        "PLUGIN_WASM_EXECUTION_FAILED",
        format!("{prefix}: {message}"),
    )
}

fn gateway_hook_result_from_wasm_output(value: Value) -> AppResult<GatewayHookResult> {
    let object = value.as_object().ok_or_else(|| {
        AppError::new(
            "PLUGIN_WASM_INVALID_OUTPUT",
            "WASM plugin output must be a JSON object",
        )
    })?;
    if object.contains_key("contextPatch") {
        return Err(AppError::new(
            "PLUGIN_WASM_INVALID_OUTPUT",
            "WASM plugin output used legacy contextPatch; use requestBody, responseBody, streamChunk, logMessage, or headers",
        ));
    }

    let action = object
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::new(
                "PLUGIN_WASM_INVALID_OUTPUT",
                "WASM plugin output must include string action",
            )
        })?;
    let mut result = GatewayHookResult::continue_unchanged();
    match action {
        "pass" => {}
        "warn" => {
            result.reason = optional_string(object, "message")?;
        }
        "block" => {
            result.action = GatewayHookAction::Block;
            result.reason = optional_string(object, "reason")?;
        }
        "replace" => {
            result.request_body = optional_string(object, "requestBody")?;
            result.response_body = optional_string(object, "responseBody")?;
            result.stream_chunk = optional_string(object, "streamChunk")?;
            result.log_message = optional_string(object, "logMessage")?;
            result.headers = optional_string_map(object, "headers")?.unwrap_or_default();
        }
        other => {
            return Err(AppError::new(
                "PLUGIN_WASM_INVALID_OUTPUT",
                format!("unsupported WASM plugin action: {other}"),
            ));
        }
    }
    Ok(result)
}

fn optional_string(
    object: &serde_json::Map<String, Value>,
    key: &'static str,
) -> AppResult<Option<String>> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(AppError::new(
            "PLUGIN_WASM_INVALID_OUTPUT",
            format!("WASM plugin output field {key} must be a string"),
        )),
    }
}

fn optional_string_map(
    object: &serde_json::Map<String, Value>,
    key: &'static str,
) -> AppResult<Option<BTreeMap<String, String>>> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(map) = value.as_object() else {
        return Err(AppError::new(
            "PLUGIN_WASM_INVALID_OUTPUT",
            format!("WASM plugin output field {key} must be an object"),
        ));
    };
    let mut out = BTreeMap::new();
    for (name, value) in map {
        let Some(header_value) = value.as_str() else {
            return Err(AppError::new(
                "PLUGIN_WASM_INVALID_OUTPUT",
                format!("WASM plugin output header {name} must be a string"),
            ));
        };
        out.insert(name.clone(), header_value.to_string());
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_pass_module() -> Vec<u8> {
        wat::parse_str(
            r#"
            (module
              (memory (export "memory") 1)
              (data (i32.const 1024) "{\"action\":\"pass\"}")
              (func (export "aio_plugin_handle") (param i32 i32) (result i64)
                i64.const 1024
                i64.const 32
                i64.shl
                i64.const 17
                i64.or))
            "#,
        )
        .expect("valid wat")
    }

    fn wasi_filesystem_import_module() -> Vec<u8> {
        wat::parse_str(
            r#"
            (module
              (import "wasi_snapshot_preview1" "path_open"
                (func $path_open
                  (param i32 i32 i32 i32 i32 i64 i64 i32 i32)
                  (result i32)))
              (memory (export "memory") 1)
              (data (i32.const 1024) "{\"action\":\"pass\"}")
              (func (export "aio_plugin_handle") (param i32 i32) (result i64)
                i64.const 1024
                i64.const 32
                i64.shl
                i64.const 17
                i64.or))
            "#,
        )
        .expect("valid wat")
    }

    fn dead_loop_module() -> Vec<u8> {
        wat::parse_str(
            r#"
            (module
              (memory (export "memory") 1)
              (func (export "aio_plugin_handle") (param i32 i32) (result i64)
                (loop
                  br 0)
                i64.const 0))
            "#,
        )
        .expect("valid wat")
    }

    #[test]
    fn plugin_wasm_executes_valid_module() {
        let executor = WasmPluginExecutor::default();
        let output = executor
            .execute_module_bytes(
                &valid_pass_module(),
                WasmHookInvocation {
                    plugin_id: "example.echo".to_string(),
                    hook: "gateway.request.afterBodyRead".to_string(),
                    trace_id: Some("trace-1".to_string()),
                    config: json!({}),
                    context: json!({"body": {"messages": []}}),
                },
            )
            .expect("valid wasm execution");

        assert_eq!(output, json!({"action": "pass"}));
    }

    #[test]
    fn plugin_wasm_denies_wasi_filesystem_imports() {
        let executor = WasmPluginExecutor::default();
        let err = executor
            .execute_module_bytes(
                &wasi_filesystem_import_module(),
                WasmHookInvocation {
                    plugin_id: "example.fs".to_string(),
                    hook: "gateway.request.afterBodyRead".to_string(),
                    trace_id: None,
                    config: json!({}),
                    context: json!({}),
                },
            )
            .expect_err("WASI filesystem imports are not linked");

        assert!(err.to_string().contains("PLUGIN_WASM_IMPORT_DENIED"));
    }

    #[test]
    fn plugin_wasm_terminates_dead_loop_with_fuel() {
        let executor = WasmPluginExecutor {
            limits: WasmRuntimeLimits {
                fuel: 10_000,
                ..WasmRuntimeLimits::default()
            },
        };
        let err = executor
            .execute_module_bytes(
                &dead_loop_module(),
                WasmHookInvocation {
                    plugin_id: "example.loop".to_string(),
                    hook: "gateway.request.afterBodyRead".to_string(),
                    trace_id: None,
                    config: json!({}),
                    context: json!({}),
                },
            )
            .expect_err("dead loop exhausts fuel");

        assert!(
            err.to_string().contains("PLUGIN_WASM_FUEL_EXHAUSTED"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn wasm_output_replace_request_body_maps_to_gateway_hook_result() {
        let result = gateway_hook_result_from_wasm_output(json!({
            "action": "replace",
            "requestBody": "{\"messages\":[]}",
            "headers": { "x-plugin-redacted": "1" }
        }))
        .expect("wasm output maps");

        assert_eq!(result.request_body.as_deref(), Some("{\"messages\":[]}"));
        assert_eq!(
            result.headers.get("x-plugin-redacted").map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn wasm_output_rejects_legacy_context_patch_in_vnext() {
        let err = gateway_hook_result_from_wasm_output(json!({
            "action": "replace",
            "contextPatch": { "request": { "body": "x" } }
        }))
        .expect_err("legacy contextPatch is not active vNext ABI");

        assert_eq!(err.code(), "PLUGIN_WASM_INVALID_OUTPUT");
    }
}
