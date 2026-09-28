//! Usage: Pure transport layer for the image generation page: path allowlist,
//! scheme checks, private-host rejection, and body-size caps. No provider semantics.

use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::net::IpAddr;
use std::time::Duration;

const ALLOWED_IMAGE_GEN_PATHS: &[&str] = &["/v1/images/generations", "/v1/images/edits"];
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAX_MULTIPART_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 600;
const MAX_TIMEOUT_SECS: u64 = 900;
const MAX_ERROR_EXCERPT_CHARS: usize = 512;

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageGenHttpResponse {
    pub status: u16,
    pub body_text: String,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageGenFetchedImage {
    pub mime: String,
    pub data_b64: String,
}

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageGenMultipartFile {
    pub field: String,
    pub filename: String,
    pub mime: String,
    pub data_b64: String,
}

#[derive(Debug)]
pub(super) struct DecodedMultipartFile {
    pub field: String,
    pub filename: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

pub(super) fn resolve_timeout(timeout_secs: Option<u32>) -> Duration {
    let secs = timeout_secs
        .map(u64::from)
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .clamp(1, MAX_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

pub(super) fn validate_request_path(path: &str) -> Result<&str, String> {
    if ALLOWED_IMAGE_GEN_PATHS.contains(&path) {
        Ok(path)
    } else {
        Err(format!("SEC_INVALID_INPUT: path is not allowed: {path}"))
    }
}

/// Joins the stored base_url with an allowlisted path. Scheme must be https;
/// http is only allowed for 127.0.0.1 / localhost (local gateway debugging).
/// A base_url that already ends with `/v1` is deduplicated against the `/v1`
/// path prefix so `https://host/v1` + `/v1/images/generations` does not double up.
pub(super) fn build_request_url(base_url: &str, path: &str) -> Result<reqwest::Url, String> {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        return Err("SEC_INVALID_INPUT: image gen base_url is not configured".to_string());
    }

    let parsed = reqwest::Url::parse(base_url)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid base_url: {e}"))?;
    match parsed.scheme() {
        "https" => {}
        "http" => {
            let host = parsed.host_str().unwrap_or("");
            if host != "127.0.0.1" && host != "localhost" {
                return Err(
                    "SEC_INVALID_INPUT: base_url must use https (http is only allowed for 127.0.0.1/localhost)"
                        .to_string(),
                );
            }
        }
        other => {
            return Err(format!(
                "SEC_INVALID_INPUT: unsupported base_url scheme: {other}"
            ));
        }
    }

    let joined = if base_url.ends_with("/v1") {
        format!("{base_url}{}", path.trim_start_matches("/v1"))
    } else {
        format!("{base_url}{path}")
    };
    reqwest::Url::parse(&joined).map_err(|e| format!("SEC_INVALID_INPUT: invalid request url: {e}"))
}

fn auth_headers(api_key: &str) -> Result<HeaderMap, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("SEC_INVALID_INPUT: image gen api_key is not configured".to_string());
    }
    let mut value = HeaderValue::from_str(&format!("Bearer {api_key}"))
        .map_err(|_| "SEC_INVALID_INPUT: api_key contains invalid characters".to_string())?;
    value.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, value);
    Ok(headers)
}

fn truncate_excerpt(text: &str) -> String {
    text.chars().take(MAX_ERROR_EXCERPT_CHARS).collect()
}

async fn read_body_capped(response: &mut reqwest::Response) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if buf.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                    return Err(format!(
                        "HTTP_ERROR: response body exceeds {MAX_RESPONSE_BYTES} bytes limit"
                    ));
                }
                buf.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(e) => return Err(format!("HTTP_ERROR: failed to read response body: {e}")),
        }
    }
    Ok(buf)
}

async fn read_http_response(
    mut response: reqwest::Response,
) -> Result<ImageGenHttpResponse, String> {
    let status = response.status().as_u16();
    let body = read_body_capped(&mut response).await?;
    Ok(ImageGenHttpResponse {
        status,
        body_text: String::from_utf8_lossy(&body).into_owned(),
    })
}

pub(crate) async fn post_json(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    path: &str,
    body: &serde_json::Value,
    timeout_secs: Option<u32>,
) -> Result<ImageGenHttpResponse, String> {
    let path = validate_request_path(path)?;
    let url = build_request_url(base_url, path)?;
    let headers = auth_headers(api_key)?;
    let body_bytes = serde_json::to_vec(body)
        .map_err(|e| format!("SYSTEM_ERROR: failed to encode body JSON: {e}"))?;

    let response = client
        .post(url)
        .headers(headers)
        .header(CONTENT_TYPE, "application/json")
        .body(body_bytes)
        .timeout(resolve_timeout(timeout_secs))
        .send()
        .await
        .map_err(|e| format!("HTTP_ERROR: {e}"))?;

    read_http_response(response).await
}

pub(super) fn decode_multipart_files(
    files: &[ImageGenMultipartFile],
) -> Result<Vec<DecodedMultipartFile>, String> {
    let mut decoded = Vec::with_capacity(files.len());
    let mut total_bytes = 0usize;
    for (index, file) in files.iter().enumerate() {
        if file.field.trim().is_empty() {
            return Err(format!(
                "SEC_INVALID_INPUT: file #{index} field is required"
            ));
        }
        if file.filename.trim().is_empty() {
            return Err(format!(
                "SEC_INVALID_INPUT: file #{index} filename is required"
            ));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(file.data_b64.as_bytes())
            .map_err(|e| format!("SEC_INVALID_INPUT: file #{index} data_b64 is invalid: {e}"))?;
        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > MAX_MULTIPART_TOTAL_BYTES {
            return Err(format!(
                "SEC_INVALID_INPUT: multipart files exceed {MAX_MULTIPART_TOTAL_BYTES} bytes limit"
            ));
        }
        decoded.push(DecodedMultipartFile {
            field: file.field.clone(),
            filename: file.filename.clone(),
            mime: file.mime.clone(),
            bytes,
        });
    }
    Ok(decoded)
}

fn build_multipart_form(
    fields: &[(String, String)],
    files: Vec<DecodedMultipartFile>,
) -> Result<reqwest::multipart::Form, String> {
    let mut form = reqwest::multipart::Form::new();
    for (name, value) in fields {
        form = form.text(name.clone(), value.clone());
    }
    for file in files {
        let part = reqwest::multipart::Part::bytes(file.bytes)
            .file_name(file.filename)
            .mime_str(&file.mime)
            .map_err(|e| format!("SEC_INVALID_INPUT: invalid mime type {}: {e}", file.mime))?;
        form = form.part(file.field, part);
    }
    Ok(form)
}

pub(crate) async fn post_multipart(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    path: &str,
    fields: &[(String, String)],
    files: &[ImageGenMultipartFile],
    timeout_secs: Option<u32>,
) -> Result<ImageGenHttpResponse, String> {
    let path = validate_request_path(path)?;
    let url = build_request_url(base_url, path)?;
    let headers = auth_headers(api_key)?;
    let form = build_multipart_form(fields, decode_multipart_files(files)?)?;

    let response = client
        .post(url)
        .headers(headers)
        .multipart(form)
        .timeout(resolve_timeout(timeout_secs))
        .send()
        .await
        .map_err(|e| format!("HTTP_ERROR: {e}"))?;

    read_http_response(response).await
}

pub(super) fn is_disallowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_disallowed_ip(IpAddr::V4(mapped));
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique local fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link local fe80::/10
        }
    }
}

fn bare_host(url: &reqwest::Url) -> Result<&str, String> {
    let host = url
        .host_str()
        .ok_or_else(|| "SEC_INVALID_INPUT: image url is missing a host".to_string())?;
    Ok(host.trim_start_matches('[').trim_end_matches(']'))
}

/// Static checks only (scheme + IP-literal hosts). Hostname DNS resolution is
/// covered by `ensure_public_host`.
pub(super) fn validate_fetch_image_url(url: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(url.trim())
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid image url: {e}"))?;
    if url.scheme() != "https" {
        return Err("SEC_INVALID_INPUT: image url must use https".to_string());
    }
    let host = bare_host(&url)?;
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_disallowed_ip(ip) {
            return Err(format!(
                "SEC_INVALID_INPUT: image url host is not allowed: {host}"
            ));
        }
    }
    Ok(url)
}

async fn ensure_public_host(url: &reqwest::Url) -> Result<(), String> {
    let host = bare_host(url)?;
    if host.parse::<IpAddr>().is_ok() {
        // IP literals are already validated in validate_fetch_image_url.
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("HTTP_ERROR: failed to resolve image host {host}: {e}"))?;
    let mut resolved_any = false;
    for addr in addrs {
        resolved_any = true;
        if is_disallowed_ip(addr.ip()) {
            return Err(format!(
                "SEC_INVALID_INPUT: image url host resolves to a private address: {host}"
            ));
        }
    }
    if !resolved_any {
        return Err(format!("HTTP_ERROR: image host did not resolve: {host}"));
    }
    Ok(())
}

pub(super) fn is_image_content_type(content_type: &str) -> bool {
    content_type
        .trim()
        .to_ascii_lowercase()
        .starts_with("image/")
}

pub(crate) async fn fetch_image(
    client: &reqwest::Client,
    url: &str,
    timeout_secs: Option<u32>,
) -> Result<ImageGenFetchedImage, String> {
    let url = validate_fetch_image_url(url)?;
    ensure_public_host(&url).await?;

    let mut response = client
        .get(url)
        .timeout(resolve_timeout(timeout_secs))
        .send()
        .await
        .map_err(|e| format!("HTTP_ERROR: {e}"))?;

    // ponytail: redirects are followed by the shared client, so a redirect to a
    // private host is contacted before this final-URL re-check rejects the
    // response; upgrade path is a dedicated no-redirect client if this matters.
    validate_fetch_image_url(response.url().as_str())?;
    ensure_public_host(response.url()).await?;

    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        let body = read_body_capped(&mut response).await.unwrap_or_default();
        return Err(format!(
            "HTTP_ERROR: image download failed with status {status}: {}",
            truncate_excerpt(&String::from_utf8_lossy(&body))
        ));
    }

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !is_image_content_type(&content_type) {
        return Err(format!(
            "HTTP_ERROR: image download returned a non-image content type: {}",
            truncate_excerpt(&content_type)
        ));
    }

    let body = read_body_capped(&mut response).await?;
    Ok(ImageGenFetchedImage {
        mime: content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_string(),
        data_b64: base64::engine::general_purpose::STANDARD.encode(&body),
    })
}
