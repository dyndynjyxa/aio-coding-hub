//! Usage: Built-in official plugin catalog.

use crate::plugins::{validate_manifest, PluginManifest};
use crate::shared::error::{AppError, AppResult};
use serde_json::Value;
use std::path::{Path, PathBuf};

const OFFICIAL_SOURCE_RESOURCE_ROOT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/resources/plugins/official");
pub(crate) const OFFICIAL_RESOURCE_RELATIVE_ROOT: &str = "resources/plugins/official";

pub(crate) struct OfficialPluginFixture {
    pub(crate) manifest: PluginManifest,
    pub(crate) root_dir: PathBuf,
    pub(crate) default_config: Value,
}

pub(crate) fn official_plugin(plugin_id: &str) -> AppResult<OfficialPluginFixture> {
    official_plugin_from_root(plugin_id, &official_source_resource_root())
}

pub(crate) fn official_plugin_from_root(
    plugin_id: &str,
    official_resource_root: &Path,
) -> AppResult<OfficialPluginFixture> {
    let root_dir = official_plugin_root(plugin_id, official_resource_root)?;
    let manifest_path = root_dir.join("plugin.json");
    let bytes = crate::shared::fs::read_file_with_max_len(&manifest_path, 256 * 1024)?;
    let manifest: PluginManifest = serde_json::from_slice(&bytes).map_err(|err| {
        AppError::new(
            "PLUGIN_INVALID_MANIFEST",
            format!("failed to parse official plugin manifest: {err}"),
        )
    })?;
    if manifest.id != plugin_id {
        return Err(AppError::new(
            "PLUGIN_INVALID_MANIFEST",
            format!(
                "official plugin manifest id mismatch: expected {plugin_id}, got {}",
                manifest.id
            ),
        ));
    }
    validate_manifest(&manifest, env!("CARGO_PKG_VERSION"))?;
    let default_config = official_default_config(plugin_id);

    Ok(OfficialPluginFixture {
        manifest,
        root_dir,
        default_config,
    })
}

pub(crate) fn official_plugin_ids() -> &'static [&'static str] {
    &["official.privacy-filter"]
}

pub(crate) fn official_source_resource_root() -> PathBuf {
    PathBuf::from(OFFICIAL_SOURCE_RESOURCE_ROOT)
}

#[cfg(test)]
pub(crate) fn official_resource_root_for_tests() -> PathBuf {
    official_source_resource_root()
}

fn official_plugin_root(plugin_id: &str, official_resource_root: &Path) -> AppResult<PathBuf> {
    let name = match plugin_id {
        "official.privacy-filter" => "privacy-filter",
        _ => {
            let known = official_plugin_ids().join(", ");
            return Err(AppError::new(
                "PLUGIN_UNKNOWN_OFFICIAL_PLUGIN",
                format!("unknown official plugin: {plugin_id}; expected one of: {known}"),
            ));
        }
    };
    Ok(official_resource_root.join(name))
}

fn official_default_config(plugin_id: &str) -> Value {
    match plugin_id {
        "official.privacy-filter" => serde_json::json!({
            "redactBeforeUpstream": true,
            "redactLogs": true,
            "profile": "balanced",
            "redactionScopes": [
                "system_instructions",
                "user_prompts",
                "tool_results",
                "legacy_prompt"
            ],
            "sensitiveTypes": [
                "email",
                "cn_phone",
                "cn_id_card",
                "bank_card_candidate",
                "ipv4",
                "openai_key",
                "aws_access_key",
                "github_token",
                "google_api_key",
                "slack_token",
                "jwt",
                "private_key",
                "context_secret"
            ]
        }),
        _ => serde_json::json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::plugins::runtime_executor::RuntimeGatewayPluginExecutor;
    use crate::domain::plugins::{PluginInstallSource, PluginRuntime, PluginStatus};
    use crate::gateway::plugins::context::{GatewayPluginHookName, GatewayRequestHookInput};
    use crate::gateway::plugins::pipeline::{GatewayPluginPipeline, GatewayPluginPipelineConfig};
    use axum::body::Bytes;
    use axum::http::{HeaderMap, Method};
    use serde_json::json;
    use std::sync::Arc;

    fn enabled_official_plugin(plugin_id: &str) -> crate::domain::plugins::PluginDetail {
        let fixture = official_plugin(plugin_id).expect("official plugin fixture");
        let permissions = fixture.manifest.permissions.clone();
        let runtime = match &fixture.manifest.runtime {
            PluginRuntime::DeclarativeRules { .. } => "declarativeRules".to_string(),
            PluginRuntime::Native { engine } => format!("native:{engine}"),
            PluginRuntime::Wasm { .. } => "wasm".to_string(),
        };
        crate::domain::plugins::PluginDetail {
            summary: crate::domain::plugins::PluginSummary {
                id: 1,
                plugin_id: fixture.manifest.id.clone(),
                name: fixture.manifest.name.clone(),
                current_version: Some(fixture.manifest.version.clone()),
                status: PluginStatus::Enabled,
                runtime,
                permission_risk: crate::domain::plugins::PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: fixture.manifest,
            install_source: PluginInstallSource::Official,
            installed_dir: Some(fixture.root_dir.to_string_lossy().to_string()),
            config: fixture.default_config,
            granted_permissions: permissions,
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        }
    }

    #[test]
    fn official_catalog_exposes_only_privacy_filter() {
        assert_eq!(official_plugin_ids(), &["official.privacy-filter"]);

        match official_plugin("official.redactor") {
            Ok(_) => panic!("retired official plugin should not be available"),
            Err(err) => assert!(err
                .to_string()
                .starts_with("PLUGIN_UNKNOWN_OFFICIAL_PLUGIN:")),
        }
    }

    #[test]
    fn official_catalog_uses_packaged_privacy_filter_resource_root() {
        let fixture = official_plugin("official.privacy-filter").expect("official plugin fixture");
        let root = fixture.root_dir.to_string_lossy();
        assert!(
            root.contains("resources/plugins/official/privacy-filter"),
            "official plugin root must be a packaged resource path, got {root}"
        );
    }

    #[test]
    fn official_privacy_filter_schema_explains_strategy_groups_and_gitleaks_coverage() {
        let fixture = official_plugin("official.privacy-filter").expect("official plugin fixture");
        let schema = fixture
            .manifest
            .config_schema
            .as_ref()
            .expect("config schema");
        let sensitive_description = schema
            .pointer("/properties/sensitiveTypes/description")
            .and_then(Value::as_str)
            .expect("sensitiveTypes description");
        assert!(sensitive_description.contains("200+ Gitleaks"));

        let section_description = schema
            .pointer("/x-aio-ui/sections/1/description")
            .and_then(Value::as_str)
            .expect("content section description");
        assert!(section_description.contains("策略大类"));
    }

    #[test]
    fn official_privacy_filter_loads_full_packaged_gitleaks_rule_set() {
        let fixture = official_plugin("official.privacy-filter").expect("official plugin fixture");
        let raw = std::fs::read_to_string(fixture.root_dir.join("rules/gitleaks.toml"))
            .expect("read packaged gitleaks rules");
        let filter = crate::app::plugins::privacy_filter::PrivacyFilter::from_gitleaks_toml(&raw)
            .expect("compile packaged gitleaks rules");
        let stats = filter.stats();
        assert!(
            stats.rules >= 200,
            "packaged privacy-filter must expose the full gitleaks rule set, got {} compiled rules",
            stats.rules
        );
    }

    #[tokio::test]
    async fn official_privacy_filter_plugin_redacts_pii_and_secrets_before_upstream_and_logs() {
        let plugin = enabled_official_plugin("official.privacy-filter");
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin],
            Arc::new(RuntimeGatewayPluginExecutor::default()),
            GatewayPluginPipelineConfig::default(),
        );

        let request = pipeline
            .run_request_hook(GatewayRequestHookInput {
                hook_name: GatewayPluginHookName::RequestAfterBodyRead,
                trace_id: "trace-privacy-filter".to_string(),
                cli_key: "codex".to_string(),
                method: Method::POST,
                path: "/v1/chat/completions".to_string(),
                query: None,
                headers: HeaderMap::new(),
                body: Bytes::from(
                    json!({
                        "messages": [{
                            "role": "user",
                            "content": concat!(
                                "邮箱 test.user@example.com 手机 13812345678 ",
                                "身份证 11010519900307743X ",
                                "Authorization: Bearer abcDEF1234567890/xyzABC4567890== ",
                                "OpenAI sk-proj-abcdefghijklmnopqrstuvwxyz123456"
                            )
                        }],
                        "input": "api_key = aB3xK9pLmN2qR7sT5vW1zYQwErTyUiOp"
                    })
                    .to_string(),
                ),
                requested_model: Some("gpt-test".to_string()),
            })
            .await
            .expect("privacy filter request hook");
        let request_text = String::from_utf8(request.body.to_vec()).expect("utf8 body");

        assert!(request_text.contains("[邮箱]"));
        assert!(request_text.contains("[电话]"));
        assert!(request_text.contains("[身份证]"));
        assert!(request_text.contains("Bearer [密钥]"));
        assert!(request_text.contains("[密钥]"));
        assert!(!request_text.contains("test.user@example.com"));
        assert!(!request_text.contains("13812345678"));
        assert!(!request_text.contains("11010519900307743X"));
        assert!(!request_text.contains("sk-proj-abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!request_text.contains("aB3xK9pLmN2qR7sT5vW1zYQwErTyUiOp"));

        let log = pipeline
            .run_log_hook(crate::gateway::plugins::context::GatewayLogHookInput {
                trace_id: "trace-privacy-filter".to_string(),
                message: concat!(
                    "ip=192.168.1.10 github=ghp_abcdefghijklmnopqrstuvwxyzABCDEFGHIJ ",
                    "aws=AKIAIOSFODNN7EXAMPLE"
                )
                .to_string(),
            })
            .await
            .expect("privacy filter log hook");

        assert!(log.message.contains("[IP]"));
        assert!(log.message.contains("[密钥]"));
        assert!(!log.message.contains("192.168.1.10"));
        assert!(!log
            .message
            .contains("ghp_abcdefghijklmnopqrstuvwxyzABCDEFGHIJ"));
    }

    #[tokio::test]
    async fn official_privacy_filter_plugin_redacts_responses_input_text_parts() {
        let plugin = enabled_official_plugin("official.privacy-filter");
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin],
            Arc::new(RuntimeGatewayPluginExecutor::default()),
            GatewayPluginPipelineConfig::default(),
        );

        let request = pipeline
            .run_request_hook(GatewayRequestHookInput {
                hook_name: GatewayPluginHookName::RequestAfterBodyRead,
                trace_id: "trace-privacy-filter-responses".to_string(),
                cli_key: "codex".to_string(),
                method: Method::POST,
                path: "/v1/responses".to_string(),
                query: None,
                headers: HeaderMap::new(),
                body: Bytes::from(
                    json!({
                        "input": [{
                            "type": "message",
                            "role": "user",
                            "content": [{
                                "type": "input_text",
                                "text": "你知道 13344441520 是哪里的手机号嘛"
                            }]
                        }]
                    })
                    .to_string(),
                ),
                requested_model: Some("gpt-test".to_string()),
            })
            .await
            .expect("privacy filter request hook");
        let request_text = String::from_utf8(request.body.to_vec()).expect("utf8 body");

        assert!(request_text.contains("[电话]"));
        assert!(!request_text.contains("13344441520"));
    }

    #[tokio::test]
    async fn official_privacy_filter_plugin_matches_upstream_algorithmic_behavior() {
        let plugin = enabled_official_plugin("official.privacy-filter");
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin],
            Arc::new(RuntimeGatewayPluginExecutor::default()),
            GatewayPluginPipelineConfig::default(),
        );

        let request = pipeline
            .run_request_hook(GatewayRequestHookInput {
                hook_name: GatewayPluginHookName::RequestAfterBodyRead,
                trace_id: "trace-privacy-filter-upstream".to_string(),
                cli_key: "codex".to_string(),
                method: Method::POST,
                path: "/v1/responses".to_string(),
                query: None,
                headers: HeaderMap::new(),
                body: Bytes::from(
                    json!({
                        "input": [{
                            "type": "message",
                            "role": "user",
                            "content": [{
                                "type": "input_text",
                                "text": concat!(
                                    "付款卡号 4111111111111111 ",
                                    "订单编号 1234567890123456 ",
                                    "路径 /home/user/AbCdEfGh1234567890XyZ ",
                                    "Authorization: Bearer abcDEF1234567890/xyzABC4567890=="
                                )
                            }]
                        }]
                    })
                    .to_string(),
                ),
                requested_model: Some("gpt-test".to_string()),
            })
            .await
            .expect("privacy filter request hook");
        let request_text = String::from_utf8(request.body.to_vec()).expect("utf8 body");

        assert!(request_text.contains("[银行卡]"));
        assert!(request_text.contains("[密钥]"));
        assert!(request_text.contains("1234567890123456"));
        assert!(request_text.contains("/home/user/AbCdEfGh1234567890XyZ"));
        assert!(!request_text.contains("4111111111111111"));
        assert!(!request_text.contains("abcDEF1234567890/xyzABC4567890=="));
    }
}
