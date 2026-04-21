//! Global HTTP client module for gateway upstream requests.
//!
//! Provides a shared HTTP client with optional proxy support.
//! The client can be hot-reloaded at runtime when proxy settings change.

use crate::settings::{self, AppSettings, GatewayListenMode};
use crate::{gateway::listen, wsl};
use if_addrs::get_if_addrs;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::{Client, StatusCode, Url};
use std::collections::BTreeSet;
use std::env;
use std::error::Error as StdError;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

/// Global HTTP client instance.
static GLOBAL_CLIENT: OnceLock<RwLock<Client>> = OnceLock::new();

/// Current proxy URL (for logging and status queries).
static CURRENT_PROXY_URL: OnceLock<RwLock<Option<String>>> = OnceLock::new();

/// Current gateway self-loop context.
static GATEWAY_SELF_CONTEXT: OnceLock<RwLock<GatewaySelfCheckContext>> = OnceLock::new();

/// Default connection timeout for upstream requests.
const UPSTREAM_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const PROXY_TEST_TIMEOUT: Duration = Duration::from_secs(8);
const PROXY_TEST_URL: &str = "https://ifconfig.me/";
const PROXY_EXIT_IP_URL: &str = "https://ifconfig.me/ip";

#[cfg(test)]
static TEST_PROXY_TEST_URL_OVERRIDE: OnceLock<RwLock<Option<String>>> = OnceLock::new();
#[cfg(test)]
static TEST_PROXY_EXIT_IP_URL_OVERRIDE: OnceLock<RwLock<Option<String>>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GatewaySelfCheckContext {
    gateway_port: u16,
    hosts: BTreeSet<String>,
}

#[derive(Clone, Default)]
struct Ipv4FirstResolver;

impl Resolve for Ipv4FirstResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();

        Box::pin(async move {
            let mut addrs: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            sort_socket_addrs_ipv4_first(&mut addrs);
            let addrs: Addrs = Box::new(addrs.into_iter());
            Ok(addrs)
        })
    }
}

fn sort_socket_addrs_ipv4_first(addrs: &mut [SocketAddr]) {
    addrs.sort_by_key(|addr| if addr.is_ipv4() { 0 } else { 1 });
}

/// Initialize the global HTTP client.
///
/// Should be called once at application startup.
///
/// # Arguments
/// * `proxy_url` - Proxy URL like `http://127.0.0.1:7890` or `socks5://127.0.0.1:1080`.
///   Pass None or empty string for direct connection.
pub fn init(proxy_url: Option<&str>) -> Result<(), String> {
    let effective_url = proxy_url.filter(|s| !s.trim().is_empty());
    let client = build_client(effective_url)?;

    if GLOBAL_CLIENT.set(RwLock::new(client.clone())).is_err() {
        tracing::warn!(
            "[HttpClient] Already initialized, updating instead: {}",
            effective_url
                .map(mask_url)
                .unwrap_or_else(|| "direct connection".to_string())
        );
        return apply_proxy(proxy_url);
    }

    let _ = CURRENT_PROXY_URL.set(RwLock::new(effective_url.map(mask_url)));

    tracing::info!(
        "[HttpClient] Initialized: {}",
        effective_url
            .map(mask_url)
            .unwrap_or_else(|| "direct connection".to_string())
    );

    Ok(())
}

pub(crate) fn sync_runtime_context(port: u16, bind_host: &str, base_host: &str) {
    set_gateway_self_context(build_self_check_context(
        port,
        &[bind_host.to_string(), base_host.to_string()],
    ));
}

pub(crate) fn sync_from_settings(settings: &AppSettings) -> Result<(), String> {
    let context = self_check_context_from_settings(settings).map_err(|err| err.to_string())?;
    set_gateway_self_context(context);
    let proxy_url = effective_proxy_url(settings)?;
    apply_proxy(proxy_url.as_deref())
}

pub(crate) fn self_check_context_from_settings(
    settings: &AppSettings,
) -> crate::shared::error::AppResult<GatewaySelfCheckContext> {
    let preferred_port = settings
        .preferred_port
        .max(settings::DEFAULT_GATEWAY_PORT)
        .max(1024);

    let (bind_host, fixed_port) = match settings.gateway_listen_mode {
        GatewayListenMode::Localhost => ("127.0.0.1".to_string(), None),
        GatewayListenMode::Lan => ("0.0.0.0".to_string(), None),
        GatewayListenMode::WslAuto => (wsl::resolve_wsl_host(settings), None),
        GatewayListenMode::Custom => {
            let parsed =
                listen::parse_custom_listen_address(&settings.gateway_custom_listen_address)?;
            (parsed.host, parsed.port)
        }
    };

    let port = fixed_port.unwrap_or(preferred_port);
    let base_host = match settings.gateway_listen_mode {
        GatewayListenMode::Lan => "127.0.0.1".to_string(),
        GatewayListenMode::Custom if listen::is_wildcard_host(&bind_host) => {
            "127.0.0.1".to_string()
        }
        _ => bind_host.clone(),
    };

    Ok(build_self_check_context(port, &[bind_host, base_host]))
}

pub(crate) fn validate_proxy_for_settings(settings: &AppSettings) -> Result<(), String> {
    let context = self_check_context_from_settings(settings).map_err(|err| err.to_string())?;
    let proxy_url = effective_proxy_url(settings)?;
    validate_proxy_with_context(proxy_url.as_deref(), &context)
}

pub(crate) fn build_effective_proxy_url(
    proxy_url: Option<&str>,
    proxy_username: Option<&str>,
    proxy_password: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(raw_url) = proxy_url.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    let username = proxy_username
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let password = proxy_password.unwrap_or_default();
    let has_separate_credentials = username.is_some() || !password.is_empty();

    if !password.is_empty() && username.is_none() {
        return Err(
            "upstream_proxy_username cannot be empty when upstream_proxy_password is set"
                .to_string(),
        );
    }

    validate_proxy_url(raw_url)?;

    let mut parsed = Url::parse(raw_url)
        .map_err(|err| format!("Invalid proxy URL '{}': {err}", mask_url(raw_url)))?;
    let url_has_credentials = !parsed.username().is_empty() || parsed.password().is_some();

    if url_has_credentials && has_separate_credentials {
        return Err(
            "Proxy credentials must be specified either in proxy URL or username/password fields, not both"
                .to_string(),
        );
    }

    if !url_has_credentials {
        if let Some(username) = username {
            parsed
                .set_username(username)
                .map_err(|_| "Invalid upstream proxy username".to_string())?;
            parsed
                .set_password(Some(password))
                .map_err(|_| "Invalid upstream proxy password".to_string())?;
        }
    }

    Ok(Some(parsed.to_string()))
}

pub(crate) fn validate_proxy_with_context(
    proxy_url: Option<&str>,
    context: &GatewaySelfCheckContext,
) -> Result<(), String> {
    let effective_url = proxy_url.filter(|s| !s.trim().is_empty());

    if let Some(url) = effective_url {
        if proxy_points_to_gateway_with_context(url, context) {
            return Err("Proxy URL points to the gateway itself (self-loop detected)".to_string());
        }
        validate_proxy_url(url)?;
    }

    Ok(())
}

pub(crate) async fn test_proxy_with_context(
    proxy_url: Option<&str>,
    context: &GatewaySelfCheckContext,
) -> Result<(), String> {
    let effective_url = proxy_url
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Proxy URL cannot be empty".to_string())?;

    validate_proxy_with_context(Some(effective_url), context)?;

    let client = build_client(Some(effective_url))?;
    let test_url = proxy_test_url();
    let response = client
        .get(&test_url)
        .timeout(PROXY_TEST_TIMEOUT)
        .send()
        .await;
    let response = match response {
        Ok(response) => response,
        Err(err) => {
            return Err(format_proxy_request_error(
                "Proxy connectivity test failed",
                effective_url,
                &test_url,
                &err,
            )
            .await)
        }
    };

    if response.status() == StatusCode::PROXY_AUTHENTICATION_REQUIRED {
        return Err(format!(
            "Proxy authentication failed for '{}': HTTP 407",
            mask_url(effective_url)
        ));
    }

    tracing::info!(
        status = %response.status(),
        proxy = %mask_url(effective_url),
        "proxy connectivity test completed"
    );

    Ok(())
}

pub(crate) async fn detect_proxy_exit_ip_with_context(
    proxy_url: Option<&str>,
    context: &GatewaySelfCheckContext,
) -> Result<String, String> {
    let effective_url = proxy_url
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Proxy URL cannot be empty".to_string())?;

    validate_proxy_with_context(Some(effective_url), context)?;

    let client = build_client(Some(effective_url))?;
    let detect_url = proxy_exit_ip_url();
    let response = client
        .get(&detect_url)
        .timeout(PROXY_TEST_TIMEOUT)
        .send()
        .await;
    let response = match response {
        Ok(response) => response,
        Err(err) => {
            return Err(format_proxy_request_error(
                "Proxy exit IP detection failed",
                effective_url,
                &detect_url,
                &err,
            )
            .await)
        }
    };

    if response.status() == StatusCode::PROXY_AUTHENTICATION_REQUIRED {
        return Err(format!(
            "Proxy authentication failed for '{}': HTTP 407",
            mask_url(effective_url)
        ));
    }

    if !response.status().is_success() {
        return Err(format!(
            "Proxy exit IP detection failed for '{}': HTTP {}",
            mask_url(effective_url),
            response.status()
        ));
    }

    let body = response.text().await.map_err(|err| {
        format!(
            "Proxy exit IP detection failed for '{}': failed to read probe response: {}",
            mask_url(effective_url),
            format_error_chain(&err)
        )
    })?;

    parse_exit_ip_response(&body).ok_or_else(|| {
        format!(
            "Proxy exit IP detection failed for '{}': probe response is not a valid IP address ({})",
            mask_url(effective_url),
            summarize_probe_body(&body)
        )
    })
}

fn format_error_chain(err: &(dyn StdError + 'static)) -> String {
    let mut parts = vec![err.to_string()];
    let mut current = err.source();

    while let Some(source) = current {
        parts.push(source.to_string());
        current = source.source();
    }

    parts.join(" | caused by: ")
}

fn proxy_uses_socks5_local_dns(proxy_url: &str) -> bool {
    Url::parse(proxy_url)
        .ok()
        .is_some_and(|parsed| parsed.scheme() == "socks5")
}

fn proxy_test_url() -> String {
    #[cfg(test)]
    if let Some(lock) = TEST_PROXY_TEST_URL_OVERRIDE.get() {
        if let Ok(url) = lock.read() {
            if let Some(override_url) = url.clone() {
                return override_url;
            }
        }
    }

    PROXY_TEST_URL.to_string()
}

fn proxy_exit_ip_url() -> String {
    #[cfg(test)]
    if let Some(lock) = TEST_PROXY_EXIT_IP_URL_OVERRIDE.get() {
        if let Ok(url) = lock.read() {
            if let Some(override_url) = url.clone() {
                return override_url;
            }
        }
    }

    PROXY_EXIT_IP_URL.to_string()
}

#[cfg(test)]
fn set_proxy_test_url_for_tests(url: Option<&str>) {
    let lock = TEST_PROXY_TEST_URL_OVERRIDE.get_or_init(|| RwLock::new(None));
    let mut current = lock.write().expect("proxy test url override lock");
    *current = url.map(str::to_string);
}

#[cfg(test)]
fn set_proxy_exit_ip_url_for_tests(url: Option<&str>) {
    let lock = TEST_PROXY_EXIT_IP_URL_OVERRIDE.get_or_init(|| RwLock::new(None));
    let mut current = lock.write().expect("proxy exit ip url override lock");
    *current = url.map(str::to_string);
}

fn proxy_test_url_has_hostname(test_url: &str) -> bool {
    Url::parse(test_url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .and_then(|host| normalize_host_token(&host))
        .is_some_and(|host| host.parse::<IpAddr>().is_err())
}

fn to_socks5h_url(proxy_url: &str) -> Option<String> {
    let mut parsed = Url::parse(proxy_url).ok()?;
    if parsed.scheme() != "socks5" {
        return None;
    }
    parsed.set_scheme("socks5h").ok()?;
    Some(parsed.to_string())
}

async fn probe_socks5h_fallback(proxy_url: &str, test_url: &str) -> bool {
    let Some(fallback_proxy_url) = to_socks5h_url(proxy_url) else {
        return false;
    };

    let Ok(client) = build_client(Some(&fallback_proxy_url)) else {
        return false;
    };

    client
        .get(test_url)
        .timeout(PROXY_TEST_TIMEOUT)
        .send()
        .await
        .is_ok()
}

fn is_socks5_handshake_eof(err: &reqwest::Error) -> bool {
    format_error_chain(err).contains("SOCKS error: io error during SOCKS handshake")
}

fn build_socks5_local_dns_hint(proxy_url: &str, test_url: &str) -> String {
    let suggested = to_socks5h_url(proxy_url).unwrap_or_else(|| proxy_url.to_string());
    format!(
        "Local-DNS SOCKS5 handshake failed for hostname test target '{}'. This proxy works with proxy-side DNS on the current reqwest stack. Try '{}' instead.",
        test_url,
        suggested
    )
}

fn default_proxy_request_error(operation: &str, proxy_url: &str, err: &reqwest::Error) -> String {
    format!(
        "{} for '{}': {}",
        operation,
        mask_url(proxy_url),
        format_error_chain(err)
    )
}

async fn format_proxy_request_error(
    operation: &str,
    proxy_url: &str,
    test_url: &str,
    err: &reqwest::Error,
) -> String {
    if proxy_uses_socks5_local_dns(proxy_url)
        && proxy_test_url_has_hostname(test_url)
        && is_socks5_handshake_eof(err)
        && probe_socks5h_fallback(proxy_url, test_url).await
    {
        return format!(
            "{} {}",
            default_proxy_request_error(operation, proxy_url, err),
            build_socks5_local_dns_hint(proxy_url, test_url)
        );
    }

    default_proxy_request_error(operation, proxy_url, err)
}

fn parse_exit_ip_response(body: &str) -> Option<String> {
    let candidate = body.trim();
    if candidate.parse::<IpAddr>().is_ok() {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn summarize_probe_body(body: &str) -> String {
    let normalized = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "<empty>".to_string();
    }

    let mut snippet = normalized.chars().take(80).collect::<String>();
    if normalized.chars().count() > 80 {
        snippet.push_str("...");
    }
    snippet
}

/// Validate proxy URL format without building a client.
fn validate_proxy_url(url: &str) -> Result<(), String> {
    let parsed =
        Url::parse(url).map_err(|e| format!("Invalid proxy URL '{}': {}", mask_url(url), e))?;

    let scheme = parsed.scheme();
    if !["http", "https", "socks5", "socks5h"].contains(&scheme) {
        return Err(format!(
            "Invalid proxy scheme '{}' in URL '{}'. Supported: http, https, socks5, socks5h",
            scheme,
            mask_url(url)
        ));
    }

    Ok(())
}

/// Apply proxy configuration (assumes already validated).
///
/// Should be called after validate_proxy succeeds.
pub fn apply_proxy(proxy_url: Option<&str>) -> Result<(), String> {
    let effective_url = proxy_url.filter(|s| !s.trim().is_empty());
    let new_client = build_client(effective_url)?;

    if let Some(lock) = GLOBAL_CLIENT.get() {
        let mut client = lock.write().unwrap_or_else(|poisoned| {
            tracing::warn!("[HttpClient] Recovered poisoned client lock");
            poisoned.into_inner()
        });
        *client = new_client;
    } else {
        return init(proxy_url);
    }

    if let Some(lock) = CURRENT_PROXY_URL.get() {
        let mut url = lock.write().unwrap_or_else(|poisoned| {
            tracing::warn!("[HttpClient] Recovered poisoned proxy URL lock");
            poisoned.into_inner()
        });
        *url = effective_url.map(mask_url);
    }

    tracing::info!(
        "[HttpClient] Proxy applied: {}",
        effective_url
            .map(mask_url)
            .unwrap_or_else(|| "direct connection".to_string())
    );

    Ok(())
}

/// Get the global HTTP client.
///
/// Returns the client configured with proxy (if set), otherwise a direct-connection client.
pub fn get() -> Client {
    GLOBAL_CLIENT
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|c| c.clone())
        .unwrap_or_else(|| {
            tracing::warn!("[HttpClient] Client not initialized, using fallback");
            build_client(None).unwrap_or_default()
        })
}

/// Get the current proxy URL.
///
/// Returns None if direct connection is configured.
pub fn get_current_proxy_url() -> Option<String> {
    CURRENT_PROXY_URL
        .get()
        .and_then(|lock| lock.read().ok())
        .and_then(|url| url.clone())
}

/// Check if proxy is currently enabled.
#[allow(dead_code)]
pub fn is_proxy_enabled() -> bool {
    get_current_proxy_url().is_some()
}

fn effective_proxy_url(settings: &AppSettings) -> Result<Option<String>, String> {
    if !settings.upstream_proxy_enabled {
        return Ok(None);
    }

    build_effective_proxy_url(
        Some(settings.upstream_proxy_url.as_str()),
        Some(settings.upstream_proxy_username.as_str()),
        Some(settings.upstream_proxy_password.as_str()),
    )
}

fn set_gateway_self_context(context: GatewaySelfCheckContext) {
    if let Some(lock) = GATEWAY_SELF_CONTEXT.get() {
        let mut current = lock.write().unwrap_or_else(|poisoned| {
            tracing::warn!("[HttpClient] Recovered poisoned self-context lock");
            poisoned.into_inner()
        });
        *current = context;
    } else {
        let _ = GATEWAY_SELF_CONTEXT.set(RwLock::new(context));
    }
}

fn current_self_context() -> GatewaySelfCheckContext {
    GATEWAY_SELF_CONTEXT
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|ctx| ctx.clone())
        .unwrap_or_else(default_self_check_context)
}

fn default_self_check_context() -> GatewaySelfCheckContext {
    build_self_check_context(
        settings::DEFAULT_GATEWAY_PORT,
        &["127.0.0.1".to_string(), "localhost".to_string()],
    )
}

fn build_self_check_context(port: u16, configured_hosts: &[String]) -> GatewaySelfCheckContext {
    let local_hosts = collect_local_host_tokens();
    let mut hosts = BTreeSet::new();

    for host in configured_hosts {
        extend_self_hosts(&mut hosts, host, &local_hosts);
    }

    if hosts.is_empty() {
        hosts.extend(local_hosts.loopback.iter().cloned());
    }

    GatewaySelfCheckContext {
        gateway_port: port,
        hosts,
    }
}

#[derive(Default)]
struct LocalHostTokens {
    all: BTreeSet<String>,
    loopback: BTreeSet<String>,
}

fn collect_local_host_tokens() -> LocalHostTokens {
    let mut tokens = LocalHostTokens::default();

    for host in ["localhost", "127.0.0.1", "::1"] {
        if let Some(token) = normalize_host_token(host) {
            tokens.all.insert(token.clone());
            tokens.loopback.insert(token);
        }
    }

    if let Ok(ifaces) = get_if_addrs() {
        for iface in ifaces {
            if let Some(token) = normalize_host_token(&iface.ip().to_string()) {
                let is_loopback = iface.ip().is_loopback();
                tokens.all.insert(token.clone());
                if is_loopback {
                    tokens.loopback.insert(token);
                }
            }
        }
    }

    if let Ok(value) = hostname::get() {
        if let Some(hostname) = value.to_str().and_then(normalize_host_token) {
            tokens.all.insert(hostname);
        }
    }

    for key in ["HOSTNAME", "COMPUTERNAME"] {
        if let Ok(value) = env::var(key) {
            if let Some(token) = normalize_host_token(&value) {
                tokens.all.insert(token);
            }
        }
    }

    tokens
}

fn extend_self_hosts(targets: &mut BTreeSet<String>, host: &str, local_hosts: &LocalHostTokens) {
    let Some(normalized) = normalize_host_token(host) else {
        return;
    };

    if matches!(normalized.as_str(), "0.0.0.0" | "::") {
        targets.extend(local_hosts.all.iter().cloned());
        return;
    }

    targets.insert(normalized.clone());
    if local_hosts.all.contains(&normalized) {
        targets.extend(resolve_host_tokens(&normalized, local_hosts));
    }
}

fn resolve_host_tokens(host: &str, local_hosts: &LocalHostTokens) -> BTreeSet<String> {
    let mut resolved = BTreeSet::new();

    if let Some(token) = normalize_host_token(host) {
        resolved.insert(token.clone());
        if local_hosts.loopback.contains(&token) {
            resolved.extend(local_hosts.loopback.iter().cloned());
        }
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if let Some(token) = normalize_host_token(&ip.to_string()) {
            resolved.insert(token);
        }
        return resolved;
    }

    if let Ok(addrs) = (host, 0u16).to_socket_addrs() {
        for addr in addrs {
            if let Some(token) = normalize_host_token(&addr.ip().to_string()) {
                if local_hosts.all.contains(&token) {
                    resolved.insert(token);
                }
            }
        }
    }

    resolved
}

fn normalize_host_token(host: &str) -> Option<String> {
    let trimmed = host.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        return Some(ip.to_string());
    }

    Some(trimmed.to_ascii_lowercase())
}

/// Build HTTP client with optional proxy.
fn build_client(proxy_url: Option<&str>) -> Result<Client, String> {
    let mut builder = Client::builder()
        .user_agent(format!(
            "aio-coding-hub-gateway/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .connect_timeout(UPSTREAM_CONNECT_TIMEOUT)
        .pool_max_idle_per_host(10)
        .tcp_keepalive(Duration::from_secs(60));

    if let Some(url) = proxy_url {
        validate_proxy_url(url)?;
        if proxy_uses_socks5_local_dns(url) {
            builder = builder.dns_resolver2(Ipv4FirstResolver);
        }
        let proxy = reqwest::Proxy::all(url)
            .map_err(|e| format!("Invalid proxy URL '{}': {}", mask_url(url), e))?;
        builder = builder.proxy(proxy);
        tracing::debug!("[HttpClient] Proxy configured: {}", mask_url(url));
    } else if system_proxy_points_to_gateway() {
        builder = builder.no_proxy();
        tracing::warn!("[HttpClient] System proxy points to gateway, bypassing to avoid recursion");
    } else {
        tracing::debug!("[HttpClient] Following system proxy (no explicit proxy configured)");
    }

    builder
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))
}

/// Check if system proxy environment variables point to the gateway.
fn system_proxy_points_to_gateway() -> bool {
    const KEYS: [&str; 6] = [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ];

    let context = current_self_context();
    KEYS.iter()
        .filter_map(|key| env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .any(|value| proxy_points_to_gateway_with_context(&value, &context))
}

fn proxy_points_to_gateway_with_context(value: &str, context: &GatewaySelfCheckContext) -> bool {
    fn matches_host_and_port(parsed: &Url, context: &GatewaySelfCheckContext) -> bool {
        let Some(host) = parsed.host_str() else {
            return false;
        };

        let Some(normalized) = normalize_host_token(host) else {
            return false;
        };

        parsed.port() == Some(context.gateway_port) && context.hosts.contains(&normalized)
    }

    if let Ok(parsed) = Url::parse(value) {
        return matches_host_and_port(&parsed, context);
    }

    let with_scheme = format!("http://{value}");
    if let Ok(parsed) = Url::parse(&with_scheme) {
        return matches_host_and_port(&parsed, context);
    }

    false
}

/// Mask sensitive information in URL (for logging).
pub fn mask_url(url: &str) -> String {
    if let Ok(parsed) = Url::parse(url) {
        let host = parsed.host_str().unwrap_or("?");
        match parsed.port() {
            Some(port) => format!("{}://{}:{}", parsed.scheme(), host, port),
            None => format!("{}://{}", parsed.scheme(), host),
        }
    } else if url.len() > 20 {
        format!("{}...", &url[..20])
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener};
    use std::sync::{mpsc, Mutex, MutexGuard, OnceLock};
    use std::thread;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn settings_with_listen_mode(listen_mode: GatewayListenMode) -> AppSettings {
        AppSettings {
            gateway_listen_mode: listen_mode,
            preferred_port: 37123,
            ..AppSettings::default()
        }
    }

    struct ProxyTestUrlGuard<'a> {
        _guard: MutexGuard<'a, ()>,
    }

    impl Drop for ProxyTestUrlGuard<'_> {
        fn drop(&mut self) {
            set_proxy_test_url_for_tests(None);
            set_proxy_exit_ip_url_for_tests(None);
        }
    }

    fn use_http_proxy_test_url() -> ProxyTestUrlGuard<'static> {
        let guard = env_lock().lock().unwrap();
        set_proxy_test_url_for_tests(Some("http://example.com/"));
        ProxyTestUrlGuard { _guard: guard }
    }

    fn use_http_proxy_exit_ip_test_url() -> ProxyTestUrlGuard<'static> {
        let guard = env_lock().lock().unwrap();
        set_proxy_exit_ip_url_for_tests(Some("http://ifconfig.me/ip"));
        ProxyTestUrlGuard { _guard: guard }
    }

    fn spawn_http_proxy_server_with_response(
        response: Vec<u8>,
    ) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind proxy listener");
        let addr = listener.local_addr().expect("proxy listener addr");
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept proxy request");
            let mut buf = [0_u8; 4096];
            let size = stream.read(&mut buf).expect("read proxy request");
            let request = String::from_utf8_lossy(&buf[..size]).to_string();
            tx.send(request).expect("send proxy request");
            stream.write_all(&response).expect("write proxy response");
        });

        (format!("http://127.0.0.1:{}", addr.port()), rx)
    }

    fn spawn_http_proxy_server() -> (String, mpsc::Receiver<String>) {
        spawn_http_proxy_server_with_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        )
    }

    fn spawn_http_origin_server() -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind origin listener");
        let addr = listener.local_addr().expect("origin listener addr");
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept origin request");
            let mut buf = [0_u8; 4096];
            let size = stream.read(&mut buf).expect("read origin request");
            let request = String::from_utf8_lossy(&buf[..size]).to_string();
            tx.send(request).expect("send origin request");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .expect("write origin response");
        });

        (format!("http://127.0.0.1:{}/", addr.port()), rx)
    }

    #[test]
    fn test_mask_url() {
        assert_eq!(mask_url("http://127.0.0.1:7890"), "http://127.0.0.1:7890");
        assert_eq!(
            mask_url("http://user:pass@127.0.0.1:7890"),
            "http://127.0.0.1:7890"
        );
        assert_eq!(
            mask_url("socks5://admin:secret@proxy.example.com:1080"),
            "socks5://proxy.example.com:1080"
        );
        assert_eq!(
            mask_url("http://proxy.example.com"),
            "http://proxy.example.com"
        );
        assert_eq!(
            mask_url("https://user:pass@proxy.example.com"),
            "https://proxy.example.com"
        );
    }

    #[test]
    fn test_sort_socket_addrs_ipv4_first_prefers_ipv4_entries() {
        let mut addrs = vec![
            SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 443),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 443),
        ];

        sort_socket_addrs_ipv4_first(&mut addrs);

        assert!(addrs[0].is_ipv4());
        assert!(addrs[1].is_ipv6());
    }

    #[test]
    fn test_build_client_direct() {
        let result = build_client(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_with_http_proxy() {
        let result = build_client(Some("http://127.0.0.1:7890"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_with_https_proxy() {
        let result = build_client(Some("https://127.0.0.1:8443"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_with_socks5_proxy() {
        let result = build_client(Some("socks5://127.0.0.1:1080"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_with_socks5h_proxy() {
        let result = build_client(Some("socks5h://127.0.0.1:1080"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_client_invalid_scheme() {
        let result = build_client(Some("invalid-scheme://127.0.0.1:7890"));
        assert!(result.is_err(), "Should reject invalid proxy scheme");
    }

    #[test]
    fn test_proxy_points_to_gateway_loopback_and_custom_hosts() {
        let wildcard_context =
            build_self_check_context(37123, &["0.0.0.0".to_string(), "127.0.0.1".to_string()]);
        let custom_context = build_self_check_context(
            37123,
            &["192.168.1.10".to_string(), "devbox.internal".to_string()],
        );

        assert!(proxy_points_to_gateway_with_context(
            "http://127.0.0.1:37123",
            &wildcard_context
        ));
        assert!(proxy_points_to_gateway_with_context(
            "socks5://localhost:37123",
            &wildcard_context
        ));
        assert!(proxy_points_to_gateway_with_context(
            "http://192.168.1.10:37123",
            &custom_context
        ));
        assert!(proxy_points_to_gateway_with_context(
            "https://devbox.internal:37123",
            &custom_context
        ));

        assert!(!proxy_points_to_gateway_with_context(
            "http://127.0.0.1:7890",
            &wildcard_context
        ));
        assert!(!proxy_points_to_gateway_with_context(
            "socks5://localhost:1080",
            &wildcard_context
        ));
    }

    #[test]
    fn test_system_proxy_points_to_gateway() {
        let _guard = env_lock().lock().unwrap();

        set_gateway_self_context(build_self_check_context(
            37123,
            &["127.0.0.1".to_string(), "localhost".to_string()],
        ));

        let keys = [
            "HTTP_PROXY",
            "http_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
        ];

        for key in &keys {
            std::env::remove_var(key);
        }

        std::env::set_var("HTTP_PROXY", "http://127.0.0.1:37123");
        assert!(system_proxy_points_to_gateway());

        std::env::set_var("HTTP_PROXY", "http://127.0.0.1:7890");
        assert!(!system_proxy_points_to_gateway());

        for key in &keys {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn test_validate_proxy_for_settings_rejects_lan_self_loop() {
        let mut settings = settings_with_listen_mode(GatewayListenMode::Lan);
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = "http://127.0.0.1:37123".to_string();
        let result = validate_proxy_for_settings(&settings);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("self-loop"));
    }

    #[test]
    fn test_validate_proxy_for_settings_rejects_custom_host_self_loop() {
        let mut settings = settings_with_listen_mode(GatewayListenMode::Custom);
        settings.gateway_custom_listen_address = "devbox.internal:37123".to_string();
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = "http://devbox.internal:37123".to_string();
        let result = validate_proxy_for_settings(&settings);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_effective_proxy_url_with_separate_http_credentials() {
        let effective = build_effective_proxy_url(
            Some("http://127.0.0.1:7890"),
            Some("proxy-user"),
            Some("secret"),
        )
        .expect("build proxy url")
        .expect("proxy url should exist");
        let parsed = Url::parse(&effective).expect("parse effective proxy url");

        assert_eq!(parsed.scheme(), "http");
        assert_eq!(parsed.username(), "proxy-user");
        assert_eq!(parsed.password(), Some("secret"));
        assert_eq!(parsed.host_str(), Some("127.0.0.1"));
        assert_eq!(parsed.port(), Some(7890));
    }

    #[test]
    fn test_build_effective_proxy_url_with_separate_socks_credentials() {
        let effective = build_effective_proxy_url(
            Some("socks5://127.0.0.1:1080"),
            Some("proxy-user"),
            Some("secret"),
        )
        .expect("build proxy url")
        .expect("proxy url should exist");
        let parsed = Url::parse(&effective).expect("parse effective proxy url");

        assert_eq!(parsed.scheme(), "socks5");
        assert_eq!(parsed.username(), "proxy-user");
        assert_eq!(parsed.password(), Some("secret"));
        assert_eq!(parsed.host_str(), Some("127.0.0.1"));
        assert_eq!(parsed.port(), Some(1080));
    }

    #[test]
    fn test_build_effective_proxy_url_preserves_embedded_credentials() {
        let effective =
            build_effective_proxy_url(Some("http://inline:secret@127.0.0.1:7890"), None, None)
                .expect("build proxy url")
                .expect("proxy url should exist");
        let parsed = Url::parse(&effective).expect("parse effective proxy url");

        assert_eq!(parsed.username(), "inline");
        assert_eq!(parsed.password(), Some("secret"));
        assert_eq!(parsed.host_str(), Some("127.0.0.1"));
        assert_eq!(parsed.port(), Some(7890));
    }

    #[test]
    fn test_build_effective_proxy_url_rejects_password_without_username() {
        let err =
            build_effective_proxy_url(Some("http://127.0.0.1:7890"), Some("   "), Some("secret"))
                .expect_err("password without username should fail");

        assert!(err.contains("upstream_proxy_username cannot be empty"));
    }

    #[test]
    fn test_build_effective_proxy_url_rejects_mixed_embedded_and_separate_credentials() {
        let err = build_effective_proxy_url(
            Some("http://inline:secret@127.0.0.1:7890"),
            Some("proxy-user"),
            Some("override"),
        )
        .expect_err("mixed credentials should fail");

        assert!(err.contains("either in proxy URL or username/password fields"));
    }

    #[test]
    fn test_validate_proxy_for_settings_accepts_separate_credentials() {
        let mut settings = settings_with_listen_mode(GatewayListenMode::Localhost);
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = "https://proxy.example.com:8443".to_string();
        settings.upstream_proxy_username = "proxy-user".to_string();
        settings.upstream_proxy_password = "secret".to_string();

        let result = validate_proxy_for_settings(&settings);
        assert!(result.is_ok(), "expected settings validation to succeed");
    }

    #[test]
    fn test_validate_proxy_for_settings_rejects_conflicting_credentials() {
        let mut settings = settings_with_listen_mode(GatewayListenMode::Localhost);
        settings.upstream_proxy_enabled = true;
        settings.upstream_proxy_url = "http://inline:secret@proxy.example.com:7890".to_string();
        settings.upstream_proxy_username = "proxy-user".to_string();
        settings.upstream_proxy_password = "override".to_string();

        let err = validate_proxy_for_settings(&settings)
            .expect_err("mixed credentials should be rejected");
        assert!(err.contains("either in proxy URL or username/password fields"));
    }

    #[test]
    fn test_validate_proxy_self_loop_uses_current_context() {
        set_gateway_self_context(build_self_check_context(
            37123,
            &["127.0.0.1".to_string(), "localhost".to_string()],
        ));

        let result =
            validate_proxy_with_context(Some("http://127.0.0.1:37123"), &current_self_context());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("self-loop"));

        let result =
            validate_proxy_with_context(Some("http://127.0.0.1:7890"), &current_self_context());
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_exit_ip_response_accepts_ipv4_and_ipv6() {
        assert_eq!(
            parse_exit_ip_response("203.0.113.42\n"),
            Some("203.0.113.42".to_string())
        );
        assert_eq!(
            parse_exit_ip_response("2001:db8::1"),
            Some("2001:db8::1".to_string())
        );
    }

    #[test]
    fn test_parse_exit_ip_response_rejects_non_ip_body() {
        assert_eq!(parse_exit_ip_response("<html>not an ip</html>"), None);
        assert_eq!(parse_exit_ip_response(""), None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_proxy_performs_real_network_request() {
        let _guard = use_http_proxy_test_url();
        let (proxy_url, request_rx) = spawn_http_proxy_server();
        set_gateway_self_context(build_self_check_context(
            37123,
            &["127.0.0.1".to_string(), "localhost".to_string()],
        ));

        test_proxy_with_context(Some(&proxy_url), &current_self_context())
            .await
            .expect("proxy test should succeed");

        let request = request_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("proxy should receive a request");
        assert!(request.starts_with("GET http://example.com/ HTTP/1.1"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_detect_proxy_exit_ip_returns_probe_ip() {
        let _guard = use_http_proxy_exit_ip_test_url();
        let (proxy_url, request_rx) = spawn_http_proxy_server_with_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\n203.0.113.42\n"
                .to_vec(),
        );
        set_gateway_self_context(build_self_check_context(
            37123,
            &["127.0.0.1".to_string(), "localhost".to_string()],
        ));

        let exit_ip = detect_proxy_exit_ip_with_context(Some(&proxy_url), &current_self_context())
            .await
            .expect("proxy exit ip detection should succeed");

        assert_eq!(exit_ip, "203.0.113.42");

        let request = request_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("proxy should receive exit ip request");
        assert!(request.starts_with("GET http://ifconfig.me/ip HTTP/1.1"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_apply_proxy_none_restores_direct_connection() {
        let _guard = use_http_proxy_test_url();
        for key in [
            "HTTP_PROXY",
            "http_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
        ] {
            std::env::remove_var(key);
        }

        let (proxy_url, proxy_request_rx) = spawn_http_proxy_server();
        let (origin_url, origin_request_rx) = spawn_http_origin_server();

        init(Some(&proxy_url)).expect("init proxy");

        let proxied_response = get()
            .get("http://example.com/")
            .send()
            .await
            .expect("proxied request");
        assert_eq!(proxied_response.status(), StatusCode::OK);
        let proxied_request = proxy_request_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("proxy should receive proxied request");
        assert!(proxied_request.starts_with("GET http://example.com/ HTTP/1.1"));

        apply_proxy(None).expect("disable proxy");

        let direct_response = get()
            .get(&origin_url)
            .send()
            .await
            .expect("direct request after disabling proxy");
        assert_eq!(direct_response.status(), StatusCode::OK);

        let direct_request = origin_request_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("origin should receive direct request");
        assert!(direct_request.starts_with("GET / HTTP/1.1"));
    }
}
