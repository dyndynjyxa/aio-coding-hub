//! Usage: Align request log status codes with claude-code-hub semantics (e.g. 499/524).

pub(in crate::gateway) fn status_override_for_error_code(error_code: Option<&str>) -> Option<u16> {
    match error_code {
        Some("GW_REQUEST_ABORTED") | Some("GW_STREAM_ABORTED") => Some(499),
        Some("GW_UPSTREAM_TIMEOUT") | Some("GW_STREAM_IDLE_TIMEOUT") => Some(524),
        Some("GW_STREAM_ERROR")
        | Some("GW_UPSTREAM_READ_ERROR")
        | Some("GW_UPSTREAM_CONNECT_FAILED")
        | Some("GW_UPSTREAM_BODY_READ_ERROR")
        | Some("GW_UPSTREAM_ALL_FAILED") => Some(502),
        Some("GW_ALL_PROVIDERS_UNAVAILABLE") | Some("GW_NO_ENABLED_PROVIDER") => Some(503),
        Some("GW_CLI_PROXY_DISABLED") => Some(403),
        Some("GW_INVALID_CLI_KEY") => Some(400),
        Some("GW_BODY_TOO_LARGE") => Some(413),
        Some("GW_RESPONSE_BUILD_ERROR")
        | Some("GW_INTERNAL_ERROR")
        | Some("GW_HTTP_CLIENT_INIT") => Some(500),
        _ => None,
    }
}

pub(in crate::gateway) fn effective_status(
    status: Option<u16>,
    error_code: Option<&str>,
) -> Option<u16> {
    status_override_for_error_code(error_code).or(status)
}

pub(in crate::gateway) fn is_client_abort(error_code: Option<&str>) -> bool {
    matches!(error_code, Some("GW_REQUEST_ABORTED" | "GW_STREAM_ABORTED"))
}

#[cfg(test)]
mod tests {
    use super::{effective_status, is_client_abort, status_override_for_error_code};

    #[test]
    fn status_override_maps_cch_codes() {
        assert_eq!(
            status_override_for_error_code(Some("GW_REQUEST_ABORTED")),
            Some(499)
        );
        assert_eq!(
            status_override_for_error_code(Some("GW_STREAM_ABORTED")),
            Some(499)
        );
        assert_eq!(
            status_override_for_error_code(Some("GW_UPSTREAM_TIMEOUT")),
            Some(524)
        );
        assert_eq!(
            status_override_for_error_code(Some("GW_STREAM_IDLE_TIMEOUT")),
            Some(524)
        );
        assert_eq!(
            status_override_for_error_code(Some("GW_UPSTREAM_READ_ERROR")),
            Some(502)
        );
        assert_eq!(
            status_override_for_error_code(Some("GW_STREAM_ERROR")),
            Some(502)
        );
        assert_eq!(
            status_override_for_error_code(Some("GW_ALL_PROVIDERS_UNAVAILABLE")),
            Some(503)
        );
    }

    #[test]
    fn effective_status_overrides_even_when_original_is_200() {
        assert_eq!(
            effective_status(Some(200), Some("GW_STREAM_IDLE_TIMEOUT")),
            Some(524)
        );
        assert_eq!(
            effective_status(Some(200), Some("GW_STREAM_ABORTED")),
            Some(499)
        );
        assert_eq!(
            effective_status(Some(404), Some("GW_UPSTREAM_4XX")),
            Some(404)
        );
    }

    #[test]
    fn client_abort_detection() {
        assert!(is_client_abort(Some("GW_REQUEST_ABORTED")));
        assert!(is_client_abort(Some("GW_STREAM_ABORTED")));
        assert!(!is_client_abort(Some("GW_UPSTREAM_TIMEOUT")));
        assert!(!is_client_abort(None));
    }
}
