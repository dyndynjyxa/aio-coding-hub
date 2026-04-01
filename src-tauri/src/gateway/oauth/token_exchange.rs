//! Usage: OAuth token exchange (authorization_code grant) and refresh (refresh_token grant).

use super::provider_trait::OAuthTokenSet;
use crate::shared::security::mask_token;
use crate::shared::time::now_unix_seconds;

#[derive(Debug)]
pub(crate) struct TokenExchangeRequest {
    pub token_uri: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub code: String,
    pub redirect_uri: String,
    pub code_verifier: String,
    pub state: Option<String>,
}

#[derive(Debug)]
pub(crate) struct TokenRefreshRequest {
    pub token_uri: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub refresh_token: String,
}

pub(crate) async fn exchange_authorization_code(
    client: &reqwest::Client,
    req: &TokenExchangeRequest,
) -> Result<OAuthTokenSet, String> {
    tracing::info!(
        token_uri = %req.token_uri,
        client_id = %req.client_id,
        redirect_uri = %req.redirect_uri,
        code_len = req.code.len(),
        code_verifier_len = req.code_verifier.len(),
        "exchanging authorization code for tokens"
    );

    // Anthropic requires JSON body, others use form-encoded
    let is_anthropic = is_anthropic_oauth_token_uri(&req.token_uri);

    let resp = if is_anthropic {
        let missing_state = req
            .state
            .as_ref()
            .map(|state| state.trim().is_empty())
            .unwrap_or(true);
        if missing_state {
            return Err(
                "SEC_INVALID_INPUT: Anthropic token exchange requires non-empty OAuth state"
                    .to_string(),
            );
        }

        let body = build_anthropic_exchange_json(req);

        client
            .post(&req.token_uri)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("token exchange request failed: {e}"))?
    } else {
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", &req.code),
            ("redirect_uri", &req.redirect_uri),
            ("client_id", &req.client_id),
            ("code_verifier", &req.code_verifier),
        ];

        let secret_ref;
        if let Some(ref secret) = req.client_secret {
            secret_ref = secret.clone();
            form.push(("client_secret", &secret_ref));
        }

        client
            .post(&req.token_uri)
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("token exchange request failed: {e}"))?
    };

    parse_token_response(resp).await
}

fn build_anthropic_exchange_json(req: &TokenExchangeRequest) -> serde_json::Value {
    let mut body = serde_json::json!({
        "grant_type": "authorization_code",
        "code": req.code,
        "redirect_uri": req.redirect_uri,
        "client_id": req.client_id,
        "code_verifier": req.code_verifier,
    });

    if let Some(ref state) = req.state {
        body["state"] = serde_json::json!(state);
    }

    if let Some(ref secret) = req.client_secret {
        body["client_secret"] = serde_json::json!(secret);
    }

    body
}

pub(crate) async fn refresh_access_token(
    client: &reqwest::Client,
    req: &TokenRefreshRequest,
) -> Result<OAuthTokenSet, String> {
    tracing::debug!(
        token_uri = %req.token_uri,
        refresh_token = %mask_token(&req.refresh_token),
        "refreshing access token"
    );

    // Anthropic requires JSON body, others use form-encoded
    let is_anthropic = is_anthropic_oauth_token_uri(&req.token_uri);

    let resp = if is_anthropic {
        let body = build_anthropic_refresh_json(req);

        client
            .post(&req.token_uri)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("token refresh request failed: {e}"))?
    } else {
        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", &req.refresh_token),
            ("client_id", &req.client_id),
        ];

        let secret_ref;
        if let Some(ref secret) = req.client_secret {
            secret_ref = secret.clone();
            form.push(("client_secret", &secret_ref));
        }

        client
            .post(&req.token_uri)
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("token refresh request failed: {e}"))?
    };

    parse_token_response(resp).await
}

fn build_anthropic_refresh_json(req: &TokenRefreshRequest) -> serde_json::Value {
    let mut body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": req.refresh_token,
        "client_id": req.client_id,
    });

    if let Some(ref secret) = req.client_secret {
        body["client_secret"] = serde_json::json!(secret);
    }

    body
}

fn is_anthropic_oauth_token_uri(token_uri: &str) -> bool {
    let uri = token_uri.trim().to_ascii_lowercase();
    uri.contains("api.anthropic.com/v1/oauth/token")
        || uri.contains("platform.claude.com/v1/oauth/token")
        || (uri.contains("/v1/oauth/token")
            && (uri.contains("anthropic.com") || uri.contains("claude.com")))
}

async fn parse_token_response(resp: reqwest::Response) -> Result<OAuthTokenSet, String> {
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("failed to read token response body: {e}"))?;

    if !status.is_success() {
        // Try to parse error details
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            // Anthropic uses nested error structure: {"type":"error","error":{"type":"...","message":"..."}}
            let (error, desc) =
                if let Some(error_obj) = json.get("error").and_then(|v| v.as_object()) {
                    // Nested structure (Anthropic format)
                    let error_type = error_obj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let error_msg = error_obj
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    (error_type, error_msg)
                } else {
                    // Flat structure (standard OAuth format)
                    let error = json
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let desc = json
                        .get("error_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    (error, desc)
                };

            if error == "invalid_grant" && desc.contains("refresh_token") {
                return Err(
                    "AUTH_RELOGIN_REQUIRED: refresh token is invalid or expired".to_string()
                );
            }

            return Err(format!("token endpoint error ({status}): {error}: {desc}"));
        }
        // Non-JSON body – likely a Cloudflare challenge page or HTML error.
        // Include a truncated snippet for diagnosis.
        let snippet: String = body.chars().take(200).collect();
        tracing::warn!(
            %status,
            body_snippet = %snippet,
            "token endpoint returned non-JSON error; possible WAF/Cloudflare block"
        );
        return Err(format!(
            "token endpoint returned {status} (non-JSON response, possible Cloudflare block)"
        ));
    }

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse token response JSON: {e}"))?;

    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("token response missing access_token")?
        .to_string();

    let refresh_token = json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let id_token = json
        .get("id_token")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let expires_at = json
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .map(|secs| now_unix_seconds() + secs);

    Ok(OAuthTokenSet {
        access_token,
        refresh_token,
        expires_at,
        id_token,
    })
}
