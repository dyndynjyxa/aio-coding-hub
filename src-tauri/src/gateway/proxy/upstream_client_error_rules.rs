//! Usage: Detect non-retryable client input errors from upstream error bodies (align with

/// Limit how much of the upstream error body we scan (defensive against huge error payloads).
const MAX_SCAN_BYTES: usize = 64 * 1024;

/// Maximum body size to read when content_length is unknown.
///
/// Keep this aligned with `MAX_SCAN_BYTES` so scan behavior is consistent regardless of whether
/// upstream returns `content-length`.
const MAX_BODY_READ_BYTES: u64 = MAX_SCAN_BYTES as u64;

/// Returns whether we should attempt to scan the upstream error body to detect non-retryable
/// client input errors.
///
/// This prevents wasting work on statuses where:
/// - failover is always preferred (e.g. 402/404), or
/// - we already have dedicated retry/backoff logic (e.g. 408/429), or
/// - the body is too large to scan without reading the entire response into memory.
pub(super) fn should_attempt_non_retryable_match(
    status: reqwest::StatusCode,
    content_length: Option<u64>,
) -> bool {
    if !status.is_client_error() {
        return false;
    }

    match status.as_u16() {
        401 | 402 | 403 | 404 | 408 | 429 => return false,
        _ => {}
    }

    if let Some(len) = content_length {
        if len > MAX_SCAN_BYTES as u64 {
            return false;
        }
    }

    true
}

/// Returns the maximum number of bytes to read from an upstream error body when
/// content_length is unknown.
pub(super) fn max_body_read_bytes() -> u64 {
    MAX_BODY_READ_BYTES
}

/// Abort unmatched catch-all 4xx to prevent pointless retries and circuit breaker pollution.
///
/// Catch-all 4xx (anything outside 401-404, 408, 429) that was not matched by the body-scanning
/// non-retryable rules is almost certainly a deterministic client error. Retrying the identical
/// request will produce the identical result, wasting attempts and inflating the provider failure
/// count until the circuit breaker opens.
pub(super) fn should_abort_unmatched_client_error(
    status: reqwest::StatusCode,
    matched_rule_id: Option<&'static str>,
) -> bool {
    status.is_client_error()
        && !matches!(status.as_u16(), 401 | 402 | 403 | 404 | 408 | 429)
        && matched_rule_id.is_none()
}

/// Returns whether a 429 response body indicates upstream concurrency saturation.
///
/// This is intentionally conservative: only match clear concurrency-limit signatures.
pub(super) fn match_429_concurrency_limit(body: &[u8]) -> bool {
    if body.is_empty() {
        return false;
    }

    let scan = if body.len() > MAX_SCAN_BYTES {
        &body[..MAX_SCAN_BYTES]
    } else {
        body
    };

    let haystack_lower = String::from_utf8_lossy(scan).to_ascii_lowercase();

    let has_direct_phrase = haystack_lower.contains("concurrency limit exceeded")
        || haystack_lower.contains("too many concurrent requests");
    let has_structured_fields = haystack_lower.contains("currentconcurrency")
        && haystack_lower.contains("concurrencylimit");

    has_direct_phrase || has_structured_fields
}

/// Returns whether a response body clearly indicates account/provider quota exhaustion.
pub(super) fn match_quota_exhausted(body: &[u8]) -> bool {
    if body.is_empty() {
        return false;
    }

    let scan = if body.len() > MAX_SCAN_BYTES {
        &body[..MAX_SCAN_BYTES]
    } else {
        body
    };

    let haystack_lower = String::from_utf8_lossy(scan).to_ascii_lowercase();

    haystack_lower.contains("insufficient_quota")
        || haystack_lower.contains("quota exhausted")
        || haystack_lower.contains("resource_exhausted")
        || haystack_lower.contains("exceeded your current quota")
        || haystack_lower.contains("you exceeded your current quota")
        || (haystack_lower.contains("quota") && haystack_lower.contains("exceeded"))
        || (haystack_lower.contains("quota") && haystack_lower.contains("exhausted"))
        || (haystack_lower.contains("rate limit") && haystack_lower.contains("exceeded"))
        || (haystack_lower.contains("rate_limit") && haystack_lower.contains("exceeded"))
        || (haystack_lower.contains("usage limit") && haystack_lower.contains("reached"))
}

struct Rule {
    id: &'static str,
    any_of: &'static [&'static str],
    all_of: &'static [&'static str],
}

fn matches_rule(haystack_lower: &str, rule: &Rule) -> bool {
    if !rule
        .all_of
        .iter()
        .all(|needle| haystack_lower.contains(needle))
    {
        return false;
    }
    if rule.any_of.is_empty() {
        return true;
    }
    rule.any_of
        .iter()
        .any(|needle| haystack_lower.contains(needle))
}

//       We intentionally keep this list small and conservative to avoid false positives.
const NON_RETRYABLE_RULES: &[Rule] = &[
    Rule {
        id: "parameter_alt_sse",
        any_of: &["alt=sse"],
        all_of: &[],
    },
    Rule {
        id: "prompt_limit",
        any_of: &["prompt is too long", "prompt too long"],
        all_of: &[],
    },
    Rule {
        id: "input_limit",
        any_of: &["input is too long", "content_length_exceeds_threshold"],
        all_of: &[],
    },
    Rule {
        id: "token_limit",
        any_of: &["max_tokens", "maximum tokens", "max tokens"],
        all_of: &["exceed"],
    },
    Rule {
        id: "context_limit",
        any_of: &[
            "context window",
            "context length",
            "pricing plan does not include long context",
        ],
        all_of: &[],
    },
    Rule {
        id: "content_filter",
        any_of: &["content filter", "blocked by content filter"],
        all_of: &[],
    },
    Rule {
        id: "validation_exception",
        any_of: &["validationexception"],
        all_of: &[],
    },
    Rule {
        id: "validation_tool_use_ids_unique",
        any_of: &["tool_use", "tool names must be unique"],
        all_of: &["must be unique"],
    },
    Rule {
        id: "validation_message_non_empty",
        any_of: &["all messages must have non-empty content"],
        all_of: &[],
    },
    Rule {
        id: "validation_server_tool_use_id",
        any_of: &["server_tool_use", "srvtoolu_"],
        all_of: &["match pattern"],
    },
    Rule {
        id: "validation_tool_use_id_in_tool_result",
        any_of: &["tool_use_id", "tool_result"],
        all_of: &["unexpected"],
    },
    Rule {
        id: "validation_tool_result_missing_tool_use",
        any_of: &["tool_result"],
        all_of: &["tool_use", "corresponding"],
    },
    Rule {
        id: "validation_tool_use_missing_tool_result",
        any_of: &["tool_use"],
        all_of: &["tool_result", "next message"],
    },
    Rule {
        id: "parameter_missing_model",
        any_of: &["model is required"],
        all_of: &[],
    },
    Rule {
        id: "parameter_missing_or_extra",
        any_of: &[
            "missing required parameter",
            "extra inputs",
            "not permitted",
        ],
        all_of: &[],
    },
    Rule {
        id: "signature_field_required",
        any_of: &["field required"],
        all_of: &["signature"],
    },
    Rule {
        id: "pdf_limit",
        any_of: &["pdf has too many pages"],
        all_of: &[],
    },
    Rule {
        id: "media_limit",
        any_of: &["too much media"],
        all_of: &[],
    },
    Rule {
        id: "thinking_error_missing_block_prefix",
        any_of: &["must start with a thinking block"],
        all_of: &[],
    },
    Rule {
        id: "thinking_error_expected_block",
        any_of: &["expected"],
        all_of: &["thinking", "tool_use"],
    },
    Rule {
        id: "cache_limit",
        any_of: &["cache_control"],
        all_of: &["block", "limit"],
    },
    Rule {
        id: "image_size_limit",
        any_of: &["image exceeds"],
        all_of: &["maximum", "bytes"],
    },
    Rule {
        id: "thinking_error_reasoning_effort",
        any_of: &["unsupported value"],
        all_of: &["supported values", "model"],
    },
];

/// Returns a matched rule id if the upstream error should be treated as a non-retryable client
/// input error (abort, no provider switch).
///
/// This is used to avoid wasting failover attempts and to prevent circuit/cooldown pollution
/// on errors caused by invalid client requests (prompt/token limits, content filter, schema
/// validation, etc.).
pub(super) fn match_non_retryable_client_error(
    _cli_key: &str,
    status: reqwest::StatusCode,
    body: &[u8],
) -> Option<&'static str> {
    if !status.is_client_error() {
        return None;
    }
    if matches!(status.as_u16(), 401 | 402 | 403 | 404 | 408 | 429) {
        return None;
    }
    if body.is_empty() {
        return None;
    }

    let scan = if body.len() > MAX_SCAN_BYTES {
        &body[..MAX_SCAN_BYTES]
    } else {
        body
    };

    let haystack_lower = String::from_utf8_lossy(scan).to_ascii_lowercase();
    for rule in NON_RETRYABLE_RULES {
        if matches_rule(&haystack_lower, rule) {
            return Some(rule.id);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        match_429_concurrency_limit, match_non_retryable_client_error, match_quota_exhausted,
        should_abort_unmatched_client_error,
    };

    #[test]
    fn matches_prompt_limit() {
        let body = b"{\"error\":{\"message\":\"Prompt is too long. Maximum tokens exceeded\"}}";
        assert_eq!(
            match_non_retryable_client_error("claude", reqwest::StatusCode::BAD_REQUEST, body),
            Some("prompt_limit")
        );
    }

    #[test]
    fn does_not_match_subscription_402_message() {
        let body = b"No available asset for API access, please purchase a subscription";
        assert_eq!(
            match_non_retryable_client_error("claude", reqwest::StatusCode::PAYMENT_REQUIRED, body),
            None
        );
    }

    #[test]
    fn does_not_match_on_402_even_if_body_contains_limit_text() {
        let body = b"Prompt is too long. Maximum tokens exceeded";
        assert_eq!(
            match_non_retryable_client_error("claude", reqwest::StatusCode::PAYMENT_REQUIRED, body),
            None
        );
    }

    #[test]
    fn matches_429_concurrency_limit_by_phrase() {
        let body =
            b"{\"error\":\"Concurrency limit exceeded\",\"message\":\"Too many concurrent requests. Limit: 5\"}";
        assert!(match_429_concurrency_limit(body));
    }

    #[test]
    fn matches_429_concurrency_limit_by_structured_fields() {
        let body = b"{\"error\":\"x\",\"currentConcurrency\":5,\"concurrencyLimit\":5}";
        assert!(match_429_concurrency_limit(body));
    }

    #[test]
    fn does_not_match_429_concurrency_limit_for_generic_rate_limit() {
        let body = b"{\"error\":\"rate limit\",\"message\":\"too many requests per minute\"}";
        assert!(!match_429_concurrency_limit(body));
    }

    #[test]
    fn matches_quota_exhausted_bodies() {
        assert!(match_quota_exhausted(
            br#"{"error":{"message":"quota exhausted","type":"insufficient_quota"}}"#
        ));
        assert!(match_quota_exhausted(
            b"You exceeded your current quota, please check your plan and billing details"
        ));
        assert!(match_quota_exhausted(
            b"Rate limit exceeded for this account"
        ));
        assert!(match_quota_exhausted(
            b"RESOURCE_EXHAUSTED: Resource has been exhausted (e.g. check quota)."
        ));
    }

    #[test]
    fn does_not_match_generic_throttling_without_exhaustion() {
        assert!(!match_quota_exhausted(
            b"{\"error\":\"rate limit\",\"message\":\"too many requests per minute\"}"
        ));
        assert!(!match_quota_exhausted(b"concurrency limit exceeded"));
    }

    #[test]
    fn unmatched_400_aborts_for_any_cli() {
        assert!(should_abort_unmatched_client_error(
            reqwest::StatusCode::BAD_REQUEST,
            None,
        ));
    }

    #[test]
    fn unmatched_422_aborts() {
        assert!(should_abort_unmatched_client_error(
            reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            None,
        ));
    }

    #[test]
    fn unmatched_409_aborts() {
        assert!(should_abort_unmatched_client_error(
            reqwest::StatusCode::CONFLICT,
            None,
        ));
    }

    #[test]
    fn matched_rule_does_not_abort() {
        assert!(!should_abort_unmatched_client_error(
            reqwest::StatusCode::BAD_REQUEST,
            Some("prompt_limit"),
        ));
    }

    #[test]
    fn excluded_4xx_codes_do_not_abort() {
        for status in [
            reqwest::StatusCode::UNAUTHORIZED,
            reqwest::StatusCode::PAYMENT_REQUIRED,
            reqwest::StatusCode::FORBIDDEN,
            reqwest::StatusCode::NOT_FOUND,
            reqwest::StatusCode::REQUEST_TIMEOUT,
            reqwest::StatusCode::TOO_MANY_REQUESTS,
        ] {
            assert!(
                !should_abort_unmatched_client_error(status, None),
                "status {} should not abort",
                status.as_u16()
            );
        }
    }

    #[test]
    fn non_4xx_does_not_abort() {
        assert!(!should_abort_unmatched_client_error(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            None,
        ));
        assert!(!should_abort_unmatched_client_error(
            reqwest::StatusCode::OK,
            None,
        ));
    }
}
