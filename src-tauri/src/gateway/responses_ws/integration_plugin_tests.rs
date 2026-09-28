//! Verify the existing plugin pipeline across transport fallback and provider failover.

use super::*;
use crate::domain::plugin_contributions::PluginContributes;
use crate::domain::plugins::{
    PluginDetail, PluginHook, PluginHostCompatibility, PluginInstallSource, PluginManifest,
    PluginPermissionRisk, PluginRuntime, PluginStatus, PluginSummary,
};
use crate::gateway::plugins::context::{GatewayHookResult, GatewayPluginHookName};
use crate::gateway::plugins::pipeline::{
    GatewayPluginPipelineConfig, InMemoryGatewayPluginExecutor,
};
use std::collections::BTreeMap;

fn counting_plugin() -> PluginDetail {
    let permissions = [
        "request.body.read",
        "request.body.write",
        "stream.inspect",
        "stream.modify",
    ];
    PluginDetail {
        summary: PluginSummary {
            id: 1,
            plugin_id: "ws-test-hooks".into(),
            name: "WS test hooks".into(),
            current_version: Some("1.0.0".into()),
            status: PluginStatus::Enabled,
            runtime: "extensionHost".into(),
            permission_risk: PluginPermissionRisk::High,
            update_available: false,
            last_error: None,
            created_at: 1,
            updated_at: 1,
        },
        manifest: PluginManifest {
            id: "ws-test-hooks".into(),
            name: "WS test hooks".into(),
            version: "1.0.0".into(),
            api_version: "1.0.0".into(),
            runtime: PluginRuntime::ExtensionHost {
                language: "typescript".into(),
            },
            hooks: vec![],
            permissions: vec![],
            main: Some("dist/index.js".into()),
            activation_events: vec![],
            contributes: Some(PluginContributes {
                providers: vec![],
                protocols: vec![],
                protocol_bridges: vec![],
                commands: vec![],
                gateway_hooks: [
                    GatewayPluginHookName::RequestAfterBodyRead,
                    GatewayPluginHookName::RequestBeforeSend,
                    GatewayPluginHookName::ResponseChunk,
                ]
                .into_iter()
                .map(|hook| PluginHook {
                    name: hook.as_str().into(),
                    priority: 0,
                    failure_policy: Some("fail-closed".into()),
                    timeout_ms: None,
                })
                .collect(),
                ui: BTreeMap::new(),
            }),
            capabilities: vec!["gateway.hooks".into()],
            host_compatibility: PluginHostCompatibility {
                app: ">=0.56.0 <1.0.0".into(),
                plugin_api: "^1.0.0".into(),
                platforms: vec![],
            },
            entry: None,
            config_schema: None,
            config_version: None,
            description: None,
            author: None,
            homepage: None,
            repository: None,
            license: None,
            checksum: None,
            signature: None,
            category: None,
        },
        install_source: PluginInstallSource::Local,
        installed_dir: None,
        config: json!({}),
        granted_permissions: permissions.into_iter().map(str::to_owned).collect(),
        pending_permissions: vec![],
        audit_logs: vec![],
        runtime_failures: vec![],
        rollback_versions: vec![],
    }
}

fn counting_pipeline(calls: Arc<Mutex<Vec<GatewayPluginHookName>>>) -> Arc<GatewayPluginPipeline> {
    let request_calls = calls.clone();
    let stream_calls = calls.clone();
    let executor = InMemoryGatewayPluginExecutor::new()
        .with_request_handler("ws-test-hooks", move |context| {
            let hook = GatewayPluginHookName::from_str(&context.hook_name).unwrap();
            request_calls.lock().unwrap().push(hook);
            let mut body: Value =
                serde_json::from_str(context.request.body.as_deref().unwrap()).unwrap();
            let old = body
                .get("plugin_phases")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let suffix = if hook == GatewayPluginHookName::RequestAfterBodyRead {
                "body;"
            } else {
                "send;"
            };
            body["plugin_phases"] = json!(format!("{old}{suffix}"));
            GatewayHookResult {
                request_body: Some(body.to_string()),
                ..GatewayHookResult::continue_unchanged()
            }
        })
        .with_stream_handler("ws-test-hooks", move |context| {
            stream_calls
                .lock()
                .unwrap()
                .push(GatewayPluginHookName::ResponseChunk);
            GatewayHookResult {
                // A duplicated chunk wrapper would visibly add this prefix twice.
                stream_chunk: Some(
                    context
                        .stream
                        .chunk
                        .unwrap()
                        .replace("answer-", "plugin-answer-"),
                ),
                ..GatewayHookResult::continue_unchanged()
            }
        });
    Arc::new(GatewayPluginPipeline::for_tests(
        vec![counting_plugin()],
        Arc::new(executor),
        GatewayPluginPipelineConfig::default(),
    ))
}

#[tokio::test(flavor = "current_thread")]
async fn plugin_hooks_run_once_per_request_attempt_and_output_across_fallback_and_failover() {
    for failover in [false, true] {
        let fixture = Fixture::new(true).await;
        let (first, first_server) = Stub::start(
            "A",
            if failover {
                Behavior::AllFail
            } else {
                Behavior::UnsupportedWs
            },
        )
        .await;
        let (next, next_server) = Stub::start("B", Behavior::Complete).await;
        fixture.provider("A", &first_server.origin(), true);
        fixture.provider("B", &next_server.origin(), false);
        let calls = Arc::new(Mutex::new(Vec::new()));
        let pipeline = counting_pipeline(calls.clone());
        let (gateway, mut logs) = fixture.start_with_pipeline(pipeline).await;
        let observed = generate(&gateway, "plugin-counts").await;
        let answer = if failover { "B" } else { "A" };
        let terminal = observed.last().unwrap();
        assert_eq!(terminal["type"], "response.completed", "{observed:?}");
        assert_eq!(
            terminal["response"]["output"][0]["content"][0]["text"],
            format!("plugin-answer-{answer}")
        );
        assert_eq!(first.transports(), ["ws", "http"]);
        assert_eq!(next.transports().len(), usize::from(failover));
        for upstream in [&first, &next] {
            for (transport, _, body) in upstream.calls.lock().unwrap().iter() {
                if transport == "http" {
                    assert_eq!(
                        body["plugin_phases"], "body;send;",
                        "attempt mutations must not accumulate"
                    );
                }
            }
        }
        let calls = calls.lock().unwrap();
        assert_eq!(calls[0], GatewayPluginHookName::RequestAfterBodyRead);
        assert_eq!(
            calls
                .iter()
                .filter(|&&hook| hook == GatewayPluginHookName::RequestAfterBodyRead)
                .count(),
            1
        );
        let attempts = if failover { 3 } else { 2 };
        assert_eq!(
            calls
                .iter()
                .filter(|&&hook| hook == GatewayPluginHookName::RequestBeforeSend)
                .count(),
            attempts
        );
        assert!(calls
            .iter()
            .skip(attempts + 1)
            .all(|&hook| hook == GatewayPluginHookName::ResponseChunk));
        assert!(calls.len() > attempts + 1);
        drop(calls);
        let log = terminal_log(&mut logs).await;
        assert!(log.error_code.is_none());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn ordinary_http_retry_applies_before_send_to_the_provider_baseline() {
    let mut fixture = Fixture::new(false).await;
    fixture.circuit = Arc::new(circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        },
        Default::default(),
        None,
    ));
    let mut settings = settings::read(fixture.app.handle()).unwrap();
    settings.failover_max_attempts_per_provider = 2;
    settings.circuit_breaker_failure_threshold = 3;
    settings::write(fixture.app.handle(), &settings).unwrap();
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let server = Server::start(Router::new().route(
        "/v1/responses",
        axum::routing::post({
            let requests = requests.clone();
            move |Json(body): Json<Value>| {
                let requests = requests.clone();
                async move {
                    let mut requests = requests.lock().unwrap();
                    requests.push(body);
                    if requests.len() == 1 {
                        return (
                            StatusCode::SERVICE_UNAVAILABLE,
                            Json(json!({"error":{"message":"retry"}})),
                        )
                            .into_response();
                    }
                    let bytes: Vec<u8> = events("HTTP")
                        .iter()
                        .flat_map(|event| protocol::sse_bytes(event).to_vec())
                        .collect();
                    ([(header::CONTENT_TYPE, "text/event-stream")], bytes).into_response()
                }
            }
        }),
    ))
    .await;
    fixture.provider("HTTP", &server.origin(), false);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (gateway, mut logs) = fixture
        .start_with_pipeline(counting_pipeline(calls.clone()))
        .await;
    let response = reqwest::Client::new()
        .post(format!("{}/v1/responses", gateway.origin()))
        .json(
            &json!({"model":"gpt-test","input":[{"role":"user","content":"hello"}],"stream":true}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("plugin-answer-HTTP"));
    assert!(!body.contains("plugin-plugin-answer-HTTP"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        assert_eq!(request["plugin_phases"], "body;send;");
    }
    drop(requests);
    let calls = calls.lock().unwrap();
    assert_eq!(
        calls
            .iter()
            .filter(|&&hook| hook == GatewayPluginHookName::RequestAfterBodyRead)
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|&&hook| hook == GatewayPluginHookName::RequestBeforeSend)
            .count(),
        2
    );
    drop(calls);
    assert!(terminal_log(&mut logs).await.error_code.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn custom_headers_plugin_identity_changes_cannot_reuse_handshake() {
    let fixture = Fixture::new(true).await;
    let (stub, upstream) = Stub::start("A", Behavior::KeepAlive).await;
    fixture.provider_with_headers(
        "A",
        &upstream.origin(),
        true,
        None,
        Some("configured-secret"),
    );
    let mut plugin = counting_plugin();
    plugin.granted_permissions = vec!["request.header.read".into(), "request.header.write".into()];
    plugin
        .manifest
        .contributes
        .as_mut()
        .unwrap()
        .gateway_hooks
        .retain(|hook| hook.name == "gateway.request.beforeSend");
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    let executor = InMemoryGatewayPluginExecutor::new().with_request_handler(
        "ws-test-hooks",
        move |context| {
            assert!(
                !context.request.headers.unwrap().contains_key("x-tenant"),
                "ordinary plugin must not see configured secrets"
            );
            let generation = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            GatewayHookResult {
                headers: BTreeMap::from([(
                    "x-tenant".into(),
                    format!("plugin-tenant-{generation}"),
                )]),
                ..GatewayHookResult::continue_unchanged()
            }
        },
    );
    let pipeline = Arc::new(GatewayPluginPipeline::for_tests(
        vec![plugin],
        Arc::new(executor),
        GatewayPluginPipelineConfig::default(),
    ));
    let (gateway, _logs) = fixture.start_with_pipeline(pipeline).await;
    let mut socket = connect_with_user_agent(&gateway, "plugin-identity", None)
        .await
        .unwrap();
    for _ in 0..2 {
        socket.send(create_message(None)).await.unwrap();
        recv_until(&mut socket, "response.completed").await;
    }
    let upstream_calls = stub.calls.lock().unwrap();
    assert_eq!(upstream_calls.len(), 2);
    assert_eq!(upstream_calls[0].1["x-tenant"], "plugin-tenant-0");
    assert_eq!(upstream_calls[1].1["x-tenant"], "plugin-tenant-1");
}
