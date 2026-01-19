//! Usage: Small shared types for the gateway proxy module.

#[derive(Debug, Clone, Copy)]
pub(in crate::gateway) enum ErrorCategory {
    SystemError,
    ProviderError,
    NonRetryableClientError,
    ResourceNotFound,
    ClientAbort,
}

impl ErrorCategory {
    pub(in crate::gateway) fn as_str(self) -> &'static str {
        match self {
            Self::SystemError => "SYSTEM_ERROR",
            Self::ProviderError => "PROVIDER_ERROR",
            Self::NonRetryableClientError => "NON_RETRYABLE_CLIENT_ERROR",
            Self::ResourceNotFound => "RESOURCE_NOT_FOUND",
            Self::ClientAbort => "CLIENT_ABORT",
        }
    }
}
