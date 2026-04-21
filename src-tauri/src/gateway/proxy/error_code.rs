//! Usage: Centralized gateway error-code enum for stable classification/mapping.
//! SYNC: When adding/removing variants, also update `src/constants/gatewayErrorCodes.ts`
//!       (GatewayErrorCodes + GatewayErrorShortLabels + GatewayErrorDescriptions).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::gateway) enum GatewayErrorCode {
    AllProvidersUnavailable,
    UpstreamAllFailed,
    NoEnabledProvider,
    UpstreamTimeout,
    UpstreamConnectFailed,
    Upstream5xx,
    Upstream4xx,
    UpstreamReadError,
    UpstreamBodyReadError,
    StreamError,
    StreamAborted,
    StreamIdleTimeout,
    RequestAborted,
    InternalError,
    BodyTooLarge,
    LargeBodyMissingModel,
    InvalidCliKey,
    InvalidBaseUrl,
    PortInUse,
    ResponseBuildError,
    ProviderRateLimited,
    ProviderCircuitOpen,
    CliProxyDisabled,
    CliProxyGuardError,
    HttpClientInit,
    AttemptLogChannelClosed,
    AttemptLogEnqueueTimeout,
    AttemptLogDropped,
    RequestLogChannelClosed,
    RequestLogEnqueueTimeout,
    RequestLogWriteThroughOnBackpressure,
    RequestLogWriteThroughRateLimited,
    RequestLogDropped,
    Fake200,
}

impl GatewayErrorCode {
    pub(in crate::gateway) const fn as_str(self) -> &'static str {
        match self {
            Self::AllProvidersUnavailable => "GW_ALL_PROVIDERS_UNAVAILABLE",
            Self::UpstreamAllFailed => "GW_UPSTREAM_ALL_FAILED",
            Self::NoEnabledProvider => "GW_NO_ENABLED_PROVIDER",
            Self::UpstreamTimeout => "GW_UPSTREAM_TIMEOUT",
            Self::UpstreamConnectFailed => "GW_UPSTREAM_CONNECT_FAILED",
            Self::Upstream5xx => "GW_UPSTREAM_5XX",
            Self::Upstream4xx => "GW_UPSTREAM_4XX",
            Self::UpstreamReadError => "GW_UPSTREAM_READ_ERROR",
            Self::UpstreamBodyReadError => "GW_UPSTREAM_BODY_READ_ERROR",
            Self::StreamError => "GW_STREAM_ERROR",
            Self::StreamAborted => "GW_STREAM_ABORTED",
            Self::StreamIdleTimeout => "GW_STREAM_IDLE_TIMEOUT",
            Self::RequestAborted => "GW_REQUEST_ABORTED",
            Self::InternalError => "GW_INTERNAL_ERROR",
            Self::BodyTooLarge => "GW_BODY_TOO_LARGE",
            Self::LargeBodyMissingModel => "GW_LARGE_BODY_MISSING_MODEL",
            Self::InvalidCliKey => "GW_INVALID_CLI_KEY",
            Self::InvalidBaseUrl => "GW_INVALID_BASE_URL",
            Self::PortInUse => "GW_PORT_IN_USE",
            Self::ResponseBuildError => "GW_RESPONSE_BUILD_ERROR",
            Self::ProviderRateLimited => "GW_PROVIDER_RATE_LIMITED",
            Self::ProviderCircuitOpen => "GW_PROVIDER_CIRCUIT_OPEN",
            Self::CliProxyDisabled => "GW_CLI_PROXY_DISABLED",
            Self::CliProxyGuardError => "GW_CLI_PROXY_GUARD_ERROR",
            Self::HttpClientInit => "GW_HTTP_CLIENT_INIT",
            Self::AttemptLogChannelClosed => "GW_ATTEMPT_LOG_CHANNEL_CLOSED",
            Self::AttemptLogEnqueueTimeout => "GW_ATTEMPT_LOG_ENQUEUE_TIMEOUT",
            Self::AttemptLogDropped => "GW_ATTEMPT_LOG_DROPPED",
            Self::RequestLogChannelClosed => "GW_REQUEST_LOG_CHANNEL_CLOSED",
            Self::RequestLogEnqueueTimeout => "GW_REQUEST_LOG_ENQUEUE_TIMEOUT",
            Self::RequestLogWriteThroughOnBackpressure => {
                "GW_REQUEST_LOG_WRITE_THROUGH_ON_BACKPRESSURE"
            }
            Self::RequestLogWriteThroughRateLimited => "GW_REQUEST_LOG_WRITE_THROUGH_RATE_LIMITED",
            Self::RequestLogDropped => "GW_REQUEST_LOG_DROPPED",
            Self::Fake200 => "GW_FAKE_200",
        }
    }

    pub(in crate::gateway) fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "GW_ALL_PROVIDERS_UNAVAILABLE" => Self::AllProvidersUnavailable,
            "GW_UPSTREAM_ALL_FAILED" => Self::UpstreamAllFailed,
            "GW_NO_ENABLED_PROVIDER" => Self::NoEnabledProvider,
            "GW_UPSTREAM_TIMEOUT" => Self::UpstreamTimeout,
            "GW_UPSTREAM_CONNECT_FAILED" => Self::UpstreamConnectFailed,
            "GW_UPSTREAM_5XX" => Self::Upstream5xx,
            "GW_UPSTREAM_4XX" => Self::Upstream4xx,
            "GW_UPSTREAM_READ_ERROR" => Self::UpstreamReadError,
            "GW_UPSTREAM_BODY_READ_ERROR" => Self::UpstreamBodyReadError,
            "GW_STREAM_ERROR" => Self::StreamError,
            "GW_STREAM_ABORTED" => Self::StreamAborted,
            "GW_STREAM_IDLE_TIMEOUT" => Self::StreamIdleTimeout,
            "GW_REQUEST_ABORTED" => Self::RequestAborted,
            "GW_INTERNAL_ERROR" => Self::InternalError,
            "GW_BODY_TOO_LARGE" => Self::BodyTooLarge,
            "GW_LARGE_BODY_MISSING_MODEL" => Self::LargeBodyMissingModel,
            "GW_INVALID_CLI_KEY" => Self::InvalidCliKey,
            "GW_INVALID_BASE_URL" => Self::InvalidBaseUrl,
            "GW_PORT_IN_USE" => Self::PortInUse,
            "GW_RESPONSE_BUILD_ERROR" => Self::ResponseBuildError,
            "GW_PROVIDER_RATE_LIMITED" => Self::ProviderRateLimited,
            "GW_PROVIDER_CIRCUIT_OPEN" => Self::ProviderCircuitOpen,
            "GW_CLI_PROXY_DISABLED" => Self::CliProxyDisabled,
            "GW_CLI_PROXY_GUARD_ERROR" => Self::CliProxyGuardError,
            "GW_HTTP_CLIENT_INIT" => Self::HttpClientInit,
            "GW_ATTEMPT_LOG_CHANNEL_CLOSED" => Self::AttemptLogChannelClosed,
            "GW_ATTEMPT_LOG_ENQUEUE_TIMEOUT" => Self::AttemptLogEnqueueTimeout,
            "GW_ATTEMPT_LOG_DROPPED" => Self::AttemptLogDropped,
            "GW_REQUEST_LOG_CHANNEL_CLOSED" => Self::RequestLogChannelClosed,
            "GW_REQUEST_LOG_ENQUEUE_TIMEOUT" => Self::RequestLogEnqueueTimeout,
            "GW_REQUEST_LOG_WRITE_THROUGH_ON_BACKPRESSURE" => {
                Self::RequestLogWriteThroughOnBackpressure
            }
            "GW_REQUEST_LOG_WRITE_THROUGH_RATE_LIMITED" => Self::RequestLogWriteThroughRateLimited,
            "GW_REQUEST_LOG_DROPPED" => Self::RequestLogDropped,
            "GW_FAKE_200" => Self::Fake200,
            _ => return None,
        })
    }

    pub(in crate::gateway) const fn is_client_abort(self) -> bool {
        matches!(self, Self::RequestAborted | Self::StreamAborted)
    }
}

#[cfg(test)]
mod tests {
    use super::GatewayErrorCode;

    #[test]
    fn round_trip_known_error_codes() {
        let codes = [
            GatewayErrorCode::UpstreamTimeout,
            GatewayErrorCode::StreamError,
            GatewayErrorCode::ProviderRateLimited,
            GatewayErrorCode::PortInUse,
            GatewayErrorCode::RequestLogDropped,
        ];

        for code in codes {
            let parsed = GatewayErrorCode::from_str(code.as_str());
            assert_eq!(parsed, Some(code));
        }
    }

    #[test]
    fn unknown_code_returns_none() {
        assert_eq!(GatewayErrorCode::from_str("GW_UNKNOWN"), None);
    }

    #[test]
    fn client_abort_flags() {
        assert!(GatewayErrorCode::RequestAborted.is_client_abort());
        assert!(GatewayErrorCode::StreamAborted.is_client_abort());
        assert!(!GatewayErrorCode::UpstreamTimeout.is_client_abort());
    }
}
