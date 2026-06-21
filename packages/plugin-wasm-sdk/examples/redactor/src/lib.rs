use aio_plugin_wasm_sdk::{aio_plugin_entrypoint, HookRequest, HookResult};

fn handle(request: HookRequest) -> HookResult {
    let Some(input) = request
        .context
        .pointer("/request/body/input")
        .and_then(serde_json::Value::as_str)
    else {
        return HookResult::pass();
    };

    if !input.contains("SECRET_") {
        return HookResult::pass();
    }

    HookResult::replace_request_body(format!(
        "{{\"input\":{}}}",
        serde_json::to_string(&input.replace("SECRET_", "[REDACTED]_"))
            .unwrap_or_else(|_| "\"[REDACTED]\"".to_string())
    ))
}

aio_plugin_entrypoint!(handle);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redactor_example_replaces_secret_marker() {
        let result = handle(HookRequest {
            abi_version: "1.0.0".to_string(),
            plugin_id: "acme.redactor".to_string(),
            hook: "gateway.request.afterBodyRead".to_string(),
            trace_id: None,
            config: json!({}),
            context: json!({ "request": { "body": { "input": "hello SECRET_TOKEN" } } }),
        });

        let body: serde_json::Value =
            serde_json::from_str(result.request_body.as_deref().expect("request body"))
                .expect("request body json");
        assert_eq!(body["input"], "hello [REDACTED]_TOKEN");
    }
}
