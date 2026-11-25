//! Run `cargo test --lib agy_loopback -- --ignored` with AIO_TEST_AGY set to an installed agy.

use super::*;
use axum::response::IntoResponse;

const CATALOG: &str = r#"{"models":[{"name":"models/gemini-test","displayName":"AIO Test Model","supportedGenerationMethods":["generateContent"]}]}"#;
const ANSWER: &str = "AIO_STREAM_VERIFIED";

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a local Antigravity CLI executable and macOS sandbox-exec"]
async fn agy_loopback_discovers_and_streams_through_the_managed_gateway() {
    let binary =
        std::fs::canonicalize(std::env::var_os("AIO_TEST_AGY").expect("set AIO_TEST_AGY")).unwrap();
    let _lock = crate::test_support::test_env_lock();
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let mut env = isolate_app_env(&root);
    env.set_var("AIO_CODING_HUB_TEST_HOME", &root);
    env.set_var("NO_PROXY", "*");
    let app = tauri::test::mock_app();
    settings::write(app.handle(), &settings::AppSettings::default()).unwrap();
    let db = db::init_for_tests(&root.join("test.sqlite")).unwrap();

    let mock = axum::Router::new().fallback(|req: Request<Body>| async move {
        assert_eq!(req.headers().get("x-goog-api-key").unwrap(), "sk-test");
        match (req.method(), req.uri().path(), req.uri().query()) {
            (&Method::GET, "/v1beta/models", Some("pageSize=1000")) => {
                ([(header::CONTENT_TYPE, "application/json")], CATALOG).into_response()
            }
            (&Method::POST, "/v1beta/models/gemini-test:streamGenerateContent", Some("alt=sse")) => {
                let body = format!("data: {}\r\n\r\n", serde_json::json!({
                    "candidates":[{"index":0,"content":{"role":"model","parts":[{"text":ANSWER}]},"finishReason":"STOP"}],
                    "usageMetadata":{"promptTokenCount":7,"candidatesTokenCount":2,"totalTokenCount":9}
                }));
                ([(header::CONTENT_TYPE, "text/event-stream")], body).into_response()
            }
            _ => StatusCode::NOT_FOUND.into_response(),
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let upstream = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
    let id = insert_provider_with_priority(&db, "gemini", "loopback fixture", url, 0);
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let enable =
        crate::cli_proxy::set_enabled(app.handle(), "gemini", true, &format!("http://{addr}"))
            .unwrap();
    assert!(enable.ok, "{}", enable.message);
    assert!(root.join(".gemini/config/config.json").is_file());
    let router = build_router(gateway_state(app.handle().clone(), db, tx));
    let gateway = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

    let executable = root.join("agy");
    std::os::unix::fs::symlink(binary, &executable).unwrap();
    let workspace = root.join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    // agy starts a local listener. Permit that and the gateway port, but no other destination.
    let policy = format!(
        r#"(version 1)(allow default)(deny network*)(allow network-bind (local ip "localhost:*"))(allow network-inbound (local ip "localhost:*"))(allow network-outbound (remote ip "localhost:{}"))(deny file-write*)(allow file-write* (subpath {}))(allow file-write-data (literal "/dev/null"))"#,
        addr.port(),
        serde_json::to_string(&root).unwrap()
    );
    let run = |args: Vec<String>| {
        let mut cmd = tokio::process::Command::new("/usr/bin/sandbox-exec");
        cmd.args(["-p", &policy])
            .arg(&executable)
            .args(args)
            .env_clear()
            .env("HOME", &root)
            .env("TMPDIR", &root)
            .env("PATH", "/usr/bin:/bin")
            .env("TERM", "dumb")
            .current_dir(&workspace)
            .kill_on_drop(true);
        async move {
            let result = tokio::time::timeout(Duration::from_secs(45), cmd.output())
                .await
                .expect("agy timed out")
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            String::from_utf8(result.stdout).unwrap()
        }
    };
    let models = run(vec!["models".into()]).await;
    let model_id = models
        .lines()
        .find_map(|line| {
            let (id, name) = line.split_once('\t')?;
            (name.trim() == "AIO Test Model").then_some(id.trim())
        })
        .expect("model from the gateway catalog");
    assert_eq!(model_id, "gemini-test");
    let text = run([
        "--model",
        model_id,
        "--print-timeout",
        "20s",
        "-p",
        "Return the test reply.",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect())
    .await;
    assert_eq!(text.trim(), ANSWER);
    loop {
        let log = recv_terminal_request_log(&mut rx).await;
        if log.method == "POST" {
            assert_eq!(log.cli_key, "gemini");
            assert_eq!(log.status, Some(200));
            assert_single_success_attempt(&log, id);
            assert_eq!(log.total_tokens, Some(9));
            break;
        }
    }
    gateway.abort();
    upstream.abort();
}
