use serde::{Deserialize, Serialize};

pub type PluginId = String;

const SUPPORTED_PLUGIN_API_MAJOR: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
pub struct PluginManifest {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub runtime: PluginRuntime,
    pub hooks: Vec<PluginHook>,
    pub permissions: Vec<String>,
    #[serde(rename = "hostCompatibility")]
    pub host_compatibility: PluginHostCompatibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(rename = "configSchema")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<serde_json::Value>,
    #[serde(rename = "configVersion")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PluginRuntime {
    DeclarativeRules {
        rules: Vec<String>,
    },
    Native {
        engine: String,
    },
    Wasm {
        #[serde(rename = "abiVersion")]
        abi_version: String,
        #[serde(rename = "memoryLimitBytes")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        memory_limit_bytes: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct PluginHook {
    pub name: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(rename = "failurePolicy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct PluginHostCompatibility {
    pub app: String,
    #[serde(rename = "pluginApi")]
    pub plugin_api: String,
    #[serde(default)]
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    Available,
    Installed,
    Enabled,
    Disabled,
    UpdateAvailable,
    Incompatible,
    Quarantined,
    Uninstalled,
}

impl PluginStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Installed => "installed",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::UpdateAvailable => "update_available",
            Self::Incompatible => "incompatible",
            Self::Quarantined => "quarantined",
            Self::Uninstalled => "uninstalled",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "available" => Some(Self::Available),
            "installed" => Some(Self::Installed),
            "enabled" => Some(Self::Enabled),
            "disabled" => Some(Self::Disabled),
            "update_available" => Some(Self::UpdateAvailable),
            "incompatible" => Some(Self::Incompatible),
            "quarantined" => Some(Self::Quarantined),
            "uninstalled" => Some(Self::Uninstalled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginInstallSource {
    Local,
    Market,
    GithubRelease,
    Offline,
    Official,
}

impl PluginInstallSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Market => "market",
            Self::GithubRelease => "github_release",
            Self::Offline => "offline",
            Self::Official => "official",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "local" => Some(Self::Local),
            "market" => Some(Self::Market),
            "github_release" => Some(Self::GithubRelease),
            "offline" => Some(Self::Offline),
            "official" => Some(Self::Official),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermissionRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct PluginValidationError {
    pub code: String,
    pub message: String,
}

impl PluginValidationError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
pub struct PluginSummary {
    pub id: i64,
    pub plugin_id: String,
    pub name: String,
    pub current_version: Option<String>,
    pub status: PluginStatus,
    pub runtime: String,
    pub permission_risk: PluginPermissionRisk,
    pub update_available: bool,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
pub struct PluginDetail {
    pub summary: PluginSummary,
    pub manifest: PluginManifest,
    pub install_source: PluginInstallSource,
    pub installed_dir: Option<String>,
    pub config: serde_json::Value,
    pub granted_permissions: Vec<String>,
    pub pending_permissions: Vec<String>,
    pub audit_logs: Vec<PluginAuditLog>,
    pub runtime_failures: Vec<PluginRuntimeFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
pub struct PluginAuditLog {
    pub id: i64,
    pub plugin_id: Option<String>,
    pub trace_id: Option<String>,
    pub event_type: String,
    pub risk_level: String,
    pub message: String,
    pub details: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq)]
pub struct PluginRuntimeFailure {
    pub id: i64,
    pub plugin_id: String,
    pub hook_name: Option<String>,
    pub failure_kind: String,
    pub message: String,
    pub trace_id: Option<String>,
    pub created_at: i64,
}

impl From<PluginValidationError> for crate::shared::error::AppError {
    fn from(value: PluginValidationError) -> Self {
        crate::shared::error::AppError::new(value.code, value.message)
    }
}

pub fn validate_manifest(
    manifest: &PluginManifest,
    host_version: &str,
) -> Result<(), PluginValidationError> {
    validate_plugin_id(&manifest.id)?;
    validate_semver(&manifest.version, "PLUGIN_INVALID_VERSION")?;
    validate_semver(&manifest.api_version, "PLUGIN_INVALID_API_VERSION")?;
    validate_runtime(manifest)?;
    validate_hooks(&manifest.hooks)?;
    validate_permissions(&manifest.permissions)?;
    validate_hook_permissions(&manifest.hooks, &manifest.permissions)?;
    validate_permission_scope(&manifest.hooks, &manifest.permissions)?;
    validate_config_schema(manifest.config_schema.as_ref())?;
    validate_host_compatibility(&manifest.host_compatibility, host_version)?;
    Ok(())
}

pub fn permission_risk(permission: &str) -> Option<PluginPermissionRisk> {
    match permission {
        "request.meta.read" => Some(PluginPermissionRisk::Low),
        "request.header.read" => Some(PluginPermissionRisk::Medium),
        "request.header.readSensitive" => Some(PluginPermissionRisk::High),
        "request.header.write" => Some(PluginPermissionRisk::High),
        "request.body.read" => Some(PluginPermissionRisk::High),
        "request.body.write" => Some(PluginPermissionRisk::High),
        "response.header.read" => Some(PluginPermissionRisk::Low),
        "response.header.write" => Some(PluginPermissionRisk::Medium),
        "response.body.read" => Some(PluginPermissionRisk::High),
        "response.body.write" => Some(PluginPermissionRisk::High),
        "stream.inspect" => Some(PluginPermissionRisk::High),
        "stream.modify" => Some(PluginPermissionRisk::High),
        "log.redact" => Some(PluginPermissionRisk::Medium),
        "plugin.storage" => Some(PluginPermissionRisk::Medium),
        "network.fetch" => Some(PluginPermissionRisk::High),
        "file.read" => Some(PluginPermissionRisk::High),
        "file.write" => Some(PluginPermissionRisk::High),
        "secret.read" => Some(PluginPermissionRisk::Critical),
        _ => None,
    }
}

pub fn is_known_hook(hook: &str) -> bool {
    is_active_gateway_hook(hook) || is_reserved_gateway_hook(hook)
}

pub fn is_active_gateway_hook(hook: &str) -> bool {
    matches!(
        hook,
        "gateway.request.afterBodyRead"
            | "gateway.request.beforeSend"
            | "gateway.response.chunk"
            | "gateway.response.after"
            | "gateway.error"
            | "log.beforePersist"
    )
}

pub fn is_reserved_gateway_hook(hook: &str) -> bool {
    matches!(
        hook,
        "gateway.request.received"
            | "gateway.request.beforeProviderResolution"
            | "gateway.response.headers"
    )
}

pub fn is_reserved_permission(permission: &str) -> bool {
    matches!(
        permission,
        "plugin.storage" | "network.fetch" | "file.read" | "file.write" | "secret.read"
    )
}

fn validate_plugin_id(plugin_id: &str) -> Result<(), PluginValidationError> {
    if crate::app_paths::plugin_id_path_segment(plugin_id).is_err() || !plugin_id.contains('.') {
        return Err(PluginValidationError::new(
            "PLUGIN_INVALID_ID",
            "plugin id must match publisher.plugin-name",
        ));
    }
    Ok(())
}

fn validate_semver(version: &str, code: &str) -> Result<(), PluginValidationError> {
    parse_semver(version)
        .map(|_| ())
        .ok_or_else(|| PluginValidationError::new(code, format!("invalid SemVer: {version}")))
}

fn validate_runtime(manifest: &PluginManifest) -> Result<(), PluginValidationError> {
    match &manifest.runtime {
        PluginRuntime::DeclarativeRules { rules } => {
            if rules.is_empty() {
                return Err(PluginValidationError::new(
                    "PLUGIN_INVALID_RUNTIME",
                    "declarativeRules runtime requires at least one rules file",
                ));
            }
        }
        PluginRuntime::Native { engine } => {
            if manifest.id != "official.privacy-filter" || engine != "privacyFilter" {
                return Err(PluginValidationError::new(
                    "PLUGIN_UNSUPPORTED_RUNTIME",
                    "native runtime is reserved for official plugins",
                ));
            }
        }
        PluginRuntime::Wasm { abi_version, .. } => {
            let Some((major, _, _)) = parse_semver(abi_version) else {
                return Err(PluginValidationError::new(
                    "PLUGIN_INVALID_RUNTIME",
                    "WASM abiVersion must be SemVer",
                ));
            };
            if major != SUPPORTED_PLUGIN_API_MAJOR {
                return Err(PluginValidationError::new(
                    "PLUGIN_UNSUPPORTED_RUNTIME",
                    "WASM ABI major version is not supported",
                ));
            }
        }
    }
    Ok(())
}

fn validate_hooks(hooks: &[PluginHook]) -> Result<(), PluginValidationError> {
    if hooks.is_empty() {
        return Err(PluginValidationError::new(
            "PLUGIN_MISSING_HOOKS",
            "plugin must declare at least one hook",
        ));
    }
    for hook in hooks {
        if is_reserved_gateway_hook(&hook.name) {
            return Err(PluginValidationError::new(
                "PLUGIN_RESERVED_HOOK",
                format!(
                    "hook is reserved for a future host integration and is not active in plugin API v1: {}",
                    hook.name
                ),
            ));
        }
        if !is_known_hook(&hook.name) {
            return Err(PluginValidationError::new(
                "PLUGIN_UNKNOWN_HOOK",
                format!("unknown hook: {}", hook.name),
            ));
        }
    }
    Ok(())
}

fn validate_permissions(permissions: &[String]) -> Result<(), PluginValidationError> {
    for permission in permissions {
        if is_reserved_permission(permission) {
            return Err(PluginValidationError::new(
                "PLUGIN_RESERVED_PERMISSION",
                format!(
                    "permission is reserved for a future host-mediated API and is not active in plugin API v1: {permission}"
                ),
            ));
        }
        if permission_risk(permission).is_none() {
            return Err(PluginValidationError::new(
                "PLUGIN_UNKNOWN_PERMISSION",
                format!("unknown permission: {permission}"),
            ));
        }
    }
    Ok(())
}

fn validate_hook_permissions(
    hooks: &[PluginHook],
    permissions: &[String],
) -> Result<(), PluginValidationError> {
    let has = |permission: &str| permissions.iter().any(|item| item == permission);
    for hook in hooks {
        match hook.name.as_str() {
            "gateway.request.afterBodyRead"
                if has("request.body.write") && !has("request.body.read") =>
            {
                return Err(PluginValidationError::new(
                    "PLUGIN_PERMISSION_MISMATCH",
                    "request.body.write requires request.body.read",
                ));
            }
            "gateway.response.chunk" if has("stream.modify") && !has("stream.inspect") => {
                return Err(PluginValidationError::new(
                    "PLUGIN_PERMISSION_MISMATCH",
                    "stream.modify requires stream.inspect",
                ));
            }
            "gateway.response.after"
                if has("response.body.write") && !has("response.body.read") =>
            {
                return Err(PluginValidationError::new(
                    "PLUGIN_PERMISSION_MISMATCH",
                    "response.body.write requires response.body.read",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn hook_allows_permission(hook_name: &str, permission: &str) -> bool {
    match permission {
        "request.meta.read"
        | "request.header.read"
        | "request.header.readSensitive"
        | "request.header.write"
        | "request.body.read"
        | "request.body.write" => matches!(
            hook_name,
            "gateway.request.afterBodyRead" | "gateway.request.beforeSend"
        ),
        "response.header.read"
        | "response.header.write"
        | "response.body.read"
        | "response.body.write" => {
            matches!(hook_name, "gateway.response.after" | "gateway.error")
        }
        "stream.inspect" | "stream.modify" => hook_name == "gateway.response.chunk",
        "log.redact" => hook_name == "log.beforePersist",
        _ => false,
    }
}

fn validate_permission_scope(
    hooks: &[PluginHook],
    permissions: &[String],
) -> Result<(), PluginValidationError> {
    for permission in permissions {
        if is_reserved_permission(permission) {
            continue;
        }
        let allowed = hooks
            .iter()
            .any(|hook| hook_allows_permission(&hook.name, permission));
        if !allowed {
            return Err(PluginValidationError::new(
                "PLUGIN_PERMISSION_SCOPE_MISMATCH",
                format!("permission {permission} does not apply to any declared hook"),
            ));
        }
    }
    Ok(())
}

fn validate_config_schema(schema: Option<&serde_json::Value>) -> Result<(), PluginValidationError> {
    let Some(schema) = schema else {
        return Ok(());
    };
    validate_schema_node(schema)
}

fn validate_schema_node(schema: &serde_json::Value) -> Result<(), PluginValidationError> {
    let Some(schema_type) = schema.get("type").and_then(|value| value.as_str()) else {
        return Err(PluginValidationError::new(
            "PLUGIN_INVALID_CONFIG_SCHEMA",
            "config schema node must declare type",
        ));
    };

    match schema_type {
        "string" | "number" | "integer" | "boolean" | "password" => Ok(()),
        "array" => {
            if let Some(items) = schema.get("items") {
                validate_schema_node(items)?;
            }
            Ok(())
        }
        "object" => {
            if let Some(properties) = schema.get("properties").and_then(|value| value.as_object()) {
                for property in properties.values() {
                    validate_schema_node(property)?;
                }
            }
            Ok(())
        }
        _ => Err(PluginValidationError::new(
            "PLUGIN_INVALID_CONFIG_SCHEMA",
            format!("unsupported config schema type: {schema_type}"),
        )),
    }
}

fn validate_host_compatibility(
    compatibility: &PluginHostCompatibility,
    host_version: &str,
) -> Result<(), PluginValidationError> {
    let Some(host) = parse_semver(host_version) else {
        return Err(PluginValidationError::new(
            "PLUGIN_INVALID_HOST_VERSION",
            format!("invalid host version: {host_version}"),
        ));
    };

    if !matches_version_range(&compatibility.app, host) {
        return Err(PluginValidationError::new(
            "PLUGIN_INCOMPATIBLE_HOST",
            format!(
                "host version {host_version} does not satisfy {}",
                compatibility.app
            ),
        ));
    }

    let plugin_api = compatibility.plugin_api.trim();
    let api_supported = if let Some(required) = plugin_api.strip_prefix('^') {
        parse_semver(required).is_some_and(|(major, _, _)| major == SUPPORTED_PLUGIN_API_MAJOR)
    } else {
        parse_semver(plugin_api).is_some_and(|(major, _, _)| major == SUPPORTED_PLUGIN_API_MAJOR)
    };
    if !api_supported {
        return Err(PluginValidationError::new(
            "PLUGIN_INCOMPATIBLE_API",
            format!("plugin API range is not supported: {plugin_api}"),
        ));
    }

    Ok(())
}

fn matches_version_range(range: &str, version: (u64, u64, u64)) -> bool {
    let mut saw_constraint = false;
    for part in range.split_whitespace() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        saw_constraint = true;
        if let Some(raw) = part.strip_prefix(">=") {
            let Some(bound) = parse_semver(raw) else {
                return false;
            };
            if version < bound {
                return false;
            }
        } else if let Some(raw) = part.strip_prefix('<') {
            let Some(bound) = parse_semver(raw) else {
                return false;
            };
            if version >= bound {
                return false;
            }
        } else if let Some(raw) = part.strip_prefix('^') {
            let Some(bound) = parse_semver(raw) else {
                return false;
            };
            if version < bound || version.0 != bound.0 {
                return false;
            }
        } else {
            let Some(bound) = parse_semver(part) else {
                return false;
            };
            if version != bound {
                return false;
            }
        }
    }
    saw_constraint
}

fn parse_semver(version: &str) -> Option<(u64, u64, u64)> {
    let core = version
        .trim()
        .split_once('-')
        .map_or(version.trim(), |(core, _)| core);
    let mut parts = core.split('.');
    let major = parse_semver_number(parts.next()?)?;
    let minor = parse_semver_number(parts.next()?)?;
    let patch = parse_semver_number(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn parse_semver_number(value: &str) -> Option<u64> {
    if value.is_empty() || (value.len() > 1 && value.starts_with('0')) {
        return None;
    }
    value.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> serde_json::Value {
        serde_json::json!({
            "id": "community.prompt-helper",
            "name": "Community Prompt Helper",
            "version": "1.0.0",
            "apiVersion": "1.0.0",
            "runtime": {
                "kind": "declarativeRules",
                "rules": ["rules/main.json"]
            },
            "hooks": [
                {
                    "name": "gateway.request.afterBodyRead",
                    "priority": 100,
                    "failurePolicy": "fail-open"
                }
            ],
            "permissions": ["request.body.read", "request.body.write"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            }
        })
    }

    #[test]
    fn manifest_json_deserializes_and_validates() {
        let manifest: PluginManifest = serde_json::from_value(valid_manifest()).unwrap();
        validate_manifest(&manifest, "0.56.0").unwrap();
        assert_eq!(manifest.id.as_str(), "community.prompt-helper");
    }

    #[test]
    fn manifest_rejects_unknown_permission() {
        let mut raw = valid_manifest();
        raw["permissions"] = serde_json::json!(["request.body.read", "unknown.permission"]);
        let manifest: PluginManifest = serde_json::from_value(raw).unwrap();
        let err = validate_manifest(&manifest, "0.56.0").unwrap_err();
        assert_eq!(err.code, "PLUGIN_UNKNOWN_PERMISSION");
    }

    #[test]
    fn manifest_rejects_unknown_hook() {
        let mut raw = valid_manifest();
        raw["hooks"][0]["name"] = serde_json::json!("gateway.request.missing");
        let manifest: PluginManifest = serde_json::from_value(raw).unwrap();
        let err = validate_manifest(&manifest, "0.56.0").unwrap_err();
        assert_eq!(err.code, "PLUGIN_UNKNOWN_HOOK");
    }

    #[test]
    fn validate_manifest_rejects_reserved_hook_until_it_is_wired() {
        let mut raw = valid_manifest();
        raw["hooks"][0]["name"] = serde_json::json!("gateway.request.received");
        raw["permissions"] = serde_json::json!(["request.meta.read"]);
        let manifest: PluginManifest = serde_json::from_value(raw).unwrap();
        let err = validate_manifest(&manifest, "0.56.0").unwrap_err();
        assert_eq!(err.code, "PLUGIN_RESERVED_HOOK");
        assert!(err.message.contains("gateway.request.received"));
    }

    #[test]
    fn validate_manifest_accepts_active_vnext_hooks() {
        let cases = [
            (
                "gateway.request.afterBodyRead",
                serde_json::json!(["request.body.read", "request.body.write"]),
            ),
            (
                "gateway.request.beforeSend",
                serde_json::json!(["request.body.read", "request.body.write"]),
            ),
            (
                "gateway.response.chunk",
                serde_json::json!(["stream.inspect", "stream.modify"]),
            ),
            (
                "gateway.response.after",
                serde_json::json!(["response.body.read", "response.body.write"]),
            ),
            ("gateway.error", serde_json::json!(["response.body.read"])),
            ("log.beforePersist", serde_json::json!(["log.redact"])),
        ];

        for (hook_name, permissions) in cases {
            let mut raw = valid_manifest();
            raw["hooks"][0]["name"] = serde_json::json!(hook_name);
            raw["permissions"] = permissions;
            let manifest: PluginManifest = serde_json::from_value(raw).unwrap();
            validate_manifest(&manifest, "0.56.0")
                .unwrap_or_else(|err| panic!("active hook {hook_name} rejected: {err:?}"));
        }
    }

    #[test]
    fn validate_manifest_rejects_reserved_permissions_until_host_apis_exist() {
        for permission in [
            "plugin.storage",
            "network.fetch",
            "file.read",
            "file.write",
            "secret.read",
        ] {
            let mut raw = valid_manifest();
            raw["permissions"] =
                serde_json::json!(["request.body.read", "request.body.write", permission]);
            let manifest: PluginManifest = serde_json::from_value(raw).unwrap();
            let err = validate_manifest(&manifest, "0.56.0").unwrap_err();
            assert_eq!(err.code, "PLUGIN_RESERVED_PERMISSION");
            assert!(err.message.contains(permission));
        }
    }

    #[test]
    fn manifest_rejects_permissions_that_do_not_apply_to_declared_hooks() {
        let mut raw = valid_manifest();
        raw["hooks"] = serde_json::json!([
            { "name": "log.beforePersist", "priority": 10, "failurePolicy": "fail-open" }
        ]);
        raw["permissions"] = serde_json::json!(["request.body.read", "log.redact"]);
        let manifest: PluginManifest = serde_json::from_value(raw).unwrap();
        let err = validate_manifest(&manifest, "0.56.0").unwrap_err();
        assert_eq!(err.code, "PLUGIN_PERMISSION_SCOPE_MISMATCH");
        assert!(err.message.contains("request.body.read"));
    }

    #[test]
    fn manifest_rejects_incompatible_host() {
        let mut raw = valid_manifest();
        raw["hostCompatibility"]["app"] = serde_json::json!(">=9.0.0");
        let manifest: PluginManifest = serde_json::from_value(raw).unwrap();
        let err = validate_manifest(&manifest, "0.56.0").unwrap_err();
        assert_eq!(err.code, "PLUGIN_INCOMPATIBLE_HOST");
    }

    #[test]
    fn manifest_rejects_invalid_runtime() {
        let mut raw = valid_manifest();
        raw["runtime"] = serde_json::json!({ "kind": "node" });
        let err = serde_json::from_value::<PluginManifest>(raw).unwrap_err();
        assert!(err.to_string().contains("unknown variant"));
    }

    #[test]
    fn manifest_allows_only_official_privacy_filter_native_runtime() {
        let mut official = valid_manifest();
        official["id"] = serde_json::json!("official.privacy-filter");
        official["runtime"] = serde_json::json!({
            "kind": "native",
            "engine": "privacyFilter"
        });
        let manifest: PluginManifest = serde_json::from_value(official).unwrap();
        validate_manifest(&manifest, "0.56.0").unwrap();

        let mut local = valid_manifest();
        local["id"] = serde_json::json!("local.privacy-filter");
        local["runtime"] = serde_json::json!({
            "kind": "native",
            "engine": "privacyFilter"
        });
        let manifest: PluginManifest = serde_json::from_value(local).unwrap();
        let err = validate_manifest(&manifest, "0.56.0").unwrap_err();
        assert_eq!(err.code, "PLUGIN_UNSUPPORTED_RUNTIME");
    }
}
