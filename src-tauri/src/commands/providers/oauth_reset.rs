use crate::app_state::{ensure_db_ready, DbInitState};
use crate::blocking;
use crate::commands::providers::oauth_limits::ProviderOAuthLimitsResult;
use crate::domain::provider_oauth_limits::OAuthLimitSnapshotInput;
use crate::shared::http_body::read_text_with_limit;
use crate::shared::ipc_confirm::RiskyIpcConfirm;
use rand::RngCore;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

pub(crate) const PROVIDER_OAUTH_RESET_CODEX_QUOTA_ACTION: &str = "provider_oauth_reset_codex_quota";
const CODEX_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CODEX_RESET_URL: &str =
    "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits/consume";
const CODEX_QUOTA_ORIGINATOR: &str = "codex_cli_rs";
const CODEX_USAGE_RESPONSE_BODY_LIMIT: usize = 1024 * 1024;
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

fn build_codex_quota_headers(
    access_token: &str,
    chatgpt_account_id: &str,
) -> Vec<(&'static str, String)> {
    vec![
        ("authorization", format!("Bearer {access_token}")),
        ("chatgpt-account-id", chatgpt_account_id.to_string()),
        ("accept", "application/json".to_string()),
        ("content-type", "application/json".to_string()),
        ("oai-language", "zh-CN".to_string()),
        ("originator", CODEX_QUOTA_ORIGINATOR.to_string()),
        (
            "user-agent",
            format!(
                "{} WindowsTerminal",
                crate::gateway::oauth::DEFAULT_OAUTH_USER_AGENT
            ),
        ),
    ]
}

fn apply_codex_quota_headers(
    request: reqwest::RequestBuilder,
    access_token: &str,
    chatgpt_account_id: &str,
) -> reqwest::RequestBuilder {
    build_codex_quota_headers(access_token, chatgpt_account_id)
        .into_iter()
        .fold(request, |request, (name, value)| {
            request.header(name, value)
        })
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

pub(super) async fn fetch_codex_usage_limits(
    client: &reqwest::Client,
    endpoints: &CodexQuotaEndpoints,
    access_token: &str,
    chatgpt_account_id: &str,
) -> Result<ProviderOAuthLimitsResult, String> {
    let response = apply_codex_quota_headers(
        client.get(&endpoints.usage_url),
        access_token,
        chatgpt_account_id,
    )
    .send()
    .await
    .map_err(|e| format!("codex usage fetch failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = read_text_with_limit(response, CODEX_RESET_RESPONSE_BODY_LIMIT, "codex usage")
            .await
            .unwrap_or_default();
        return Err(format!("codex usage fetch status: {status} - {text}"));
    }

    let body = read_text_with_limit(response, CODEX_USAGE_RESPONSE_BODY_LIMIT, "codex usage")
        .await
        .map_err(|e| format!("codex usage body read failed: {e}"))?;
    let raw_json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("codex usage parse failed: {e}"))?;
    Ok(
        super::oauth_limits::provider_oauth_limits_result_from_parts(
            "codex",
            None,
            None,
            None,
            Some(&raw_json),
        ),
    )
}

async fn consume_codex_reset_credit_and_refresh(
    client: &reqwest::Client,
    endpoints: &CodexQuotaEndpoints,
    access_token: &str,
    chatgpt_account_id: &str,
) -> Result<ProviderOAuthResetCodexQuotaResult, String> {
    let redeem_request_id = generate_redeem_request_id();
    let response = apply_codex_quota_headers(
        client.post(&endpoints.reset_url),
        access_token,
        chatgpt_account_id,
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

    match fetch_codex_usage_limits(client, endpoints, access_token, chatgpt_account_id).await {
        Ok(refreshed_limits) => Ok(ProviderOAuthResetCodexQuotaResult {
            success: true,
            code: consumed.code,
            windows_reset: consumed.windows_reset,
            refreshed_limits: Some(refreshed_limits),
            refresh_error: None,
        }),
        Err(refresh_error) => Ok(ProviderOAuthResetCodexQuotaResult {
            success: true,
            code: consumed.code,
            windows_reset: consumed.windows_reset,
            refreshed_limits: None,
            refresh_error: Some(refresh_error),
        }),
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

    let db = ensure_db_ready(app, db_state.inner()).await?;
    let mut details = blocking::run("provider_oauth_reset_codex_quota_load", {
        let db = db.clone();
        move || crate::providers::get_oauth_details(&db, provider_id)
    })
    .await
    .map_err(Into::<String>::into)?;
    validate_codex_reset_details(&details)?;
    let _reset_guard = try_enter_codex_reset(provider_id)?;

    let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter_for_details(&details)?;
    let client = crate::gateway::oauth::build_oauth_http_client(
        &format!("aio-coding-hub-oauth-reset/{}", env!("CARGO_PKG_VERSION")),
        20,
        10,
    )?;

    if super::oauth::oauth_details_can_refresh(&details)
        && crate::gateway::oauth::refresh::should_refresh_now(
            details.oauth_expires_at,
            details.oauth_refresh_lead_s,
        )
    {
        details =
            super::oauth::refresh_oauth_details_for_limits(&db, &client, &details, adapter).await?;
    }

    let access_token = super::oauth::effective_oauth_access_token(&details, adapter)?;
    let (account_id, _) = super::oauth::extract_codex_identity(details.oauth_id_token.as_deref());
    let account_id = account_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "SEC_INVALID_INPUT: Codex OAuth missing chatgpt_account_id; please re-login this provider".to_string()
        })?;

    let result = consume_codex_reset_credit_and_refresh(
        &client,
        &CodexQuotaEndpoints::default(),
        &access_token,
        &account_id,
    )
    .await?;

    if let Some(ref refreshed_limits) = result.refreshed_limits {
        blocking::run("provider_oauth_reset_codex_quota_save_snapshot", {
            let db = db.clone();
            let refreshed_limits = refreshed_limits.clone();
            move || {
                crate::domain::provider_oauth_limits::save_snapshot(
                    &db,
                    OAuthLimitSnapshotInput {
                        provider_id,
                        limit_short_label: refreshed_limits.limit_short_label.as_deref(),
                        limit_5h_text: refreshed_limits.limit_5h_text.as_deref(),
                        limit_weekly_text: refreshed_limits.limit_weekly_text.as_deref(),
                        limit_5h_reset_at: refreshed_limits.limit_5h_reset_at,
                        limit_weekly_reset_at: refreshed_limits.limit_weekly_reset_at,
                        reset_credit_available_count: refreshed_limits.reset_credit_available_count,
                    },
                )
            }
        })
        .await
        .map_err(Into::<String>::into)?;
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

        let result =
            consume_codex_reset_credit_and_refresh(&client, &endpoints, "access-token", "acct_123")
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
    }

    #[tokio::test]
    async fn consume_success_refreshes_limits_and_reset_count() {
        let (endpoints, _requests) = start_quota_server(StatusCode::OK).await;
        let client = reqwest::Client::new();

        let result =
            consume_codex_reset_credit_and_refresh(&client, &endpoints, "access-token", "acct_123")
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
}
