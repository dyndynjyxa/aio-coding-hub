use super::GatewayErrorCode;

pub(in crate::gateway) fn status_override_for_error_code(error_code: Option<&str>) -> Option<u16> {
    let code = error_code.and_then(GatewayErrorCode::from_str)?;
    match code {
        GatewayErrorCode::RequestAborted | GatewayErrorCode::StreamAborted => Some(499),
        GatewayErrorCode::UpstreamTimeout | GatewayErrorCode::StreamIdleTimeout => Some(524),
        GatewayErrorCode::StreamError
        | GatewayErrorCode::Fake200
        | GatewayErrorCode::UpstreamReadError
        | GatewayErrorCode::UpstreamConnectFailed
        | GatewayErrorCode::UpstreamBodyReadError
        | GatewayErrorCode::UpstreamAllFailed => Some(502),
        GatewayErrorCode::AllProvidersUnavailable | GatewayErrorCode::NoEnabledProvider => {
            Some(503)
        }
        GatewayErrorCode::CliProxyDisabled => Some(403),
        GatewayErrorCode::InvalidCliKey => Some(400),
        GatewayErrorCode::BodyTooLarge => Some(413),
        GatewayErrorCode::LargeBodyMissingModel => Some(400),
        GatewayErrorCode::ResponseBuildError
        | GatewayErrorCode::InternalError
        | GatewayErrorCode::HttpClientInit => Some(500),
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
    error_code
        .and_then(GatewayErrorCode::from_str)
        .map(GatewayErrorCode::is_client_abort)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{effective_status, is_client_abort, status_override_for_error_code};
    use crate::gateway::proxy::GatewayErrorCode;

    #[test]
    fn status_override_maps_cch_codes() {
        assert_eq!(
            status_override_for_error_code(Some(GatewayErrorCode::RequestAborted.as_str())),
            Some(499)
        );
        assert_eq!(
            status_override_for_error_code(Some(GatewayErrorCode::StreamAborted.as_str())),
            Some(499)
        );
        assert_eq!(
            status_override_for_error_code(Some(GatewayErrorCode::UpstreamTimeout.as_str())),
            Some(524)
        );
        assert_eq!(
            status_override_for_error_code(Some(GatewayErrorCode::StreamIdleTimeout.as_str())),
            Some(524)
        );
        assert_eq!(
            status_override_for_error_code(Some(GatewayErrorCode::UpstreamReadError.as_str())),
            Some(502)
        );
        assert_eq!(
            status_override_for_error_code(Some(GatewayErrorCode::StreamError.as_str())),
            Some(502)
        );
        assert_eq!(
            status_override_for_error_code(Some(GatewayErrorCode::Fake200.as_str())),
            Some(502)
        );
        assert_eq!(
            status_override_for_error_code(Some(
                GatewayErrorCode::AllProvidersUnavailable.as_str()
            )),
            Some(503)
        );
    }

    #[test]
    fn effective_status_overrides_even_when_original_is_200() {
        assert_eq!(
            effective_status(
                Some(200),
                Some(GatewayErrorCode::StreamIdleTimeout.as_str())
            ),
            Some(524)
        );
        assert_eq!(
            effective_status(Some(200), Some(GatewayErrorCode::StreamAborted.as_str())),
            Some(499)
        );
        assert_eq!(
            effective_status(Some(200), Some(GatewayErrorCode::Fake200.as_str())),
            Some(502)
        );
        assert_eq!(
            effective_status(Some(404), Some(GatewayErrorCode::Upstream4xx.as_str())),
            Some(404)
        );
    }

    #[test]
    fn client_abort_detection() {
        assert!(is_client_abort(Some(
            GatewayErrorCode::RequestAborted.as_str()
        )));
        assert!(is_client_abort(Some(
            GatewayErrorCode::StreamAborted.as_str()
        )));
        assert!(!is_client_abort(Some(
            GatewayErrorCode::UpstreamTimeout.as_str()
        )));
        assert!(!is_client_abort(None));
    }
}
