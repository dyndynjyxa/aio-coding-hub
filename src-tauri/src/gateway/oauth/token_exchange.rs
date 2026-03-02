//! Usage: OAuth token endpoint helpers (authorization_code + refresh_token grants).

use crate::shared::error::AppResult;
use crate::shared::security::mask_token;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct TokenExchangeRequest {
    pub(crate) token_uri: String,
    pub(crate) client_id: String,
    pub(crate) client_secret: Option<String>,
    pub(crate) code: String,
    pub(crate) redirect_uri: String,
    pub(crate) code_verifier: String,
}

#[derive(Debug, Clone)]
pub(crate) struct TokenRefreshRequest {
    pub(crate) token_uri: String,
    pub(crate) client_id: String,
    pub(crate) client_secret: Option<String>,
    pub(crate) refresh_token: String,
}

#[derive(Debug, Clone)]
pub(crate) struct OAuthTokenSet {
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
    pub(crate) expires_at: Option<i64>,
    pub(crate) id_token: Option<String>,
}

pub(crate) async fn exchange_authorization_code(
    client: &reqwest::Client,
    req: &TokenExchangeRequest,
) -> AppResult<OAuthTokenSet> {
    let mut form: HashMap<&str, String> = HashMap::new();
    form.insert("grant_type", "authorization_code".to_string());
    form.insert("code", req.code.trim().to_string());
    form.insert("redirect_uri", req.redirect_uri.trim().to_string());
    form.insert("client_id", req.client_id.trim().to_string());
    form.insert("code_verifier", req.code_verifier.trim().to_string());
    if let Some(secret) = req.client_secret.as_deref().map(str::trim) {
        if !secret.is_empty() {
            form.insert("client_secret", secret.to_string());
        }
    }

    let response = client
        .post(req.token_uri.trim())
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("SYSTEM_ERROR: oauth token exchange request failed: {e}"))?;

    parse_token_response(response).await
}

pub(crate) async fn refresh_access_token(
    client: &reqwest::Client,
    req: &TokenRefreshRequest,
) -> AppResult<OAuthTokenSet> {
    let mut form: HashMap<&str, String> = HashMap::new();
    form.insert("grant_type", "refresh_token".to_string());
    form.insert("refresh_token", req.refresh_token.trim().to_string());
    form.insert("client_id", req.client_id.trim().to_string());
    if let Some(secret) = req.client_secret.as_deref().map(str::trim) {
        if !secret.is_empty() {
            form.insert("client_secret", secret.to_string());
        }
    }

    let response = client
        .post(req.token_uri.trim())
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("SYSTEM_ERROR: oauth refresh request failed: {e}"))?;

    parse_token_response(response).await
}

pub(crate) fn resolve_effective_access_token(
    cli_key: &str,
    token_set: &OAuthTokenSet,
    stored_id_token: Option<&str>,
) -> (String, Option<String>) {
    if cli_key == "gemini" && !token_set.access_token.trim().starts_with("ya29.") {
        tracing::warn!(
            "gemini oauth access_token does not match expected ya29.* format; exchange response may be invalid"
        );
    }

    let id_token_to_store = token_set.id_token.clone().or_else(|| {
        stored_id_token
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    });

    (token_set.access_token.clone(), id_token_to_store)
}

async fn parse_token_response(response: reqwest::Response) -> AppResult<OAuthTokenSet> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("SYSTEM_ERROR: oauth token response read failed: {e}"))?;

    if !status.is_success() {
        let (error_code, error_message) = parse_oauth_error_details(&body);
        if status == reqwest::StatusCode::UNAUTHORIZED
            && is_refresh_token_reused_error(error_code.as_deref(), error_message.as_deref())
        {
            return Err(
                "AUTH_RELOGIN_REQUIRED: oauth refresh token was already rotated/reused; please use 浏览器登录 to reauthorize this account"
                    .to_string()
                    .into(),
            );
        }

        let snippet = sanitize_oauth_error_body_snippet(&body);
        let mut msg = format!(
            "SYSTEM_ERROR: oauth token endpoint returned status={}",
            status.as_u16()
        );
        if let Some(code) = error_code {
            msg.push_str(" code=");
            msg.push_str(code.as_str());
        }
        if let Some(detail) = error_message {
            msg.push_str(" message=");
            msg.push_str(detail.chars().take(240).collect::<String>().as_str());
        }
        msg.push_str(" body=");
        msg.push_str(snippet.as_str());
        return Err(msg.into());
    }

    let value: Value = serde_json::from_str(&body)
        .map_err(|e| format!("SYSTEM_ERROR: oauth token response json invalid: {e}"))?;

    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "SYSTEM_ERROR: oauth token response missing access_token".to_string())?
        .to_string();

    let refresh_token = value
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);

    let id_token = value
        .get("id_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);

    let expires_in = value.get("expires_in").and_then(parse_i64_lossy);
    let now = crate::shared::time::now_unix_seconds();
    let expires_at = expires_in.and_then(|v| {
        if v <= 0 {
            None
        } else {
            Some(now.saturating_add(v))
        }
    });

    Ok(OAuthTokenSet {
        access_token,
        refresh_token,
        expires_at,
        id_token,
    })
}

fn parse_i64_lossy(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key_lc = key.trim().to_ascii_lowercase();
    key_lc.contains("token")
        || key_lc.contains("secret")
        || key_lc == "authorization"
        || key_lc == "proxy-authorization"
}

fn redact_sensitive_json_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, nested) in map {
                if is_sensitive_key(key) {
                    if let Some(raw) = nested.as_str() {
                        *nested = Value::String(mask_token(raw));
                        continue;
                    }
                }
                redact_sensitive_json_fields(nested);
            }
        }
        Value::Array(items) => {
            for nested in items {
                redact_sensitive_json_fields(nested);
            }
        }
        _ => {}
    }
}

fn sanitize_oauth_error_body_snippet(body: &str) -> String {
    if let Ok(mut value) = serde_json::from_str::<Value>(body) {
        redact_sensitive_json_fields(&mut value);
        if let Ok(encoded) = serde_json::to_string(&value) {
            return encoded.chars().take(500).collect();
        }
    }
    body.chars().take(500).collect()
}

fn parse_oauth_error_details(body: &str) -> (Option<String>, Option<String>) {
    let value: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };

    let mut code = value
        .get("code")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let mut message = value
        .get("error_description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);

    if let Some(error_value) = value.get("error") {
        if let Some(err_str) = error_value.as_str() {
            if code.is_none() {
                code = Some(err_str.trim().to_string());
            }
        } else if let Some(err_obj) = error_value.as_object() {
            if code.is_none() {
                code = err_obj
                    .get("code")
                    .and_then(Value::as_str)
                    .or_else(|| err_obj.get("type").and_then(Value::as_str))
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string);
            }
            if message.is_none() {
                message = err_obj
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string);
            }
        }
    }

    (code, message)
}

fn is_refresh_token_reused_error(code: Option<&str>, message: Option<&str>) -> bool {
    let code_hit = code
        .map(str::trim)
        .is_some_and(|v| v.eq_ignore_ascii_case("refresh_token_reused"));
    if code_hit {
        return true;
    }
    message
        .map(str::to_ascii_lowercase)
        .is_some_and(|v| v.contains("refresh token has already been used"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::security::mask_token;

    #[test]
    fn parse_i64_lossy_supports_number_and_string() {
        assert_eq!(parse_i64_lossy(&Value::from(1200)), Some(1200));
        assert_eq!(parse_i64_lossy(&Value::from("3600")), Some(3600));
        assert_eq!(parse_i64_lossy(&Value::from("x")), None);
    }

    #[test]
    fn codex_uses_oauth_access_token_even_without_id_token() {
        let token_set = OAuthTokenSet {
            access_token: "oauth-access-token".to_string(),
            refresh_token: None,
            expires_at: None,
            id_token: None,
        };

        let resolved = resolve_effective_access_token("codex", &token_set, None);

        assert_eq!(resolved.0, "oauth-access-token");
        assert_eq!(resolved.1, None);
    }

    #[test]
    fn resolver_keeps_stored_id_token_when_refresh_response_omits_it() {
        let token_set = OAuthTokenSet {
            access_token: "oauth-access-token".to_string(),
            refresh_token: None,
            expires_at: None,
            id_token: None,
        };

        let resolved = resolve_effective_access_token("codex", &token_set, Some("id-from-store"));

        assert_eq!(resolved.0, "oauth-access-token");
        assert_eq!(resolved.1.as_deref(), Some("id-from-store"));
    }

    #[test]
    fn parse_oauth_error_details_supports_nested_error_payload() {
        let payload = r#"{
          "error": {
            "message": "Your refresh token has already been used.",
            "type": "invalid_request_error",
            "code": "refresh_token_reused"
          }
        }"#;

        let (code, message) = parse_oauth_error_details(payload);
        assert_eq!(code.as_deref(), Some("refresh_token_reused"));
        assert_eq!(
            message.as_deref(),
            Some("Your refresh token has already been used.")
        );
    }

    #[test]
    fn parse_oauth_error_details_supports_oauth_standard_fields() {
        let payload = r#"{
          "error": "invalid_grant",
          "error_description": "token is invalid"
        }"#;

        let (code, message) = parse_oauth_error_details(payload);
        assert_eq!(code.as_deref(), Some("invalid_grant"));
        assert_eq!(message.as_deref(), Some("token is invalid"));
    }

    #[test]
    fn refresh_token_reused_detector_matches_code_and_message() {
        assert!(is_refresh_token_reused_error(
            Some("refresh_token_reused"),
            None
        ));
        assert!(is_refresh_token_reused_error(
            None,
            Some("Your refresh token has already been used to generate a new access token.")
        ));
        assert!(!is_refresh_token_reused_error(Some("invalid_grant"), None));
    }

    #[test]
    fn sanitize_oauth_error_body_snippet_masks_token_fields() {
        let raw = r#"{
          "error": {
            "message": "invalid token",
            "refresh_token": "abcd1234xyz9876",
            "nested": {"id_token": "idtokenvalue123456"}
          }
        }"#;
        let snippet = sanitize_oauth_error_body_snippet(raw);
        assert!(snippet.contains(mask_token("abcd1234xyz9876").as_str()));
        assert!(snippet.contains(mask_token("idtokenvalue123456").as_str()));
        assert!(!snippet.contains("abcd1234xyz9876"));
        assert!(!snippet.contains("idtokenvalue123456"));
    }
}
