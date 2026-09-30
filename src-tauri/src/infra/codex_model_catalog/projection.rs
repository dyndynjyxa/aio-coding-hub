//! Build the temporary Codex catalog used by the local CLI proxy.

use crate::providers::{ProviderModelEligibility, ProviderModelMode, ProviderModelPolicyV1};
use crate::{cli_manager, codex_paths, db, providers};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(crate) const AIO_CODEX_MODEL_CATALOG_FILENAME: &str = "aio-codex-model-catalog.json";

pub(crate) fn is_aio_owned_catalog_pointer(pointer: &str, codex_home: &Path) -> bool {
    let referenced = Path::new(pointer);
    let resolved = if referenced.is_absolute() {
        referenced.to_path_buf()
    } else {
        codex_home.join(referenced)
    };
    resolved == codex_home.join(AIO_CODEX_MODEL_CATALOG_FILENAME)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogProjection {
    pub(crate) bytes: Vec<u8>,
    pub(crate) affected_sources: Vec<String>,
}

pub(crate) fn build_for_proxy<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    original_config: Option<&[u8]>,
    original_aio_catalog: Option<&[u8]>,
    db_override: Option<&db::Db>,
) -> crate::shared::error::AppResult<Option<CatalogProjection>> {
    let policies = load_routable_ready_policies(app, db_override)?;
    if mapping_source_signature(&policies).is_empty() {
        return Ok(None);
    }

    let launch = cli_manager::codex_launch_spec(app)?
        .ok_or_else(|| "CLI_PROXY_CODEX_CATALOG_FAILED: Codex CLI not found".to_string())?;
    let codex_home = codex_paths::codex_home_dir(app)?;
    let bundled_bytes = cli_manager::codex_bundled_model_catalog_json(&launch, &codex_home)?;
    let bundled = parse_catalog_json(&bundled_bytes, "bundled Codex catalog")?;
    let user_catalog = load_user_catalog(original_config, original_aio_catalog, &codex_home)?;

    build_projection(&bundled, user_catalog.as_ref(), &policies).map_err(Into::into)
}

pub(crate) fn parse_catalog_json(bytes: &[u8], label: &str) -> Result<Value, String> {
    let value = serde_json::from_slice::<Value>(bytes).map_err(|error| {
        format!("CLI_PROXY_CODEX_CATALOG_FAILED: {label} is invalid JSON: {error}")
    })?;
    validate_catalog_shape(&value, label)?;
    Ok(value)
}

/// Inputs must come from `parse_catalog_json`, which already validated the catalog shape.
pub(crate) fn build_projection(
    bundled: &Value,
    user_catalog: Option<&Value>,
    policies: &[ProviderModelPolicyV1],
) -> Result<Option<CatalogProjection>, String> {
    let mut models = merge_models(bundled, user_catalog)?;
    let baseline_slugs = models
        .iter()
        .filter_map(model_slug)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let affected_sources = collect_affected_sources(&baseline_slugs, policies);
    if affected_sources.is_empty() {
        return Ok(None);
    }

    let template = models
        .first()
        .cloned()
        .ok_or_else(|| "CLI_PROXY_CODEX_CATALOG_FAILED: catalog has no models".to_string())?;
    let affected = affected_sources.iter().collect::<HashSet<_>>();
    let mut seen = HashSet::new();

    for model in &mut models {
        let Some(slug) = model_slug(model).map(str::to_string) else {
            return Err(
                "CLI_PROXY_CODEX_CATALOG_FAILED: catalog model slug is required".to_string(),
            );
        };
        if affected.contains(&slug) {
            apply_function_compatible_capability(model);
        }
        seen.insert(slug);
    }

    for source in &affected_sources {
        if seen.contains(source) {
            continue;
        }
        let mut model = template.clone();
        let Some(object) = model.as_object_mut() else {
            return Err(
                "CLI_PROXY_CODEX_CATALOG_FAILED: catalog model must be an object".to_string(),
            );
        };
        object.insert("slug".to_string(), Value::String(source.clone()));
        object.insert("display_name".to_string(), Value::String(source.clone()));
        object.insert("description".to_string(), Value::String(source.clone()));
        apply_function_compatible_capability(&mut model);
        models.push(model);
    }

    let mut root = Map::new();
    for source in [Some(bundled), user_catalog] {
        if let Some(object) = source.and_then(Value::as_object) {
            for (key, value) in object {
                if key != "models" {
                    root.insert(key.clone(), value.clone());
                }
            }
        }
    }
    root.insert("models".to_string(), Value::Array(models));
    let mut bytes = serde_json::to_vec_pretty(&Value::Object(root))
        .map_err(|error| format!("CLI_PROXY_CODEX_CATALOG_FAILED: serialize catalog: {error}"))?;
    bytes.push(b'\n');
    if bytes.len() > super::CODEX_CATALOG_MAX_BYTES {
        return Err(format!(
            "CLI_PROXY_CODEX_CATALOG_FAILED: projected catalog exceeds {} bytes",
            super::CODEX_CATALOG_MAX_BYTES
        ));
    }

    Ok(Some(CatalogProjection {
        bytes,
        affected_sources,
    }))
}

fn validate_catalog_shape(value: &Value, label: &str) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Err(format!(
            "CLI_PROXY_CODEX_CATALOG_FAILED: {label} root must be an object"
        ));
    };
    let Some(models) = object.get("models").and_then(Value::as_array) else {
        return Err(format!(
            "CLI_PROXY_CODEX_CATALOG_FAILED: {label} models must be an array"
        ));
    };
    for model in models {
        let Some(model_object) = model.as_object() else {
            return Err(format!(
                "CLI_PROXY_CODEX_CATALOG_FAILED: {label} model must be an object"
            ));
        };
        let slug = model_object
            .get("slug")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|slug| !slug.is_empty());
        if slug.is_none() {
            return Err(format!(
                "CLI_PROXY_CODEX_CATALOG_FAILED: {label} model slug is required"
            ));
        }
    }
    if models.is_empty() {
        return Err(format!(
            "CLI_PROXY_CODEX_CATALOG_FAILED: {label} has no models"
        ));
    }
    Ok(())
}

fn load_routable_ready_policies<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db_override: Option<&db::Db>,
) -> crate::shared::error::AppResult<Vec<ProviderModelPolicyV1>> {
    if let Some(db) = db_override {
        providers::list_ready_model_policies_for_configured_routes(db, "codex")
    } else {
        let db_path = db::db_path(app)?;
        if !db_path.exists() {
            return Ok(Vec::new());
        }
        let db = db::init(app)?;
        providers::list_ready_model_policies_for_configured_routes(&db, "codex")
    }
}

pub(crate) fn routable_mapping_signature(
    db: &db::Db,
) -> crate::shared::error::AppResult<Vec<(String, Vec<String>)>> {
    let policies = providers::list_ready_model_policies_for_configured_routes(db, "codex")?;
    Ok(mapping_policy_signature(&policies))
}

fn mapping_source_signature(policies: &[ProviderModelPolicyV1]) -> Vec<String> {
    let mut sources = policies
        .iter()
        .flat_map(|policy| policy.mappings.iter().map(|mapping| mapping.source.clone()))
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    sources
}

fn mapping_policy_signature(policies: &[ProviderModelPolicyV1]) -> Vec<(String, Vec<String>)> {
    let mut entries = Vec::new();
    for policy in policies {
        let excluded_patterns = if policy.mode == ProviderModelMode::Excluded {
            policy.model_patterns.clone()
        } else {
            Vec::new()
        };
        for mapping in &policy.mappings {
            if !mapping.source.contains('*')
                && policy.eligibility(&mapping.source) != ProviderModelEligibility::Explicit
            {
                continue;
            }
            entries.push((mapping.source.clone(), excluded_patterns.clone()));
        }
    }
    entries.sort();
    entries.dedup();
    entries
}

fn load_user_catalog(
    original_config: Option<&[u8]>,
    original_aio_catalog: Option<&[u8]>,
    codex_home: &Path,
) -> crate::shared::error::AppResult<Option<Value>> {
    let Some(config) = original_config else {
        return Ok(None);
    };
    let config = std::str::from_utf8(config).map_err(|error| {
        format!("CLI_PROXY_CODEX_CATALOG_FAILED: config.toml is not UTF-8: {error}")
    })?;
    let document = config.parse::<toml::Value>().map_err(|error| {
        format!("CLI_PROXY_CODEX_CATALOG_FAILED: config.toml is invalid TOML: {error}")
    })?;
    let Some(pointer) = document
        .get("model_catalog_json")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|pointer| !pointer.is_empty())
    else {
        return Ok(None);
    };

    let path = if Path::new(pointer).is_absolute() {
        Path::new(pointer).to_path_buf()
    } else {
        codex_home.join(pointer)
    };
    let bytes = if is_aio_owned_catalog_pointer(pointer, codex_home) {
        let Some(bytes) = original_aio_catalog else {
            return Ok(None);
        };
        bytes.to_owned()
    } else {
        crate::shared::fs::read_file_with_max_len(&path, super::CODEX_CATALOG_MAX_BYTES)?
    };

    parse_catalog_json(&bytes, &format!("user Codex catalog {}", path.display()))
        .map(Some)
        .map_err(Into::into)
}

fn merge_models(bundled: &Value, user_catalog: Option<&Value>) -> Result<Vec<Value>, String> {
    let mut models = Vec::new();
    let mut positions = HashMap::new();

    for source in [Some(bundled), user_catalog] {
        let Some(source) = source else { continue };
        let source_models = source
            .get("models")
            .and_then(Value::as_array)
            .ok_or_else(|| "CLI_PROXY_CODEX_CATALOG_FAILED: models must be an array".to_string())?;
        for model in source_models {
            let slug = model_slug(model)
                .ok_or_else(|| {
                    "CLI_PROXY_CODEX_CATALOG_FAILED: model slug is required".to_string()
                })?
                .to_string();
            if let Some(index) = positions.get(&slug).copied() {
                models[index] = model.clone();
            } else {
                positions.insert(slug, models.len());
                models.push(model.clone());
            }
        }
    }

    Ok(models)
}

fn collect_affected_sources(
    baseline_slugs: &[String],
    policies: &[ProviderModelPolicyV1],
) -> Vec<String> {
    let mut exact = HashSet::new();
    let mut wildcard_matches = HashSet::new();

    for policy in policies {
        for mapping in &policy.mappings {
            if !mapping.source.contains('*')
                && policy.eligibility(&mapping.source) == ProviderModelEligibility::Explicit
            {
                exact.insert(mapping.source.clone());
            }
        }
        for slug in baseline_slugs {
            if policy.has_mapping_match(slug)
                && policy.eligibility(slug) == ProviderModelEligibility::Explicit
            {
                wildcard_matches.insert(slug.clone());
            }
        }
    }

    let mut sources = exact.into_iter().collect::<Vec<_>>();
    sources.extend(wildcard_matches);
    sources.sort();
    sources.dedup();
    sources
}

fn model_slug(value: &Value) -> Option<&str> {
    value
        .get("slug")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
}

fn apply_function_compatible_capability(value: &mut Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    object.insert(
        "shell_type".to_string(),
        Value::String("shell_command".to_string()),
    );
    object.remove("apply_patch_tool_type");
    object.remove("web_search_tool_type");
    object.remove("tools");
    object.remove("tool_mode");
    object.remove("use_responses_lite");
    object.insert(
        "supports_parallel_tool_calls".to_string(),
        Value::Bool(false),
    );
    object.insert("supports_search_tool".to_string(), Value::Bool(false));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{
        DailyResetMode, ProviderAuthMode, ProviderBaseUrlMode, ProviderModelMapping,
        ProviderModelMode, ProviderUpsertParams,
    };

    fn provider_params(name: &str, enabled: bool, source: &str) -> ProviderUpsertParams {
        ProviderUpsertParams {
            provider_id: None,
            cli_key: "codex".to_string(),
            name: name.to_string(),
            base_urls: vec!["https://api.example.com/v1".to_string()],
            base_url_mode: ProviderBaseUrlMode::Order,
            auth_mode: Some(ProviderAuthMode::ApiKey),
            api_key: Some("sk-test".to_string()),
            enabled,
            cost_multiplier: 1.0,
            priority: Some(100),
            claude_models: None,
            model_policy: Some(policy(&[(source, "target")])),
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: Some(DailyResetMode::Fixed),
            daily_reset_time: Some("00:00:00".to_string()),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: None,
            note: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            extension_values: None,
        }
    }

    fn policy(mappings: &[(&str, &str)]) -> ProviderModelPolicyV1 {
        ProviderModelPolicyV1 {
            version: 1,
            mode: ProviderModelMode::All,
            model_patterns: Vec::new(),
            mappings: mappings
                .iter()
                .map(|(source, target)| ProviderModelMapping {
                    source: (*source).to_string(),
                    target: (*target).to_string(),
                })
                .collect(),
        }
    }

    fn bundled() -> Value {
        serde_json::json!({
            "models": [
                {
                    "slug": "gpt-5.6-luna",
                    "display_name": "Luna",
                    "apply_patch_tool_type": "freeform",
                    "web_search_tool_type": "text_and_image",
                    "supports_parallel_tool_calls": true,
                    "supports_search_tool": true,
                    "tools": [{"type": "custom", "name": "apply_patch"}],
                    "tool_mode": "code_mode_only",
                    "use_responses_lite": true,
                    "model_messages": {"instructions_template": "keep me"},
                    "shell_type": "shell_command"
                },
                {"slug": "gpt-5.6-sol", "display_name": "Sol"}
            ],
            "minimal_client_version": "0.1.0"
        })
    }

    #[test]
    fn exact_mapping_downgrades_existing_source_and_keeps_full_baseline() {
        let result = build_projection(&bundled(), None, &[policy(&[("gpt-5.6-luna", "deepseek")])])
            .expect("projection")
            .expect("mapping projection");
        let value: Value = serde_json::from_slice(&result.bytes).expect("json");
        assert_eq!(value["models"].as_array().unwrap().len(), 2);
        let luna = &value["models"][0];
        assert_eq!(luna["slug"], "gpt-5.6-luna");
        assert_eq!(luna["shell_type"], "shell_command");
        assert_eq!(luna["supports_parallel_tool_calls"], false);
        assert_eq!(luna["supports_search_tool"], false);
        assert!(luna.get("apply_patch_tool_type").is_none());
        assert!(luna.get("web_search_tool_type").is_none());
        assert!(luna.get("tools").is_none());
        assert!(luna.get("tool_mode").is_none());
        assert!(luna.get("use_responses_lite").is_none());
        assert_eq!(luna["model_messages"]["instructions_template"], "keep me");
        assert_eq!(result.affected_sources, vec!["gpt-5.6-luna"]);
    }

    #[test]
    fn exact_mapping_adds_unknown_source_but_wildcard_does_not() {
        let result = build_projection(
            &bundled(),
            None,
            &[policy(&[
                ("vendor-special", "deepseek"),
                ("gpt-*", "other-*"),
            ])],
        )
        .expect("projection")
        .expect("mapping projection");
        let value: Value = serde_json::from_slice(&result.bytes).expect("json");
        let slugs = value["models"]
            .as_array()
            .unwrap()
            .iter()
            .map(|model| model["slug"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(slugs, vec!["gpt-5.6-luna", "gpt-5.6-sol", "vendor-special"]);
    }

    #[test]
    fn user_catalog_overrides_same_slug_and_keeps_unique_entry() {
        let user = serde_json::json!({
            "models": [
                {"slug": "gpt-5.6-sol", "display_name": "User Sol", "custom": true},
                {"slug": "user-model", "display_name": "User Model"}
            ],
            "user_key": "preserved"
        });
        let result = build_projection(
            &bundled(),
            Some(&user),
            &[policy(&[("user-model", "target")])],
        )
        .expect("projection")
        .expect("mapping projection");
        let value: Value = serde_json::from_slice(&result.bytes).expect("json");
        assert_eq!(value["user_key"], "preserved");
        assert_eq!(value["models"][1]["display_name"], "User Sol");
        assert_eq!(value["models"][1]["custom"], true);
        assert_eq!(value["models"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn user_catalog_absolute_pointer_can_live_outside_codex_home() {
        let codex_home = tempfile::tempdir().expect("codex home");
        let catalog_dir = tempfile::tempdir().expect("catalog dir");
        let catalog_path = catalog_dir.path().join("user-models.json");
        std::fs::write(&catalog_path, br#"{"models":[{"slug":"external-model"}]}"#)
            .expect("write catalog");
        let config = format!(
            "model_catalog_json = {}\n",
            toml::Value::String(catalog_path.to_string_lossy().into_owned())
        );

        let value = load_user_catalog(Some(config.as_bytes()), None, codex_home.path())
            .expect("read user catalog")
            .expect("catalog");

        assert_eq!(value["models"][0]["slug"], "external-model");
    }

    #[test]
    fn aio_catalog_pointer_matches_only_the_owned_target() {
        let codex_home = tempfile::tempdir().expect("codex home");
        let absolute = codex_home.path().join(AIO_CODEX_MODEL_CATALOG_FILENAME);

        assert!(is_aio_owned_catalog_pointer(
            AIO_CODEX_MODEL_CATALOG_FILENAME,
            codex_home.path()
        ));
        assert!(is_aio_owned_catalog_pointer(
            absolute.to_string_lossy().as_ref(),
            codex_home.path()
        ));
        assert!(!is_aio_owned_catalog_pointer(
            "user-models.json",
            codex_home.path()
        ));
        assert!(!is_aio_owned_catalog_pointer(
            tempfile::tempdir()
                .expect("external dir")
                .path()
                .join(AIO_CODEX_MODEL_CATALOG_FILENAME)
                .to_string_lossy()
                .as_ref(),
            codex_home.path()
        ));
    }

    #[test]
    fn orphaned_aio_catalog_pointer_is_not_treated_as_a_user_catalog() {
        let codex_home = tempfile::tempdir().expect("codex home");
        let config = format!(
            "model_catalog_json = {}\n",
            toml::Value::String(AIO_CODEX_MODEL_CATALOG_FILENAME.to_string())
        );

        let value = load_user_catalog(Some(config.as_bytes()), None, codex_home.path())
            .expect("orphaned AIO pointer should be ignored");

        assert!(value.is_none());
    }

    #[test]
    fn missing_external_user_catalog_still_fails() {
        let codex_home = tempfile::tempdir().expect("codex home");
        let config = "model_catalog_json = \"missing-user-models.json\"\n";

        let error = load_user_catalog(Some(config.as_bytes()), None, codex_home.path())
            .expect_err("missing external user catalog must fail");

        assert!(error.to_string().contains("missing-user-models.json"));
    }

    #[test]
    fn no_mapping_returns_no_projection() {
        assert!(build_projection(&bundled(), None, &[])
            .expect("projection")
            .is_none());
    }

    #[test]
    fn duplicate_provider_sources_share_one_projection_entry() {
        let mappings = [("gpt-5.6-luna", "deepseek")];
        let result = build_projection(&bundled(), None, &[policy(&mappings), policy(&mappings)])
            .expect("projection")
            .expect("mapping projection");

        assert_eq!(result.affected_sources, vec!["gpt-5.6-luna"]);
        let value: Value = serde_json::from_slice(&result.bytes).expect("json");
        assert_eq!(
            value["models"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|model| model["slug"] == "gpt-5.6-luna")
                .count(),
            1
        );
    }

    #[test]
    fn mapping_source_signature_ignores_targets_and_provider_duplicates() {
        let first = policy(&[("gpt-5.6-luna", "deepseek-a"), ("vendor-*", "target-*")]);
        let second = policy(&[("gpt-5.6-luna", "deepseek-b")]);

        assert_eq!(
            mapping_source_signature(&[first, second]),
            vec!["gpt-5.6-luna", "vendor-*"]
        );
    }

    #[test]
    fn mapping_policy_signature_tracks_exclusions_but_ignores_targets_and_duplicates() {
        let all = policy(&[("gpt-*", "deepseek-*")]);
        let duplicate_with_other_target = policy(&[("gpt-*", "other-*")]);
        let mut excluded = policy(&[("gpt-*", "deepseek-*")]);
        excluded.mode = ProviderModelMode::Excluded;
        excluded.model_patterns = vec!["gpt-5.6-*".to_string()];

        assert_eq!(
            mapping_policy_signature(&[all, duplicate_with_other_target, excluded]),
            vec![
                ("gpt-*".to_string(), vec![]),
                ("gpt-*".to_string(), vec!["gpt-5.6-*".to_string()])
            ]
        );
    }

    #[test]
    fn excluded_mapping_source_is_not_projected_when_provider_blocks_it() {
        let mut excluded = policy(&[("gpt-5.6-luna", "deepseek-v4-flash")]);
        excluded.mode = ProviderModelMode::Excluded;
        excluded.model_patterns = vec!["gpt-5.6-*".to_string()];

        assert!(build_projection(&bundled(), None, &[excluded])
            .expect("projection")
            .is_none());
    }

    #[test]
    fn long_unicode_and_large_mapping_sets_are_not_truncated() {
        let long = format!("source-{}", "模".repeat(201));
        let mappings = (0..501)
            .map(|index| (format!("model-{index}"), "target".to_string()))
            .collect::<Vec<_>>();
        let policy = ProviderModelPolicyV1 {
            version: 1,
            mode: ProviderModelMode::All,
            model_patterns: Vec::new(),
            mappings: mappings
                .into_iter()
                .map(|(source, target)| ProviderModelMapping { source, target })
                .chain(std::iter::once(ProviderModelMapping {
                    source: long.clone(),
                    target: "target".to_string(),
                }))
                .collect(),
        };
        let result = build_projection(&bundled(), None, &[policy])
            .expect("projection")
            .expect("mapping projection");
        assert!(result.affected_sources.contains(&long));
        assert!(result
            .bytes
            .windows(long.len())
            .any(|window| window == long.as_bytes()));
    }

    #[test]
    fn routable_sources_follow_default_and_all_configured_sort_modes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db =
            db::init_for_tests(&dir.path().join("routable-codex-mappings.db")).expect("init db");
        let default = providers::upsert(&db, provider_params("default", true, "source-default"))
            .expect("insert default provider");
        let disabled_default = providers::upsert(
            &db,
            provider_params("disabled-default", false, "source-disabled-default"),
        )
        .expect("insert disabled default provider");
        let sort_only = providers::upsert(&db, provider_params("sort-only", false, "source-sort"))
            .expect("insert sort provider");
        let disabled_sort = providers::upsert(
            &db,
            provider_params("disabled-sort", true, "source-disabled-sort"),
        )
        .expect("insert disabled sort provider");
        let invalid = providers::upsert(&db, provider_params("invalid", true, "source-invalid"))
            .expect("insert invalid provider");
        let unconfigured = providers::upsert(
            &db,
            provider_params("unconfigured", true, "source-unconfigured"),
        )
        .expect("insert unconfigured provider");

        providers::default_route_set_order(
            &db,
            "codex",
            vec![default.id, disabled_default.id, invalid.id],
        )
        .expect("set default route");
        let inactive_mode =
            crate::sort_modes::create_mode(&db, "Inactive").expect("create inactive mode");
        crate::sort_modes::set_mode_providers_order(
            &db,
            inactive_mode.id,
            "codex",
            vec![sort_only.id],
        )
        .expect("set inactive mode providers");
        let active_mode =
            crate::sort_modes::create_mode(&db, "Active").expect("create active mode");
        crate::sort_modes::set_mode_providers_order(
            &db,
            active_mode.id,
            "codex",
            vec![disabled_sort.id],
        )
        .expect("set active mode providers");
        crate::sort_modes::set_mode_provider_enabled(
            &db,
            active_mode.id,
            "codex",
            disabled_sort.id,
            false,
        )
        .expect("disable active mode provider");
        crate::sort_modes::set_active(&db, "codex", Some(active_mode.id)).expect("activate mode");
        {
            let conn = db.open_connection().expect("open db");
            conn.execute(
                "UPDATE providers SET model_policy_json = ?1 WHERE id = ?2",
                rusqlite::params![r#"{"version":99}"#, invalid.id],
            )
            .expect("invalidate policy");
        }

        assert_eq!(
            routable_mapping_signature(&db).expect("list routable mappings"),
            vec![
                ("source-default".to_string(), vec![]),
                ("source-sort".to_string(), vec![])
            ]
        );

        crate::sort_modes::set_active(&db, "codex", Some(inactive_mode.id))
            .expect("switch active mode");
        assert_eq!(
            routable_mapping_signature(&db).expect("list after active switch"),
            vec![
                ("source-default".to_string(), vec![]),
                ("source-sort".to_string(), vec![])
            ]
        );

        providers::default_route_set_order(&db, "codex", vec![]).expect("clear default route");
        assert_eq!(
            routable_mapping_signature(&db).expect("list after default removal"),
            vec![("source-sort".to_string(), vec![])]
        );

        crate::sort_modes::set_mode_provider_enabled(
            &db,
            inactive_mode.id,
            "codex",
            sort_only.id,
            false,
        )
        .expect("disable sort provider");
        assert!(routable_mapping_signature(&db)
            .expect("list after sort disable")
            .is_empty());

        crate::sort_modes::set_mode_provider_enabled(
            &db,
            inactive_mode.id,
            "codex",
            sort_only.id,
            true,
        )
        .expect("enable sort provider");
        crate::sort_modes::delete_mode(&db, inactive_mode.id).expect("delete sort mode");
        assert!(routable_mapping_signature(&db)
            .expect("list after mode delete")
            .is_empty());

        assert!(unconfigured.enabled);
    }

    #[test]
    fn duplicate_mapping_source_does_not_change_catalog_signature() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db =
            db::init_for_tests(&dir.path().join("duplicate-codex-mapping.db")).expect("init db");
        let source = providers::upsert(&db, provider_params("source", true, "gpt-5.6-luna"))
            .expect("insert source provider");
        providers::default_route_set_order(&db, "codex", vec![source.id])
            .expect("set default route");
        let before = routable_mapping_signature(&db).expect("list before duplicate");

        providers::duplicate(
            &db,
            source.id,
            provider_params("source copy", true, "gpt-5.6-luna"),
        )
        .expect("duplicate provider");

        assert_eq!(
            routable_mapping_signature(&db).expect("list after duplicate"),
            before
        );
    }
}
