use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub(super) fn parse_request_json(request_json: &str) -> Result<super::ParsedRequest, String> {
    let value: serde_json::Value = serde_json::from_str(request_json)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid JSON: {e}"))?;

    let Some(obj) = value.as_object() else {
        return Err("SEC_INVALID_INPUT: request_json must be a JSON object".to_string());
    };

    // Accept either:
    // - Wrapper: { headers, body, expect }
    // - Raw body: { model, messages, ... }
    let (headers_value, body_value, expect_value, path_value, query_value, roundtrip_value) =
        if obj.contains_key("body") {
            (
                obj.get("headers").cloned(),
                obj.get("body").cloned(),
                obj.get("expect").cloned(),
                obj.get("path").cloned(),
                obj.get("query").cloned(),
                obj.get("roundtrip").cloned(),
            )
        } else {
            (None, Some(value.clone()), None, None, None, None)
        };

    let headers_map = headers_value
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let Some(body) = body_value else {
        return Err("SEC_INVALID_INPUT: request_json.body is required".to_string());
    };
    if !body.is_object() {
        return Err("SEC_INVALID_INPUT: request_json.body must be an object".to_string());
    }

    let expect_max_output_chars = expect_value
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("max_output_chars"))
        .and_then(|v| v.as_u64())
        .and_then(|v| usize::try_from(v).ok())
        .filter(|v| *v > 0);

    let expect_exact_output_chars = expect_value
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("exact_output_chars"))
        .and_then(|v| v.as_u64())
        .and_then(|v| usize::try_from(v).ok())
        .filter(|v| *v > 0);

    let (forwarded_path, forwarded_query_from_path) = path_value
        .and_then(|v| v.as_str().map(|s| s.trim().to_string()))
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return None;
            }

            let (path_part, query_part) = match trimmed.split_once('?') {
                Some((p, q)) => (p, Some(q)),
                None => (trimmed, None),
            };

            let mut path = path_part.trim().to_string();
            if path.is_empty() {
                return None;
            }
            if !path.starts_with('/') {
                path.insert(0, '/');
            }

            let query = query_part
                .map(|q| q.trim().trim_start_matches('?').to_string())
                .filter(|q| !q.is_empty());

            Some((path, query))
        })
        .unwrap_or_else(|| ("/v1/messages".to_string(), None));

    let forwarded_query = query_value
        .and_then(|v| {
            v.as_str()
                .map(|s| s.trim().trim_start_matches('?').to_string())
        })
        .filter(|s| !s.is_empty())
        .or(forwarded_query_from_path);

    let roundtrip = roundtrip_value
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(parse_roundtrip_config);

    Ok(super::ParsedRequest {
        request_value: value,
        headers: headers_map,
        body,
        expect_max_output_chars,
        expect_exact_output_chars,
        forwarded_path,
        forwarded_query,
        roundtrip,
    })
}

fn parse_roundtrip_config(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Option<super::RoundtripConfig> {
    let kind = obj
        .get("kind")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();

    let step2_user_prompt = obj
        .get("step2_user_prompt")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    match kind.as_str() {
        "signature" => {
            let enable_tamper = obj
                .get("enable_tamper")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Some(super::RoundtripConfig::Signature(
                super::SignatureRoundtripConfig {
                    enable_tamper,
                    step2_user_prompt,
                },
            ))
        }
        "cache" => Some(super::RoundtripConfig::Cache(super::CacheRoundtripConfig {
            step2_user_prompt,
        })),
        _ => None,
    }
}

pub(super) fn build_target_url(
    base_url: &str,
    forwarded_path: &str,
    forwarded_query: Option<&str>,
) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid base_url: {e}"))?;

    let base_path = url.path().trim_end_matches('/');
    let forwarded_path = if base_path.ends_with("/v1")
        && (forwarded_path == "/v1" || forwarded_path.starts_with("/v1/"))
    {
        forwarded_path.strip_prefix("/v1").unwrap_or(forwarded_path)
    } else {
        forwarded_path
    };

    let mut combined_path = String::new();
    combined_path.push_str(base_path);
    combined_path.push_str(forwarded_path);

    if combined_path.is_empty() {
        combined_path.push('/');
    }
    if !combined_path.starts_with('/') {
        combined_path.insert(0, '/');
    }

    url.set_path(&combined_path);
    let forwarded_query = forwarded_query
        .map(str::trim)
        .map(|v| v.trim_start_matches('?'))
        .filter(|v| !v.is_empty());
    url.set_query(forwarded_query);
    Ok(url)
}

pub(super) fn header_map_from_json(
    headers_json: &serde_json::Map<String, serde_json::Value>,
    provider_api_key: &str,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let wants_authorization = headers_json
        .keys()
        .any(|k| k.trim().eq_ignore_ascii_case("authorization"));

    for (k, v) in headers_json {
        let Some(value_str) = v.as_str() else {
            continue;
        };
        let name_lc = k.trim().to_lowercase();
        if name_lc.is_empty() {
            continue;
        }
        if name_lc == "x-api-key" || name_lc == "authorization" || name_lc == "host" {
            // Never accept caller-provided auth.
            continue;
        }

        let Ok(name) = HeaderName::from_bytes(name_lc.as_bytes()) else {
            continue;
        };
        let Ok(value) = HeaderValue::from_str(value_str) else {
            continue;
        };
        headers.insert(name, value);
    }

    headers.insert(
        HeaderName::from_static("x-api-key"),
        HeaderValue::from_str(provider_api_key).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    if wants_authorization {
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {provider_api_key}"))
                .unwrap_or_else(|_| HeaderValue::from_static("")),
        );
    }

    if !headers.contains_key("anthropic-version") {
        headers.insert(
            HeaderName::from_static("anthropic-version"),
            HeaderValue::from_static(super::DEFAULT_ANTHROPIC_VERSION),
        );
    }

    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("application/json"),
    );

    headers
}
