//! Standalone search must cross the real gateway without Responses mutations.

use super::*;

const SEARCH_RESPONSE: &str = r#"{"encrypted_output":"ciphertext","output":"search result","results":[{"ref_id":"result-1","future_field":true}]}"#;
const SEARCH_SESSION: &str = "11111111-1111-1111-1111-111111111111";

#[tokio::test(flavor = "current_thread")]
async fn alpha_search_preserves_wire_and_session_with_completion_enabled_or_disabled() {
    let _env_lock = crate::test_support::test_env_lock();
    for (route, forwarded, completion, compressed, polluted) in [
        ("/v1/alpha/search", "/v1/alpha/search", true, false, false),
        ("/codex/alpha/search/", "/alpha/search/", true, true, true),
        (
            "/codex/v1/codex/alpha/search",
            "/v1/codex/alpha/search",
            false,
            false,
            true,
        ),
        (
            "/codex/codex/alpha/search",
            "/codex/alpha/search",
            false,
            true,
            false,
        ),
    ] {
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        let mut config = settings::AppSettings::default();
        config.enable_codex_session_id_completion = completion;
        settings::write(app.handle(), &config).unwrap();
        crate::cli_proxy::set_enabled(app.handle(), "codex", true, "http://127.0.0.1:37123")
            .unwrap();
        let db = db::init_for_tests(&home.path().join("search.sqlite")).unwrap();
        let (upstream_url, captured_rx, upstream_task) =
            spawn_capturing_raw_upstream(SEARCH_RESPONSE).await;
        let provider_id = insert_codex_provider(&db, upstream_url);
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(8);
        let state = gateway_state(app.handle().clone(), db, log_tx);
        let session = state.session.clone();
        let expected = serde_json::json!({
            "id": SEARCH_SESSION,
            "model": "gpt-search",
            "input": "search this",
            "commands": {"search_query": [{"q": "example"}]},
            "settings": {"external_web_access": true},
            "max_output_tokens": 123,
            "future_field": {"keep": true},
        });
        let mut body = expected.clone();
        if polluted {
            body["prompt_cache_key"] = serde_json::json!("wrong-responses-session");
            body["prompt_cache_retention"] = serde_json::json!("24h");
            body["store"] = serde_json::json!(false);
        }
        let mut request = Request::builder()
            .method(Method::POST)
            .uri(format!("{route}?feature=standalone"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer client-placeholder")
            .header(
                "x-codex-turn-metadata",
                r#"{"session_id":"search-session","turn_id":"turn-1"}"#,
            );
        let raw = serde_json::to_vec(&body).unwrap();
        let wire = if compressed {
            request = request.header(header::CONTENT_ENCODING, "gzip");
            gzip_bytes(&raw)
        } else {
            raw
        };
        let response = build_router(state)
            .oneshot(request.body(Body::from(wire.clone())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{route}");
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .as_ref(),
            SEARCH_RESPONSE.as_bytes(),
        );
        let captured = captured_rx.await.unwrap();
        assert!(captured
            .head
            .starts_with(&format!("POST {forwarded}?feature=standalone HTTP/1.1")));
        assert!(captured.has_header_line("authorization: Bearer sk-test"));
        assert!(captured.has_header_line("x-codex-turn-metadata:"));
        assert!(!captured.has_header_line("session_id:"));
        assert!(!captured.has_header_line("x-session-id:"));
        assert_eq!(
            captured.has_header_line("content-encoding: gzip"),
            compressed
        );
        if !polluted {
            assert_eq!(captured.body, wire, "clean search must preserve wire bytes");
        }
        let bytes = if compressed {
            gunzip_bytes(&captured.body)
        } else {
            captured.body
        };
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            expected,
            "{route}"
        );
        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.session_id.as_deref(), Some(SEARCH_SESSION));
        assert_eq!(
            session.get_bound_provider("codex", SEARCH_SESSION, log.created_at),
            Some(provider_id)
        );
        assert!(!log
            .special_settings_json
            .as_deref()
            .unwrap_or("")
            .contains("codex_session_id_completion"));
        upstream_task.abort();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn alpha_search_cleans_after_custom_headers_and_before_send_plugin() {
    let _env_lock = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().unwrap();
    let _env = isolate_app_env(home.path());
    let app = tauri::test::mock_app();
    crate::cli_proxy::set_enabled(app.handle(), "codex", true, "http://127.0.0.1:37123").unwrap();
    let db = db::init_for_tests(&home.path().join("search-plugin.sqlite")).unwrap();
    let (url, captured_rx, upstream_task) = spawn_capturing_raw_upstream(SEARCH_RESPONSE).await;
    let provider_id = insert_codex_provider(&db, url);
    db.open_connection().unwrap().execute(
        "UPDATE providers SET custom_headers_json = ?1 WHERE id = ?2",
        rusqlite::params![r#"[{"name":"openai-beta","value":"responses=experimental"},{"name":"conversation_id","value":"private-conversation"},{"name":"x-codex-beta-features","value":"feature"},{"name":"x-openai-internal-codex-responses-lite","value":"true"},{"name":"x-tenant","value":"keep-tenant"}]"#, provider_id],
    ).unwrap();
    let mut plugin = before_send_header_plugin();
    plugin
        .granted_permissions
        .extend(["request.body.read".into(), "request.body.write".into()]);
    persist_plugin_detail(&db, &plugin);
    let executor =
        InMemoryGatewayPluginExecutor::new().with_request_handler("test.before-send", |ctx| {
            let mut result = GatewayHookResult::continue_unchanged();
            let mut body: Value = serde_json::from_str(&ctx.request.body.unwrap()).unwrap();
            body["prompt_cache_key"] = serde_json::json!("private-cache");
            body["prompt_cache_retention"] = serde_json::json!("24h");
            body["store"] = serde_json::json!(false);
            body["input"] = serde_json::json!("plugin-redacted-query");
            result.request_body = Some(body.to_string());
            result
                .headers
                .insert("x-codex-turn-state".into(), "private-turn".into());
            result
                .headers
                .insert("session_id".into(), "private-session".into());
            result
                .headers
                .insert("x-session-id".into(), "private-session".into());
            result
                .headers
                .insert("x-plugin-safe".into(), "keep-plugin".into());
            result
        });
    let pipeline = GatewayPluginPipeline::for_tests_shared(
        vec![plugin],
        Arc::new(executor),
        GatewayPluginPipelineConfig::default(),
    );
    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
    let router = build_router(gateway_state_with_plugin_pipeline(
        app.handle().clone(),
        db,
        log_tx,
        pipeline,
    ));
    let response = router.oneshot(Request::builder()
        .method(Method::POST).uri("/codex/v1/alpha/search")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_ENCODING, "gzip")
        .header("x-codex-turn-metadata", "keep-metadata")
        .body(Body::from(gzip_bytes(serde_json::json!({"id": SEARCH_SESSION,"model":"gpt-search","input":"original-query","future_field":true}).to_string().as_bytes()))).unwrap()
    ).await.unwrap();
    let status = response.status();
    let response_body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let log = recv_terminal_request_log(&mut log_rx).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "response={} attempts={}",
        String::from_utf8_lossy(&response_body),
        log.attempts_json
    );
    let captured = captured_rx.await.unwrap();
    assert!(captured.has_header_line("content-encoding: gzip"));
    assert!(captured.has_header_line("authorization: Bearer sk-test"));
    for header in [
        "session_id",
        "x-session-id",
        "openai-beta",
        "conversation_id",
        "x-codex-beta-features",
        "x-codex-turn-state",
        "x-openai-internal-codex-responses-lite",
    ] {
        assert!(!captured.has_header_line(&format!("{header}:")), "{header}");
    }
    for header in [
        "x-tenant: keep-tenant",
        "x-plugin-safe: keep-plugin",
        "x-codex-turn-metadata: keep-metadata",
    ] {
        assert!(captured.has_header_line(header), "{header}");
    }
    assert_eq!(
        serde_json::from_slice::<Value>(&gunzip_bytes(&captured.body)).unwrap(),
        serde_json::json!({"id": SEARCH_SESSION,"model":"gpt-search","input":"plugin-redacted-query","future_field":true})
    );
    let markers: Value =
        serde_json::from_str(log.special_settings_json.as_deref().unwrap()).unwrap();
    let marker = markers
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["type"] == "codex_alpha_search_compat")
        .unwrap();
    assert_eq!(marker["providerId"], provider_id);
    assert_eq!(marker["removedFields"].as_array().unwrap().len(), 3);
    assert_eq!(marker["removedHeaders"].as_array().unwrap().len(), 7);
    assert!(!marker.to_string().contains("private-"));
    upstream_task.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn alpha_search_reuses_bound_provider_for_commands_and_separates_search_ids() {
    let _env_lock = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().unwrap();
    let _env = isolate_app_env(home.path());
    let app = tauri::test::mock_app();
    crate::cli_proxy::set_enabled(app.handle(), "codex", true, "http://127.0.0.1:37123").unwrap();
    let db = db::init_for_tests(&home.path().join("search-routing.sqlite")).unwrap();
    let (first_url, first_rx, first_task) = spawn_capturing_raw_upstream(SEARCH_RESPONSE).await;
    let (second_url, mut second_rx, second_task) =
        spawn_capturing_sequence_upstream(vec![("200 OK", SEARCH_RESPONSE); 2]).await;
    let first_id = insert_codex_provider_with_priority(&db, "First", first_url, 0);
    let second_id = insert_codex_provider_with_priority(&db, "Second", second_url, 1);
    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(8);
    let state = gateway_state(app.handle().clone(), db, log_tx);
    let session = state.session.clone();
    session.bind_success(
        "codex",
        SEARCH_SESSION,
        second_id,
        None,
        crate::shared::time::now_unix_seconds(),
    );
    let router = build_router(state);
    for (id, input, expected_provider) in [
        (SEARCH_SESSION, None, second_id),
        (SEARCH_SESSION, Some("changed query"), second_id),
        ("different-session", Some("changed query"), first_id),
    ] {
        let mut body = serde_json::json!({"id":id, "model":"gpt-search", "commands":{"open":[{"ref_id":"opaque-result"}]}});
        if let Some(input) = input {
            body["input"] = serde_json::json!(input);
        }
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/codex/v1/alpha/search")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let log = recv_terminal_request_log(&mut log_rx).await;
        assert_eq!(log.session_id.as_deref(), Some(id));
        assert_eq!(
            session.get_bound_provider("codex", id, log.created_at),
            Some(expected_provider)
        );
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(attempts.as_array().unwrap().len(), 1);
        assert_eq!(attempts[0]["provider_id"], expected_provider);
    }
    assert!(first_rx.await.unwrap().text().contains("different-session"));
    for _ in 0..2 {
        assert!(second_rx
            .recv()
            .await
            .unwrap()
            .text()
            .contains(SEARCH_SESSION));
    }
    first_task.abort();
    second_task.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn alpha_search_fix_keeps_responses_completion_and_cache_fields() {
    let _env_lock = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().unwrap();
    let _env = isolate_app_env(home.path());
    let app = tauri::test::mock_app();
    crate::cli_proxy::set_enabled(app.handle(), "codex", true, "http://127.0.0.1:37123").unwrap();
    let db = db::init_for_tests(&home.path().join("responses-sibling.sqlite")).unwrap();
    let (url, captured_rx, upstream_task) =
        spawn_capturing_raw_upstream(r#"{"id":"resp-ok","object":"response","output":[]}"#).await;
    insert_codex_provider(&db, url);
    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
    let response = build_router(gateway_state(app.handle().clone(), db, log_tx)).oneshot(Request::builder()
        .method(Method::POST).uri("/codex/v1/responses")
        .header(header::CONTENT_TYPE, "application/json")
        .header("session_id", SEARCH_SESSION)
        .header("openai-beta", "responses=experimental")
        .body(Body::from(r#"{"model":"gpt-test","input":[],"store":false,"prompt_cache_retention":"24h"}"#)).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let captured = captured_rx.await.unwrap();
    assert!(captured.has_header_line(&format!("session_id: {SEARCH_SESSION}")));
    assert!(captured.has_header_line("openai-beta: responses=experimental"));
    let body: Value = serde_json::from_slice(&captured.body).unwrap();
    assert_eq!(body["prompt_cache_key"], SEARCH_SESSION);
    assert_eq!(body["prompt_cache_retention"], "24h");
    assert_eq!(body["store"], false);
    let log = recv_terminal_request_log(&mut log_rx).await;
    let markers = log.special_settings_json.as_deref().unwrap();
    assert!(markers.contains("codex_session_id_completion"));
    assert!(!markers.contains("codex_alpha_search_compat"));
    upstream_task.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn alpha_search_preserves_upstream_client_error_without_repair_retry() {
    let _env_lock = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().unwrap();
    let _env = isolate_app_env(home.path());
    let app = tauri::test::mock_app();
    crate::cli_proxy::set_enabled(app.handle(), "codex", true, "http://127.0.0.1:37123").unwrap();
    let db = db::init_for_tests(&home.path().join("search-error.sqlite")).unwrap();
    let (url, captured_rx, upstream_task) = spawn_capturing_status_raw_upstream(
        "400 Bad Request",
        r#"{"error":{"message":"search input is invalid","type":"invalid_request_error"}}"#,
    )
    .await;
    insert_codex_provider(&db, url);
    let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
    let response = build_router(gateway_state(app.handle().clone(), db, log_tx))
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/codex/v1/alpha/search")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"id":"search-error","model":"gpt-search","commands":{}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let _ = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let captured = captured_rx.await.unwrap();
    assert!(!captured.has_header_line("session_id:"));
    let log = recv_terminal_request_log(&mut log_rx).await;
    assert_eq!(log.status, Some(400));
    let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
    assert_eq!(attempts.as_array().unwrap().len(), 1);
    assert_eq!(attempts[0]["status"], 400);
    assert!(attempts[0]["reason"]
        .as_str()
        .unwrap()
        .contains("search input is invalid"));
    upstream_task.abort();
}
