use crate::app_state::{ensure_db_ready, DbInitState};
use crate::blocking;
use crate::commands::providers::oauth_limits::ProviderOAuthLimitsResult;
use crate::gateway::oauth::limits::{
    self, apply_codex_quota_headers, fetch_codex_usage_limits, CODEX_USAGE_URL,
};
use crate::shared::http_body::read_text_with_limit;
use crate::shared::ipc_confirm::RiskyIpcConfirm;
use rand::RngCore;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

pub(crate) const PROVIDER_OAUTH_RESET_CODEX_QUOTA_ACTION: &str = "provider_oauth_reset_codex_quota";
const CODEX_RESET_URL: &str =
    "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits/consume";
const CODEX_RESET_RESPONSE_BODY_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub(super) struct CodexQuotaEndpoints {
    usage_url: String,
    reset_url: String,
}

impl Default for CodexQuotaEndpoints {
    fn default() -> Self {
        Self {
            usage_url: CODEX_USAGE_URL.to_string(),
            reset_url: CODEX_RESET_URL.to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub(crate) struct ProviderOAuthResetCodexQuotaResult {
    pub success: bool,
    pub code: Option<String>,
    pub windows_reset: Option<i64>,
    pub refreshed_limits: Option<ProviderOAuthLimitsResult>,
    pub refresh_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CodexResetConsumeResponse {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    windows_reset: Option<i64>,
}

pub(crate) fn codex_reset_confirm_resource(provider_id: i64) -> String {
    format!("provider:{provider_id}:codex_reset_credit")
}

fn require_codex_reset_confirm(
    provider_id: i64,
    confirm: Option<RiskyIpcConfirm>,
) -> Result<(), String> {
    RiskyIpcConfirm::require(
        confirm,
        PROVIDER_OAUTH_RESET_CODEX_QUOTA_ACTION,
        codex_reset_confirm_resource(provider_id),
    )
}

fn validate_codex_reset_details(
    details: &crate::providers::ProviderOAuthDetails,
) -> Result<(), String> {
    if details.cli_key != "codex" || details.oauth_provider_type.trim() != "codex_oauth" {
        return Err(format!(
            "SEC_INVALID_INPUT: reset credit is only supported for Codex OAuth providers (provider_id={})",
            details.id
        ));
    }
    Ok(())
}

fn codex_reset_account_id(
    details: &crate::providers::ProviderOAuthDetails,
) -> Result<String, String> {
    limits::codex_account_id(details)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "SEC_INVALID_INPUT: Codex OAuth missing chatgpt_account_id; please re-login this provider".to_string()
        })
}

fn codex_reset_in_flight() -> &'static Mutex<HashSet<i64>> {
    static IN_FLIGHT: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();
    IN_FLIGHT.get_or_init(|| Mutex::new(HashSet::new()))
}

#[derive(Debug)]
struct CodexResetInFlightGuard {
    provider_id: i64,
}

impl Drop for CodexResetInFlightGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = codex_reset_in_flight().lock() {
            guard.remove(&self.provider_id);
        }
    }
}

fn try_enter_codex_reset(provider_id: i64) -> Result<CodexResetInFlightGuard, String> {
    if provider_id <= 0 {
        return Err(format!(
            "SEC_INVALID_INPUT: invalid provider_id={provider_id}"
        ));
    }
    let mut guard = codex_reset_in_flight()
        .lock()
        .map_err(|_| "SEC_INVALID_STATE: codex reset guard poisoned".to_string())?;
    if !guard.insert(provider_id) {
        return Err(format!(
            "OAUTH_RESET_IN_PROGRESS: codex reset already in progress for provider_id={provider_id}"
        ));
    }
    Ok(CodexResetInFlightGuard { provider_id })
}

fn generate_redeem_request_id() -> String {
    let mut bytes = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

async fn consume_codex_reset_credit_and_refresh(
    db: &crate::db::Db,
    client: &reqwest::Client,
    endpoints: &CodexQuotaEndpoints,
    details: &mut crate::providers::ProviderOAuthDetails,
) -> Result<(ProviderOAuthResetCodexQuotaResult, Option<i64>), String> {
    let provider_id = details.id;
    let chatgpt_account_id = codex_reset_account_id(details)?;
    let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter_for_details(details)?;
    if limits::oauth_details_can_refresh(details)
        && crate::gateway::oauth::refresh::should_refresh_now(
            details.oauth_expires_at,
            details.oauth_refresh_lead_s,
        )
    {
        *details = limits::refresh_oauth_details_for_limits(db, client, details, adapter).await?;
    }
    if codex_reset_account_id(details)? != chatgpt_account_id {
        return Err(
            "OAUTH_QUOTA_STALE_TOKEN: OAuth account changed before resetting quota; please retry"
                .to_string(),
        );
    }
    let access_token = limits::effective_oauth_access_token(details, adapter)?;
    let redeem_request_id = generate_redeem_request_id();
    let response = apply_codex_quota_headers(
        client.post(&endpoints.reset_url),
        &access_token,
        Some(&chatgpt_account_id),
    )
    .json(&serde_json::json!({ "redeem_request_id": redeem_request_id }))
    .send()
    .await
    .map_err(|e| format!("codex reset consume failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = read_text_with_limit(response, CODEX_RESET_RESPONSE_BODY_LIMIT, "codex reset")
            .await
            .unwrap_or_default();
        return Err(format!("codex reset consume status: {status} - {text}"));
    }

    let body = read_text_with_limit(response, CODEX_RESET_RESPONSE_BODY_LIMIT, "codex reset")
        .await
        .map_err(|e| format!("codex reset body read failed: {e}"))?;
    let consumed = serde_json::from_str::<CodexResetConsumeResponse>(&body)
        .map_err(|e| format!("codex reset parse failed: {e}"))?;

    let refreshed = async {
        let revision = limits::read_snapshot_revision(db, provider_id).await?;
        let refreshed_limits = fetch_codex_usage_limits(
            client,
            &endpoints.usage_url,
            &access_token,
            Some(&chatgpt_account_id),
        )
        .await?;
        Ok::<_, String>((refreshed_limits, revision))
    }
    .await;
    match refreshed {
        Ok((refreshed_limits, revision)) => Ok((
            ProviderOAuthResetCodexQuotaResult {
                success: true,
                code: consumed.code,
                windows_reset: consumed.windows_reset,
                refreshed_limits: Some(refreshed_limits),
                refresh_error: None,
            },
            Some(revision),
        )),
        Err(refresh_error) => Ok((
            ProviderOAuthResetCodexQuotaResult {
                success: true,
                code: consumed.code,
                windows_reset: consumed.windows_reset,
                refreshed_limits: None,
                refresh_error: Some(refresh_error),
            },
            None,
        )),
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn provider_oauth_reset_codex_quota(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    confirm: Option<RiskyIpcConfirm>,
) -> Result<ProviderOAuthResetCodexQuotaResult, String> {
    require_codex_reset_confirm(provider_id, confirm)?;

    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let _reset_guard = try_enter_codex_reset(provider_id)?;
    let mut details = blocking::run("provider_oauth_reset_codex_quota_load", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;
    validate_codex_reset_details(&details)?;
    let expected_token = details.oauth_access_token.clone();
    let mut quota_guard = limits::lock_for_reset(provider_id).await?;
    *quota_guard = None;
    details = blocking::run("provider_oauth_reset_codex_quota_reload", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;
    validate_codex_reset_details(&details)?;
    if details.oauth_access_token != expected_token {
        return Err("OAUTH_QUOTA_STALE_TOKEN: OAuth credentials changed before resetting quota; please retry".to_string());
    }

    let client = crate::gateway::oauth::build_oauth_http_client(
        &app,
        &format!("aio-coding-hub-oauth-reset/{}", env!("CARGO_PKG_VERSION")),
        20,
        10,
    )?;

    let (mut result, expected_revision) = consume_codex_reset_credit_and_refresh(
        &db,
        &client,
        &CodexQuotaEndpoints::default(),
        &mut details,
    )
    .await?;

    if let (Some(ref mut refreshed_limits), Some(revision)) =
        (&mut result.refreshed_limits, expected_revision)
    {
        if let Err(err) = limits::save_result(&app, &db, &details, refreshed_limits, revision).await
        {
            // The credit was already consumed. Report only the refresh failure so
            // the caller does not retry a successful non-idempotent action.
            result.refreshed_limits = None;
            result.refresh_error = Some(err);
        }
    }

    *quota_guard = match &result.refreshed_limits {
        Some(refreshed) => Some((details.oauth_access_token.clone(), Ok(refreshed.clone()))),
        None => result
            .refresh_error
            .clone()
            .map(|error| (details.oauth_access_token.clone(), Err(error))),
    };
    drop(quota_guard);
    if let Some(ref mut refreshed_limits) = result.refreshed_limits {
        limits::attach_quota_estimates(&db, provider_id, refreshed_limits).await;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ProviderOAuthDetails;
    use crate::shared::ipc_confirm::{IpcConfirm, RiskyIpcConfirm};
    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::{get, post};
    use axum::Router;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    struct CapturedRequest {
        method: String,
        authorization: Option<String>,
        account_id: Option<String>,
        headers: HeaderMap,
        body: String,
    }

    #[derive(Clone)]
    struct TestQuotaState {
        usage_status: StatusCode,
        requests: Arc<Mutex<Vec<CapturedRequest>>>,
    }

    fn confirm(action: &str, resource: &str) -> RiskyIpcConfirm {
        RiskyIpcConfirm {
            confirm: IpcConfirm {
                action: action.to_string(),
                resource: resource.to_string(),
                nonce: "abcDEF1234567890".to_string(),
                issued_at_ms: crate::shared::time::now_unix_millis(),
                ttl_ms: 60_000,
            },
        }
    }

    fn codex_details(provider_id: i64) -> ProviderOAuthDetails {
        ProviderOAuthDetails {
            id: provider_id,
            cli_key: "codex".to_string(),
            oauth_provider_type: "codex_oauth".to_string(),
            oauth_access_token: "access-token".to_string(),
            oauth_refresh_token: Some("refresh-token".to_string()),
            oauth_id_token: None,
            oauth_token_uri: Some("https://auth.openai.com/oauth/token".to_string()),
            oauth_client_id: Some("client-id".to_string()),
            oauth_client_secret: None,
            oauth_expires_at: Some(crate::shared::time::now_unix_seconds() + 3_600),
            oauth_email: Some("codex@example.com".to_string()),
            oauth_refresh_lead_s: 60,
            oauth_last_refreshed_at: Some(1),
        }
    }

    fn account_id_token(account_id: &str) -> String {
        use base64::Engine;
        format!(
            "e30.{}.sig",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
                serde_json::json!({"https://api.openai.com/auth":{"chatgpt_account_id":account_id}})
                    .to_string()
            )
        )
    }

    async fn record_consume(
        State(state): State<TestQuotaState>,
        headers: HeaderMap,
        body: Bytes,
    ) -> impl IntoResponse {
        state
            .requests
            .lock()
            .expect("lock requests")
            .push(CapturedRequest {
                method: "POST".to_string(),
                authorization: headers
                    .get("authorization")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string),
                account_id: headers
                    .get("chatgpt-account-id")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string),
                headers: headers.clone(),
                body: String::from_utf8_lossy(&body).to_string(),
            });
        (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "code": "ok", "windows_reset": 2 })),
        )
    }

    async fn record_usage(
        State(state): State<TestQuotaState>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        state
            .requests
            .lock()
            .expect("lock requests")
            .push(CapturedRequest {
                method: "GET".to_string(),
                authorization: headers
                    .get("authorization")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string),
                account_id: headers
                    .get("chatgpt-account-id")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string),
                headers: headers.clone(),
                body: String::new(),
            });
        if state.usage_status != StatusCode::OK {
            return (state.usage_status, "usage failed").into_response();
        }
        (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "rate_limit": {
                    "primary_window": { "used_percent": 25.0, "reset_at": 1_800 },
                    "secondary_window": { "used_percent": 10.0, "reset_at": 3_600 }
                },
                "rate_limit_reset_credits": { "available_count": 7 }
            })),
        )
            .into_response()
    }

    async fn start_quota_server(
        usage_status: StatusCode,
    ) -> (CodexQuotaEndpoints, Arc<Mutex<Vec<CapturedRequest>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = TestQuotaState {
            usage_status,
            requests: requests.clone(),
        };
        let router = Router::new()
            .route("/consume", post(record_consume))
            .route("/usage", get(record_usage))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind quota server");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("serve quota server");
        });
        (
            CodexQuotaEndpoints {
                usage_url: format!("http://{addr}/usage"),
                reset_url: format!("http://{addr}/consume"),
            },
            requests,
        )
    }

    #[test]
    fn codex_reset_confirm_resource_is_provider_scoped() {
        assert_eq!(
            codex_reset_confirm_resource(42),
            "provider:42:codex_reset_credit"
        );
    }

    #[test]
    fn require_codex_reset_confirm_rejects_wrong_provider_resource() {
        let err = require_codex_reset_confirm(
            9,
            Some(confirm(
                PROVIDER_OAUTH_RESET_CODEX_QUOTA_ACTION,
                "provider:8:codex_reset_credit",
            )),
        )
        .unwrap_err();

        assert!(err.starts_with("SEC_CONFIRM_RESOURCE_MISMATCH:"));
    }

    #[test]
    fn validate_codex_reset_details_rejects_non_codex_oauth_provider() {
        let mut details = codex_details(7);
        details.cli_key = "claude".to_string();

        let err = validate_codex_reset_details(&details).unwrap_err();

        assert!(err.contains("Codex OAuth"));
    }

    #[test]
    fn codex_reset_in_flight_guard_is_provider_scoped() {
        let first = try_enter_codex_reset(1).expect("enter first provider");
        let duplicate = try_enter_codex_reset(1).unwrap_err();
        let second = try_enter_codex_reset(2).expect("enter second provider");

        assert!(duplicate.contains("already in progress"));
        drop(second);
        drop(first);
        assert!(try_enter_codex_reset(1).is_ok());
    }

    #[tokio::test]
    async fn consume_success_and_usage_refresh_failure_returns_partial_success() {
        let (endpoints, requests) = start_quota_server(StatusCode::INTERNAL_SERVER_ERROR).await;
        let client = reqwest::Client::new();

        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("reset.db")).expect("init db");
        let mut details = codex_details(1);
        details.oauth_id_token = Some(account_id_token("acct_123"));
        let (result, _) =
            consume_codex_reset_credit_and_refresh(&db, &client, &endpoints, &mut details)
                .await
                .expect("partial success result");

        assert!(result.success);
        assert_eq!(result.code.as_deref(), Some("ok"));
        assert_eq!(result.windows_reset, Some(2));
        assert!(result.refreshed_limits.is_none());
        assert!(result
            .refresh_error
            .as_deref()
            .unwrap_or_default()
            .contains("codex usage fetch status"));

        let requests = requests.lock().expect("lock requests");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "POST");
        assert_eq!(
            requests[0].authorization.as_deref(),
            Some("Bearer access-token")
        );
        assert_eq!(requests[0].account_id.as_deref(), Some("acct_123"));
        assert!(requests[0].body.contains("redeem_request_id"));
        assert_eq!(requests[1].method, "GET");
        assert_eq!(requests[1].account_id.as_deref(), Some("acct_123"));
        for request in requests.iter() {
            assert_eq!(
                request
                    .headers
                    .get("user-agent")
                    .and_then(|value| value.to_str().ok()),
                Some(crate::gateway::oauth::DEFAULT_OAUTH_USER_AGENT)
            );
            assert_eq!(
                request
                    .headers
                    .get("originator")
                    .and_then(|value| value.to_str().ok()),
                Some("codex_cli_rs")
            );
            assert!(!request.headers.contains_key("x-openai-codex-luna-reserve"));
        }
    }

    #[tokio::test]
    async fn consume_success_refreshes_limits_and_reset_count() {
        let (endpoints, _requests) = start_quota_server(StatusCode::OK).await;
        let client = reqwest::Client::new();

        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("reset.db")).expect("init db");
        let mut details = codex_details(1);
        details.oauth_id_token = Some(account_id_token("acct_123"));
        let (result, _) =
            consume_codex_reset_credit_and_refresh(&db, &client, &endpoints, &mut details)
                .await
                .expect("reset success");
        let refreshed = result.refreshed_limits.expect("refreshed limits");

        assert!(result.success);
        assert_eq!(refreshed.limit_5h_text.as_deref(), Some("75%"));
        assert_eq!(refreshed.limit_weekly_text.as_deref(), Some("90%"));
        assert_eq!(refreshed.limit_5h_reset_at, Some(1_800));
        assert_eq!(refreshed.limit_weekly_reset_at, Some(3_600));
        assert_eq!(refreshed.reset_credit_available_count, Some(7));
    }

    #[tokio::test]
    async fn reset_refresh_keeps_the_original_account_before_consuming_credit() {
        for replacement_account in [None, Some("account-a"), Some("account-b")] {
            let (endpoints, requests) = start_quota_server(StatusCode::OK).await;
            let refresh_started = Arc::new(tokio::sync::Notify::new());
            let release_refresh = Arc::new(tokio::sync::Notify::new());
            let token_router = Router::new().route(
                "/token",
                post({
                    let started = refresh_started.clone();
                    let release = release_refresh.clone();
                    move || {
                        let started = started.clone();
                        let release = release.clone();
                        async move {
                            started.notify_one();
                            release.notified().await;
                            axum::Json(serde_json::json!({
                                "access_token": "rotated-access-token",
                                "refresh_token": "rotated-refresh-token",
                                "expires_in": 3600
                            }))
                        }
                    }
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let token_uri = format!("http://{}/token", listener.local_addr().unwrap());
            let token_task = tokio::spawn(async move {
                axum::serve(listener, token_router).await.unwrap();
            });
            let dir = tempfile::tempdir().unwrap();
            let db = crate::db::init_for_tests(&dir.path().join("reset-account.db")).unwrap();
            let mut details = codex_details(1);
            details.oauth_id_token = Some(account_id_token("account-a"));
            details.oauth_token_uri = Some(token_uri);
            details.oauth_expires_at = Some(crate::shared::time::now_unix_seconds() - 1);
            db.open_connection().unwrap().execute(
                "INSERT INTO providers(id,cli_key,name,base_url,api_key_plaintext,created_at,updated_at,auth_mode,oauth_provider_type,oauth_access_token,oauth_refresh_token,oauth_id_token,oauth_token_uri,oauth_client_id,oauth_expires_at,oauth_last_refreshed_at)
                 VALUES (1,'codex','reset-account','','',1,1,'oauth','codex_oauth',?1,'refresh-token',?2,?3,'client-id',?4,1)",
                rusqlite::params![details.oauth_access_token,details.oauth_id_token,details.oauth_token_uri,details.oauth_expires_at],
            ).unwrap();
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            let reset =
                consume_codex_reset_credit_and_refresh(&db, &client, &endpoints, &mut details);
            let replace_during_refresh = async {
                refresh_started.notified().await;
                if let Some(account) = replacement_account {
                    db.open_connection().unwrap().execute(
                        "UPDATE providers SET oauth_access_token='replacement-access-token', oauth_id_token=?1, oauth_last_refreshed_at=2 WHERE id=1",
                        [account_id_token(account)],
                    ).unwrap();
                }
                release_refresh.notify_one();
            };
            let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                tokio::join!(reset, replace_during_refresh)
            })
            .await;
            token_task.abort();
            let (result, ()) = result.expect("refresh and reset finish");
            let requests = requests.lock().unwrap();
            if replacement_account == Some("account-b") {
                assert!(result.unwrap_err().starts_with("OAUTH_QUOTA_STALE_TOKEN:"));
                assert!(requests.is_empty(), "must not consume or query account B");
                let persisted = crate::providers::get_oauth_details(&db, 1).unwrap();
                assert_eq!(codex_reset_account_id(&persisted).unwrap(), "account-b");
            } else {
                assert!(result.unwrap().0.success);
                assert_eq!(requests.len(), 2);
                assert_eq!(requests[0].method, "POST");
                assert_eq!(requests[1].method, "GET");
                let expected_authorization = if replacement_account.is_some() {
                    "Bearer replacement-access-token"
                } else {
                    "Bearer rotated-access-token"
                };
                for request in requests.iter() {
                    assert_eq!(request.account_id.as_deref(), Some("account-a"));
                    assert_eq!(
                        request.authorization.as_deref(),
                        Some(expected_authorization)
                    );
                }
            }
        }
    }

    #[test]
    fn quota_account_id_prefers_id_token_and_falls_back_to_access_token() {
        let mut details = codex_details(42);
        details.oauth_access_token = account_id_token("access-account");
        assert_eq!(
            limits::codex_account_id(&details).as_deref(),
            Some("access-account")
        );
        details.oauth_id_token = Some(account_id_token("id-account"));
        assert_eq!(
            limits::codex_account_id(&details).as_deref(),
            Some("id-account")
        );
        details.oauth_id_token = Some("invalid".to_string());
        assert_eq!(
            limits::codex_account_id(&details).as_deref(),
            Some("access-account")
        );
    }
}
