use super::*;

const CATALOG_PATH: &str = "/gemini/v1beta/models?pageSize=1000";

async fn list(router: &axum::Router) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(CATALOG_PATH)
                .header("x-goog-api-key", "client-placeholder")
                .header("x-session-id", "picker-session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test(flavor = "current_thread")]
async fn gemini_model_catalog_requires_an_enabled_proxy_and_provider_and_projects_upstream() {
    let _guard = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().unwrap();
    let mut env = isolate_app_env(home.path());
    env.set_var("AIO_CODING_HUB_TEST_HOME", home.path());
    env.set_var("NO_PROXY", "*");
    let app = tauri::test::mock_app();
    let handle = app.handle();
    settings::write(handle, &settings::AppSettings::default()).unwrap();
    let db = db::init_for_tests(&home.path().join("catalog.sqlite")).unwrap();
    let make_router = || {
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        build_router(gateway_state(handle.clone(), db.clone(), tx))
    };
    // The guard caches proxy enablement for 500 ms across router instances.
    tokio::time::sleep(Duration::from_millis(550)).await;
    let (status, disabled) = list(&make_router()).await;
    assert!(!status.is_success());
    assert!(disabled
        .to_string()
        .contains(crate::gateway::proxy::GatewayErrorCode::CliProxyDisabled.as_str()));
    let enabled =
        crate::cli_proxy::set_enabled(handle, "gemini", true, "http://127.0.0.1:37123").unwrap();
    assert!(enabled.ok, "{}", enabled.message);
    tokio::time::sleep(Duration::from_millis(550)).await;
    let (status, empty) = list(&make_router()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(empty
        .to_string()
        .contains(crate::gateway::proxy::GatewayErrorCode::NoEnabledProvider.as_str()));
    const UPSTREAM: &str = r#"{"models":[{"name":"models/vendor-7","displayName":"Vendor Seven","inputTokenLimit":8192,"supportedGenerationMethods":["generateContent"]},{"name":"models/permitted"},{"name":"models/blocked","displayName":"Blocked"},{"name":"models/embed","supportedGenerationMethods":["embedContent"]}],"nextPageToken":"another"}"#;
    let (url, capture, task) = spawn_capturing_raw_upstream(UPSTREAM).await;
    let mut policy = selected_model_policy("alias-*", Some("vendor-*"));
    policy
        .model_patterns
        .extend(["permitted".into(), "embed".into()]);
    insert_provider_with_priority_and_policy(&db, "gemini", "Catalog", url, 0, Some(policy));
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let state = gateway_state(handle.clone(), db.clone(), tx);
    let session = Arc::clone(&state.session);
    let (status, page) = list(&build_router(state)).await;
    assert_eq!(
        session.get_bound_provider(
            "gemini",
            "picker-session",
            crate::gateway::util::now_unix_seconds() as i64
        ),
        None
    );
    assert_eq!(status, StatusCode::OK, "{page}");
    let req = capture.await.unwrap();
    assert!(req.head.starts_with("GET /v1beta/models?pageSize=1000 "));
    assert!(req.has_header_line("x-goog-api-key: sk-test"));
    assert!(!req.text().contains("client-placeholder"));
    assert_eq!(page["nextPageToken"], "another");
    assert_eq!(
        page["models"],
        serde_json::json!([
            {"name":"models/alias-7","displayName":"alias-7","inputTokenLimit":8192,"supportedGenerationMethods":["generateContent"]},
            {"name":"models/permitted","displayName":"permitted"}
        ])
    );
    task.abort();
}

#[tokio::test(flavor = "current_thread")]
async fn gemini_model_catalog_oauth_uses_only_configured_names_and_requires_credentials() {
    let _guard = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().unwrap();
    let mut env = isolate_app_env(home.path());
    env.set_var("AIO_CODING_HUB_TEST_HOME", home.path());
    let app = tauri::test::mock_app();
    settings::write(app.handle(), &settings::AppSettings::default()).unwrap();
    let enabled =
        crate::cli_proxy::set_enabled(app.handle(), "gemini", true, "http://127.0.0.1:37123")
            .unwrap();
    assert!(enabled.ok, "{}", enabled.message);
    let db = db::init_for_tests(&home.path().join("oauth-catalog.sqlite")).unwrap();
    let mut policy = selected_model_policy("enrolled-alias", Some("vendor-id"));
    policy.model_patterns.push("wildcard-*".into());
    let id = insert_provider_with_priority_and_policy(
        &db,
        "gemini",
        "OAuth",
        "http://127.0.0.1:1".into(),
        0,
        Some(policy),
    );
    db.open_connection().unwrap().execute("UPDATE providers SET auth_mode='oauth', oauth_provider_type='gemini_oauth', oauth_access_token='ya29.test-fixture', oauth_expires_at=4102444800 WHERE id=?1", [id]).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let state = gateway_state(app.handle().clone(), db.clone(), tx);
    let circuit = Arc::clone(&state.circuit);
    let now = crate::gateway::util::now_unix_seconds() as i64;
    circuit.record_failure(id, now, None);
    let router = build_router(state);
    let (status, catalog) = list(&router).await;
    assert_eq!(
        circuit.snapshot(id, now).failure_count,
        1,
        "a local catalog cannot clear upstream failures"
    );
    assert_eq!(status, StatusCode::OK, "{catalog}");
    assert_eq!(
        catalog["models"],
        serde_json::json!([{"name":"models/enrolled-alias","displayName":"enrolled-alias","supportedGenerationMethods":["generateContent"]}])
    );
    db.open_connection()
        .unwrap()
        .execute(
            "UPDATE providers SET model_policy_json=?1 WHERE id=?2",
            rusqlite::params![
                providers::ProviderModelPolicyV1::all().to_json().unwrap(),
                id
            ],
        )
        .unwrap();
    let (status, empty) = list(&router).await;
    assert_eq!(status, StatusCode::OK, "{empty}");
    assert_eq!(empty["models"], serde_json::json!([]));
    db.open_connection()
        .unwrap()
        .execute(
            "UPDATE providers SET oauth_access_token='' WHERE id=?1",
            [id],
        )
        .unwrap();
    assert!(!list(&router).await.0.is_success());
}

#[tokio::test(flavor = "current_thread")]
async fn gemini_model_catalog_404_keeps_generation_circuit_available() {
    let _guard = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().unwrap();
    let mut env = isolate_app_env(home.path());
    env.set_var("AIO_CODING_HUB_TEST_HOME", home.path());
    env.set_var("NO_PROXY", "*");
    let app = tauri::test::mock_app();
    let db = db::init_for_tests(&home.path().join("generation-only.sqlite")).unwrap();
    let (url, calls, upstream) = spawn_counting_status_upstream(
        StatusCode::NOT_FOUND,
        r#"{"error":"models.list unavailable"}"#,
    )
    .await;
    let id = insert_provider_with_priority(&db, "gemini", "Generation only", url, 0);
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let state = gateway_state(app.handle().clone(), db, tx);
    let circuit = Arc::clone(&state.circuit);
    let now = crate::gateway::util::now_unix_seconds() as i64;
    circuit.record_failure(id, now, None);
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/gemini/_aio/provider/{id}/v1beta/models?pageSize=1000"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!response.status().is_success());
    assert!(calls.load(std::sync::atomic::Ordering::SeqCst) > 0);
    let admission = circuit.should_allow(id, now);
    assert!(admission.allow);
    assert_eq!(admission.after.failure_count, 1);
    assert_eq!(admission.after.cooldown_until, None);
    upstream.abort();
}
