//! Usage: Gateway plugin permission trimming and result enforcement.

use super::context::{GatewayHookResult, GatewayPluginHookName};
use super::pipeline::GatewayPluginAuditEvent;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GatewayPluginError {
    code: &'static str,
    message: String,
    audit_events: Vec<GatewayPluginAuditEvent>,
}

impl GatewayPluginError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            audit_events: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn code(&self) -> &'static str {
        self.code
    }

    pub(crate) fn with_audit_events(
        mut self,
        mut audit_events: Vec<GatewayPluginAuditEvent>,
    ) -> Self {
        audit_events.extend(self.audit_events);
        self.audit_events = audit_events;
        self
    }

    #[cfg(test)]
    pub(crate) fn audit_events(&self) -> &[GatewayPluginAuditEvent] {
        &self.audit_events
    }

    pub(crate) fn take_audit_events(&mut self) -> Vec<GatewayPluginAuditEvent> {
        std::mem::take(&mut self.audit_events)
    }
}

impl fmt::Display for GatewayPluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for GatewayPluginError {}

#[cfg(test)]
pub(crate) fn permission_allows_hook_access(
    permission: &str,
    hook_name: GatewayPluginHookName,
) -> bool {
    match permission {
        "request.meta.read"
        | "request.header.read"
        | "request.header.readSensitive"
        | "request.header.write"
        | "request.body.read"
        | "request.body.write" => hook_name.is_request_hook(),
        "response.header.read"
        | "response.header.write"
        | "response.body.read"
        | "response.body.write" => {
            matches!(
                hook_name,
                GatewayPluginHookName::ResponseHeaders
                    | GatewayPluginHookName::ResponseAfter
                    | GatewayPluginHookName::Error
            )
        }
        "stream.inspect" | "stream.modify" => {
            matches!(hook_name, GatewayPluginHookName::ResponseChunk)
        }
        "log.redact" => matches!(hook_name, GatewayPluginHookName::LogBeforePersist),
        "plugin.storage" => true,
        "network.fetch" | "file.read" | "file.write" | "secret.read" => false,
        _ => false,
    }
}

pub(crate) fn enforce_hook_result_permissions(
    hook_name: GatewayPluginHookName,
    permissions: &[String],
    result: &GatewayHookResult,
) -> Result<(), GatewayPluginError> {
    if result.request_body.is_some() {
        require_hook(
            hook_name.is_request_hook(),
            "request body mutation",
            hook_name,
        )?;
        require_permission(permissions, "request.body.write")?;
    }
    if result.response_body.is_some() {
        require_hook(
            matches!(
                hook_name,
                GatewayPluginHookName::ResponseAfter | GatewayPluginHookName::Error
            ),
            "response body mutation",
            hook_name,
        )?;
        require_permission(permissions, "response.body.write")?;
    }
    if result.stream_chunk.is_some() {
        require_hook(
            matches!(hook_name, GatewayPluginHookName::ResponseChunk),
            "stream chunk mutation",
            hook_name,
        )?;
        require_permission(permissions, "stream.modify")?;
    }
    if !result.headers.is_empty() {
        if hook_name.is_request_hook() {
            require_permission(permissions, "request.header.write")?;
        } else if hook_name.is_response_hook() {
            require_permission(permissions, "response.header.write")?;
        } else {
            return Err(GatewayPluginError::new(
                "PLUGIN_PERMISSION_DENIED",
                format!("headers cannot be mutated in {}", hook_name.as_str()),
            ));
        }
    }
    if result.log_message.is_some() {
        require_hook(
            matches!(hook_name, GatewayPluginHookName::LogBeforePersist),
            "log mutation",
            hook_name,
        )?;
        require_permission(permissions, "log.redact")?;
    }
    Ok(())
}

fn require_permission(
    permissions: &[String],
    permission: &'static str,
) -> Result<(), GatewayPluginError> {
    if permissions.iter().any(|item| item == permission) {
        Ok(())
    } else {
        Err(GatewayPluginError::new(
            "PLUGIN_PERMISSION_DENIED",
            format!("missing plugin permission: {permission}"),
        ))
    }
}

fn require_hook(
    allowed: bool,
    operation: &'static str,
    hook_name: GatewayPluginHookName,
) -> Result<(), GatewayPluginError> {
    if allowed {
        Ok(())
    } else {
        Err(GatewayPluginError::new(
            "PLUGIN_PERMISSION_DENIED",
            format!("{operation} is not allowed in {}", hook_name.as_str()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::plugins::context::GatewayHookAction;

    #[test]
    fn gateway_plugin_context_permission_allows_expected_hook_access() {
        assert!(permission_allows_hook_access(
            "request.body.read",
            GatewayPluginHookName::RequestAfterBodyRead
        ));
        assert!(permission_allows_hook_access(
            "response.body.write",
            GatewayPluginHookName::ResponseAfter
        ));
        assert!(permission_allows_hook_access(
            "stream.modify",
            GatewayPluginHookName::ResponseChunk
        ));
        assert!(!permission_allows_hook_access(
            "stream.modify",
            GatewayPluginHookName::RequestBeforeSend
        ));
    }

    #[test]
    fn gateway_plugin_context_permission_enforces_write_permissions() {
        let request_result = GatewayHookResult {
            action: GatewayHookAction::Continue,
            request_body: Some("changed".to_string()),
            response_body: None,
            stream_chunk: None,
            headers: Default::default(),
            log_message: None,
            reason: None,
        };
        let err = enforce_hook_result_permissions(
            GatewayPluginHookName::RequestBeforeSend,
            &[],
            &request_result,
        )
        .expect_err("request body write should require permission");
        assert_eq!(err.code(), "PLUGIN_PERMISSION_DENIED");

        enforce_hook_result_permissions(
            GatewayPluginHookName::RequestBeforeSend,
            &["request.body.write".to_string()],
            &request_result,
        )
        .expect("request body write granted");

        let stream_result = GatewayHookResult {
            stream_chunk: Some("data: changed\n\n".to_string()),
            ..GatewayHookResult::continue_unchanged()
        };
        let err = enforce_hook_result_permissions(
            GatewayPluginHookName::ResponseChunk,
            &["stream.inspect".to_string()],
            &stream_result,
        )
        .expect_err("stream modify should require stream.modify");
        assert_eq!(err.code(), "PLUGIN_PERMISSION_DENIED");
    }
}
