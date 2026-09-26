//! Usage: Fetch and normalize model catalogs for Provider editor drafts.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::gateway::oauth::adapters::codex::codex_model_discovery_version;
use crate::gateway::util::build_target_url;
use crate::{blocking, gateway, providers};
use reqwest::header::{HeaderMap, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(15);
const DISCOVERY_BODY_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderModelDiscoveryInput {
    pub provider_id: Option<i64>,
    pub cli_key: String,
    pub auth_mode: providers::ProviderAuthMode,
    pub base_urls: Vec<String>,
    pub base_url_mode: providers::ProviderBaseUrlMode,
    pub api_key: Option<String>,
    pub source_provider_id: Option<i64>,
    pub bridge_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderModelDiscoveryUnsupportedReason {
    Oauth,
    #[serde(rename = "cx_2cc")]
    Cx2cc,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderModelDiscoveryErrorCode {
    InvalidConfig,
    Redirect,
    Unauthorized,
    Timeout,
    Network,
    InvalidResponse,
    TooLarge,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum ProviderModelDiscoveryResult {
    Ready {
        models: Vec<String>,
        origin: String,
        base_url_index: Option<u32>,
    },
    Empty {
        origin: String,
        base_url_index: Option<u32>,
    },
    Unsupported {
        reason: ProviderModelDiscoveryUnsupportedReason,
    },
    Error {
        code: ProviderModelDiscoveryErrorCode,
        http_status: Option<u16>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelCatalogFormat {
    DataIds,
    CodexOAuthSlugs,
    GeminiNames,
    GrokOAuth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiscoveryParseError {
    InvalidResponse,
}

fn catalog_format(cli_key: &str) -> Option<ModelCatalogFormat> {
    match cli_key {
        "claude" | "claude_desktop" | "codex" | "grok" => Some(ModelCatalogFormat::DataIds),
        "gemini" => Some(ModelCatalogFormat::GeminiNames),
        _ => None,
    }
}

fn has_pagination_signal(root: &Value) -> bool {
    let Some(object) = root.as_object() else {
        return false;
    };

    object.get("has_more").and_then(Value::as_bool) == Some(true)
        || [
            "next",
            "next_page_token",
            "nextPageToken",
            "next_cursor",
            "nextCursor",
        ]
        .iter()
        .any(|key| {
            object
                .get(*key)
                .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
        })
}

fn catalog_items(root: &Value, format: ModelCatalogFormat) -> Option<&[Value]> {
    let key = match format {
        ModelCatalogFormat::DataIds | ModelCatalogFormat::GrokOAuth => "data",
        ModelCatalogFormat::GeminiNames | ModelCatalogFormat::CodexOAuthSlugs => "models",
    };
    root.get(key)?.as_array().map(Vec::as_slice)
}

fn grok_oauth_model_id(object: &serde_json::Map<String, Value>) -> Option<String> {
    let mut candidates = ["model", "modelId", "model_id", "id"]
        .into_iter()
        .filter_map(|key| object.get(key).and_then(Value::as_str));
    if let Some(value) = candidates.find(|value| !value.trim().is_empty()) {
        return Some(value.to_string());
    }

    let metadata = object.get("_meta").and_then(Value::as_object);
    let mut metadata_candidates = metadata.into_iter().flat_map(|metadata| {
        ["model", "modelId", "model_id", "id", "name"]
            .into_iter()
            .filter_map(|key| metadata.get(key).and_then(Value::as_str))
    });
    if let Some(value) = metadata_candidates.find(|value| !value.trim().is_empty()) {
        return Some(value.to_string());
    }

    object
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn parse_model_catalog_with_format(
    format: ModelCatalogFormat,
    body: &str,
) -> Result<Vec<String>, DiscoveryParseError> {
    let root =
        serde_json::from_str::<Value>(body).map_err(|_| DiscoveryParseError::InvalidResponse)?;
    if has_pagination_signal(&root) {
        return Err(DiscoveryParseError::InvalidResponse);
    }

    let items = catalog_items(&root, format).ok_or(DiscoveryParseError::InvalidResponse)?;
    let mut models = Vec::with_capacity(items.len());
    for item in items {
        let object = item
            .as_object()
            .ok_or(DiscoveryParseError::InvalidResponse)?;
        let raw = match format {
            ModelCatalogFormat::DataIds => {
                object.get("id").and_then(Value::as_str).map(str::to_string)
            }
            ModelCatalogFormat::CodexOAuthSlugs => object
                .get("slug")
                .and_then(Value::as_str)
                .map(str::to_string),
            ModelCatalogFormat::GeminiNames => object
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string),
            ModelCatalogFormat::GrokOAuth => grok_oauth_model_id(object),
        }
        .ok_or(DiscoveryParseError::InvalidResponse)?;
        let raw = match format {
            ModelCatalogFormat::GeminiNames | ModelCatalogFormat::GrokOAuth => {
                raw.strip_prefix("models/").unwrap_or(&raw)
            }
            ModelCatalogFormat::DataIds | ModelCatalogFormat::CodexOAuthSlugs => &raw,
        };
        let normalized = providers::normalize_concrete_model_id(raw)
            .map_err(|_| DiscoveryParseError::InvalidResponse)?;
        models.push(normalized);
    }

    models.sort_unstable();
    models.dedup();
    Ok(models)
}

#[cfg(test)]
fn parse_model_catalog(cli_key: &str, body: &str) -> Result<Vec<String>, DiscoveryParseError> {
    let format = catalog_format(cli_key).ok_or(DiscoveryParseError::InvalidResponse)?;
    parse_model_catalog_with_format(format, body)
}

fn discovery_error(
    code: ProviderModelDiscoveryErrorCode,
    http_status: Option<u16>,
) -> ProviderModelDiscoveryResult {
    ProviderModelDiscoveryResult::Error { code, http_status }
}

fn classify_body_error(error: &str) -> ProviderModelDiscoveryErrorCode {
    if error.starts_with("provider model discovery body exceeds") {
        ProviderModelDiscoveryErrorCode::TooLarge
    } else if error.contains("body is not valid UTF-8") {
        ProviderModelDiscoveryErrorCode::InvalidResponse
    } else {
        ProviderModelDiscoveryErrorCode::Network
    }
}

fn origin_for_url(url: &reqwest::Url) -> String {
    url.origin().ascii_serialization()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelCatalogDescriptor<'a> {
    ApiKey { format: ModelCatalogFormat },
    CodexOAuth { client_version: &'a str },
    GrokOAuth,
}

impl<'a> ModelCatalogDescriptor<'a> {
    fn client_version(self) -> Option<&'a str> {
        match self {
            Self::CodexOAuth { client_version } => Some(client_version),
            _ => None,
        }
    }

    fn format(self) -> ModelCatalogFormat {
        match self {
            Self::ApiKey { format } => format,
            Self::CodexOAuth { .. } => ModelCatalogFormat::CodexOAuthSlugs,
            Self::GrokOAuth => ModelCatalogFormat::GrokOAuth,
        }
    }

    fn endpoint_path(self) -> &'static str {
        match self {
            Self::ApiKey {
                format: ModelCatalogFormat::GeminiNames,
            } => "/v1beta/models",
            Self::ApiKey { .. } => "/v1/models",
            Self::CodexOAuth { .. } | Self::GrokOAuth => "/models",
        }
    }
}

fn api_key_descriptor(cli_key: &str) -> Option<ModelCatalogDescriptor<'static>> {
    Some(ModelCatalogDescriptor::ApiKey {
        format: catalog_format(cli_key)?,
    })
}

async fn fetch_model_catalog(
    cli_key: &str,
    api_key: &str,
    base_url: &str,
    base_url_index: Option<u32>,
    client: &reqwest::Client,
    timeout: Duration,
) -> ProviderModelDiscoveryResult {
    let Some(descriptor) = api_key_descriptor(cli_key) else {
        return discovery_error(ProviderModelDiscoveryErrorCode::InvalidConfig, None);
    };
    let mut headers = HeaderMap::new();
    // api_key_descriptor() above already limits cli_key to the supported CLIs.
    match cli_key {
        "claude" | "claude_desktop" => {
            headers.insert(
                "x-api-key",
                match reqwest::header::HeaderValue::from_str(api_key) {
                    Ok(value) => value,
                    Err(_) => {
                        return discovery_error(
                            ProviderModelDiscoveryErrorCode::InvalidConfig,
                            None,
                        )
                    }
                },
            );
            headers.insert(
                "anthropic-version",
                reqwest::header::HeaderValue::from_static("2023-06-01"),
            );
        }
        "gemini" => {
            headers.insert(
                "x-goog-api-key",
                match reqwest::header::HeaderValue::from_str(api_key) {
                    Ok(value) => value,
                    Err(_) => {
                        return discovery_error(
                            ProviderModelDiscoveryErrorCode::InvalidConfig,
                            None,
                        )
                    }
                },
            );
        }
        _ => {
            headers.insert(
                AUTHORIZATION,
                match reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}")) {
                    Ok(value) => value,
                    Err(_) => {
                        return discovery_error(
                            ProviderModelDiscoveryErrorCode::InvalidConfig,
                            None,
                        )
                    }
                },
            );
        }
    }

    fetch_model_catalog_with_descriptor(
        descriptor,
        base_url,
        base_url_index,
        headers,
        client,
        timeout,
    )
    .await
}

async fn fetch_model_catalog_with_descriptor(
    descriptor: ModelCatalogDescriptor<'_>,
    base_url: &str,
    base_url_index: Option<u32>,
    headers: HeaderMap,
    client: &reqwest::Client,
    timeout: Duration,
) -> ProviderModelDiscoveryResult {
    let mut url = match build_target_url(base_url, descriptor.endpoint_path(), None) {
        Ok(url) => url,
        Err(_) => return discovery_error(ProviderModelDiscoveryErrorCode::InvalidConfig, None),
    };
    if let Some(version) = descriptor.client_version() {
        url.query_pairs_mut().append_pair("client_version", version);
    }
    let origin = origin_for_url(&url);
    let request = client.get(url).headers(headers);

    let deadline = tokio::time::Instant::now() + timeout;
    let response = match tokio::time::timeout_at(deadline, request.send()).await {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => {
            let code = if error.is_timeout() {
                ProviderModelDiscoveryErrorCode::Timeout
            } else {
                ProviderModelDiscoveryErrorCode::Network
            };
            return discovery_error(code, None);
        }
        Err(_) => return discovery_error(ProviderModelDiscoveryErrorCode::Timeout, None),
    };

    let status = response.status();
    if status.is_redirection() {
        return discovery_error(
            ProviderModelDiscoveryErrorCode::Redirect,
            Some(status.as_u16()),
        );
    }
    if matches!(status.as_u16(), 401 | 403) {
        return discovery_error(
            ProviderModelDiscoveryErrorCode::Unauthorized,
            Some(status.as_u16()),
        );
    }
    if !status.is_success() {
        return discovery_error(
            ProviderModelDiscoveryErrorCode::InvalidResponse,
            Some(status.as_u16()),
        );
    }

    let body = match tokio::time::timeout_at(
        deadline,
        crate::shared::http_body::read_text_with_limit(
            response,
            DISCOVERY_BODY_LIMIT,
            "provider model discovery",
        ),
    )
    .await
    {
        Ok(Ok(body)) => body,
        Ok(Err(error)) => return discovery_error(classify_body_error(&error), None),
        Err(_) => return discovery_error(ProviderModelDiscoveryErrorCode::Timeout, None),
    };

    let models = match parse_model_catalog_with_format(descriptor.format(), &body) {
        Ok(models) => models,
        Err(DiscoveryParseError::InvalidResponse) => {
            return discovery_error(ProviderModelDiscoveryErrorCode::InvalidResponse, None)
        }
    };

    if models.is_empty() {
        ProviderModelDiscoveryResult::Empty {
            origin,
            base_url_index,
        }
    } else {
        ProviderModelDiscoveryResult::Ready {
            models,
            origin,
            base_url_index,
        }
    }
}

fn is_expected_provider_input_error(error: &crate::shared::error::AppError) -> bool {
    matches!(error.code(), "DB_NOT_FOUND" | "SEC_INVALID_INPUT")
}

fn oauth_catalog_descriptor(
    cli_key: &str,
    adapter: &'static dyn crate::gateway::oauth::provider_trait::OAuthProvider,
) -> Option<ModelCatalogDescriptor<'static>> {
    if adapter.cli_key() != cli_key {
        return None;
    }

    match (cli_key, adapter.provider_type()) {
        ("codex", "codex_oauth") => Some(ModelCatalogDescriptor::CodexOAuth {
            client_version: codex_model_discovery_version(None),
        }),
        ("grok", "grok_oauth") => Some(ModelCatalogDescriptor::GrokOAuth),
        _ => None,
    }
}

pub(crate) async fn provider_models_discover<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    db_state: &DbInitState,
    input: ProviderModelDiscoveryInput,
) -> Result<ProviderModelDiscoveryResult, String> {
    if providers::is_cx2cc_bridge(input.source_provider_id, input.bridge_type.as_deref()) {
        return Ok(ProviderModelDiscoveryResult::Unsupported {
            reason: ProviderModelDiscoveryUnsupportedReason::Cx2cc,
        });
    }
    let deadline = tokio::time::Instant::now() + DISCOVERY_TIMEOUT;
    if input.auth_mode == providers::ProviderAuthMode::Oauth {
        let Some(provider_id) = input.provider_id else {
            return Ok(ProviderModelDiscoveryResult::Unsupported {
                reason: ProviderModelDiscoveryUnsupportedReason::Oauth,
            });
        };
        let db = ensure_db_ready(app.clone(), db_state).await?;
        let cli_key = input.cli_key.clone();
        let lookup = blocking::run("provider_models_discover_oauth_provider", {
            let db = db.clone();
            let cli_key = cli_key.clone();
            move || -> crate::shared::error::AppResult<providers::ProviderOAuthDetails> {
                let conn = db.open_connection()?;
                let provider = providers::get_by_id(&conn, provider_id)?;
                if provider.cli_key != cli_key
                    || provider.auth_mode != providers::ProviderAuthMode::Oauth.as_str()
                    || providers::is_cx2cc_bridge(
                        provider.source_provider_id,
                        provider.bridge_type.as_deref(),
                    )
                {
                    return Err("SEC_INVALID_INPUT: provider connection mismatch"
                        .to_string()
                        .into());
                }
                drop(conn);
                providers::get_oauth_details(&db, provider_id)
            }
        })
        .await;
        let details = match lookup {
            Ok(details) => details,
            Err(error) if is_expected_provider_input_error(&error) => {
                return Ok(ProviderModelDiscoveryResult::Unsupported {
                    reason: ProviderModelDiscoveryUnsupportedReason::Oauth,
                })
            }
            Err(error) => return Err(error.into()),
        };
        if details.cli_key != input.cli_key {
            return Ok(ProviderModelDiscoveryResult::Unsupported {
                reason: ProviderModelDiscoveryUnsupportedReason::Oauth,
            });
        }
        let adapter =
            match crate::gateway::oauth::registry::resolve_oauth_adapter_for_details(&details) {
                Ok(adapter) => adapter,
                Err(_) => {
                    return Ok(ProviderModelDiscoveryResult::Unsupported {
                        reason: ProviderModelDiscoveryUnsupportedReason::Oauth,
                    })
                }
            };
        let Some(descriptor) = oauth_catalog_descriptor(&input.cli_key, adapter) else {
            return Ok(ProviderModelDiscoveryResult::Unsupported {
                reason: ProviderModelDiscoveryUnsupportedReason::Oauth,
            });
        };
        let client = crate::gateway::http_client::get_no_redirect()?;
        let access_token = details.oauth_access_token.trim();
        if access_token.is_empty() {
            return Ok(discovery_error(
                ProviderModelDiscoveryErrorCode::InvalidConfig,
                None,
            ));
        }
        let local_version = if descriptor.client_version().is_some() {
            // Reuse the CLI manager's bounded resolver/--version process lifecycle.
            // The discovery deadline also includes time waiting for a blocking permit.
            match tokio::time::timeout_at(
                deadline,
                blocking::run("provider_models_discover_codex_version", move || {
                    crate::cli_manager::codex_discovery_version(&app, deadline.into_std())
                }),
            )
            .await
            {
                Ok(result) => result.ok().flatten(),
                Err(_) => {
                    return Ok(discovery_error(
                        ProviderModelDiscoveryErrorCode::Timeout,
                        None,
                    ))
                }
            }
        } else {
            None
        };
        let descriptor = match descriptor {
            ModelCatalogDescriptor::CodexOAuth { .. } => ModelCatalogDescriptor::CodexOAuth {
                client_version: codex_model_discovery_version(local_version.as_deref()),
            },
            other => other,
        };
        let mut headers = HeaderMap::new();
        if adapter
            .inject_model_discovery_headers(
                &mut headers,
                access_token,
                details.oauth_id_token.as_deref(),
                descriptor.client_version(),
            )
            .is_err()
        {
            return Ok(discovery_error(
                ProviderModelDiscoveryErrorCode::InvalidConfig,
                None,
            ));
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(discovery_error(
                ProviderModelDiscoveryErrorCode::Timeout,
                None,
            ));
        }
        return Ok(fetch_model_catalog_with_descriptor(
            descriptor,
            adapter.default_base_url(),
            None,
            headers,
            &client,
            remaining,
        )
        .await);
    }
    if catalog_format(&input.cli_key).is_none() {
        return Ok(discovery_error(
            ProviderModelDiscoveryErrorCode::InvalidConfig,
            None,
        ));
    }

    let base_urls = match providers::normalize_base_urls(input.base_urls) {
        Ok(base_urls) => base_urls,
        Err(error) => {
            if is_expected_provider_input_error(&error) {
                return Ok(discovery_error(
                    ProviderModelDiscoveryErrorCode::InvalidConfig,
                    None,
                ));
            }
            return Err(error.into());
        }
    };

    let submitted_key = input
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let api_key = if let Some(submitted_key) = submitted_key {
        submitted_key
    } else if let Some(provider_id) = input.provider_id {
        let db = ensure_db_ready(app.clone(), db_state).await?;
        let cli_key = input.cli_key.clone();
        let lookup = blocking::run("provider_models_discover_provider", move || {
            let conn = db.open_connection()?;
            let provider = providers::get_by_id(&conn, provider_id)?;
            if provider.cli_key != cli_key
                || provider.auth_mode != providers::ProviderAuthMode::ApiKey.as_str()
                || provider.source_provider_id.is_some()
                || provider.bridge_type.as_deref() == Some(providers::CX2CC_BRIDGE_TYPE)
            {
                return Err("SEC_INVALID_INPUT: provider connection mismatch".to_string());
            }
            drop(conn);
            Ok(providers::get_api_key_plaintext(&db, provider_id)?)
        })
        .await;
        match lookup {
            Ok(stored_key) => stored_key,
            Err(error) if is_expected_provider_input_error(&error) => {
                return Ok(discovery_error(
                    ProviderModelDiscoveryErrorCode::InvalidConfig,
                    None,
                ))
            }
            Err(error) => return Err(error.into()),
        }
    } else {
        return Ok(discovery_error(
            ProviderModelDiscoveryErrorCode::InvalidConfig,
            None,
        ));
    };

    if api_key.trim().is_empty() {
        return Ok(discovery_error(
            ProviderModelDiscoveryErrorCode::InvalidConfig,
            None,
        ));
    }

    let selected_base_url = match tokio::time::timeout_at(
        deadline,
        gateway::select_provider_base_url_for_discovery(&base_urls, input.base_url_mode),
    )
    .await
    {
        Ok(base_url) => base_url,
        Err(_) => {
            return Ok(discovery_error(
                ProviderModelDiscoveryErrorCode::Timeout,
                None,
            ))
        }
    };
    let base_url_index = base_urls
        .iter()
        .position(|value| value == &selected_base_url)
        .and_then(|index| u32::try_from(index + 1).ok());
    let client = crate::gateway::http_client::get_no_redirect()?;

    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        return Ok(discovery_error(
            ProviderModelDiscoveryErrorCode::Timeout,
            None,
        ));
    }

    Ok(fetch_model_catalog(
        &input.cli_key,
        &api_key,
        &selected_base_url,
        base_url_index,
        &client,
        remaining,
    )
    .await)
}

#[cfg(test)]
mod tests {
    use super::{
        classify_body_error, codex_model_discovery_version, fetch_model_catalog,
        fetch_model_catalog_with_descriptor, oauth_catalog_descriptor, parse_model_catalog,
        parse_model_catalog_with_format, provider_models_discover, DiscoveryParseError,
        ModelCatalogDescriptor, ModelCatalogFormat, ProviderModelDiscoveryErrorCode,
        ProviderModelDiscoveryInput, ProviderModelDiscoveryResult,
        ProviderModelDiscoveryUnsupportedReason,
    };
    use crate::app_state::DbInitState;
    use base64::Engine;
    use reqwest::header::HeaderMap;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn fixture_server(response: &str) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind fixture server");
        let address = listener.local_addr().expect("fixture server address");
        let response = response.to_string();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept fixture request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let read = stream
                    .read(&mut buffer)
                    .await
                    .expect("read fixture request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write fixture response");
            String::from_utf8_lossy(&request).into_owned()
        });
        (format!("http://{address}"), task)
    }

    fn api_key_input(
        provider_id: Option<i64>,
        base_url: String,
        api_key: Option<&str>,
    ) -> ProviderModelDiscoveryInput {
        ProviderModelDiscoveryInput {
            provider_id,
            cli_key: "codex".to_string(),
            auth_mode: crate::providers::ProviderAuthMode::ApiKey,
            base_urls: vec![base_url],
            base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
            api_key: api_key.map(str::to_string),
            source_provider_id: None,
            bridge_type: None,
        }
    }

    #[tokio::test]
    async fn fetches_each_api_key_descriptor_with_expected_path_and_auth() {
        let cases = [
            (
                "claude",
                "",
                "/v1/models",
                "x-api-key: secret",
                r#"{"data":[{"id":"claude-3"}]}"#,
            ),
            (
                "claude_desktop",
                "",
                "/v1/models",
                "x-api-key: secret",
                r#"{"data":[{"id":"claude-3"}]}"#,
            ),
            (
                "codex",
                "/v1",
                "/v1/models",
                "authorization: Bearer secret",
                r#"{"data":[{"id":"gpt-5.4"}]}"#,
            ),
            (
                "gemini",
                "/v1beta",
                "/v1beta/models",
                "x-goog-api-key: secret",
                r#"{"models":[{"name":"models/gemini-2.5-pro"}]}"#,
            ),
            (
                "grok",
                "/proxy/api",
                "/proxy/api/v1/models",
                "authorization: Bearer secret",
                r#"{"data":[{"id":"grok-3"}]}"#,
            ),
        ];

        for (cli_key, base_suffix, expected_path, expected_header, body) in cases {
            let (origin, request_task) = fixture_server(&format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            ))
            .await;
            let client = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("build fixture client");

            let result = fetch_model_catalog(
                cli_key,
                "secret",
                &format!("{origin}{base_suffix}"),
                Some(1),
                &client,
                Duration::from_secs(2),
            )
            .await;
            let request = request_task.await.expect("fixture request task");

            assert!(request.starts_with(&format!("GET {expected_path} HTTP/1.1")));
            assert!(request
                .to_ascii_lowercase()
                .contains(&expected_header.to_ascii_lowercase()));
            assert!(!request.contains("client_version="));
            assert!(matches!(result, ProviderModelDiscoveryResult::Ready { .. }));
        }
    }

    #[tokio::test]
    async fn maps_redirect_without_following_or_returning_location() {
        let (origin, request_task) = fixture_server(
            "HTTP/1.1 302 Found\r\nLocation: https://secret.example/next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build fixture client");

        let result = fetch_model_catalog(
            "codex",
            "secret",
            &origin,
            Some(1),
            &client,
            Duration::from_secs(2),
        )
        .await;
        let request = request_task.await.expect("fixture request task");

        assert!(request.starts_with("GET /v1/models HTTP/1.1"));
        assert!(matches!(
            result,
            ProviderModelDiscoveryResult::Error {
                code: ProviderModelDiscoveryErrorCode::Redirect,
                http_status: Some(302)
            }
        ));
    }

    #[test]
    fn serializes_cx2cc_unsupported_reason_to_generated_contract_value() {
        assert_eq!(
            serde_json::to_value(ProviderModelDiscoveryUnsupportedReason::Cx2cc)
                .expect("serialize CX2CC unsupported reason"),
            serde_json::json!("cx_2cc")
        );
    }

    #[test]
    fn classifies_invalid_utf8_as_invalid_response() {
        assert!(matches!(
            classify_body_error("provider model discovery body is not valid UTF-8"),
            ProviderModelDiscoveryErrorCode::InvalidResponse
        ));
    }

    #[tokio::test]
    async fn cx2cc_discovery_returns_unsupported_before_db_initialization() {
        let app = tauri::test::mock_app();
        let db_state = DbInitState(tokio::sync::Mutex::new(None));
        let result = provider_models_discover(
            app.handle().clone(),
            &db_state,
            ProviderModelDiscoveryInput {
                provider_id: None,
                cli_key: "claude".to_string(),
                auth_mode: crate::providers::ProviderAuthMode::ApiKey,
                base_urls: vec!["not-a-url".to_string()],
                base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
                api_key: None,
                source_provider_id: Some(7),
                bridge_type: Some("cx2cc".to_string()),
            },
        )
        .await
        .expect("CX2CC discovery should be a structured result");

        assert!(matches!(
            result,
            ProviderModelDiscoveryResult::Unsupported {
                reason: ProviderModelDiscoveryUnsupportedReason::Cx2cc
            }
        ));
    }

    #[tokio::test]
    async fn edit_with_empty_api_key_uses_stored_key_without_request_log() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("provider-model-discovery-stored-key.db");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let (origin, request_task) = fixture_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"data\":[{\"id\":\"gpt-5.4\"}]} ",
        )
        .await;
        let saved = crate::providers::upsert(
            &db,
            crate::providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: "stored-key-discovery".to_string(),
                base_urls: vec![origin.clone()],
                base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
                auth_mode: Some(crate::providers::ProviderAuthMode::ApiKey),
                api_key: Some("stored-secret".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(100),
                claude_models: None,
                model_policy: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: Some(crate::providers::DailyResetMode::Fixed),
                daily_reset_time: Some("00:00:00".to_string()),
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
                extension_values: None,
            },
        )
        .expect("save provider");
        let app = tauri::test::mock_app();
        let db_state = DbInitState(tokio::sync::Mutex::new(Some(Ok(db.clone()))));

        let result = provider_models_discover(
            app.handle().clone(),
            &db_state,
            api_key_input(Some(saved.id), origin, None),
        )
        .await
        .expect("stored key discovery should return a structured result");
        let request = request_task.await.expect("fixture request task");

        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer stored-secret"));
        assert!(matches!(result, ProviderModelDiscoveryResult::Ready { .. }));
        let conn = db.open_connection().expect("open db connection");
        let request_log_count: i64 = conn
            .query_row("SELECT COUNT(1) FROM request_logs", [], |row| row.get(0))
            .expect("count request logs");
        assert_eq!(request_log_count, 0);
    }

    #[tokio::test]
    async fn submitted_api_key_does_not_validate_saved_auth_mode() {
        let (origin, request_task) = fixture_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"data\":[{\"id\":\"gpt-5.4\"}]} ",
        )
        .await;
        let app = tauri::test::mock_app();
        let db_state = DbInitState(tokio::sync::Mutex::new(None));

        let result = provider_models_discover(
            app.handle().clone(),
            &db_state,
            api_key_input(Some(42), origin, Some("draft-secret")),
        )
        .await
        .expect("draft API key discovery should not read the saved provider");
        let request = request_task.await.expect("fixture request task");

        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer draft-secret"));
        assert!(matches!(result, ProviderModelDiscoveryResult::Ready { .. }));
    }

    #[tokio::test]
    async fn oauth_discovery_does_not_refresh_or_write_saved_tokens() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir
            .path()
            .join("provider-model-discovery-oauth-read-only.db");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let saved = crate::providers::upsert(
            &db,
            crate::providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: "oauth-read-only-discovery".to_string(),
                base_urls: vec!["https://example.com".to_string()],
                base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
                auth_mode: Some(crate::providers::ProviderAuthMode::ApiKey),
                api_key: Some("bootstrap-key".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(100),
                claude_models: None,
                model_policy: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: Some(crate::providers::DailyResetMode::Fixed),
                daily_reset_time: Some("00:00:00".to_string()),
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
                extension_values: None,
            },
        )
        .expect("save provider");
        crate::providers::update_oauth_tokens(
            &db,
            saved.id,
            crate::providers::ProviderAuthMode::Oauth.as_str(),
            "codex_oauth",
            "",
            Some("refresh-unchanged"),
            Some("id-unchanged"),
            "http://127.0.0.1:1/token",
            "client-id",
            Some("client-secret"),
            Some(0),
            Some("user@example.com"),
        )
        .expect("save OAuth tokens");
        let before = crate::providers::get_oauth_details(&db, saved.id).expect("OAuth details");
        let app = tauri::test::mock_app();
        let db_state = DbInitState(tokio::sync::Mutex::new(Some(Ok(db.clone()))));

        let result = provider_models_discover(
            app.handle().clone(),
            &db_state,
            ProviderModelDiscoveryInput {
                provider_id: Some(saved.id),
                cli_key: "codex".to_string(),
                auth_mode: crate::providers::ProviderAuthMode::Oauth,
                base_urls: Vec::new(),
                base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
                api_key: None,
                source_provider_id: None,
                bridge_type: None,
            },
        )
        .await
        .expect("OAuth discovery result");
        let after = crate::providers::get_oauth_details(&db, saved.id).expect("OAuth details");

        assert!(matches!(
            result,
            ProviderModelDiscoveryResult::Error {
                code: ProviderModelDiscoveryErrorCode::InvalidConfig,
                http_status: None,
            }
        ));
        assert_eq!(after.oauth_access_token, before.oauth_access_token);
        assert_eq!(after.oauth_refresh_token, before.oauth_refresh_token);
        assert_eq!(after.oauth_id_token, before.oauth_id_token);
        assert_eq!(after.oauth_expires_at, before.oauth_expires_at);
        assert_eq!(
            after.oauth_last_refreshed_at,
            before.oauth_last_refreshed_at
        );
    }

    #[tokio::test]
    async fn maps_auth_statuses_to_unauthorized_without_following_redirects() {
        for status in [401, 403] {
            let (origin, request_task) = fixture_server(&format!(
                "HTTP/1.1 {status} Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ))
            .await;
            let client = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("build fixture client");

            let result = fetch_model_catalog(
                "codex",
                "secret",
                &origin,
                Some(1),
                &client,
                Duration::from_secs(2),
            )
            .await;
            let _ = request_task.await.expect("fixture request task");

            assert!(matches!(
                result,
                ProviderModelDiscoveryResult::Error {
                    code: ProviderModelDiscoveryErrorCode::Unauthorized,
                    http_status: Some(value)
                } if value == status
            ));
        }
    }

    #[tokio::test]
    async fn maps_hanging_upstream_to_timeout() {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind hanging fixture server");
        let address = listener.local_addr().expect("hanging fixture address");
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("accept hanging request");
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build fixture client");

        let result = fetch_model_catalog(
            "codex",
            "secret",
            &format!("http://{address}"),
            Some(1),
            &client,
            Duration::from_millis(20),
        )
        .await;
        server.abort();
        let _ = server.await;

        assert!(matches!(
            result,
            ProviderModelDiscoveryResult::Error {
                code: ProviderModelDiscoveryErrorCode::Timeout,
                http_status: None
            }
        ));
    }

    #[tokio::test]
    async fn rejects_catalog_before_reading_an_oversized_content_length() {
        let (origin, request_task) = fixture_server(
            "HTTP/1.1 200 OK\r\nContent-Length: 8388609\r\nConnection: close\r\n\r\n",
        )
        .await;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build fixture client");

        let result = fetch_model_catalog(
            "codex",
            "secret",
            &origin,
            Some(1),
            &client,
            Duration::from_secs(2),
        )
        .await;
        let _ = request_task.await.expect("fixture request task");

        assert!(matches!(
            result,
            ProviderModelDiscoveryResult::Error {
                code: ProviderModelDiscoveryErrorCode::TooLarge,
                http_status: None
            }
        ));
    }

    #[test]
    fn parses_openai_compatible_catalog_with_stable_ids() {
        let body = r#"{
          "data": [{"id":" gpt-5.4 "}, {"id":"gpt-5.4"}, {"id":"claude-3"}],
          "object":"list"
        }"#;

        let models = parse_model_catalog("codex", body).expect("catalog should parse");

        assert_eq!(models, vec!["claude-3", "gpt-5.4"]);
    }

    #[test]
    fn rejects_invalid_item_instead_of_returning_partial_models() {
        let body = r#"{"data":[{"id":"gpt-5.4"},{"id":42}]}"#;

        assert_eq!(
            parse_model_catalog("grok", body),
            Err(DiscoveryParseError::InvalidResponse)
        );
    }

    #[test]
    fn rejects_bare_array_for_descriptor_specific_catalog() {
        assert_eq!(
            parse_model_catalog("codex", r#"[{"id":"gpt-5.4"}]"#),
            Err(DiscoveryParseError::InvalidResponse)
        );
    }

    #[test]
    fn parses_gemini_names_and_strips_resource_prefix() {
        let body =
            r#"{"models":[{"name":"models/gemini-2.5-pro"},{"name":"models/gemini-2.5-flash"}]}"#;

        let models = parse_model_catalog("gemini", body).expect("catalog should parse");

        assert_eq!(models, vec!["gemini-2.5-flash", "gemini-2.5-pro"]);
    }

    #[test]
    fn parses_grok_oauth_ids_by_protocol_priority() {
        let body = r#"{
          "data": [
            {"model":"grok-primary","modelId":"grok-secondary","id":"grok-id","name":"Display"},
            {"_meta":{"model_id":"grok-meta"},"name":"Meta display"}
          ]
        }"#;

        let models = parse_model_catalog_with_format(ModelCatalogFormat::GrokOAuth, body)
            .expect("Grok OAuth catalog should parse");

        assert_eq!(models, vec!["grok-meta", "grok-primary"]);
    }

    #[test]
    fn saved_oauth_capability_matrix_only_enables_codex_and_grok() {
        for (cli_key, provider_type, supported) in [
            ("claude", "claude_oauth", false),
            ("codex", "codex_oauth", true),
            ("gemini", "gemini_oauth", false),
            ("grok", "grok_oauth", true),
        ] {
            let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter(
                cli_key,
                1,
                Some(provider_type),
            )
            .expect("registered OAuth adapter");
            assert_eq!(
                oauth_catalog_descriptor(cli_key, adapter).is_some(),
                supported,
                "unexpected discovery capability for {cli_key}"
            );
        }
    }

    async fn fetch_codex_oauth_fixture(
        body: &str,
        local_version: Option<&str>,
    ) -> (ProviderModelDiscoveryResult, String, String) {
        let (origin, request_task) = fixture_server(&format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ))
        .await;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build fixture client");
        let adapter =
            crate::gateway::oauth::registry::resolve_oauth_adapter("codex", 1, Some("codex_oauth"))
                .expect("Codex OAuth adapter");
        let descriptor = oauth_catalog_descriptor("codex", adapter).expect("OAuth descriptor");
        let descriptor = match descriptor {
            ModelCatalogDescriptor::CodexOAuth { .. } => ModelCatalogDescriptor::CodexOAuth {
                client_version: codex_model_discovery_version(local_version),
            },
            other => panic!("expected Codex descriptor, got {other:?}"),
        };
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"https://api.openai.com/auth":{"chatgpt_account_id":"fixture-account"}}"#);
        let id_token = format!("header.{payload}.signature");
        let mut headers = HeaderMap::new();
        adapter
            .inject_model_discovery_headers(
                &mut headers,
                "oauth-secret",
                Some(&id_token),
                descriptor.client_version(),
            )
            .expect("inject real adapter discovery headers");
        let backend_path = reqwest::Url::parse(adapter.default_base_url())
            .expect("adapter base URL")
            .path()
            .to_string();

        let result = fetch_model_catalog_with_descriptor(
            descriptor,
            &format!("{origin}{backend_path}"),
            None,
            headers,
            &client,
            Duration::from_secs(2),
        )
        .await;
        let request = request_task.await.expect("fixture request task");
        (result, request, origin)
    }

    #[test]
    fn codex_oauth_discovery_selects_local_version_without_a_minimum_clamp() {
        for (raw, expected) in [
            ("codex-cli 0.144.4", "0.144.4"),
            ("  codex-cli 0.150.2  ", "0.150.2"),
            ("codex-cli 0.137.0", "0.137.0"),
            ("0.150.2", "0.150.2"),
            ("codex-cli v1.2.3-alpha.1+build.7", "1.2.3-alpha.1+build.7"),
        ] {
            assert_eq!(codex_model_discovery_version(Some(raw)), expected);
        }
        for raw in [
            None,
            Some(""),
            Some("unknown"),
            Some("codex-cli nope"),
            Some("codex-cli 0.144"),
            Some("codex-cli 0.144.4 garbage"),
            Some("codex-cli 0.144.4\r\nInjected: header"),
            Some("1.2.3-"),
            Some("1.2.3.4"),
            Some("1.2.3+"),
            Some("1.2.3-alpha..1"),
        ] {
            assert_eq!(codex_model_discovery_version(raw), "0.144.4", "{raw:?}");
        }
    }

    #[tokio::test]
    async fn codex_oauth_discovery_keeps_dynamic_query_and_identity_versions_coherent() {
        for (raw, expected) in [
            (Some("codex-cli 0.150.2"), "0.150.2"),
            (Some("codex-cli 0.137.0"), "0.137.0"),
            (
                Some("codex-cli 1.2.3-alpha.1+build.7"),
                "1.2.3-alpha.1+build.7",
            ),
            (Some("malformed version"), "0.144.4"),
            (None, "0.144.4"),
        ] {
            let (result, request, origin) =
                fetch_codex_oauth_fixture(r#"{"models":[]}"#, raw).await;
            assert!(matches!(result, ProviderModelDiscoveryResult::Empty { .. }));
            let target = request.split_whitespace().nth(1).expect("request target");
            let url = reqwest::Url::parse(&format!("{origin}{target}")).expect("request URL");
            assert_eq!(url.path(), "/backend-api/codex/models");
            assert_eq!(
                url.query_pairs().collect::<Vec<_>>(),
                vec![("client_version".into(), expected.into())]
            );
            assert!(request.contains(&format!("\r\nuser-agent: codex_cli_rs/{expected}\r\n")));
            assert!(request.contains(&format!("\r\nversion: {expected}\r\n")));
        }
    }

    #[tokio::test]
    async fn codex_oauth_discovery_sends_versioned_backend_path_and_adapter_identity() {
        let (_, request, _) = fetch_codex_oauth_fixture(r#"{"models":[]}"#, None).await;
        assert!(
            request
                .starts_with("GET /backend-api/codex/models?client_version=0.144.4 HTTP/1.1\r\n"),
            "unexpected request: {request}"
        );
        let request = request.to_ascii_lowercase();
        for header in [
            "authorization: bearer oauth-secret",
            "chatgpt-account-id: fixture-account",
            "originator: codex_cli_rs",
            "user-agent: codex_cli_rs/0.144.4",
            "version: 0.144.4",
        ] {
            assert!(
                request.contains(&format!("\r\n{header}\r\n")),
                "missing {header}"
            );
        }
    }

    #[tokio::test]
    async fn codex_oauth_discovery_preserves_unknown_slugs_sorted_and_deduplicated() {
        let (result, _, expected_origin) = fetch_codex_oauth_fixture(
            r#"{"models":[{"slug":" z-future-model ","display_name":"Ignored"},{"slug":"models/astra"},{"slug":"astra","capabilities":{"unknown":true}},{"slug":"z-future-model"}]}"#,
            None,
        )
        .await;
        match result {
            ProviderModelDiscoveryResult::Ready {
                models,
                origin,
                base_url_index,
            } => {
                assert_eq!(models, vec!["astra", "models/astra", "z-future-model"]);
                assert_eq!(origin, expected_origin);
                assert_eq!(base_url_index, None);
            }
            other => panic!("expected ready manifest, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn codex_oauth_discovery_returns_empty_manifest() {
        let (result, _, expected_origin) =
            fetch_codex_oauth_fixture(r#"{"models":[]}"#, None).await;
        assert!(
            matches!(result, ProviderModelDiscoveryResult::Empty { origin, base_url_index: None }
            if origin == expected_origin)
        );
    }

    #[tokio::test]
    async fn codex_oauth_discovery_rejects_malformed_or_paginated_manifests() {
        for body in [
            "not json",
            r#"{}"#,
            r#"{"data":[{"id":"legacy-id"}]}"#,
            r#"{"models":{}}"#,
            r#"{"models":[{"slug":"valid"},null]}"#,
            r#"{"models":[{"slug":"valid"},{"id":"not-a-slug"}]}"#,
            r#"{"models":[{"slug":"valid"},{"slug":42}]}"#,
            r#"{"models":[{"slug":"valid"},{"slug":" "}]}"#,
            r#"{"models":[{"slug":"valid"},{"slug":"wild*"}]}"#,
            r#"{"models":[{"slug":"valid"}],"has_more":true}"#,
            r#"{"models":[],"next":"page"}"#,
            r#"{"models":[],"next_page_token":"page"}"#,
            r#"{"models":[],"nextPageToken":"page"}"#,
            r#"{"models":[],"next_cursor":"page"}"#,
            r#"{"models":[],"nextCursor":"page"}"#,
        ] {
            let (result, _, _) = fetch_codex_oauth_fixture(body, None).await;
            assert!(
                matches!(
                    result,
                    ProviderModelDiscoveryResult::Error {
                        code: ProviderModelDiscoveryErrorCode::InvalidResponse,
                        http_status: None,
                    }
                ),
                "expected invalid response for {body}, got {result:?}"
            );
        }
    }

    #[test]
    fn rejects_pagination_that_would_make_catalog_incomplete() {
        let body = r#"{"data":[{"id":"gpt-5.4"}],"has_more":true}"#;

        assert_eq!(
            parse_model_catalog("claude", body),
            Err(DiscoveryParseError::InvalidResponse)
        );
    }

    #[test]
    fn accepts_empty_and_large_catalogs() {
        assert_eq!(
            parse_model_catalog("codex", r#"{"data":[]}"#),
            Ok(Vec::new())
        );

        let body = serde_json::json!({
            "data": (0..501)
                .map(|index| serde_json::json!({ "id": format!("model-{index}") }))
                .collect::<Vec<_>>()
        })
        .to_string();

        let models = parse_model_catalog("codex", &body).expect("large catalog should parse");
        assert_eq!(models.len(), 501);
    }
}
