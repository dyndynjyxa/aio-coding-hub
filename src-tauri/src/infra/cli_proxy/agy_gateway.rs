//! Antigravity app 2.30+ and agy share this GenAI gateway in ~/.gemini/config/config.json.
use crate::shared::error::AppResult;
use serde_json::{json, Map, Value};

fn config(bytes: Option<&[u8]>) -> AppResult<Map<String, Value>> {
    match bytes {
        None => Ok(Map::new()),
        Some(bytes) => serde_json::from_slice(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes))
            .map_err(|e| {
                format!("AGY_INVALID_CONFIG: Antigravity 配置必须是 JSON 对象：{e}").into()
            }),
    }
}

fn gateway(origin: &str) -> Value {
    json!({"enabled": true, "url": format!("{}/gemini", origin.trim_end_matches('/')),
           "apiKey": super::PLACEHOLDER_KEY, "wireProtocol": "genai"})
}

fn encode(root: &Map<String, Value>) -> AppResult<Vec<u8>> {
    let mut bytes =
        serde_json::to_vec_pretty(root).map_err(|e| format!("序列化 Antigravity 配置失败：{e}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn is_applied(bytes: &[u8], origin: &str) -> bool {
    config(Some(bytes)).is_ok_and(|root| root.get("gateway").is_some_and(|g| is_active(g, origin)))
}

pub(crate) fn has_owned_endpoint(bytes: &[u8], origin: &str) -> bool {
    config(Some(bytes)).is_ok_and(|root| {
        root.get("gateway")
            .is_some_and(|g| owns_endpoint(g, origin))
    })
}

pub(crate) fn is_local_gateway_url(raw: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(raw) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.path() == "/gemini"
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        })
}

fn is_aio_gateway(gateway: &Value) -> bool {
    gateway.get("apiKey").and_then(Value::as_str) == Some(super::PLACEHOLDER_KEY)
        && gateway
            .get("url")
            .and_then(Value::as_str)
            .is_some_and(is_local_gateway_url)
}

pub(crate) fn has_aio_gateway(bytes: &[u8]) -> bool {
    check_managed_endpoint(bytes, &[]).unwrap_or(false)
}

pub(crate) fn check_managed_endpoint(bytes: &[u8], origins: &[&str]) -> AppResult<bool> {
    Ok(config(Some(bytes))?.get("gateway").is_some_and(|g| {
        is_aio_gateway(g) || origins.iter().any(|origin| owns_endpoint(g, origin))
    }))
}

fn owns_endpoint(current: &Value, origin: &str) -> bool {
    let url = format!("{}/gemini", origin.trim_end_matches('/'));
    current.get("url").and_then(Value::as_str) == Some(url.as_str())
        && current.get("apiKey").and_then(Value::as_str) == Some(super::PLACEHOLDER_KEY)
}

fn is_active(current: &Value, origin: &str) -> bool {
    owns_endpoint(current, origin)
        && current.get("enabled").and_then(Value::as_bool) == Some(true)
        && current.get("wireProtocol").and_then(Value::as_str) == Some("genai")
        && TRANSPORT_FIELDS
            .iter()
            .all(|key| current.get(*key).is_none())
}

// These transport settings may contain endpoint-specific credentials or trust material.
const TRANSPORT_FIELDS: &[&str] = &["headers", "proxyUrl", "caCert", "clientCert", "clientKey"];

pub(crate) fn apply(current: Option<&[u8]>, origin: &str) -> AppResult<Vec<u8>> {
    let mut root = config(current)?;
    let previous = root.entry("gateway").or_insert_with(|| json!({}));
    if is_active(previous, origin) {
        return Ok(current.unwrap_or_default().to_vec());
    }
    if previous.is_null() {
        *previous = json!({});
    }
    let target = previous
        .as_object_mut()
        .ok_or("AGY_INVALID_CONFIG: gateway 必须是对象")?;
    for field in TRANSPORT_FIELDS {
        target.remove(*field);
    }
    target.extend(gateway(origin).as_object().unwrap().clone());
    encode(&root)
}

pub(crate) fn restore(
    current: Option<&[u8]>,
    original: Option<&[u8]>,
    origin: &str,
) -> AppResult<Option<Vec<u8>>> {
    let Some(bytes) = current else {
        return Ok(None);
    };
    let mut root = config(Some(bytes))?;
    let Some(active) = root
        .get_mut("gateway")
        .filter(|g| owns_endpoint(g, origin) || is_aio_gateway(g))
    else {
        // Never restore old credentials into a manually selected new destination.
        return Ok(Some(bytes.to_vec()));
    };
    let before = config(original)?;
    let old = before.get("gateway");
    let target = active.as_object_mut().unwrap();
    for key in ["enabled", "url", "apiKey", "wireProtocol"]
        .into_iter()
        .chain(TRANSPORT_FIELDS.iter().copied())
    {
        match old.and_then(|v| v.get(key)) {
            Some(v) => {
                target.insert(key.into(), v.clone());
            }
            None => {
                target.remove(key);
            }
        }
    }
    if target.is_empty() {
        match old {
            Some(Value::Null) => *active = Value::Null,
            None => {
                root.remove("gateway");
            }
            _ => {}
        }
    }
    if root == before {
        return Ok(original.map(<[u8]>::to_vec));
    }
    encode(&root).map(Some)
}
