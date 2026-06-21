//! Rust/WASM SDK contracts for AIO Coding Hub plugins.
//!
//! The SDK mirrors plugin manifest and hook ABI shapes. It does not grant host
//! capabilities; the desktop host still trims context and enforces permissions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub runtime: PluginRuntime,
    pub hooks: Vec<PluginHook>,
    pub permissions: Vec<String>,
    pub host_compatibility: PluginHostCompatibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_version: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PluginRuntime {
    DeclarativeRules {
        rules: Vec<String>,
    },
    Wasm {
        #[serde(rename = "abiVersion")]
        abi_version: String,
        #[serde(
            rename = "memoryLimitBytes",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        memory_limit_bytes: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHook {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHostCompatibility {
    pub app: String,
    pub plugin_api: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookRequest {
    pub abi_version: String,
    pub plugin_id: String,
    pub hook: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub config: Value,
    #[serde(default)]
    pub context: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookResult {
    pub action: HookAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_chunk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audit: Vec<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookAction {
    Pass,
    Warn,
    Block,
    Replace,
}

impl HookResult {
    pub fn pass() -> Self {
        Self {
            action: HookAction::Pass,
            message: None,
            reason: None,
            request_body: None,
            response_body: None,
            stream_chunk: None,
            log_message: None,
            headers: None,
            audit: Vec::new(),
        }
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            action: HookAction::Warn,
            message: Some(message.into()),
            reason: None,
            request_body: None,
            response_body: None,
            stream_chunk: None,
            log_message: None,
            headers: None,
            audit: Vec::new(),
        }
    }

    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            action: HookAction::Block,
            message: None,
            reason: Some(reason.into()),
            request_body: None,
            response_body: None,
            stream_chunk: None,
            log_message: None,
            headers: None,
            audit: Vec::new(),
        }
    }

    fn replace() -> Self {
        Self {
            action: HookAction::Replace,
            message: None,
            reason: None,
            request_body: None,
            response_body: None,
            stream_chunk: None,
            log_message: None,
            headers: None,
            audit: Vec::new(),
        }
    }

    pub fn replace_request_body(body: impl Into<String>) -> Self {
        Self {
            request_body: Some(body.into()),
            ..Self::replace()
        }
    }

    pub fn replace_response_body(body: impl Into<String>) -> Self {
        Self {
            response_body: Some(body.into()),
            ..Self::replace()
        }
    }

    pub fn replace_stream_chunk(chunk: impl Into<String>) -> Self {
        Self {
            stream_chunk: Some(chunk.into()),
            ..Self::replace()
        }
    }

    pub fn replace_log_message(message: impl Into<String>) -> Self {
        Self {
            log_message: Some(message.into()),
            ..Self::replace()
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), value.into());
        self
    }
}

pub fn pack_ptr_len(ptr: u32, len: u32) -> u64 {
    ((ptr as u64) << 32) | len as u64
}

pub fn unpack_ptr_len(value: u64) -> (u32, u32) {
    ((value >> 32) as u32, value as u32)
}

pub fn serialize_hook_result(result: &HookResult) -> Vec<u8> {
    serde_json::to_vec(result)
        .unwrap_or_else(|_| br#"{"action":"block","reason":"serialize error"}"#.to_vec())
}

pub fn handle_hook_bytes<F>(input: &[u8], handler: F) -> Vec<u8>
where
    F: FnOnce(HookRequest) -> HookResult,
{
    let result = match serde_json::from_slice::<HookRequest>(input) {
        Ok(request) => handler(request),
        Err(error) => HookResult::block(format!("invalid hook request: {error}")),
    };
    serialize_hook_result(&result)
}

pub fn leak_output_bytes(bytes: Vec<u8>) -> u64 {
    let len = bytes.len() as u32;
    let ptr = Box::into_raw(bytes.into_boxed_slice()) as *mut u8 as u32;
    pack_ptr_len(ptr, len)
}

#[macro_export]
macro_rules! aio_plugin_entrypoint {
    ($handler:path) => {
        #[no_mangle]
        pub extern "C" fn aio_plugin_handle(ptr: i32, len: i32) -> i64 {
            let input =
                unsafe { core::slice::from_raw_parts(ptr as *const u8, len.max(0) as usize) };
            let output = $crate::handle_hook_bytes(input, $handler);
            $crate::leak_output_bytes(output) as i64
        }
    };
}
