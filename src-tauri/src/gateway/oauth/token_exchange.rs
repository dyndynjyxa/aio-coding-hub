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
    pub state: Option<String>,
    pub redirect_uri: String,
    pub code_verifier: String,
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
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("code", &req.code),
        ("redirect_uri", &req.redirect_uri),
        ("client_id", &req.client_id),
        ("code_verifier", &req.code_verifier),
    ];

    if let Some(ref state) = req.state {
        form.push(("state", state));
    }

    if let Some(ref secret) = req.client_secret {
        form.push(("client_secret", secret));
    }

    tracing::info!(
        token_uri = %req.token_uri,
        client_id = %req.client_id,
        "exchanging authorization code for tokens"
    );

    let resp = client
        .post(&req.token_uri)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("token exchange request failed: {e}"))?;

    parse_token_response(resp).await
}

pub(crate) async fn refresh_access_token(
    client: &reqwest::Client,
    req: &TokenRefreshRequest,
) -> Result<OAuthTokenSet, String> {
    let mut form = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", &req.refresh_token),
        ("client_id", &req.client_id),
    ];

    if let Some(ref secret) = req.client_secret {
        form.push(("client_secret", secret));
    }

    tracing::debug!(
        token_uri = %req.token_uri,
        refresh_token = %mask_token(&req.refresh_token),
        "refreshing access token"
    );

    let resp = client
        .post(&req.token_uri)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("token refresh request failed: {e}"))?;

    parse_token_response(resp).await
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
            let error = json
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let desc = json
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

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
