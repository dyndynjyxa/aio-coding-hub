use aio_plugin_wasm_sdk::{
    handle_hook_bytes, pack_ptr_len, unpack_ptr_len, HookAction, HookRequest, HookResult,
    PluginHook, PluginHostCompatibility, PluginManifest, PluginRuntime,
};
use serde_json::json;

#[test]
fn sdk_contract_serializes_manifest_and_hook_result_with_host_field_names() {
    let manifest = PluginManifest {
        id: "acme.redactor".to_string(),
        name: "Acme Redactor".to_string(),
        version: "0.1.0".to_string(),
        api_version: "1.0.0".to_string(),
        runtime: PluginRuntime::Wasm {
            abi_version: "1.0.0".to_string(),
            memory_limit_bytes: Some(16 * 1024 * 1024),
        },
        hooks: vec![PluginHook {
            name: "gateway.request.afterBodyRead".to_string(),
            priority: Some(50),
            failure_policy: Some("fail-open".to_string()),
        }],
        permissions: vec!["request.body.read".to_string()],
        host_compatibility: PluginHostCompatibility {
            app: ">=0.56.0 <1.0.0".to_string(),
            plugin_api: "^1.0.0".to_string(),
            platforms: Some(vec![
                "macos".to_string(),
                "windows".to_string(),
                "linux".to_string(),
            ]),
        },
        entry: Some("plugin.wasm".to_string()),
        config_schema: None,
        config_version: Some(1),
    };

    let json = serde_json::to_value(&manifest).expect("manifest serializes");
    assert_eq!(json["apiVersion"], "1.0.0");
    assert_eq!(json["runtime"]["kind"], "wasm");
    assert_eq!(json["runtime"]["abiVersion"], "1.0.0");
    assert_eq!(json["hostCompatibility"]["pluginApi"], "^1.0.0");

    let result = HookResult::replace_request_body("{\"input\":\"[REDACTED]\"}");
    let result_json = serde_json::to_value(&result).expect("result serializes");
    assert_eq!(result_json["action"], "replace");
    assert_eq!(result_json["requestBody"], "{\"input\":\"[REDACTED]\"}");
    assert!(result_json.get("contextPatch").is_none());
}

#[test]
fn hook_result_serializes_host_mutation_fields() {
    let result = HookResult::replace_request_body("{\"messages\":[]}");
    let json = serde_json::to_value(result).expect("serialize hook result");

    assert_eq!(json["action"], "replace");
    assert_eq!(json["requestBody"], "{\"messages\":[]}");
    assert!(json.get("contextPatch").is_none());
}

#[test]
fn sdk_contract_handles_hook_json_without_host_capabilities() {
    let request = HookRequest {
        abi_version: "1.0.0".to_string(),
        plugin_id: "acme.redactor".to_string(),
        hook: "gateway.request.afterBodyRead".to_string(),
        trace_id: Some("trace-1".to_string()),
        config: json!({}),
        context: json!({ "request": { "body": { "input": "secret" } } }),
    };
    let input = serde_json::to_vec(&request).expect("request bytes");

    let output = handle_hook_bytes(&input, |request| {
        assert_eq!(request.plugin_id, "acme.redactor");
        HookResult::warn("redaction candidate")
    });
    let result: HookResult = serde_json::from_slice(&output).expect("hook output");
    assert_eq!(result.action, HookAction::Warn);
    assert_eq!(result.message.as_deref(), Some("redaction candidate"));
}

#[test]
fn sdk_contract_packs_wasm_pointer_and_length() {
    let packed = pack_ptr_len(123, 456);
    assert_eq!(unpack_ptr_len(packed), (123, 456));
}
