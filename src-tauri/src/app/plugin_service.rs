use crate::app::plugins::contribution_registry::ActiveContributionSnapshot;
use crate::app::plugins::extension_host_registry::ExtensionHostInstanceRegistry;
use crate::domain::plugins::{
    manifest_effective_permissions, permission_risk, validate_manifest,
    validate_manifest_for_official_plugin, PluginCommandImpact, PluginCompatibilitySummary,
    PluginContributionChange, PluginContributionImpact, PluginContributionImpactItem, PluginDetail,
    PluginHookLifecycleSummary, PluginInstallPreview, PluginInstallSource, PluginLifecycleChange,
    PluginLifecycleNotice, PluginManifest, PluginPermissionLifecycleChange,
    PluginPermissionLifecycleSummary, PluginPermissionRisk, PluginRuntime,
    PluginRuntimeLifecycleSummary, PluginStatus, PluginTrustSummary, PluginUiSlotImpact,
    PluginUpdateDiff,
};
use crate::infra::plugins::runtime_reports::{
    record_extension_execution_report, RecordPluginExtensionExecutionReportInput,
};
use crate::infra::plugins::{package, repository, signing};
use crate::shared::error::{AppError, AppResult};
use crate::shared::time::now_unix_millis;
use rusqlite::OptionalExtension;
use std::cmp::Ordering;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

const OFFICIAL_PRIVACY_FILTER_ID: &str = "official.privacy-filter";
const UNSUPPORTED_LEGACY_RUNTIME_ERROR: &str =
    "Unsupported pre-release plugin runtime; reinstall an Extension Host version";
static PLUGIN_WORK_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn list_plugins(db: &crate::db::Db) -> AppResult<Vec<crate::plugins::PluginSummary>> {
    let mut out = Vec::new();
    for summary in repository::list_plugins(db)? {
        out.push(normalize_unsupported_legacy_plugin_summary_for_list(
            db, summary,
        )?);
    }
    Ok(out)
}

pub(crate) fn get_plugin_detail(db: &crate::db::Db, plugin_id: &str) -> AppResult<PluginDetail> {
    Ok(normalize_unsupported_legacy_plugin_detail(
        detail_with_config_defaults_for_db(db, repository::get_plugin(db, plugin_id)?)?,
    ))
}

pub(crate) fn active_plugin_contributions(
    db: &crate::db::Db,
) -> AppResult<ActiveContributionSnapshot> {
    let mut details = Vec::new();
    for summary in list_plugins(db)? {
        details.push(normalize_unsupported_legacy_plugin_detail(
            detail_with_config_defaults_for_db(
                db,
                repository::get_plugin(db, &summary.plugin_id)?,
            )?,
        ));
    }
    let duplicate_plugin_ids = duplicate_enabled_command_plugin_ids(&details);
    let active_details = details
        .into_iter()
        .filter(|detail| !duplicate_plugin_ids.contains(&detail.summary.plugin_id))
        .collect::<Vec<_>>();
    ActiveContributionSnapshot::from_plugin_details(&active_details)
}

pub(crate) async fn execute_plugin_command(
    db: &crate::db::Db,
    registry: &ExtensionHostInstanceRegistry,
    command: &str,
    args: serde_json::Value,
) -> AppResult<serde_json::Value> {
    let command = normalize_plugin_command(command)?;
    let detail = find_unique_enabled_command_owner(db, &command)?.ok_or_else(|| {
        AppError::new(
            "PLUGIN_COMMAND_NOT_FOUND",
            format!("plugin command is not declared: {command}"),
        )
    })?;
    if detail.summary.status != PluginStatus::Enabled {
        return Err(AppError::new(
            "PLUGIN_COMMAND_PLUGIN_DISABLED",
            format!(
                "plugin {} is not enabled for command {command}",
                detail.summary.plugin_id
            ),
        ));
    }
    let plugin_id = detail.summary.plugin_id.clone();
    let trace_id = args
        .get("traceId")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let started_at_ms = now_unix_millis();
    let result = registry
        .execute_command(detail.clone(), &command, args.clone())
        .await;
    let duration_ms = now_unix_millis().saturating_sub(started_at_ms);

    match result {
        Ok(output) => {
            if let Err(report_error) = record_command_execution_report(
                db,
                &plugin_id,
                &command,
                trace_id,
                started_at_ms,
                duration_ms,
                "completed",
                None,
                None,
                &args,
                Some(output.cold_start),
                Some(&output.value),
            ) {
                tracing::warn!(
                    plugin_id,
                    command,
                    error = %report_error,
                    "failed to record successful plugin command execution report"
                );
            }
            Ok(output.value)
        }
        Err(error) => {
            if let Err(report_error) = record_command_execution_report(
                db,
                &plugin_id,
                &command,
                trace_id,
                started_at_ms,
                duration_ms,
                "failed",
                Some("runtime"),
                Some(error.code()),
                &args,
                None,
                None,
            ) {
                tracing::warn!(
                    plugin_id,
                    command,
                    error = %report_error,
                    "failed to record plugin command execution report"
                );
            }
            Err(error)
        }
    }
}

fn normalize_plugin_command(command: &str) -> AppResult<String> {
    let command = command.trim();
    if command.is_empty() {
        return Err(AppError::new(
            "SEC_INVALID_INPUT",
            "plugin command is required",
        ));
    }
    Ok(command.to_string())
}

fn find_unique_enabled_command_owner(
    db: &crate::db::Db,
    command: &str,
) -> AppResult<Option<PluginDetail>> {
    let mut owners = Vec::new();
    let mut disabled_owner = None;
    for summary in list_plugins(db)? {
        let detail =
            normalize_unsupported_legacy_plugin_detail(detail_with_config_defaults_for_db(
                db,
                repository::get_plugin(db, &summary.plugin_id)?,
            )?);
        let declared = detail
            .manifest
            .contributes
            .as_ref()
            .is_some_and(|contributes| {
                contributes
                    .commands
                    .iter()
                    .any(|contribution| contribution.command == command)
            });
        if declared {
            if detail.summary.status == PluginStatus::Enabled {
                owners.push(detail);
            } else if disabled_owner.is_none() {
                disabled_owner = Some(detail);
            }
        }
    }
    match owners.len() {
        0 => Ok(disabled_owner),
        1 => Ok(owners.into_iter().next()),
        _ => {
            let plugin_ids = owners
                .iter()
                .map(|detail| detail.summary.plugin_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Err(AppError::new(
                "PLUGIN_DUPLICATE_COMMAND",
                format!("command {command} is declared by multiple enabled plugins: {plugin_ids}"),
            ))
        }
    }
}

fn duplicate_enabled_command_plugin_ids(details: &[PluginDetail]) -> BTreeSet<String> {
    let mut command_owners = BTreeMap::<String, Vec<String>>::new();
    for detail in details {
        if detail.summary.status != PluginStatus::Enabled {
            continue;
        }
        let Some(contributes) = detail.manifest.contributes.as_ref() else {
            continue;
        };
        for command in &contributes.commands {
            command_owners
                .entry(command.command.clone())
                .or_default()
                .push(detail.summary.plugin_id.clone());
        }
    }

    command_owners
        .into_values()
        .filter(|owners| owners.len() > 1)
        .flatten()
        .collect()
}

fn ensure_no_duplicate_enabled_commands(details: &[PluginDetail]) -> AppResult<()> {
    let mut command_owners = BTreeMap::<String, Vec<String>>::new();
    for detail in details {
        if detail.summary.status != PluginStatus::Enabled {
            continue;
        }
        let Some(contributes) = detail.manifest.contributes.as_ref() else {
            continue;
        };
        for command in &contributes.commands {
            let owners = command_owners.entry(command.command.clone()).or_default();
            owners.push(detail.summary.plugin_id.clone());
            if owners.len() > 1 {
                return Err(AppError::new(
                    "PLUGIN_DUPLICATE_COMMAND",
                    format!(
                        "command {} is declared by multiple enabled plugins: {}",
                        command.command,
                        owners.join(", ")
                    ),
                ));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_command_execution_report(
    db: &crate::db::Db,
    plugin_id: &str,
    command: &str,
    trace_id: Option<String>,
    started_at_ms: i64,
    duration_ms: i64,
    status: &str,
    failure_kind: Option<&str>,
    error_code: Option<&str>,
    args: &serde_json::Value,
    cold_start: Option<bool>,
    output: Option<&serde_json::Value>,
) -> AppResult<()> {
    record_extension_execution_report(
        db,
        RecordPluginExtensionExecutionReportInput {
            plugin_id: plugin_id.to_string(),
            contribution_type: "command".to_string(),
            contribution_id: command.to_string(),
            command_or_hook: Some(command.to_string()),
            trace_id,
            status: status.to_string(),
            started_at_ms,
            duration_ms,
            failure_kind: failure_kind.map(str::to_string),
            error_code: error_code.map(str::to_string),
            input_budget: command_input_budget(args, cold_start),
            output_budget: output
                .map(json_budget)
                .unwrap_or_else(|| serde_json::json!({})),
            mutation_summary: serde_json::json!({ "changed": false }),
            replayable: false,
        },
    )
    .map(|_| ())
}

fn command_input_budget(args: &serde_json::Value, cold_start: Option<bool>) -> serde_json::Value {
    let mut budget = json_budget(args);
    if let Some(cold_start) = cold_start {
        if let Some(object) = budget.as_object_mut() {
            object.insert("coldStart".to_string(), serde_json::Value::Bool(cold_start));
        }
    }
    budget
}

fn json_budget(value: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "bytes": serde_json::to_vec(value).map(|bytes| bytes.len()).unwrap_or(0)
    })
}

pub(crate) fn enabled_plugins_for_gateway(db: &crate::db::Db) -> AppResult<Vec<PluginDetail>> {
    match enabled_plugins_for_gateway_once(db) {
        Ok(plugins) => Ok(plugins),
        Err(err) if is_missing_plugin_table_error(&err) => {
            tracing::warn!(
                error = %err,
                "plugin schema missing while loading gateway plugins; repairing runtime schema"
            );
            crate::db::ensure_runtime_schema(db)?;
            enabled_plugins_for_gateway_once(db)
        }
        Err(err) => Err(err),
    }
}

fn enabled_plugins_for_gateway_once(db: &crate::db::Db) -> AppResult<Vec<PluginDetail>> {
    let mut out = Vec::new();
    for summary in list_plugins(db)? {
        if summary.status != PluginStatus::Enabled {
            continue;
        }
        let detail =
            normalize_unsupported_legacy_plugin_detail(detail_with_config_defaults_for_db(
                db,
                repository::get_plugin(db, &summary.plugin_id)?,
            )?);
        if is_unsupported_legacy_runtime_detail(&detail) {
            continue;
        }
        if let Err(err) = validate_manifest_for_source(
            &detail.manifest,
            detail.install_source,
            env!("CARGO_PKG_VERSION"),
        ) {
            tracing::warn!(
                plugin_id = %summary.plugin_id,
                error = ?err,
                "skipping enabled plugin with invalid manifest"
            );
            continue;
        }
        out.push(detail);
    }
    Ok(out)
}

fn is_missing_plugin_table_error(err: &AppError) -> bool {
    let message = err.to_string();
    message.contains("no such table: plugins") || message.contains("no such table: plugin_")
}

fn is_unsupported_legacy_runtime_summary(runtime: &str) -> bool {
    matches!(runtime, "wasm" | "process" | "native") || runtime.starts_with("native:")
}

fn is_unsupported_legacy_runtime_detail(detail: &PluginDetail) -> bool {
    is_unsupported_legacy_runtime_summary(&detail.summary.runtime)
        || matches!(detail.manifest.runtime, PluginRuntime::ExtensionHost { .. })
            && detail.manifest.main.as_deref() == Some("legacy/unsupported.js")
}

fn normalize_unsupported_legacy_plugin_summary_for_list(
    _db: &crate::db::Db,
    summary: crate::plugins::PluginSummary,
) -> AppResult<crate::plugins::PluginSummary> {
    Ok(normalize_unsupported_legacy_plugin_summary(summary))
}

fn normalize_unsupported_legacy_plugin_summary(
    mut summary: crate::plugins::PluginSummary,
) -> crate::plugins::PluginSummary {
    if is_unsupported_legacy_runtime_summary(&summary.runtime) {
        summary = mark_unsupported_legacy_plugin_summary(summary);
    }
    summary
}

fn normalize_unsupported_legacy_plugin_detail(mut detail: PluginDetail) -> PluginDetail {
    if is_unsupported_legacy_runtime_detail(&detail) {
        detail.summary = mark_unsupported_legacy_plugin_summary(detail.summary);
    }
    detail
}

fn mark_unsupported_legacy_plugin_summary(
    mut summary: crate::plugins::PluginSummary,
) -> crate::plugins::PluginSummary {
    summary.status = PluginStatus::Disabled;
    summary.update_available = false;
    summary.last_error = Some(UNSUPPORTED_LEGACY_RUNTIME_ERROR.to_string());
    summary
}

pub(crate) fn install_official_plugin(
    db: &crate::db::Db,
    plugin_id: &str,
    official_resource_root: &Path,
    host_version: &str,
    installed_root: &Path,
) -> AppResult<PluginDetail> {
    let fixture = crate::app::plugins::official::official_plugin_from_root(
        plugin_id,
        official_resource_root,
    )?;
    let installed_dir = crate::app::plugins::official_assets::materialize_official_plugin(
        plugin_id,
        &fixture.root_dir,
        installed_root,
        &fixture.manifest.version,
    )?;
    install_plugin_manifest(
        db,
        fixture.manifest.clone(),
        PluginInstallSource::Official,
        Some(installed_dir.to_string_lossy().to_string()),
        host_version,
    )?;
    repository::save_plugin_config(
        db,
        plugin_id,
        fixture.manifest.config_version.unwrap_or(1),
        &fixture.default_config,
        &[],
    )?;
    let detail = repository::save_plugin_permissions(db, plugin_id, &[], &[])?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.official.installed",
        "low",
        "Official plugin installed",
        serde_json::json!({ "source": "official" }),
    )?;
    Ok(detail)
}

pub(crate) fn install_plugin_manifest(
    db: &crate::db::Db,
    manifest: PluginManifest,
    install_source: PluginInstallSource,
    installed_dir: Option<String>,
    host_version: &str,
) -> AppResult<PluginDetail> {
    validate_manifest_for_source(&manifest, install_source, host_version)?;
    validate_reserved_official_source(&manifest, install_source)?;
    let plugin_id = manifest.id.clone();
    let detail = repository::insert_plugin(
        db,
        repository::InsertPluginInput {
            manifest,
            install_source,
            status: PluginStatus::Disabled,
            installed_dir,
        },
    )?;
    let detail = if install_source == PluginInstallSource::Official {
        detail
    } else {
        repository::save_plugin_permissions(db, &plugin_id, &[], &[])?
    };
    append_audit(
        db,
        Some(plugin_id.clone()),
        "plugin.installed",
        "low",
        "Plugin installed",
        serde_json::json!({ "source": install_source.as_str() }),
    )?;
    Ok(detail)
}

pub(crate) fn install_plugin_from_local_package(
    db: &crate::db::Db,
    package_path: &Path,
    cache_dir: &Path,
    installed_root: &Path,
    host_version: &str,
) -> AppResult<PluginDetail> {
    install_plugin_from_local_package_with_policy(
        db,
        package_path,
        cache_dir,
        installed_root,
        host_version,
        LocalPackageInstallPolicy::default(),
    )
}

#[derive(Debug, Clone)]
pub(crate) struct LocalPackageInstallPolicy {
    pub(crate) expected_plugin_id: Option<String>,
    pub(crate) expected_checksum: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) public_key: Option<String>,
    pub(crate) developer_mode: bool,
    pub(crate) install_source: PluginInstallSource,
    pub(crate) remote_source_url: Option<String>,
    pub(crate) market_source_url: Option<String>,
}

impl Default for LocalPackageInstallPolicy {
    fn default() -> Self {
        Self {
            expected_plugin_id: None,
            expected_checksum: None,
            signature: None,
            public_key: None,
            developer_mode: false,
            install_source: PluginInstallSource::Local,
            remote_source_url: None,
            market_source_url: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemotePackageInstallPolicy {
    pub(crate) install_source: PluginInstallSource,
    pub(crate) expected_plugin_id: String,
    pub(crate) expected_checksum: String,
    pub(crate) signature: Option<String>,
    pub(crate) public_key: Option<String>,
    pub(crate) market_source_url: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct PackageTrust {
    signature_verified: bool,
}

fn lifecycle_notice(
    severity: &str,
    code: &str,
    message: impl Into<String>,
) -> PluginLifecycleNotice {
    PluginLifecycleNotice {
        severity: severity.to_string(),
        code: code.to_string(),
        message: message.into(),
    }
}

fn cleanup_staging_dir(staging_root: &Path, staging_dir: &Path) {
    let _ = std::fs::remove_dir_all(staging_dir);
    let _ = std::fs::remove_dir(staging_root);
}

fn safe_work_path_label(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    let out = out.trim_matches(['.', '-']);
    if out.is_empty() {
        fallback.to_string()
    } else {
        out.to_string()
    }
}

fn unique_work_path_segment(prefix: &str) -> String {
    let counter = PLUGIN_WORK_PATH_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
    format!(
        "{}-{}-{}-{}",
        safe_work_path_label(prefix, "plugin"),
        now_unix_millis(),
        std::process::id(),
        counter
    )
}

fn unique_staging_dir(staging_root: &Path, prefix: &str) -> PathBuf {
    staging_root.join(unique_work_path_segment(prefix))
}

fn unique_cache_package_path(cache_dir: &Path, plugin_id: &str, version: &str) -> PathBuf {
    let prefix = format!(
        "{}-{}",
        safe_work_path_label(plugin_id, "plugin"),
        safe_work_path_label(version, "version")
    );
    cache_dir.join(format!("{}.aio-plugin", unique_work_path_segment(&prefix)))
}

fn unique_remote_package_path(cache_dir: &Path, plugin_id: &str) -> PathBuf {
    let prefix = format!("remote-{}", safe_work_path_label(plugin_id, "plugin"));
    cache_dir.join(format!("{}.aio-plugin", unique_work_path_segment(&prefix)))
}

fn app_error_message(error: &AppError) -> String {
    let rendered = error.to_string();
    rendered
        .split_once(':')
        .map_or(rendered.clone(), |(_, message)| message.trim().to_string())
}

fn compare_version_direction(from: &str, to: &str) -> String {
    match (parse_semver_precedence(from), parse_semver_precedence(to)) {
        (Some(left), Some(right)) => match compare_semver_precedence(&left, &right) {
            Ordering::Less => "upgrade".to_string(),
            Ordering::Greater => "downgrade".to_string(),
            Ordering::Equal => "same".to_string(),
        },
        _ => "unknown".to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemverPrecedence {
    core: (u64, u64, u64),
    prerelease: Vec<SemverPrereleaseIdentifier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SemverPrereleaseIdentifier {
    Numeric(u64),
    Text(String),
}

fn parse_semver_precedence(version: &str) -> Option<SemverPrecedence> {
    let version = version.trim();
    let version = version.split_once('+').map_or(version, |(left, _)| left);
    let (core, prerelease) = version
        .split_once('-')
        .map_or((version, None), |(core, prerelease)| {
            (core, Some(prerelease))
        });
    let mut core_parts = core.split('.');
    let major = parse_semver_core_number(core_parts.next()?)?;
    let minor = parse_semver_core_number(core_parts.next()?)?;
    let patch = parse_semver_core_number(core_parts.next()?)?;
    if core_parts.next().is_some() {
        return None;
    }
    let prerelease = match prerelease {
        Some(raw) => parse_semver_prerelease(raw)?,
        None => Vec::new(),
    };
    Some(SemverPrecedence {
        core: (major, minor, patch),
        prerelease,
    })
}

fn parse_semver_core_number(value: &str) -> Option<u64> {
    if value.is_empty() || (value.len() > 1 && value.starts_with('0')) {
        return None;
    }
    value.parse::<u64>().ok()
}

fn parse_semver_prerelease(raw: &str) -> Option<Vec<SemverPrereleaseIdentifier>> {
    raw.split('.')
        .map(|identifier| {
            if identifier.is_empty()
                || !identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            {
                return None;
            }
            if identifier.bytes().all(|byte| byte.is_ascii_digit()) {
                if identifier.len() > 1 && identifier.starts_with('0') {
                    return None;
                }
                return identifier
                    .parse::<u64>()
                    .ok()
                    .map(SemverPrereleaseIdentifier::Numeric);
            }
            Some(SemverPrereleaseIdentifier::Text(identifier.to_string()))
        })
        .collect()
}

fn compare_semver_precedence(left: &SemverPrecedence, right: &SemverPrecedence) -> Ordering {
    let core_order = left.core.cmp(&right.core);
    if core_order != Ordering::Equal {
        return core_order;
    }
    match (left.prerelease.is_empty(), right.prerelease.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => compare_prerelease_identifiers(&left.prerelease, &right.prerelease),
    }
}

fn compare_prerelease_identifiers(
    left: &[SemverPrereleaseIdentifier],
    right: &[SemverPrereleaseIdentifier],
) -> Ordering {
    for (left_identifier, right_identifier) in left.iter().zip(right.iter()) {
        let order = match (left_identifier, right_identifier) {
            (
                SemverPrereleaseIdentifier::Numeric(left_number),
                SemverPrereleaseIdentifier::Numeric(right_number),
            ) => left_number.cmp(right_number),
            (SemverPrereleaseIdentifier::Numeric(_), SemverPrereleaseIdentifier::Text(_)) => {
                Ordering::Less
            }
            (SemverPrereleaseIdentifier::Text(_), SemverPrereleaseIdentifier::Numeric(_)) => {
                Ordering::Greater
            }
            (
                SemverPrereleaseIdentifier::Text(left_text),
                SemverPrereleaseIdentifier::Text(right_text),
            ) => left_text.cmp(right_text),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

fn runtime_lifecycle_summary(
    manifest: &PluginManifest,
    _source: PluginInstallSource,
) -> PluginRuntimeLifecycleSummary {
    match &manifest.runtime {
        PluginRuntime::ExtensionHost { .. } => PluginRuntimeLifecycleSummary {
            kind: "extensionHost".to_string(),
            label: "Extension Host".to_string(),
            supported: true,
            blocking_reasons: Vec::new(),
        },
    }
}

fn hook_lifecycle_summaries(manifest: &PluginManifest) -> Vec<PluginHookLifecycleSummary> {
    declared_gateway_hooks(manifest)
        .into_iter()
        .map(|hook| PluginHookLifecycleSummary {
            name: hook.name.clone(),
            priority: hook.priority,
            failure_policy: hook.failure_policy.clone(),
            timeout_ms: hook.timeout_ms,
        })
        .collect()
}

fn declared_gateway_hooks(manifest: &PluginManifest) -> Vec<&crate::domain::plugins::PluginHook> {
    let mut hooks = manifest.hooks.iter().collect::<Vec<_>>();
    if let Some(contributes) = manifest.contributes.as_ref() {
        hooks.extend(contributes.gateway_hooks.iter());
    }
    hooks
}

fn permission_lifecycle_summaries(
    permissions: &[String],
    granted: &[String],
    pending: &[String],
) -> Vec<PluginPermissionLifecycleSummary> {
    permissions
        .iter()
        .map(|permission| PluginPermissionLifecycleSummary {
            permission: permission.clone(),
            risk: permission_risk(permission).unwrap_or(PluginPermissionRisk::Low),
            granted: granted.contains(permission),
            pending: pending.contains(permission),
        })
        .collect()
}

fn contribution_impact(manifest: &PluginManifest) -> PluginContributionImpact {
    let Some(contributes) = manifest.contributes.as_ref() else {
        return PluginContributionImpact {
            providers: Vec::new(),
            protocols: Vec::new(),
            protocol_bridges: Vec::new(),
            ui_slots: Vec::new(),
            commands: Vec::new(),
            gateway: Vec::new(),
            capabilities: manifest.capabilities.clone(),
        };
    };

    let providers = contributes
        .providers
        .iter()
        .map(|provider| PluginContributionImpactItem {
            id: provider.provider_type.clone(),
            label: Some(provider.display_name.clone()),
        })
        .collect();
    let protocols = contributes
        .protocols
        .iter()
        .map(|protocol| PluginContributionImpactItem {
            id: protocol.protocol_id.clone(),
            label: Some(format!("{:?}", protocol.direction)),
        })
        .collect();
    let protocol_bridges = contributes
        .protocol_bridges
        .iter()
        .map(|bridge| PluginContributionImpactItem {
            id: bridge.bridge_type.clone(),
            label: Some(format!(
                "{} -> {}",
                bridge.inbound_protocol, bridge.outbound_protocol
            )),
        })
        .collect();
    let ui_slots = contributes
        .ui
        .iter()
        .flat_map(|(slot_id, contributions)| {
            contributions
                .iter()
                .map(move |contribution| PluginUiSlotImpact {
                    slot_id: slot_id.clone(),
                    contribution_id: contribution.id.clone(),
                    title: contribution.title.clone(),
                })
        })
        .collect();
    let commands = contributes
        .commands
        .iter()
        .map(|command| PluginCommandImpact {
            command: command.command.clone(),
            title: command.title.clone(),
            category: command.category.clone(),
        })
        .collect();
    let gateway_hooks = contributes
        .gateway_hooks
        .iter()
        .map(|hook| PluginContributionImpactItem {
            id: hook.name.clone(),
            label: Some(format!(
                "priority={}, failurePolicy={}",
                hook.priority,
                hook.failure_policy.as_deref().unwrap_or("-")
            )),
        });
    PluginContributionImpact {
        providers,
        protocols,
        protocol_bridges,
        ui_slots,
        commands,
        gateway: gateway_hooks.collect(),
        capabilities: manifest.capabilities.clone(),
    }
}

fn compatibility_summary(
    manifest: &PluginManifest,
    host_version: &str,
) -> PluginCompatibilitySummary {
    match validate_manifest(manifest, host_version) {
        Ok(()) => PluginCompatibilitySummary {
            compatible: true,
            host_version: host_version.to_string(),
            app_range: manifest.host_compatibility.app.clone(),
            plugin_api_range: manifest.host_compatibility.plugin_api.clone(),
            platforms: manifest.host_compatibility.platforms.clone(),
            blocking_reasons: Vec::new(),
        },
        Err(error) => PluginCompatibilitySummary {
            compatible: false,
            host_version: host_version.to_string(),
            app_range: manifest.host_compatibility.app.clone(),
            plugin_api_range: manifest.host_compatibility.plugin_api.clone(),
            platforms: manifest.host_compatibility.platforms.clone(),
            blocking_reasons: vec![lifecycle_notice("error", &error.code, error.message)],
        },
    }
}

fn trust_summary(
    extracted: &package::ExtractedPluginPackage,
    policy: &LocalPackageInstallPolicy,
    trust: PackageTrust,
) -> PluginTrustSummary {
    let checksum_verified = policy.expected_checksum.as_deref().is_some_and(|expected| {
        expected
            .trim()
            .eq_ignore_ascii_case(extracted.checksum.as_str())
    });
    PluginTrustSummary {
        checksum: extracted.checksum.clone(),
        expected_checksum: policy.expected_checksum.clone(),
        checksum_verified,
        signature_verified: trust.signature_verified,
        unsigned: !trust.signature_verified,
        developer_mode: policy.developer_mode,
    }
}

pub(crate) fn preview_plugin_from_local_package_with_policy(
    db: &crate::db::Db,
    package_path: &Path,
    cache_dir: &Path,
    host_version: &str,
    policy: LocalPackageInstallPolicy,
) -> AppResult<PluginInstallPreview> {
    std::fs::create_dir_all(cache_dir).map_err(|e| {
        format!(
            "failed to create plugin cache dir {}: {e}",
            cache_dir.display()
        )
    })?;
    let staging_root = cache_dir.join("staging");
    let staging_dir = unique_staging_dir(&staging_root, "preview");
    let extracted = match package::extract_plugin_package_for_inspection(
        package_path,
        &staging_dir,
        package::PluginPackageLimits::default(),
    ) {
        Ok(extracted) => extracted,
        Err(error) => {
            cleanup_staging_dir(&staging_root, &staging_dir);
            return Err(error);
        }
    };
    let result = build_install_preview(
        db,
        &extracted,
        host_version,
        PluginInstallSource::Local,
        &policy,
    );
    cleanup_staging_dir(&staging_root, &staging_dir);
    result
}

fn build_install_preview(
    db: &crate::db::Db,
    extracted: &package::ExtractedPluginPackage,
    host_version: &str,
    source: PluginInstallSource,
    policy: &LocalPackageInstallPolicy,
) -> AppResult<PluginInstallPreview> {
    let manifest = &extracted.manifest;
    let compatibility = compatibility_summary(manifest, host_version);
    let mut blocking_reasons = compatibility.blocking_reasons.clone();
    let mut warnings = Vec::new();
    let runtime = runtime_lifecycle_summary(manifest, source);
    blocking_reasons.extend(
        runtime
            .blocking_reasons
            .iter()
            .filter(|notice| notice.severity == "error")
            .cloned(),
    );
    warnings.extend(
        runtime
            .blocking_reasons
            .iter()
            .filter(|notice| notice.severity != "error")
            .cloned(),
    );

    let trust = match verify_local_package(extracted, policy) {
        Ok(trust) => trust,
        Err(error) => {
            blocking_reasons.push(lifecycle_notice(
                "error",
                error.code(),
                app_error_message(&error),
            ));
            PackageTrust {
                signature_verified: false,
            }
        }
    };
    if let Err(error) = enforce_unsigned_install_policy(manifest, policy, trust) {
        blocking_reasons.push(lifecycle_notice(
            "error",
            error.code(),
            app_error_message(&error),
        ));
    }
    if let Err(error) = validate_reserved_official_source(manifest, source) {
        blocking_reasons.push(lifecycle_notice(
            "error",
            error.code(),
            app_error_message(&error),
        ));
    }

    let existing = repository::get_plugin(db, &manifest.id).ok();
    let existing_status = existing.as_ref().map(|detail| detail.summary.status);
    let existing_version = existing
        .as_ref()
        .and_then(|detail| detail.summary.current_version.clone());
    let effective_permissions = manifest_effective_permissions(manifest);

    Ok(PluginInstallPreview {
        plugin_id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        source,
        description: manifest.description.clone(),
        author: manifest.author.clone(),
        homepage: manifest.homepage.clone(),
        repository: manifest.repository.clone(),
        license: manifest.license.clone(),
        category: manifest.category.clone(),
        runtime,
        hooks: hook_lifecycle_summaries(manifest),
        permissions: permission_lifecycle_summaries(
            &effective_permissions,
            &effective_permissions,
            &[],
        ),
        contribution_impact: contribution_impact(manifest),
        compatibility,
        trust: trust_summary(extracted, policy, trust),
        existing_status,
        existing_version,
        blocking_reasons,
        warnings,
    })
}

pub(crate) fn preview_plugin_update_from_local_package(
    db: &crate::db::Db,
    package_path: &Path,
    cache_dir: &Path,
    host_version: &str,
    policy: LocalPackageInstallPolicy,
) -> AppResult<PluginUpdateDiff> {
    std::fs::create_dir_all(cache_dir).map_err(|e| {
        format!(
            "failed to create plugin cache dir {}: {e}",
            cache_dir.display()
        )
    })?;
    let staging_root = cache_dir.join("staging");
    let staging_dir = unique_staging_dir(&staging_root, "update-preview");
    let extracted = match package::extract_plugin_package_for_inspection(
        package_path,
        &staging_dir,
        package::PluginPackageLimits::default(),
    ) {
        Ok(extracted) => extracted,
        Err(error) => {
            cleanup_staging_dir(&staging_root, &staging_dir);
            return Err(error);
        }
    };
    let result = build_update_diff(db, &extracted, host_version, &policy);
    cleanup_staging_dir(&staging_root, &staging_dir);
    result
}

fn build_update_diff(
    db: &crate::db::Db,
    extracted: &package::ExtractedPluginPackage,
    host_version: &str,
    policy: &LocalPackageInstallPolicy,
) -> AppResult<PluginUpdateDiff> {
    let manifest = &extracted.manifest;
    let current = repository::get_plugin(db, &manifest.id)?;
    let compatibility = compatibility_summary(manifest, host_version);
    let mut blocking_reasons = compatibility.blocking_reasons.clone();
    let mut warnings = Vec::new();

    let trust = match verify_local_package(extracted, policy) {
        Ok(trust) => trust,
        Err(error) => {
            blocking_reasons.push(lifecycle_notice(
                "error",
                error.code(),
                app_error_message(&error),
            ));
            PackageTrust {
                signature_verified: false,
            }
        }
    };
    if let Err(error) = enforce_unsigned_install_policy(manifest, policy, trust) {
        blocking_reasons.push(lifecycle_notice(
            "error",
            error.code(),
            app_error_message(&error),
        ));
    }
    if let Err(error) = validate_reserved_official_source(manifest, PluginInstallSource::Local) {
        blocking_reasons.push(lifecycle_notice(
            "error",
            error.code(),
            app_error_message(&error),
        ));
    }

    let current_runtime = runtime_lifecycle_summary(&current.manifest, current.install_source);
    let next_runtime = runtime_lifecycle_summary(manifest, PluginInstallSource::Local);
    blocking_reasons.extend(
        next_runtime
            .blocking_reasons
            .iter()
            .filter(|notice| notice.severity == "error")
            .cloned(),
    );
    warnings.extend(
        next_runtime
            .blocking_reasons
            .iter()
            .filter(|notice| notice.severity != "error")
            .cloned(),
    );
    let runtime_change = (current_runtime.kind != next_runtime.kind
        || current_runtime.label != next_runtime.label
        || current_runtime.supported != next_runtime.supported)
        .then(|| PluginLifecycleChange {
            name: "runtime".to_string(),
            change: "changed".to_string(),
            before: Some(current_runtime.label),
            after: Some(next_runtime.label),
        });

    let from_version = current
        .summary
        .current_version
        .clone()
        .unwrap_or_else(|| current.manifest.version.clone());
    let version_direction = compare_version_direction(&from_version, &manifest.version);
    if version_direction == "downgrade" {
        warnings.push(lifecycle_notice(
            "warn",
            "PLUGIN_UPDATE_DOWNGRADE",
            "selected package version is lower than the installed version",
        ));
    }

    Ok(PluginUpdateDiff {
        plugin_id: manifest.id.clone(),
        from_version: from_version.clone(),
        to_version: manifest.version.clone(),
        version_direction,
        runtime_change,
        hook_changes: diff_hooks(&current.manifest, manifest),
        permission_changes: diff_permissions(&current, manifest),
        contribution_changes: diff_contributions(&current.manifest, manifest),
        config_version_change: config_version_change(&current.manifest, manifest),
        compatibility,
        trust: trust_summary(extracted, policy, trust),
        rollback_available: rollback_available(db, &manifest.id, &from_version),
        blocking_reasons,
        warnings,
    })
}

fn rollback_available(db: &crate::db::Db, plugin_id: &str, version: &str) -> bool {
    rollback_candidate_installed_dir(db, plugin_id, version)
        .is_some_and(|installed_dir| repository::plugin_installed_dir_available(&installed_dir))
}

fn rollback_candidate_installed_dir(
    db: &crate::db::Db,
    plugin_id: &str,
    version: &str,
) -> Option<String> {
    let conn = db.open_connection().ok()?;
    conn.query_row(
        r#"
SELECT installed_dir
FROM plugin_versions
WHERE plugin_id = ?1 AND version = ?2
"#,
        rusqlite::params![plugin_id, version],
        |row| row.get(0),
    )
    .optional()
    .ok()?
    .flatten()
}

fn diff_hooks(before: &PluginManifest, after: &PluginManifest) -> Vec<PluginLifecycleChange> {
    let mut changes = Vec::new();
    let before_hooks = declared_gateway_hooks(before);
    let after_hooks = declared_gateway_hooks(after);
    for hook in &before_hooks {
        match after_hooks.iter().find(|next| next.name == hook.name) {
            Some(next)
                if next.priority != hook.priority || next.failure_policy != hook.failure_policy =>
            {
                changes.push(PluginLifecycleChange {
                    name: hook.name.clone(),
                    change: "changed".to_string(),
                    before: Some(format!(
                        "priority={}, failurePolicy={}",
                        hook.priority,
                        hook.failure_policy.as_deref().unwrap_or("-")
                    )),
                    after: Some(format!(
                        "priority={}, failurePolicy={}",
                        next.priority,
                        next.failure_policy.as_deref().unwrap_or("-")
                    )),
                });
            }
            Some(_) => {}
            None => changes.push(PluginLifecycleChange {
                name: hook.name.clone(),
                change: "removed".to_string(),
                before: Some("declared".to_string()),
                after: None,
            }),
        }
    }
    for hook in &after_hooks {
        if before_hooks.iter().all(|prev| prev.name != hook.name) {
            changes.push(PluginLifecycleChange {
                name: hook.name.clone(),
                change: "added".to_string(),
                before: None,
                after: Some(format!(
                    "priority={}, failurePolicy={}",
                    hook.priority,
                    hook.failure_policy.as_deref().unwrap_or("-")
                )),
            });
        }
    }
    changes
}

fn diff_permissions(
    current: &PluginDetail,
    next: &PluginManifest,
) -> Vec<PluginPermissionLifecycleChange> {
    let current_permissions = manifest_effective_permissions(&current.manifest);
    let next_permissions = manifest_effective_permissions(next);
    let mut all = current_permissions.clone();
    for permission in &next_permissions {
        if !all.contains(permission) {
            all.push(permission.clone());
        }
    }
    all.sort();

    all.into_iter()
        .map(|permission| {
            let was_available = current_permissions.contains(&permission);
            let is_available = next_permissions.contains(&permission);
            let change = match (was_available, is_available) {
                (true, true) => "unchanged",
                (false, true) => "added",
                (true, false) => "removed",
                (false, false) => "not_available",
            };
            let risk = permission_risk(&permission).unwrap_or(PluginPermissionRisk::Low);
            PluginPermissionLifecycleChange {
                permission,
                risk,
                change: change.to_string(),
            }
        })
        .filter(|change| matches!(change.change.as_str(), "added" | "removed"))
        .collect()
}

fn diff_contributions(
    before: &PluginManifest,
    after: &PluginManifest,
) -> Vec<PluginContributionChange> {
    let before = contribution_signatures(before);
    let after = contribution_signatures(after);
    let names: BTreeSet<String> = before.keys().chain(after.keys()).cloned().collect();

    names
        .into_iter()
        .filter_map(|name| match (before.get(&name), after.get(&name)) {
            (Some(previous), Some(next)) if previous != next => Some(PluginContributionChange {
                kind: next.kind.clone(),
                name: next.name.clone(),
                label: next.label.clone(),
                change: "changed".to_string(),
                before: Some(previous.summary.clone()),
                after: Some(next.summary.clone()),
            }),
            (Some(previous), None) => Some(PluginContributionChange {
                kind: previous.kind.clone(),
                name: previous.name.clone(),
                label: previous.label.clone(),
                change: "removed".to_string(),
                before: Some(previous.summary.clone()),
                after: None,
            }),
            (None, Some(next)) => Some(PluginContributionChange {
                kind: next.kind.clone(),
                name: next.name.clone(),
                label: next.label.clone(),
                change: "added".to_string(),
                before: None,
                after: Some(next.summary.clone()),
            }),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContributionSignature {
    kind: String,
    name: String,
    label: Option<String>,
    summary: String,
    fingerprint: String,
}

fn contribution_signatures(manifest: &PluginManifest) -> BTreeMap<String, ContributionSignature> {
    let mut out = BTreeMap::new();
    let Some(contributes) = manifest.contributes.as_ref() else {
        for capability in &manifest.capabilities {
            insert_contribution_signature(
                &mut out,
                format!("capability:{capability}"),
                ContributionSignature {
                    kind: "capability".to_string(),
                    name: capability.clone(),
                    label: None,
                    summary: "declared".to_string(),
                    fingerprint: format!("capability:{capability}"),
                },
            );
        }
        return out;
    };

    for provider in &contributes.providers {
        let label = Some(provider.display_name.clone());
        insert_contribution_signature(
            &mut out,
            format!("provider:{}", provider.provider_type),
            ContributionSignature {
                kind: "provider".to_string(),
                name: provider.provider_type.clone(),
                label,
                summary: short_contribution_summary(format!(
                    "{} ({})",
                    provider.display_name, provider.provider_type
                )),
                fingerprint: contribution_fingerprint("provider", provider),
            },
        );
    }
    for protocol in &contributes.protocols {
        let direction = format!("{:?}", protocol.direction);
        insert_contribution_signature(
            &mut out,
            format!("protocol:{}", protocol.protocol_id),
            ContributionSignature {
                kind: "protocol".to_string(),
                name: protocol.protocol_id.clone(),
                label: Some(direction.clone()),
                summary: short_contribution_summary(format!(
                    "{} ({})",
                    direction, protocol.protocol_id
                )),
                fingerprint: contribution_fingerprint("protocol", protocol),
            },
        );
    }
    for bridge in &contributes.protocol_bridges {
        let route = format!(
            "{} -> {}",
            bridge.inbound_protocol, bridge.outbound_protocol
        );
        insert_contribution_signature(
            &mut out,
            format!("protocolBridge:{}", bridge.bridge_type),
            ContributionSignature {
                kind: "protocolBridge".to_string(),
                name: bridge.bridge_type.clone(),
                label: Some(route.clone()),
                summary: short_contribution_summary(format!("{} ({})", route, bridge.bridge_type)),
                fingerprint: contribution_fingerprint("protocolBridge", bridge),
            },
        );
    }
    for command in &contributes.commands {
        insert_contribution_signature(
            &mut out,
            format!("command:{}", command.command),
            ContributionSignature {
                kind: "command".to_string(),
                name: command.command.clone(),
                label: Some(command.title.clone()),
                summary: short_contribution_summary(format!(
                    "{} ({})",
                    command.title, command.command
                )),
                fingerprint: contribution_fingerprint("command", command),
            },
        );
    }
    for hook in &contributes.gateway_hooks {
        insert_contribution_signature(
            &mut out,
            format!("gatewayHook:{}", hook.name),
            ContributionSignature {
                kind: "gatewayHook".to_string(),
                name: hook.name.clone(),
                label: None,
                summary: short_contribution_summary(format!(
                    "priority={}, failurePolicy={}",
                    hook.priority,
                    hook.failure_policy.as_deref().unwrap_or("-")
                )),
                fingerprint: contribution_fingerprint("gatewayHook", hook),
            },
        );
    }
    for (slot_id, contributions) in &contributes.ui {
        for contribution in contributions {
            let label = contribution
                .title
                .clone()
                .filter(|title| !title.trim().is_empty())
                .unwrap_or_else(|| contribution.id.clone());
            insert_contribution_signature(
                &mut out,
                format!("ui:{slot_id}:{}", contribution.id),
                ContributionSignature {
                    kind: "ui".to_string(),
                    name: format!("{slot_id}/{}", contribution.id),
                    label: Some(label.clone()),
                    summary: short_contribution_summary(format!("{label} ({slot_id})")),
                    fingerprint: contribution_fingerprint("ui", contribution),
                },
            );
        }
    }

    for capability in &manifest.capabilities {
        insert_contribution_signature(
            &mut out,
            format!("capability:{capability}"),
            ContributionSignature {
                kind: "capability".to_string(),
                name: capability.clone(),
                label: None,
                summary: "declared".to_string(),
                fingerprint: format!("capability:{capability}"),
            },
        );
    }

    out
}

fn insert_contribution_signature(
    out: &mut BTreeMap<String, ContributionSignature>,
    key: String,
    signature: ContributionSignature,
) {
    if let Entry::Vacant(entry) = out.entry(key.clone()) {
        entry.insert(signature);
        return;
    }

    let mut index = 2;
    loop {
        let candidate = format!("{key}#{index}");
        if let Entry::Vacant(entry) = out.entry(candidate) {
            entry.insert(signature);
            return;
        }
        index += 1;
    }
}

fn contribution_fingerprint<T: serde::Serialize>(kind: &str, value: &T) -> String {
    serde_json::to_string(value)
        .map(|json| format!("{kind}:{json}"))
        .unwrap_or_else(|_| kind.to_string())
}

fn short_contribution_summary(value: impl AsRef<str>) -> String {
    let trimmed = value.as_ref().trim();
    const MAX_CHARS: usize = 80;
    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed.to_string();
    }

    let mut out = trimmed.chars().take(MAX_CHARS - 3).collect::<String>();
    out.push_str("...");
    out
}

fn reconcile_permissions_for_manifest(
    _current: &PluginDetail,
    _manifest: &PluginManifest,
) -> (Vec<String>, Vec<String>) {
    (Vec::new(), Vec::new())
}

fn config_for_manifest_version(
    current: &PluginDetail,
    manifest: &PluginManifest,
) -> serde_json::Value {
    config_with_schema_defaults(manifest.config_schema.as_ref(), current.config.clone())
}

fn config_version_change(before: &PluginManifest, after: &PluginManifest) -> Option<String> {
    let before_version = before.config_version.unwrap_or(1);
    let after_version = after.config_version.unwrap_or(1);
    (before_version != after_version).then(|| format!("{before_version} -> {after_version}"))
}

pub(crate) fn install_plugin_from_local_package_with_policy(
    db: &crate::db::Db,
    package_path: &Path,
    cache_dir: &Path,
    installed_root: &Path,
    host_version: &str,
    policy: LocalPackageInstallPolicy,
) -> AppResult<PluginDetail> {
    std::fs::create_dir_all(cache_dir).map_err(|e| {
        format!(
            "failed to create plugin cache dir {}: {e}",
            cache_dir.display()
        )
    })?;
    std::fs::create_dir_all(installed_root).map_err(|e| {
        format!(
            "failed to create plugin installed dir {}: {e}",
            installed_root.display()
        )
    })?;

    let staging_root = cache_dir.join("staging");
    let staging_dir = unique_staging_dir(&staging_root, "local");
    let extracted = match package::extract_plugin_package_for_inspection(
        package_path,
        &staging_dir,
        package::PluginPackageLimits::default(),
    ) {
        Ok(extracted) => extracted,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            let _ = std::fs::remove_dir(&staging_root);
            return Err(error);
        }
    };
    let trust = match validate_local_package_install(&extracted, host_version, &policy) {
        Ok(trust) => trust,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            let _ = std::fs::remove_dir(&staging_root);
            return Err(error);
        }
    };

    let plugin_id = extracted.manifest.id.clone();
    let version = extracted.manifest.version.clone();
    let installed_dir = installed_root
        .join(crate::app_paths::plugin_id_path_segment(&plugin_id)?)
        .join(crate::app_paths::plugin_id_path_segment(&version)?);
    let cache_package_path = unique_cache_package_path(cache_dir, &plugin_id, &version);

    let result = (|| -> AppResult<PluginDetail> {
        std::fs::copy(package_path, &cache_package_path).map_err(|e| {
            format!(
                "failed to copy plugin package {} -> {}: {e}",
                package_path.display(),
                cache_package_path.display()
            )
        })?;

        replace_dir(&extracted.root_dir, &installed_dir)?;
        let detail = repository::with_plugin_transaction(db, |tx| {
            repository::insert_plugin_with_tx(
                tx,
                repository::InsertPluginInput {
                    manifest: extracted.manifest.clone(),
                    install_source: policy.install_source,
                    status: PluginStatus::Disabled,
                    installed_dir: Some(installed_dir.to_string_lossy().to_string()),
                },
            )?;
            let detail = repository::save_plugin_permissions_with_tx(tx, &plugin_id, &[], &[])?;
            append_audit_with_tx(
                tx,
                Some(plugin_id.clone()),
                install_audit_event_type(policy.install_source),
                "medium",
                install_audit_message(policy.install_source),
                install_audit_details(
                    policy.install_source,
                    policy.remote_source_url.as_deref(),
                    policy.market_source_url.as_deref(),
                    serde_json::json!({
                    "source": "local",
                    "packageChecksum": extracted.checksum,
                    "cachedPackage": cache_package_path.to_string_lossy(),
                    "unsigned": !trust.signature_verified,
                    "signatureVerified": trust.signature_verified,
                    "developerMode": policy.developer_mode,
                    }),
                ),
            )?;
            Ok(detail)
        })?;
        tracing::info!(
            plugin_id,
            version,
            installed_dir = %installed_dir.display(),
            "local plugin package installed"
        );
        repository::get_plugin(db, &plugin_id).or(Ok(detail))
    })();

    let _ = std::fs::remove_dir_all(&staging_dir);
    let _ = std::fs::remove_dir(&staging_root);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&installed_dir);
        let _ = std::fs::remove_file(&cache_package_path);
    }
    result
}

pub(crate) fn install_plugin_from_remote_package_bytes(
    db: &crate::db::Db,
    package_bytes: Vec<u8>,
    source_url: &str,
    cache_dir: &Path,
    installed_root: &Path,
    host_version: &str,
    policy: RemotePackageInstallPolicy,
) -> AppResult<PluginDetail> {
    if !matches!(
        policy.install_source,
        PluginInstallSource::Market | PluginInstallSource::GithubRelease
    ) {
        return Err(AppError::new(
            "PLUGIN_REMOTE_INSTALL_SOURCE_INVALID",
            "remote plugin install source must be market or GitHub release",
        ));
    }
    if policy.expected_checksum.trim().is_empty() {
        return Err(AppError::new(
            "PLUGIN_REMOTE_CHECKSUM_REQUIRED",
            "remote plugin installation requires a package checksum",
        ));
    }
    std::fs::create_dir_all(cache_dir).map_err(|e| {
        format!(
            "failed to create plugin cache dir {}: {e}",
            cache_dir.display()
        )
    })?;
    let package_path = unique_remote_package_path(
        cache_dir,
        crate::app_paths::plugin_id_path_segment(&policy.expected_plugin_id)?,
    );
    std::fs::write(&package_path, &package_bytes).map_err(|e| {
        format!(
            "failed to write remote plugin package cache {}: {e}",
            package_path.display()
        )
    })?;

    let install_source = policy.install_source;
    let expected_plugin_id = policy.expected_plugin_id.clone();
    let expected_checksum = policy.expected_checksum.clone();
    let signature = policy.signature.clone();
    let market_source_url = (install_source == PluginInstallSource::Market)
        .then(|| policy.market_source_url.clone())
        .flatten();
    let public_key = remote_package_trusted_public_key(db, source_url, &policy)?;
    let result = install_plugin_from_local_package_with_policy(
        db,
        &package_path,
        cache_dir,
        installed_root,
        host_version,
        LocalPackageInstallPolicy {
            expected_plugin_id: Some(expected_plugin_id),
            expected_checksum: Some(expected_checksum),
            signature,
            public_key,
            developer_mode: false,
            install_source,
            remote_source_url: Some(source_url.to_string()),
            market_source_url,
        },
    )
    .inspect(|detail| {
        tracing::info!(
            plugin_id = %detail.summary.plugin_id,
            source = install_source.as_str(),
            "remote plugin package installed"
        );
    });

    let _ = std::fs::remove_file(&package_path);
    result
}

pub(crate) fn preview_plugin_update_from_remote_package_bytes(
    db: &crate::db::Db,
    package_bytes: Vec<u8>,
    source_url: &str,
    cache_dir: &Path,
    host_version: &str,
    policy: RemotePackageInstallPolicy,
) -> AppResult<PluginUpdateDiff> {
    let package_path =
        write_remote_package_bytes(cache_dir, &policy.expected_plugin_id, &package_bytes)?;
    let local_policy = local_policy_for_remote_package(db, source_url, &policy)?;
    let result = preview_plugin_update_from_local_package(
        db,
        &package_path,
        cache_dir,
        host_version,
        local_policy,
    );
    let _ = std::fs::remove_file(&package_path);
    result
}

pub(crate) fn update_plugin_from_remote_package_bytes(
    db: &crate::db::Db,
    package_bytes: Vec<u8>,
    source_url: &str,
    cache_dir: &Path,
    installed_root: &Path,
    host_version: &str,
    policy: RemotePackageInstallPolicy,
) -> AppResult<PluginDetail> {
    let install_source = policy.install_source;
    let package_path =
        write_remote_package_bytes(cache_dir, &policy.expected_plugin_id, &package_bytes)?;
    let local_policy = local_policy_for_remote_package(db, source_url, &policy)?;
    let result = update_plugin_from_local_package(
        db,
        &package_path,
        cache_dir,
        installed_root,
        host_version,
        local_policy,
    )
    .inspect(|detail| {
        tracing::info!(
            plugin_id = %detail.summary.plugin_id,
            source = install_source.as_str(),
            "remote plugin package updated"
        );
    });
    let _ = std::fs::remove_file(&package_path);
    result
}

fn write_remote_package_bytes(
    cache_dir: &Path,
    expected_plugin_id: &str,
    package_bytes: &[u8],
) -> AppResult<PathBuf> {
    std::fs::create_dir_all(cache_dir).map_err(|e| {
        format!(
            "failed to create plugin cache dir {}: {e}",
            cache_dir.display()
        )
    })?;
    let package_path = unique_remote_package_path(
        cache_dir,
        crate::app_paths::plugin_id_path_segment(expected_plugin_id)?,
    );
    std::fs::write(&package_path, package_bytes).map_err(|e| {
        format!(
            "failed to write remote plugin package cache {}: {e}",
            package_path.display()
        )
    })?;
    Ok(package_path)
}

fn local_policy_for_remote_package(
    db: &crate::db::Db,
    source_url: &str,
    policy: &RemotePackageInstallPolicy,
) -> AppResult<LocalPackageInstallPolicy> {
    if !matches!(
        policy.install_source,
        PluginInstallSource::Market | PluginInstallSource::GithubRelease
    ) {
        return Err(AppError::new(
            "PLUGIN_REMOTE_INSTALL_SOURCE_INVALID",
            "remote plugin install source must be market or GitHub release",
        ));
    }
    if policy.expected_checksum.trim().is_empty() {
        return Err(AppError::new(
            "PLUGIN_REMOTE_CHECKSUM_REQUIRED",
            "remote plugin installation requires a package checksum",
        ));
    }
    let public_key = remote_package_trusted_public_key(db, source_url, policy)?;
    Ok(LocalPackageInstallPolicy {
        expected_plugin_id: Some(policy.expected_plugin_id.clone()),
        expected_checksum: Some(policy.expected_checksum.clone()),
        signature: policy.signature.clone(),
        public_key,
        developer_mode: false,
        install_source: policy.install_source,
        remote_source_url: Some(source_url.to_string()),
        market_source_url: (policy.install_source == PluginInstallSource::Market)
            .then(|| policy.market_source_url.clone())
            .flatten(),
    })
}

fn remote_package_trusted_public_key(
    db: &crate::db::Db,
    source_url: &str,
    policy: &RemotePackageInstallPolicy,
) -> AppResult<Option<String>> {
    match policy.install_source {
        PluginInstallSource::Market => {
            if policy.signature.is_none() {
                return Ok(None);
            }
            let trust_source_url = policy.market_source_url.as_deref().unwrap_or(source_url);
            repository::trusted_market_public_key_for_url(db, trust_source_url)?
                .ok_or_else(|| {
                    AppError::new(
                    "PLUGIN_MARKET_TRUSTED_PUBLIC_KEY_REQUIRED",
                    "signed market plugin installation requires a trusted market source public key",
                )
                })
                .map(Some)
        }
        PluginInstallSource::GithubRelease => Ok(policy.public_key.clone()),
        _ => Err(AppError::new(
            "PLUGIN_REMOTE_INSTALL_SOURCE_INVALID",
            "remote plugin install source must be market or GitHub release",
        )),
    }
}

pub(crate) fn update_plugin_from_local_package(
    db: &crate::db::Db,
    package_path: &Path,
    cache_dir: &Path,
    installed_root: &Path,
    host_version: &str,
    policy: LocalPackageInstallPolicy,
) -> AppResult<PluginDetail> {
    std::fs::create_dir_all(cache_dir).map_err(|e| {
        format!(
            "failed to create plugin cache dir {}: {e}",
            cache_dir.display()
        )
    })?;
    std::fs::create_dir_all(installed_root).map_err(|e| {
        format!(
            "failed to create plugin installed dir {}: {e}",
            installed_root.display()
        )
    })?;
    let staging_root = cache_dir.join("staging");
    let staging_dir = unique_staging_dir(&staging_root, "update");
    let extracted = match package::extract_plugin_package_for_inspection(
        package_path,
        &staging_dir,
        package::PluginPackageLimits::default(),
    ) {
        Ok(extracted) => extracted,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            let _ = std::fs::remove_dir(&staging_root);
            return Err(error);
        }
    };
    let trust = match validate_local_package_install(&extracted, host_version, &policy) {
        Ok(trust) => trust,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            let _ = std::fs::remove_dir(&staging_root);
            return Err(error);
        }
    };

    let plugin_id = extracted.manifest.id.clone();
    let current = repository::get_plugin(db, &plugin_id)?;
    let installed_dir = installed_root
        .join(crate::app_paths::plugin_id_path_segment(&plugin_id)?)
        .join(crate::app_paths::plugin_id_path_segment(
            &extracted.manifest.version,
        )?);

    let result = (|| -> AppResult<PluginDetail> {
        replace_dir(&extracted.root_dir, &installed_dir)?;
        let (granted, pending) = reconcile_permissions_for_manifest(&current, &extracted.manifest);
        let next_config = config_for_manifest_version(&current, &extracted.manifest);
        let detail = repository::with_plugin_transaction(db, |tx| {
            repository::update_plugin_manifest_with_tx(
                tx,
                extracted.manifest.clone(),
                Some(installed_dir.to_string_lossy().to_string()),
            )?;
            repository::save_plugin_config_with_tx(
                tx,
                &plugin_id,
                extracted.manifest.config_version.unwrap_or(1),
                &next_config,
                &[],
            )?;
            let detail =
                repository::save_plugin_permissions_with_tx(tx, &plugin_id, &granted, &pending)?;
            append_audit_with_tx(
                tx,
                Some(plugin_id.clone()),
                "plugin.updated",
                "high",
                "Plugin updated from local package",
                serde_json::json!({
                    "fromVersion": current.summary.current_version,
                    "toVersion": extracted.manifest.version,
                    "pendingPermissions": pending,
                    "unsigned": !trust.signature_verified,
                    "signatureVerified": trust.signature_verified,
                    "developerMode": policy.developer_mode,
                }),
            )?;
            Ok(detail)
        })?;
        tracing::info!(
            plugin_id,
            version = extracted.manifest.version,
            "local plugin package updated"
        );
        Ok(detail)
    })();

    let _ = std::fs::remove_dir_all(&staging_dir);
    let _ = std::fs::remove_dir(&staging_root);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&installed_dir);
    }
    result
}

pub(crate) fn rollback_plugin_to_version(
    db: &crate::db::Db,
    plugin_id: &str,
    version: &str,
) -> AppResult<PluginDetail> {
    let (manifest, installed_dir) = repository::get_plugin_version(db, plugin_id, version)?;
    let installed_dir_value = installed_dir.as_deref().ok_or_else(|| {
        AppError::new(
            "PLUGIN_ROLLBACK_UNAVAILABLE",
            format!("plugin version {version} has no install directory"),
        )
    })?;
    if !repository::plugin_installed_dir_available(installed_dir_value) {
        return Err(AppError::new(
            "PLUGIN_ROLLBACK_UNAVAILABLE",
            format!("plugin version {version} install directory is unavailable"),
        ));
    }
    let current = repository::get_plugin(db, plugin_id)?;
    let (granted, pending) = reconcile_permissions_for_manifest(&current, &manifest);
    let next_config = config_for_manifest_version(&current, &manifest);
    let config_version = manifest.config_version.unwrap_or(1);
    let detail = repository::with_plugin_transaction(db, |tx| {
        repository::update_plugin_manifest_with_tx(tx, manifest, installed_dir)?;
        repository::save_plugin_config_with_tx(tx, plugin_id, config_version, &next_config, &[])?;
        let detail =
            repository::save_plugin_permissions_with_tx(tx, plugin_id, &granted, &pending)?;
        append_audit_with_tx(
            tx,
            Some(plugin_id.to_string()),
            "plugin.rollback",
            "high",
            "Plugin rolled back",
            serde_json::json!({
                "version": version,
                "grantedPermissions": granted,
                "pendingPermissions": pending,
                "configVersion": config_version,
            }),
        )?;
        Ok(detail)
    })?;
    tracing::warn!(plugin_id, version, "plugin rolled back to previous version");
    Ok(detail)
}

pub(crate) fn quarantine_revoked_plugin(
    db: &crate::db::Db,
    plugin_id: &str,
    reason: &str,
) -> AppResult<PluginDetail> {
    let detail =
        repository::update_plugin_status(db, plugin_id, PluginStatus::Quarantined, Some(reason))?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.quarantined",
        "critical",
        "Plugin quarantined",
        serde_json::json!({ "reason": reason, "source": "market_revoked" }),
    )?;
    tracing::warn!(plugin_id, reason, "plugin quarantined by market revocation");
    repository::get_plugin(db, plugin_id).or(Ok(detail))
}

fn enforce_unsigned_install_policy(
    _manifest: &PluginManifest,
    _policy: &LocalPackageInstallPolicy,
    _trust: PackageTrust,
) -> AppResult<()> {
    Ok(())
}

fn verify_local_package(
    extracted: &package::ExtractedPluginPackage,
    policy: &LocalPackageInstallPolicy,
) -> AppResult<PackageTrust> {
    if let Some(expected_plugin_id) = policy.expected_plugin_id.as_deref() {
        if extracted.manifest.id != expected_plugin_id {
            return Err(AppError::new(
                "PLUGIN_REMOTE_PLUGIN_ID_MISMATCH",
                format!(
                    "remote package plugin id mismatch: expected {}, got {}",
                    expected_plugin_id, extracted.manifest.id
                ),
            ));
        }
    }

    if let Some(expected) = policy.expected_checksum.as_deref() {
        signing::verify_checksum(&extracted.package_bytes, expected)?;
    }

    match (policy.signature.as_deref(), policy.public_key.as_deref()) {
        (Some(signature), Some(public_key)) => {
            signing::verify_ed25519_signature(&extracted.package_bytes, signature, public_key)?;
            Ok(PackageTrust {
                signature_verified: true,
            })
        }
        (Some(_), None) | (None, Some(_)) => Err(AppError::new(
            "PLUGIN_SIGNATURE_POLICY_INCOMPLETE",
            "plugin signature verification requires both signature and public key",
        )),
        (None, None) => Ok(PackageTrust {
            signature_verified: false,
        }),
    }
}

fn validate_local_package_install(
    extracted: &package::ExtractedPluginPackage,
    host_version: &str,
    policy: &LocalPackageInstallPolicy,
) -> AppResult<PackageTrust> {
    validate_reserved_official_source(&extracted.manifest, policy.install_source)?;
    validate_manifest(&extracted.manifest, host_version)?;
    let trust = verify_local_package(extracted, policy)?;
    enforce_unsigned_install_policy(&extracted.manifest, policy, trust)?;
    Ok(trust)
}

fn install_audit_event_type(source: PluginInstallSource) -> &'static str {
    match source {
        PluginInstallSource::Market | PluginInstallSource::GithubRelease => {
            "plugin.remote.installed"
        }
        _ => "plugin.installed",
    }
}

fn install_audit_message(source: PluginInstallSource) -> &'static str {
    match source {
        PluginInstallSource::Market | PluginInstallSource::GithubRelease => {
            "Remote plugin package installed"
        }
        _ => "Local plugin package installed",
    }
}

fn install_audit_details(
    source: PluginInstallSource,
    source_url: Option<&str>,
    market_source_url: Option<&str>,
    mut details: serde_json::Value,
) -> serde_json::Value {
    if let serde_json::Value::Object(object) = &mut details {
        object.insert(
            "source".to_string(),
            serde_json::Value::String(source.as_str().to_string()),
        );
        if let Some(source_url) = source_url {
            object.insert(
                "sourceUrl".to_string(),
                serde_json::Value::String(source_url.to_string()),
            );
        }
        if let Some(market_source_url) = market_source_url {
            object.insert(
                "marketSourceUrl".to_string(),
                serde_json::Value::String(market_source_url.to_string()),
            );
        }
    }
    details
}

fn validate_reserved_official_source(
    manifest: &PluginManifest,
    install_source: PluginInstallSource,
) -> AppResult<()> {
    if manifest.id.starts_with("official.") && install_source != PluginInstallSource::Official {
        return Err(AppError::new(
            "PLUGIN_RESERVED_OFFICIAL_ID",
            "official plugin ids are reserved for built-in official plugins",
        ));
    }
    Ok(())
}

pub(crate) fn enable_plugin(
    db: &crate::db::Db,
    plugin_id: &str,
    host_version: &str,
) -> AppResult<PluginDetail> {
    let detail = detail_with_config_defaults_for_db(db, repository::get_plugin(db, plugin_id)?)?;
    if !matches!(
        detail.summary.status,
        PluginStatus::Disabled | PluginStatus::Installed
    ) {
        return Err(AppError::new(
            "PLUGIN_INVALID_STATUS",
            format!(
                "plugin {plugin_id} cannot be enabled from status {}",
                detail.summary.status.as_str()
            ),
        ));
    }
    validate_manifest_for_source(&detail.manifest, detail.install_source, host_version)?;
    ensure_runtime_enabled(&detail.manifest)?;
    validate_config_against_schema(detail.manifest.config_schema.as_ref(), &detail.config)?;
    let mut candidate = detail.clone();
    candidate.summary.status = PluginStatus::Enabled;
    let mut active_details = Vec::new();
    for summary in list_plugins(db)? {
        if summary.plugin_id == plugin_id {
            continue;
        }
        active_details.push(normalize_unsupported_legacy_plugin_detail(
            detail_with_config_defaults_for_db(
                db,
                repository::get_plugin(db, &summary.plugin_id)?,
            )?,
        ));
    }
    active_details.push(candidate);
    ensure_no_duplicate_enabled_commands(&active_details)?;
    let next = repository::update_plugin_status(db, plugin_id, PluginStatus::Enabled, None)?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.enabled",
        "low",
        "Plugin enabled",
        serde_json::json!({}),
    )?;
    tracing::info!(plugin_id, "plugin enabled");
    detail_with_config_defaults_for_db(db, next)
}

pub(crate) fn disable_plugin(db: &crate::db::Db, plugin_id: &str) -> AppResult<PluginDetail> {
    let next = repository::update_plugin_status(db, plugin_id, PluginStatus::Disabled, None)?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.disabled",
        "low",
        "Plugin disabled",
        serde_json::json!({}),
    )?;
    tracing::info!(plugin_id, "plugin disabled");
    Ok(next)
}

pub(crate) fn uninstall_plugin(db: &crate::db::Db, plugin_id: &str) -> AppResult<PluginDetail> {
    let next = repository::update_plugin_status(db, plugin_id, PluginStatus::Uninstalled, None)?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.uninstalled",
        "medium",
        "Plugin uninstalled",
        serde_json::json!({ "auditRetained": true }),
    )?;
    tracing::info!(plugin_id, "plugin uninstalled");
    repository::get_plugin(db, plugin_id).or(Ok(next))
}

pub(crate) fn save_plugin_config(
    db: &crate::db::Db,
    plugin_id: &str,
    config: serde_json::Value,
) -> AppResult<PluginDetail> {
    let detail = detail_with_config_defaults_for_db(db, repository::get_plugin(db, plugin_id)?)?;
    let config = config_with_schema_defaults(detail.manifest.config_schema.as_ref(), config);
    validate_config_against_schema(detail.manifest.config_schema.as_ref(), &config)?;
    let config_version = detail.manifest.config_version.unwrap_or(1);
    let next = repository::save_plugin_config(db, plugin_id, config_version, &config, &[])?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.config.saved",
        "medium",
        "Plugin config saved",
        serde_json::json!({ "configVersion": config_version }),
    )?;
    detail_with_config_defaults_for_db(db, next)
}

pub(crate) fn grant_plugin_permissions(
    db: &crate::db::Db,
    plugin_id: &str,
    _permissions: Vec<String>,
) -> AppResult<PluginDetail> {
    let _ = repository::get_plugin(db, plugin_id)?;
    Err(AppError::new(
        "PLUGIN_PERMISSION_MODEL_REMOVED",
        "Extension Host access is derived from capabilities and contributions; manual permission grants are not supported",
    ))
}

pub(crate) fn revoke_plugin_permission(
    db: &crate::db::Db,
    plugin_id: &str,
    _permission: &str,
) -> AppResult<PluginDetail> {
    let _ = repository::get_plugin(db, plugin_id)?;
    Err(AppError::new(
        "PLUGIN_PERMISSION_MODEL_REMOVED",
        "Extension Host access is derived from capabilities and contributions; manual permission revocation is not supported",
    ))
}

fn ensure_runtime_enabled(manifest: &PluginManifest) -> AppResult<()> {
    match &manifest.runtime {
        PluginRuntime::ExtensionHost { .. } => Ok(()),
    }
}

fn validate_manifest_for_source(
    manifest: &PluginManifest,
    install_source: PluginInstallSource,
    host_version: &str,
) -> Result<(), crate::domain::plugins::PluginValidationError> {
    if install_source == PluginInstallSource::Official {
        validate_manifest_for_official_plugin(manifest, host_version)
    } else {
        validate_manifest(manifest, host_version)
    }
}

fn detail_with_config_defaults_for_db(
    db: &crate::db::Db,
    detail: PluginDetail,
) -> AppResult<PluginDetail> {
    let stored_config_version = repository::plugin_config_version(db, &detail.summary.plugin_id)?;
    Ok(detail_with_config_defaults(detail, stored_config_version))
}

fn detail_with_config_defaults(
    mut detail: PluginDetail,
    stored_config_version: Option<u32>,
) -> PluginDetail {
    detail = merge_packaged_official_manifest(detail);
    detail.config =
        config_with_schema_defaults(detail.manifest.config_schema.as_ref(), detail.config);
    migrate_legacy_official_privacy_filter_config(&mut detail, stored_config_version);
    detail
}

fn merge_packaged_official_manifest(mut detail: PluginDetail) -> PluginDetail {
    if detail.install_source != PluginInstallSource::Official
        || detail.summary.plugin_id != OFFICIAL_PRIVACY_FILTER_ID
    {
        return detail;
    }

    let Ok(fixture) = crate::app::plugins::official::official_plugin(&detail.summary.plugin_id)
    else {
        return detail;
    };

    detail.manifest = fixture.manifest;
    detail
}

fn migrate_legacy_official_privacy_filter_config(
    detail: &mut PluginDetail,
    stored_config_version: Option<u32>,
) {
    if detail.install_source != PluginInstallSource::Official
        || detail.summary.plugin_id != OFFICIAL_PRIVACY_FILTER_ID
    {
        return;
    }
    let current_version = detail.manifest.config_version.unwrap_or(1);
    if stored_config_version.unwrap_or(0) >= current_version {
        return;
    }
    let default_sensitive_types = detail
        .manifest
        .config_schema
        .as_ref()
        .and_then(|schema| schema.pointer("/properties/sensitiveTypes/default"))
        .and_then(serde_json::Value::as_array);
    let default_redaction_scopes = detail
        .manifest
        .config_schema
        .as_ref()
        .and_then(|schema| schema.pointer("/properties/redactionScopes/default"))
        .cloned();
    let Some(config) = detail.config.as_object_mut() else {
        return;
    };

    if let Some(default_sensitive_types) = default_sensitive_types {
        if let Some(sensitive_types) = config
            .get_mut("sensitiveTypes")
            .and_then(serde_json::Value::as_array_mut)
        {
            for item in default_sensitive_types {
                if !sensitive_types.iter().any(|existing| existing == item) {
                    sensitive_types.push(item.clone());
                }
            }
        }
    }

    if !config.contains_key("redactionScopes") {
        if let Some(default_redaction_scopes) = default_redaction_scopes {
            config.insert("redactionScopes".to_string(), default_redaction_scopes);
        }
    }
}

fn config_with_schema_defaults(
    schema: Option<&serde_json::Value>,
    mut config: serde_json::Value,
) -> serde_json::Value {
    if let Some(schema) = schema {
        apply_schema_defaults(&mut config, schema);
    }
    config
}

fn apply_schema_defaults(value: &mut serde_json::Value, schema: &serde_json::Value) {
    match schema.get("type").and_then(serde_json::Value::as_str) {
        Some("object") => {
            if !value.is_object() {
                if let Some(default) = schema.get("default") {
                    *value = default.clone();
                }
            }
            let Some(object) = value.as_object_mut() else {
                return;
            };
            let Some(properties) = schema
                .get("properties")
                .and_then(serde_json::Value::as_object)
            else {
                return;
            };
            for (key, child_schema) in properties {
                if !object.contains_key(key) {
                    if let Some(default) = child_schema.get("default") {
                        object.insert(key.clone(), default.clone());
                    }
                }
                if let Some(child_value) = object.get_mut(key) {
                    apply_schema_defaults(child_value, child_schema);
                }
            }
        }
        Some("array") => {
            if !value.is_array() {
                if let Some(default) = schema.get("default") {
                    *value = default.clone();
                }
            }
        }
        Some(_) | None => {
            if value.is_null() {
                if let Some(default) = schema.get("default") {
                    *value = default.clone();
                }
            }
        }
    }
}

fn append_audit(
    db: &crate::db::Db,
    plugin_id: Option<String>,
    event_type: &str,
    risk_level: &str,
    message: &str,
    details: serde_json::Value,
) -> AppResult<()> {
    repository::append_audit_log(
        db,
        repository::AppendPluginAuditLogInput {
            plugin_id,
            trace_id: None,
            event_type: event_type.to_string(),
            risk_level: risk_level.to_string(),
            message: message.to_string(),
            details,
        },
    )?;
    Ok(())
}

fn append_audit_with_tx(
    conn: &rusqlite::Transaction<'_>,
    plugin_id: Option<String>,
    event_type: &str,
    risk_level: &str,
    message: &str,
    details: serde_json::Value,
) -> AppResult<()> {
    repository::append_audit_log_with_tx(
        conn,
        repository::AppendPluginAuditLogInput {
            plugin_id,
            trace_id: None,
            event_type: event_type.to_string(),
            risk_level: risk_level.to_string(),
            message: message.to_string(),
            details,
        },
    )?;
    Ok(())
}

fn replace_dir(src: &Path, dst: &Path) -> AppResult<()> {
    let Some(parent) = dst.parent() else {
        return Err(AppError::new(
            "PLUGIN_INSTALL_FAILED",
            format!("invalid plugin install dir: {}", dst.display()),
        ));
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    if dst.exists() {
        std::fs::remove_dir_all(dst)
            .map_err(|e| format!("failed to remove existing {}: {e}", dst.display()))?;
    }
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_dir_recursive(src, dst)?;
            std::fs::remove_dir_all(src)
                .map_err(|e| format!("failed to remove staging {}: {e}", src.display()))?;
            Ok(())
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dst).map_err(|e| format!("failed to create {}: {e}", dst.display()))?;
    for entry in
        std::fs::read_dir(src).map_err(|e| format!("failed to read {}: {e}", src.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {e}"))?;
        let source_path = entry.path();
        let target_path: PathBuf = dst.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            std::fs::copy(&source_path, &target_path).map_err(|e| {
                format!(
                    "failed to copy {} -> {}: {e}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn validate_config_against_schema(
    schema: Option<&serde_json::Value>,
    config: &serde_json::Value,
) -> AppResult<()> {
    let Some(schema) = schema else {
        return Ok(());
    };
    validate_value_against_schema("$", schema, config)
}

fn validate_value_against_schema(
    path: &str,
    schema: &serde_json::Value,
    value: &serde_json::Value,
) -> AppResult<()> {
    let schema_type = schema
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::new("PLUGIN_INVALID_CONFIG_SCHEMA", "schema type is required"))?;

    match schema_type {
        "object" => {
            let object = value.as_object().ok_or_else(|| {
                AppError::new("PLUGIN_INVALID_CONFIG", format!("{path} must be an object"))
            })?;
            if let Some(required) = schema.get("required").and_then(serde_json::Value::as_array) {
                for key in required.iter().filter_map(serde_json::Value::as_str) {
                    if !object.contains_key(key) {
                        return Err(AppError::new(
                            "PLUGIN_INVALID_CONFIG",
                            format!("{path}.{key} is required"),
                        ));
                    }
                }
            }
            if let Some(properties) = schema
                .get("properties")
                .and_then(serde_json::Value::as_object)
            {
                for (key, child_schema) in properties {
                    if let Some(child_value) = object.get(key) {
                        validate_value_against_schema(
                            &format!("{path}.{key}"),
                            child_schema,
                            child_value,
                        )?;
                    }
                }
            }
            Ok(())
        }
        "array" => {
            let array = value.as_array().ok_or_else(|| {
                AppError::new("PLUGIN_INVALID_CONFIG", format!("{path} must be an array"))
            })?;
            if let Some(item_schema) = schema.get("items") {
                for (index, item) in array.iter().enumerate() {
                    validate_value_against_schema(&format!("{path}[{index}]"), item_schema, item)?;
                }
            }
            Ok(())
        }
        "string" | "password" => {
            if !value.is_string() {
                return Err(AppError::new(
                    "PLUGIN_INVALID_CONFIG",
                    format!("{path} must be a string"),
                ));
            }
            validate_enum(path, schema, value)
        }
        "number" => {
            if !value.is_number() {
                return Err(AppError::new(
                    "PLUGIN_INVALID_CONFIG",
                    format!("{path} must be a number"),
                ));
            }
            validate_enum(path, schema, value)
        }
        "integer" => {
            if value.as_i64().is_none() && value.as_u64().is_none() {
                return Err(AppError::new(
                    "PLUGIN_INVALID_CONFIG",
                    format!("{path} must be an integer"),
                ));
            }
            validate_enum(path, schema, value)
        }
        "boolean" => {
            if !value.is_boolean() {
                return Err(AppError::new(
                    "PLUGIN_INVALID_CONFIG",
                    format!("{path} must be a boolean"),
                ));
            }
            validate_enum(path, schema, value)
        }
        _ => Err(AppError::new(
            "PLUGIN_INVALID_CONFIG_SCHEMA",
            format!("unsupported schema type: {schema_type}"),
        )),
    }
}

fn validate_enum(
    path: &str,
    schema: &serde_json::Value,
    value: &serde_json::Value,
) -> AppResult<()> {
    let Some(allowed) = schema.get("enum").and_then(serde_json::Value::as_array) else {
        return Ok(());
    };
    if allowed.iter().any(|item| item == value) {
        Ok(())
    } else {
        Err(AppError::new(
            "PLUGIN_INVALID_CONFIG",
            format!("{path} is not an allowed value"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plugins::{PluginInstallSource, PluginManifest, PluginStatus};
    use crate::gateway::plugins::context::{GatewayPluginHookName, GatewayRequestHookInput};
    use crate::gateway::plugins::pipeline::{GatewayPluginPipeline, GatewayPluginPipelineConfig};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    fn manifest() -> PluginManifest {
        serde_json::from_value(serde_json::json!({
            "id": "community.prompt-helper",
            "name": "Community Prompt Helper",
            "version": "1.0.0",
            "apiVersion": "1.0.0",
            "runtime": {
                "kind": "extensionHost",
                "language": "typescript"
            },
            "main": "dist/extension.js",
            "contributes": {
                "gatewayHooks": [{
                    "name": "gateway.request.afterBodyRead",
                    "priority": 100,
                    "failurePolicy": "fail-open"
                }]
            },
            "capabilities": ["gateway.hooks"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            },
            "configSchema": {
                "type": "object",
                "required": ["mode"],
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["append_instruction", "rewrite_system_message"]
                    }
                }
            }
        }))
        .unwrap()
    }

    fn extension_manifest(plugin_id: &str, command: &str) -> PluginManifest {
        serde_json::from_value(serde_json::json!({
            "id": plugin_id,
            "name": "Acme Debug",
            "version": "1.0.0",
            "apiVersion": "1.0.0",
            "runtime": { "kind": "extensionHost", "language": "typescript" },
            "main": "dist/extension.js",
            "activationEvents": [format!("onCommand:{command}")],
            "contributes": {
                "commands": [
                    {
                        "command": command,
                        "title": "Export Trace",
                        "category": "Debug"
                    }
                ]
            },
            "capabilities": ["commands.execute", "storage.plugin"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            }
        }))
        .expect("extension manifest")
    }

    fn install_enabled_extension_with_command(
        db: &crate::db::Db,
        root: &Path,
        plugin_id: &str,
        command: &str,
    ) {
        std::fs::create_dir_all(root.join("dist")).expect("create dist");
        let manifest = extension_manifest(plugin_id, command);
        std::fs::write(
            root.join("plugin.json"),
            serde_json::to_vec_pretty(&manifest).expect("manifest json"),
        )
        .expect("write manifest");
        std::fs::write(
            root.join("dist/extension.js"),
            format!(
                r#"
                module.exports.activate = function(api) {{
                  api.commands.registerCommand("{command}", function(args) {{
                    api.storage.set("lastTraceId", args.traceId);
                    return {{
                      ok: true,
                      traceId: args.traceId,
                      storedTraceId: api.storage.get("lastTraceId")
                    }};
                  }});
                }};
                "#
            ),
        )
        .expect("write extension");

        install_plugin_manifest(
            db,
            manifest,
            PluginInstallSource::Local,
            Some(root.to_string_lossy().to_string()),
            env!("CARGO_PKG_VERSION"),
        )
        .expect("install extension");
        repository::update_plugin_status(db, plugin_id, PluginStatus::Enabled, None)
            .expect("enable extension");
    }

    fn install_enabled_counting_extension_with_command(
        db: &crate::db::Db,
        root: &Path,
        plugin_id: &str,
        command: &str,
    ) {
        std::fs::create_dir_all(root.join("dist")).expect("create dist");
        let manifest = extension_manifest(plugin_id, command);
        std::fs::write(
            root.join("plugin.json"),
            serde_json::to_vec_pretty(&manifest).expect("manifest json"),
        )
        .expect("write manifest");
        std::fs::write(
            root.join("dist/extension.js"),
            format!(
                r#"
                let executionCount = 0;
                let startRecorded = false;
                let currentStartCount = 0;

                module.exports.activate = function(api) {{
                  api.commands.registerCommand("{command}", function(args) {{
                    if (!startRecorded) {{
                      const startCount = (api.storage.get("startCount") || 0) + 1;
                      api.storage.set("startCount", startCount);
                      currentStartCount = startCount;
                      startRecorded = true;
                    }}
                    executionCount += 1;
                    return {{
                      ok: true,
                      traceId: args.traceId,
                      startCount: currentStartCount,
                      executionCount: executionCount
                    }};
                  }});
                }};
                "#
            ),
        )
        .expect("write extension");

        install_plugin_manifest(
            db,
            manifest,
            PluginInstallSource::Local,
            Some(root.to_string_lossy().to_string()),
            env!("CARGO_PKG_VERSION"),
        )
        .expect("install extension");
        repository::update_plugin_status(db, plugin_id, PluginStatus::Enabled, None)
            .expect("enable extension");
    }

    #[tokio::test]
    async fn plugin_command_execution_records_extension_report() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).expect("db");
        let plugin_root = dir.path().join("acme.debug");
        install_enabled_extension_with_command(
            &db,
            &plugin_root,
            "acme.debug",
            "acme.debug.exportTrace",
        );
        let registry = ExtensionHostInstanceRegistry::new(db.clone());

        let value = execute_plugin_command(
            &db,
            &registry,
            "acme.debug.exportTrace",
            serde_json::json!({ "traceId": "trace-1" }),
        )
        .await
        .expect("execute command");

        assert_eq!(
            value,
            serde_json::json!({ "ok": true, "traceId": "trace-1", "storedTraceId": "trace-1" })
        );
        let detail = repository::get_plugin(&db, "acme.debug").expect("plugin");
        assert_eq!(detail.config["storage"]["lastTraceId"], "trace-1");
        let reports = crate::infra::plugins::runtime_reports::list_extension_execution_reports(
            &db,
            Some("acme.debug"),
            Some("command"),
            Some("acme.debug.exportTrace"),
            None,
            20,
        )
        .expect("list reports");
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].contribution_type, "command");
        assert_eq!(reports[0].contribution_id, "acme.debug.exportTrace");
        assert_eq!(reports[0].input_budget["coldStart"], true);
    }

    #[tokio::test]
    async fn plugin_command_execution_reuses_registry_instance_between_calls() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).expect("db");
        let plugin_root = dir.path().join("acme.counter");
        install_enabled_counting_extension_with_command(
            &db,
            &plugin_root,
            "acme.counter",
            "acme.counter.run",
        );
        let registry = ExtensionHostInstanceRegistry::new(db.clone());

        let first = execute_plugin_command(
            &db,
            &registry,
            "acme.counter.run",
            serde_json::json!({ "traceId": "trace-1" }),
        )
        .await
        .expect("first command");
        let second = execute_plugin_command(
            &db,
            &registry,
            "acme.counter.run",
            serde_json::json!({ "traceId": "trace-2" }),
        )
        .await
        .expect("second command");

        assert_eq!(first["startCount"], 1);
        assert_eq!(first["executionCount"], 1);
        assert_eq!(
            second["startCount"], 1,
            "same command should reuse one registry-started extension host"
        );
        assert_eq!(second["executionCount"], 2);
        let reports = crate::infra::plugins::runtime_reports::list_extension_execution_reports(
            &db,
            Some("acme.counter"),
            Some("command"),
            Some("acme.counter.run"),
            None,
            20,
        )
        .expect("list reports");
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].input_budget["coldStart"], false);
        assert_eq!(reports[1].input_budget["coldStart"], true);
    }

    #[tokio::test]
    async fn plugin_command_execution_reports_disabled_declared_owner() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).expect("db");
        let manifest = extension_manifest("acme.disabled", "acme.disabled.run");
        install_plugin_manifest(
            &db,
            manifest,
            PluginInstallSource::Local,
            Some(
                dir.path()
                    .join("acme.disabled")
                    .to_string_lossy()
                    .to_string(),
            ),
            env!("CARGO_PKG_VERSION"),
        )
        .expect("install disabled extension");
        let registry = ExtensionHostInstanceRegistry::new(db.clone());

        let err = execute_plugin_command(
            &db,
            &registry,
            "acme.disabled.run",
            serde_json::json!({ "traceId": "trace-disabled" }),
        )
        .await
        .unwrap_err();

        assert_eq!(err.code(), "PLUGIN_COMMAND_PLUGIN_DISABLED");
    }

    #[test]
    fn active_plugin_contributions_isolates_duplicate_command_plugins() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).expect("db");
        install_enabled_extension_with_command(
            &db,
            &dir.path().join("acme.one"),
            "acme.one",
            "acme.shared.run",
        );
        install_enabled_extension_with_command(
            &db,
            &dir.path().join("acme.two"),
            "acme.two",
            "acme.shared.run",
        );
        install_enabled_extension_with_command(
            &db,
            &dir.path().join("acme.three"),
            "acme.three",
            "acme.three.run",
        );

        let snapshot = active_plugin_contributions(&db).expect("snapshot");

        assert_eq!(snapshot.commands.len(), 1);
        assert_eq!(snapshot.commands[0].plugin_id, "acme.three");
        assert_eq!(snapshot.commands[0].command, "acme.three.run");
    }

    #[test]
    fn enable_plugin_rejects_duplicate_command_with_enabled_plugin() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).expect("db");
        install_enabled_extension_with_command(
            &db,
            &dir.path().join("acme.enabled"),
            "acme.enabled",
            "acme.shared.run",
        );
        install_plugin_manifest(
            &db,
            extension_manifest("acme.disabled", "acme.shared.run"),
            PluginInstallSource::Local,
            Some(
                dir.path()
                    .join("acme.disabled")
                    .to_string_lossy()
                    .to_string(),
            ),
            env!("CARGO_PKG_VERSION"),
        )
        .expect("install disabled extension");

        let err = enable_plugin(&db, "acme.disabled", env!("CARGO_PKG_VERSION")).unwrap_err();

        assert_eq!(err.code(), "PLUGIN_DUPLICATE_COMMAND");
        assert!(err.to_string().contains("acme.shared.run"));
    }

    #[tokio::test]
    async fn plugin_command_execution_rejects_duplicate_enabled_command_owners() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).expect("db");
        install_enabled_extension_with_command(
            &db,
            &dir.path().join("acme.one"),
            "acme.one",
            "acme.shared.run",
        );
        install_enabled_extension_with_command(
            &db,
            &dir.path().join("acme.two"),
            "acme.two",
            "acme.shared.run",
        );
        let registry = ExtensionHostInstanceRegistry::new(db.clone());

        let err = execute_plugin_command(
            &db,
            &registry,
            "acme.shared.run",
            serde_json::json!({ "traceId": "trace-duplicate" }),
        )
        .await
        .unwrap_err();

        assert_eq!(err.code(), "PLUGIN_DUPLICATE_COMMAND");
        assert!(err.to_string().contains("acme.one"));
        assert!(err.to_string().contains("acme.two"));
    }

    #[test]
    fn plugin_package_work_paths_are_unique_for_same_prefix() {
        let root = Path::new("/tmp/aio-plugin-cache");

        let first_staging = unique_staging_dir(root, "local");
        let second_staging = unique_staging_dir(root, "local");
        let first_cache = unique_cache_package_path(root, "local.safe", "1.0.0");
        let second_cache = unique_cache_package_path(root, "local.safe", "1.0.0");

        assert_ne!(first_staging, second_staging);
        assert_ne!(first_cache, second_cache);
        assert!(first_staging
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("local-")));
        assert!(first_cache
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.starts_with("local.safe-1.0.0-") && name.ends_with(".aio-plugin")
            }));
    }

    #[test]
    fn service_enables_extension_host_without_permission_grants_and_preserves_config_on_disable() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();

        install_plugin_manifest(
            &db,
            manifest(),
            PluginInstallSource::Local,
            None,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();

        save_plugin_config(
            &db,
            "community.prompt-helper",
            serde_json::json!({"mode": "append_instruction"}),
        )
        .unwrap();
        let enabled =
            enable_plugin(&db, "community.prompt-helper", env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(enabled.summary.status, PluginStatus::Enabled);
        assert!(enabled.granted_permissions.is_empty());
        assert!(enabled.pending_permissions.is_empty());

        let disabled = disable_plugin(&db, "community.prompt-helper").unwrap();
        assert_eq!(disabled.summary.status, PluginStatus::Disabled);
        assert_eq!(disabled.config["mode"], "append_instruction");
    }

    #[test]
    fn local_plugin_install_records_no_pending_permissions_for_extension_host() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();

        let installed = install_plugin_manifest(
            &db,
            manifest(),
            PluginInstallSource::Local,
            None,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();

        assert_eq!(installed.granted_permissions, Vec::<String>::new());
        assert!(installed.pending_permissions.is_empty());
        assert_eq!(
            installed.manifest.capabilities,
            vec!["gateway.hooks".to_string()]
        );
    }

    #[test]
    fn enable_plugin_rejects_quarantined_even_when_ready() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();

        install_plugin_manifest(
            &db,
            manifest(),
            PluginInstallSource::Local,
            None,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();
        save_plugin_config(
            &db,
            "community.prompt-helper",
            serde_json::json!({"mode": "append_instruction"}),
        )
        .unwrap();
        quarantine_revoked_plugin(&db, "community.prompt-helper", "revoked").unwrap();

        let err =
            enable_plugin(&db, "community.prompt-helper", env!("CARGO_PKG_VERSION")).unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_INVALID_STATUS:"));
        let detail = get_plugin_detail(&db, "community.prompt-helper").unwrap();
        assert_eq!(detail.summary.status, PluginStatus::Quarantined);
    }

    #[test]
    fn enable_plugin_rejects_uninstalled_even_when_ready() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();

        install_plugin_manifest(
            &db,
            manifest(),
            PluginInstallSource::Local,
            None,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();
        save_plugin_config(
            &db,
            "community.prompt-helper",
            serde_json::json!({"mode": "append_instruction"}),
        )
        .unwrap();
        uninstall_plugin(&db, "community.prompt-helper").unwrap();

        let err =
            enable_plugin(&db, "community.prompt-helper", env!("CARGO_PKG_VERSION")).unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_INVALID_STATUS:"));
        let detail = get_plugin_detail(&db, "community.prompt-helper").unwrap();
        assert_eq!(detail.summary.status, PluginStatus::Uninstalled);
    }

    #[test]
    fn uninstall_keeps_audit_logs() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();

        install_plugin_manifest(
            &db,
            manifest(),
            PluginInstallSource::Local,
            None,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();

        uninstall_plugin(&db, "community.prompt-helper").unwrap();
        let detail = get_plugin_detail(&db, "community.prompt-helper").unwrap();
        assert_eq!(detail.summary.status, PluginStatus::Uninstalled);
        assert!(!detail.audit_logs.is_empty());
    }

    #[test]
    fn manual_permission_mutations_are_rejected_for_extension_host_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("permission-model.db")).unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        let installed = install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();
        assert!(installed.granted_permissions.is_empty());
        assert!(installed.pending_permissions.is_empty());

        let grant_err = grant_plugin_permissions(
            &db,
            "official.privacy-filter",
            vec!["log.redact".to_string()],
        )
        .unwrap_err();
        assert_eq!(grant_err.code(), "PLUGIN_PERMISSION_MODEL_REMOVED");

        let revoke_err =
            revoke_plugin_permission(&db, "official.privacy-filter", "log.redact").unwrap_err();
        assert_eq!(revoke_err.code(), "PLUGIN_PERMISSION_MODEL_REMOVED");
    }

    #[test]
    fn official_plugin_install_enable_and_uninstall_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("official-plugins.db")).unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        let installed = install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();
        assert_eq!(installed.install_source, PluginInstallSource::Official);
        assert_eq!(installed.summary.status, PluginStatus::Disabled);
        assert!(installed
            .installed_dir
            .as_deref()
            .is_some_and(|path| { path.contains("official.privacy-filter") }));
        let installed_dir = std::path::Path::new(installed.installed_dir.as_deref().unwrap());
        assert!(installed_dir.join("plugin.json").exists());
        assert!(installed_dir.join("rules/gitleaks.toml").exists());
        assert!(installed.granted_permissions.is_empty());
        assert!(installed.pending_permissions.is_empty());
        assert!(installed.manifest.permissions.is_empty());
        assert_eq!(installed.config["redactBeforeUpstream"], true);
        assert_eq!(installed.config["redactLogs"], true);

        let enabled =
            enable_plugin(&db, "official.privacy-filter", env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(enabled.summary.status, PluginStatus::Enabled);

        let active = enabled_plugins_for_gateway(&db).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].summary.plugin_id, "official.privacy-filter");

        let uninstalled = uninstall_plugin(&db, "official.privacy-filter").unwrap();
        assert_eq!(uninstalled.summary.status, PluginStatus::Uninstalled);
    }

    #[test]
    fn enabled_plugins_for_gateway_repairs_missing_plugin_tables() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("missing-plugin-tables.db")).unwrap();
        {
            let conn = db.open_connection().unwrap();
            conn.execute_batch(
                r#"
DROP TABLE plugin_runtime_failures;
DROP TABLE plugin_market_sources;
DROP TABLE plugin_audit_logs;
DROP TABLE plugin_permissions;
DROP TABLE plugin_configs;
DROP TABLE plugin_versions;
DROP TABLE plugins;
"#,
            )
            .unwrap();
        }

        let active = enabled_plugins_for_gateway(&db).unwrap();

        assert!(active.is_empty());
        let conn = db.open_connection().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'plugins' LIMIT 1",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(
            exists,
            "runtime schema repair should recreate plugin tables"
        );
    }

    #[test]
    fn enabled_plugins_for_gateway_skips_manifest_that_no_longer_validates() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("invalid-enabled-plugin.db")).unwrap();

        install_plugin_manifest(
            &db,
            manifest(),
            PluginInstallSource::Local,
            None,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();
        save_plugin_config(
            &db,
            "community.prompt-helper",
            serde_json::json!({"mode": "append_instruction"}),
        )
        .unwrap();
        enable_plugin(&db, "community.prompt-helper", env!("CARGO_PKG_VERSION")).unwrap();
        let mut invalid_manifest = manifest();
        invalid_manifest.api_version = "2.0.0".to_string();
        let invalid_manifest_json = serde_json::to_string(&invalid_manifest).unwrap();
        db.open_connection()
            .unwrap()
            .execute(
                "UPDATE plugins SET manifest_json = ?1 WHERE plugin_id = ?2",
                rusqlite::params![invalid_manifest_json, "community.prompt-helper"],
            )
            .unwrap();

        let active = enabled_plugins_for_gateway(&db).unwrap();

        assert!(active.is_empty());
    }

    #[test]
    fn legacy_runtime_db_row_is_disabled_for_ui_and_gateway() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("legacy-runtime-plugin.db")).unwrap();
        let manifest = legacy_wasm_package_manifest("local.legacy-db", "1.0.0");
        let manifest_json = manifest.to_string();
        let now = crate::shared::time::now_unix_seconds();
        db.open_connection()
            .unwrap()
            .execute(
                r#"
INSERT INTO plugins (
  plugin_id,
  name,
  current_version,
  install_source,
  status,
  manifest_json,
  config_json,
  granted_permissions_json,
  installed_dir,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', '[]', NULL, ?7, ?7)
"#,
                rusqlite::params![
                    "local.legacy-db",
                    "Legacy Rules Plugin",
                    "1.0.0",
                    "local",
                    "enabled",
                    manifest_json,
                    now
                ],
            )
            .unwrap();

        let listed = list_plugins(&db).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, PluginStatus::Disabled);
        assert_eq!(listed[0].runtime, "wasm");
        assert!(listed[0]
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("Unsupported pre-release plugin runtime")));

        let detail = get_plugin_detail(&db, "local.legacy-db").unwrap();
        assert_eq!(detail.summary.status, PluginStatus::Disabled);
        assert_eq!(detail.summary.runtime, "wasm");
        assert!(detail
            .summary
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("Unsupported pre-release plugin runtime")));

        let active = enabled_plugins_for_gateway(&db).unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn local_native_privacy_filter_db_row_is_disabled_for_ui_and_gateway() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("local-native-plugin.db")).unwrap();
        let mut manifest = serde_json::to_value(manifest()).unwrap();
        manifest["id"] = serde_json::json!("acme.native-privacy-filter");
        manifest["name"] = serde_json::json!("Native Privacy Filter");
        manifest["runtime"] = serde_json::json!({
            "kind": "native",
            "engine": "hostPrivateRedactor"
        });
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        let now = crate::shared::time::now_unix_seconds();
        db.open_connection()
            .unwrap()
            .execute(
                r#"
INSERT INTO plugins (
  plugin_id,
  name,
  current_version,
  install_source,
  status,
  manifest_json,
  config_json,
  granted_permissions_json,
  installed_dir,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', '[]', NULL, ?7, ?7)
"#,
                rusqlite::params![
                    "acme.native-privacy-filter",
                    "Native Privacy Filter",
                    "1.0.0",
                    "local",
                    "enabled",
                    manifest_json,
                    now
                ],
            )
            .unwrap();

        let listed = list_plugins(&db).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].runtime, "native:hostPrivateRedactor");
        assert_eq!(listed[0].status, PluginStatus::Disabled);
        assert!(listed[0]
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("Unsupported pre-release plugin runtime")));

        let detail = get_plugin_detail(&db, "acme.native-privacy-filter").unwrap();
        assert_eq!(detail.install_source, PluginInstallSource::Local);
        assert_eq!(detail.summary.runtime, "native:hostPrivateRedactor");
        assert_eq!(detail.summary.status, PluginStatus::Disabled);
        assert!(detail
            .summary
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("Unsupported pre-release plugin runtime")));

        let active = enabled_plugins_for_gateway(&db).unwrap();
        assert!(active.is_empty());
    }

    #[test]
    fn official_privacy_filter_install_uses_upstream_redaction_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("official-privacy-filter.db")).unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        let installed = install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();

        assert_eq!(installed.install_source, PluginInstallSource::Official);
        assert_eq!(installed.summary.status, PluginStatus::Disabled);
        assert_eq!(installed.summary.runtime, "extensionHost");
        assert_eq!(
            installed.manifest.runtime,
            crate::plugins::PluginRuntime::ExtensionHost {
                language: "typescript".to_string()
            }
        );
        assert_eq!(
            installed.manifest.main.as_deref(),
            Some("dist/extension.js")
        );
        assert!(installed
            .manifest
            .capabilities
            .iter()
            .any(|capability| capability == "privacy.redact"));
        assert!(installed
            .installed_dir
            .as_deref()
            .is_some_and(|path| { path.contains("official.privacy-filter") }));
        assert!(installed
            .installed_dir
            .as_deref()
            .is_some_and(|path| path.starts_with(installed_root.to_string_lossy().as_ref())));
        assert_eq!(installed.config["redactBeforeUpstream"], true);
        assert_eq!(installed.config["redactLogs"], true);
        assert_eq!(installed.config["profile"], "balanced");
        assert!(installed.granted_permissions.is_empty());
        assert!(installed.pending_permissions.is_empty());
        let effective_permissions = manifest_effective_permissions(&installed.manifest);
        assert!(effective_permissions.contains(&"request.body.write".to_string()));
        assert!(effective_permissions.contains(&"log.redact".to_string()));
    }

    #[test]
    fn enabled_official_privacy_filter_fills_missing_runtime_config_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("official-privacy-filter.db")).unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();
        repository::save_plugin_config(
            &db,
            "official.privacy-filter",
            1,
            &serde_json::json!({}),
            &[],
        )
        .unwrap();
        enable_plugin(&db, "official.privacy-filter", env!("CARGO_PKG_VERSION")).unwrap();

        let active = enabled_plugins_for_gateway(&db).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].summary.plugin_id, "official.privacy-filter");
        assert_eq!(active[0].config["redactBeforeUpstream"], true);
        assert_eq!(active[0].config["redactLogs"], true);
        assert_eq!(active[0].config["profile"], "balanced");
        assert!(active[0]
            .config
            .get("sensitiveTypes")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item == "cn_phone")));
        assert_eq!(
            active[0].config["redactionScopes"],
            serde_json::json!([
                "system_instructions",
                "user_prompts",
                "tool_results",
                "legacy_prompt"
            ])
        );

        let pipeline = GatewayPluginPipeline::for_tests(
            active,
            Arc::new(
                crate::app::plugins::runtime_executor::RuntimeGatewayPluginExecutor::with_db(
                    db.clone(),
                ),
            ),
            GatewayPluginPipelineConfig::default(),
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let output = rt
            .block_on(
                pipeline.run_request_hook(GatewayRequestHookInput {
                    hook_name: GatewayPluginHookName::RequestAfterBodyRead,
                    trace_id: "trace-privacy-filter-default-config".to_string(),
                    cli_key: "codex".to_string(),
                    method: axum::http::Method::POST,
                    path: "/v1/responses".to_string(),
                    query: None,
                    headers: axum::http::HeaderMap::new(),
                    body: axum::body::Bytes::from(
                        serde_json::json!({
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
                }),
            )
            .unwrap();
        let body = String::from_utf8(output.body.to_vec()).unwrap();
        assert!(body.contains("[电话]"));
        assert!(!body.contains("13344441520"));
    }

    #[test]
    fn enabled_official_privacy_filter_migrates_legacy_sensitive_type_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let db =
            crate::db::init_for_tests(&dir.path().join("official-privacy-filter-legacy-config.db"))
                .unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();
        repository::save_plugin_config(
            &db,
            "official.privacy-filter",
            1,
            &serde_json::json!({
                "redactBeforeUpstream": true,
                "redactLogs": true,
                "profile": "balanced",
                "sensitiveTypes": ["email"]
            }),
            &[],
        )
        .unwrap();
        enable_plugin(&db, "official.privacy-filter", env!("CARGO_PKG_VERSION")).unwrap();

        let active = enabled_plugins_for_gateway(&db).unwrap();

        assert_eq!(active.len(), 1);
        assert!(active[0]
            .config
            .get("sensitiveTypes")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item == "cn_phone")));
        assert_eq!(
            active[0].config["redactionScopes"],
            serde_json::json!([
                "system_instructions",
                "user_prompts",
                "tool_results",
                "legacy_prompt"
            ])
        );

        let pipeline = GatewayPluginPipeline::for_tests(
            active,
            Arc::new(
                crate::app::plugins::runtime_executor::RuntimeGatewayPluginExecutor::with_db(
                    db.clone(),
                ),
            ),
            GatewayPluginPipelineConfig::default(),
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let output = rt
            .block_on(
                pipeline.run_request_hook(GatewayRequestHookInput {
                    hook_name: GatewayPluginHookName::RequestBeforeSend,
                    trace_id: "trace-privacy-filter-legacy-config".to_string(),
                    cli_key: "codex".to_string(),
                    method: axum::http::Method::POST,
                    path: "/v1/responses".to_string(),
                    query: None,
                    headers: axum::http::HeaderMap::new(),
                    body: axum::body::Bytes::from(
                        serde_json::json!({
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
                }),
            )
            .unwrap();
        let body = String::from_utf8(output.body.to_vec()).unwrap();
        assert!(body.contains("[电话]"));
        assert!(!body.contains("13344441520"));
    }

    #[test]
    fn enabled_official_privacy_filter_respects_current_config_sensitive_type_choices() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(
            &dir.path().join("official-privacy-filter-current-config.db"),
        )
        .unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();
        repository::save_plugin_config(
            &db,
            "official.privacy-filter",
            3,
            &serde_json::json!({
                "redactBeforeUpstream": true,
                "redactLogs": true,
                "profile": "balanced",
                "sensitiveTypes": ["email"],
                "redactionScopes": ["user_prompts"]
            }),
            &[],
        )
        .unwrap();
        enable_plugin(&db, "official.privacy-filter", env!("CARGO_PKG_VERSION")).unwrap();

        let active = enabled_plugins_for_gateway(&db).unwrap();

        assert_eq!(active.len(), 1);
        assert!(active[0]
            .config
            .get("sensitiveTypes")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| !items.iter().any(|item| item == "cn_phone")));
        assert_eq!(
            active[0].config["redactionScopes"],
            serde_json::json!(["user_prompts"])
        );
    }

    #[test]
    fn enabled_official_privacy_filter_preserves_existing_redaction_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(
            &dir.path()
                .join("official-privacy-filter-redaction-scopes.db"),
        )
        .unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();
        repository::save_plugin_config(
            &db,
            "official.privacy-filter",
            2,
            &serde_json::json!({
                "redactBeforeUpstream": true,
                "redactLogs": true,
                "profile": "balanced",
                "sensitiveTypes": ["email"],
                "redactionScopes": ["user_prompts"]
            }),
            &[],
        )
        .unwrap();
        enable_plugin(&db, "official.privacy-filter", env!("CARGO_PKG_VERSION")).unwrap();

        let active = enabled_plugins_for_gateway(&db).unwrap();

        assert_eq!(active.len(), 1);
        assert_eq!(
            active[0].config["redactionScopes"],
            serde_json::json!(["user_prompts"])
        );
    }

    #[test]
    fn enabled_official_privacy_filter_merges_packaged_manifest_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("official-privacy-filter-hooks.db"))
            .unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();
        let mut legacy = repository::get_plugin(&db, "official.privacy-filter")
            .unwrap()
            .manifest;
        legacy
            .contributes
            .as_mut()
            .expect("official manifest contributes")
            .gateway_hooks
            .retain(|hook| hook.name != "gateway.request.beforeSend");
        repository::update_plugin_manifest(
            &db,
            legacy,
            Some(installed_root.to_string_lossy().to_string()),
        )
        .unwrap();
        enable_plugin(&db, "official.privacy-filter", env!("CARGO_PKG_VERSION")).unwrap();

        let active = enabled_plugins_for_gateway(&db).unwrap();

        assert_eq!(active.len(), 1);
        assert!(active[0]
            .manifest
            .contributes
            .as_ref()
            .expect("official manifest contributes")
            .gateway_hooks
            .iter()
            .any(|hook| hook.name == "gateway.request.beforeSend"));
    }

    #[test]
    fn official_privacy_filter_detail_and_enable_return_packaged_manifest_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("official-privacy-filter-detail.db"))
            .unwrap();
        let installed_root = dir.path().join("installed");
        let official_root = crate::app::plugins::official::official_resource_root_for_tests();

        install_official_plugin(
            &db,
            "official.privacy-filter",
            &official_root,
            env!("CARGO_PKG_VERSION"),
            &installed_root,
        )
        .unwrap();
        let mut legacy = repository::get_plugin(&db, "official.privacy-filter")
            .unwrap()
            .manifest;
        legacy
            .contributes
            .as_mut()
            .expect("official manifest contributes")
            .gateway_hooks
            .retain(|hook| hook.name != "gateway.request.beforeSend");
        repository::update_plugin_manifest(
            &db,
            legacy,
            Some(installed_root.to_string_lossy().to_string()),
        )
        .unwrap();

        let enabled =
            enable_plugin(&db, "official.privacy-filter", env!("CARGO_PKG_VERSION")).unwrap();
        let detail = get_plugin_detail(&db, "official.privacy-filter").unwrap();

        for item in [&enabled, &detail] {
            assert!(item
                .manifest
                .contributes
                .as_ref()
                .expect("official manifest contributes")
                .gateway_hooks
                .iter()
                .any(|hook| hook.name == "gateway.request.beforeSend"));
            assert_eq!(item.config["redactBeforeUpstream"], true);
            assert!(item
                .config
                .get("sensitiveTypes")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| items.iter().any(|item| item == "cn_phone")));
        }
    }

    fn local_package_manifest(plugin_id: &str, version: &str) -> serde_json::Value {
        serde_json::json!({
            "id": plugin_id,
            "name": "Local Package Plugin",
            "version": version,
            "apiVersion": "1.0.0",
            "runtime": {
                "kind": "extensionHost",
                "language": "typescript"
            },
            "main": "dist/extension.js",
            "contributes": {
                "gatewayHooks": [{
                    "name": "gateway.request.afterBodyRead",
                    "priority": 10,
                    "failurePolicy": "fail-open"
                }]
            },
            "capabilities": ["gateway.hooks"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            }
        })
    }

    struct PluginTestContext {
        _dir: tempfile::TempDir,
        db: crate::db::Db,
        package_dir: PathBuf,
        cache_dir: PathBuf,
    }

    fn plugin_test_context() -> PluginTestContext {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_dir = dir.path().join("packages");
        let cache_dir = dir.path().join("plugins/cache");
        std::fs::create_dir_all(&package_dir).unwrap();
        PluginTestContext {
            _dir: dir,
            db,
            package_dir,
            cache_dir,
        }
    }

    fn extension_package_manifest(
        plugin_id: &str,
        version: &str,
        contributes: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": plugin_id,
            "name": "Extension Package Plugin",
            "version": version,
            "apiVersion": "1.0.0",
            "main": "dist/extension.js",
            "runtime": { "kind": "extensionHost", "language": "typescript" },
            "activationEvents": ["onStartup"],
            "contributes": contributes,
            "capabilities": ["provider.extensionValues", "commands.execute"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            }
        })
    }

    fn write_extension_package(
        ctx: &PluginTestContext,
        plugin_id: &str,
        contributes: serde_json::Value,
    ) -> PathBuf {
        let package_path = ctx.package_dir.join(format!("{plugin_id}.aio-plugin"));
        write_local_package(
            &package_path,
            extension_package_manifest(plugin_id, "1.0.0", contributes),
        );
        package_path
    }

    fn write_extension_package_with_slots(
        ctx: &PluginTestContext,
        plugin_id: &str,
        slots: Vec<&str>,
    ) -> PathBuf {
        let ui = slots
            .into_iter()
            .map(|slot| {
                (
                    slot.to_string(),
                    serde_json::json!([{
                        "id": format!("{slot}.panel"),
                        "title": format!("{slot} panel"),
                        "schema": { "type": "section", "fields": [] }
                    }]),
                )
            })
            .collect::<serde_json::Map<String, serde_json::Value>>();
        write_extension_package(
            ctx,
            plugin_id,
            serde_json::json!({
                "ui": ui
            }),
        )
    }

    fn install_extension_manifest(db: &crate::db::Db, plugin_id: &str, slots: Vec<&str>) {
        let ui = slots
            .into_iter()
            .map(|slot| {
                (
                    slot.to_string(),
                    serde_json::json!([{
                        "id": format!("{slot}.panel"),
                        "title": format!("{slot} panel"),
                        "schema": { "type": "section", "fields": [] }
                    }]),
                )
            })
            .collect::<serde_json::Map<String, serde_json::Value>>();
        let manifest: PluginManifest = serde_json::from_value(extension_package_manifest(
            plugin_id,
            "1.0.0",
            serde_json::json!({ "ui": ui }),
        ))
        .unwrap();

        install_plugin_manifest(db, manifest, PluginInstallSource::Local, None, "0.62.0").unwrap();
    }

    fn write_local_package(path: &Path, manifest: serde_json::Value) {
        let file = std::fs::File::create(path).expect("create package");
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::FileOptions::<()>::default();
        zip.start_file("plugin.json", opts).expect("manifest entry");
        zip.write_all(manifest.to_string().as_bytes())
            .expect("manifest bytes");
        zip.start_file("rules/main.json", opts)
            .expect("rules entry");
        zip.write_all(br#"{"rules":[]}"#).expect("rules bytes");
        if manifest
            .get("runtime")
            .and_then(|runtime| runtime.get("kind"))
            .and_then(serde_json::Value::as_str)
            == Some("extensionHost")
        {
            zip.start_file("dist/extension.js", opts)
                .expect("extension entry");
            zip.write_all(b"export default {};")
                .expect("extension bytes");
        }
        zip.finish().expect("finish package");
    }

    fn legacy_wasm_package_manifest(plugin_id: &str, version: &str) -> serde_json::Value {
        let mut manifest = local_package_manifest(plugin_id, version);
        manifest["runtime"] = serde_json::json!({
            "kind": "wasm",
            "abiVersion": "1.0.0"
        });
        let object = manifest.as_object_mut().expect("manifest object");
        object.remove("main");
        object.remove("activationEvents");
        object.remove("contributes");
        manifest["hooks"] = serde_json::json!([{
            "name": "gateway.request.afterBodyRead",
            "priority": 10,
            "failurePolicy": "fail-open"
        }]);
        manifest["permissions"] = serde_json::json!(["request.body.read"]);
        manifest["capabilities"] = serde_json::json!([]);
        manifest
    }

    fn invalid_checksum() -> String {
        "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string()
    }

    #[test]
    fn plugin_local_install_preview_rejects_legacy_wasm_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("legacy-wasm.aio-plugin");
        write_local_package(
            &package_path,
            legacy_wasm_package_manifest("local.legacy-rules", "1.0.0"),
        );

        let err = preview_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap_err();

        assert_eq!(err.code(), "PLUGIN_UNSUPPORTED_RUNTIME");
        assert!(repository::get_plugin(&db, "local.legacy-rules").is_err());
    }

    #[test]
    fn plugin_local_install_preview_reports_identity_risk_and_trust_without_db_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("preview-safe.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.preview-safe", "1.0.0"),
        );

        let preview = preview_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(preview.plugin_id, "local.preview-safe");
        assert_eq!(preview.name, "Local Package Plugin");
        assert_eq!(preview.version, "1.0.0");
        assert_eq!(preview.source, PluginInstallSource::Local);
        assert_eq!(preview.runtime.kind, "extensionHost");
        assert!(preview.runtime.supported);
        assert!(preview.compatibility.compatible);
        assert!(preview.trust.unsigned);
        assert!(!preview.trust.signature_verified);
        assert!(preview
            .permissions
            .iter()
            .any(|permission| permission.permission == "request.body.read"));
        assert!(preview
            .permissions
            .iter()
            .any(|permission| permission.permission == "request.body.write"));
        assert!(preview
            .permissions
            .iter()
            .all(|permission| permission.granted && !permission.pending));
        assert_eq!(preview.hooks[0].name, "gateway.request.afterBodyRead");
        assert_eq!(
            preview.contribution_impact.capabilities,
            vec!["gateway.hooks".to_string()]
        );
        assert!(preview.blocking_reasons.is_empty());
        assert!(repository::get_plugin(&db, "local.preview-safe").is_err());
    }

    #[test]
    fn plugin_local_install_preview_reports_incompatible_manifest_without_installing() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("preview-incompatible.aio-plugin");
        let mut manifest = local_package_manifest("local.preview-incompatible", "1.0.0");
        manifest["hostCompatibility"] = serde_json::json!({
            "app": ">=999.0.0 <1000.0.0",
            "pluginApi": "^1.0.0",
            "platforms": ["macos", "windows", "linux"]
        });
        write_local_package(&package_path, manifest);

        let preview = preview_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(preview.plugin_id, "local.preview-incompatible");
        assert!(!preview.compatibility.compatible);
        assert!(preview
            .blocking_reasons
            .iter()
            .any(|notice| notice.code == "PLUGIN_INCOMPATIBLE_HOST"));
        assert!(repository::get_plugin(&db, "local.preview-incompatible").is_err());
    }

    #[test]
    fn plugin_local_install_preview_does_not_apply_legacy_permission_block_to_extension_host() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("preview-extension-host.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.preview-extension-host", "1.0.0"),
        );

        let preview = preview_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy::default(),
        )
        .unwrap();

        assert!(preview
            .blocking_reasons
            .iter()
            .all(|notice| notice.code != "PLUGIN_UNSIGNED_HIGH_RISK_PERMISSION"));
        assert!(preview
            .permissions
            .iter()
            .any(|permission| permission.permission == "request.body.write"));
        assert!(preview
            .permissions
            .iter()
            .all(|permission| !permission.pending));
        assert!(repository::get_plugin(&db, "local.preview-extension-host").is_err());
    }

    #[test]
    fn install_preview_describes_extension_contribution_impact() {
        let ctx = plugin_test_context();
        let package = write_extension_package(
            &ctx,
            "acme.openrouter",
            serde_json::json!({
                "providers": [{
                    "providerType": "openrouter",
                    "displayName": "OpenRouter",
                    "targetCliKeys": ["claude"],
                    "extensionNamespace": "openrouter"
                }],
                "ui": {
                    "providers.editor.sections": [{
                        "id": "openrouter-routing",
                        "title": "OpenRouter 路由",
                        "order": 10,
                        "schema": { "type": "section", "fields": [] }
                    }]
                },
                "commands": [{ "command": "acme.openrouter.refreshModels", "title": "刷新模型" }]
            }),
        );

        let preview = preview_plugin_from_local_package_with_policy(
            &ctx.db,
            &package,
            &ctx.cache_dir,
            "0.62.0",
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(preview
            .contribution_impact
            .providers
            .iter()
            .any(|p| p.id == "openrouter"));
        assert!(preview
            .contribution_impact
            .ui_slots
            .iter()
            .any(|s| s.slot_id == "providers.editor.sections"));
        assert!(preview
            .contribution_impact
            .commands
            .iter()
            .any(|c| c.command == "acme.openrouter.refreshModels"));
    }

    #[test]
    fn contribution_impact_update_diff_reports_removed_and_added_contributions() {
        let ctx = plugin_test_context();
        install_extension_manifest(&ctx.db, "acme.debug", vec!["logs.detail.tabs"]);
        let package =
            write_extension_package_with_slots(&ctx, "acme.debug", vec!["settings.sections"]);

        let diff = preview_plugin_update_from_local_package(
            &ctx.db,
            &package,
            &ctx.cache_dir,
            "0.62.0",
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(diff
            .contribution_changes
            .iter()
            .any(|c| c.name == "logs.detail.tabs/logs.detail.tabs.panel" && c.change == "removed"));
        assert!(diff.contribution_changes.iter().any(|c| {
            c.name == "settings.sections/settings.sections.panel" && c.change == "added"
        }));
    }

    #[test]
    fn contribution_impact_update_diff_keeps_same_id_contribution_types_distinct() {
        let ctx = plugin_test_context();
        let current_manifest: PluginManifest = serde_json::from_value(extension_package_manifest(
            "acme.collision",
            "1.0.0",
            serde_json::json!({
                "providers": [{
                    "providerType": "shared",
                    "displayName": "Shared Provider",
                    "targetCliKeys": ["codex"],
                    "extensionNamespace": "shared"
                }],
                "commands": [{ "command": "shared", "title": "Shared Command" }]
            }),
        ))
        .unwrap();
        install_plugin_manifest(
            &ctx.db,
            current_manifest,
            PluginInstallSource::Local,
            None,
            "0.62.0",
        )
        .unwrap();
        let package = write_extension_package(
            &ctx,
            "acme.collision",
            serde_json::json!({
                "providers": [{
                    "providerType": "shared",
                    "displayName": "Shared Provider Updated",
                    "targetCliKeys": ["codex"],
                    "extensionNamespace": "shared"
                }],
                "commands": [{ "command": "shared", "title": "Shared Command Updated" }]
            }),
        );

        let diff = preview_plugin_update_from_local_package(
            &ctx.db,
            &package,
            &ctx.cache_dir,
            "0.62.0",
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(diff
            .contribution_changes
            .iter()
            .any(|c| { c.kind == "provider" && c.name == "shared" && c.change == "changed" }));
        assert!(diff
            .contribution_changes
            .iter()
            .any(|c| c.kind == "command" && c.name == "shared" && c.change == "changed"));
    }

    #[test]
    fn contribution_impact_update_diff_reports_ui_contribution_item_replacement() {
        let ctx = plugin_test_context();
        let current_manifest: PluginManifest = serde_json::from_value(extension_package_manifest(
            "acme.ui-items",
            "1.0.0",
            serde_json::json!({
                "ui": {
                    "settings.sections": [
                        {
                            "id": "removed-panel",
                            "title": "Removed Panel",
                            "schema": { "type": "section", "fields": [] }
                        },
                        {
                            "id": "kept-panel",
                            "title": "Kept Panel",
                            "schema": { "type": "section", "fields": [] }
                        }
                    ]
                }
            }),
        ))
        .unwrap();
        install_plugin_manifest(
            &ctx.db,
            current_manifest,
            PluginInstallSource::Local,
            None,
            "0.62.0",
        )
        .unwrap();
        let package = write_extension_package(
            &ctx,
            "acme.ui-items",
            serde_json::json!({
                "ui": {
                    "settings.sections": [
                        {
                            "id": "kept-panel",
                            "title": "Kept Panel",
                            "schema": { "type": "section", "fields": [] }
                        },
                        {
                            "id": "added-panel",
                            "title": "Added Panel",
                            "schema": { "type": "section", "fields": [] }
                        }
                    ]
                }
            }),
        );

        let diff = preview_plugin_update_from_local_package(
            &ctx.db,
            &package,
            &ctx.cache_dir,
            "0.62.0",
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(diff.contribution_changes.iter().any(|c| {
            c.kind == "ui" && c.name == "settings.sections/removed-panel" && c.change == "removed"
        }));
        assert!(diff.contribution_changes.iter().any(|c| {
            c.kind == "ui" && c.name == "settings.sections/added-panel" && c.change == "added"
        }));
        assert!(!diff
            .contribution_changes
            .iter()
            .any(|c| c.name == "settings.sections" && c.change == "changed"));
    }

    #[test]
    fn contribution_impact_update_diff_uses_short_user_facing_summaries() {
        let ctx = plugin_test_context();
        let current_manifest: PluginManifest = serde_json::from_value(extension_package_manifest(
            "acme.summary",
            "1.0.0",
            serde_json::json!({
                "ui": {
                    "settings.sections": [{
                        "id": "summary-panel",
                        "title": "Summary Panel",
                        "schema": {
                            "type": "section",
                            "fields": [
                                { "type": "textarea", "key": "long", "label": "Long schema field" }
                            ]
                        }
                    }]
                }
            }),
        ))
        .unwrap();
        install_plugin_manifest(
            &ctx.db,
            current_manifest,
            PluginInstallSource::Local,
            None,
            "0.62.0",
        )
        .unwrap();
        let package = write_extension_package(
            &ctx,
            "acme.summary",
            serde_json::json!({
                "ui": {
                    "settings.sections": [{
                        "id": "summary-panel",
                        "title": "Summary Panel Updated",
                        "schema": {
                            "type": "section",
                            "fields": [
                                { "type": "textarea", "key": "long", "label": "Long schema field" },
                                { "type": "info", "key": "extra", "label": "Extra schema field", "value": "schema internals" }
                            ]
                        }
                    }]
                }
            }),
        );

        let diff = preview_plugin_update_from_local_package(
            &ctx.db,
            &package,
            &ctx.cache_dir,
            "0.62.0",
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..Default::default()
            },
        )
        .unwrap();

        let change = diff
            .contribution_changes
            .iter()
            .find(|c| c.kind == "ui" && c.name == "settings.sections/summary-panel")
            .expect("ui contribution change");
        assert_eq!(change.label.as_deref(), Some("Summary Panel Updated"));
        let rendered = serde_json::to_string(change).unwrap();
        assert!(!rendered.contains("\"schema\""));
        assert!(!rendered.contains("fields"));
        assert!(change
            .before
            .as_ref()
            .is_some_and(|before| before.len() <= 80));
        assert!(change.after.as_ref().is_some_and(|after| after.len() <= 80));
    }

    #[test]
    fn plugin_local_update_preview_reports_gateway_hook_and_config_changes() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let v1_package = dir.path().join("diff-v1.aio-plugin");
        write_local_package(&v1_package, local_package_manifest("local.diff", "1.0.0"));
        install_plugin_from_local_package_with_policy(
            &db,
            &v1_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let v2_package = dir.path().join("diff-v2.aio-plugin");
        let mut v2_manifest = local_package_manifest("local.diff", "1.1.0");
        v2_manifest["configVersion"] = serde_json::json!(2);
        v2_manifest["contributes"]["gatewayHooks"] = serde_json::json!([
            {
                "name": "gateway.request.afterBodyRead",
                "priority": 10,
                "failurePolicy": "fail-open"
            },
            {
                "name": "gateway.request.beforeSend",
                "priority": 20,
                "failurePolicy": "fail-open"
            }
        ]);
        write_local_package(&v2_package, v2_manifest);

        let diff = preview_plugin_update_from_local_package(
            &db,
            &v2_package,
            &cache_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(diff.plugin_id, "local.diff");
        assert_eq!(diff.from_version, "1.0.0");
        assert_eq!(diff.to_version, "1.1.0");
        assert_eq!(diff.version_direction, "upgrade");
        assert_eq!(diff.config_version_change.as_deref(), Some("1 -> 2"));
        assert!(diff.rollback_available);
        assert!(diff
            .hook_changes
            .iter()
            .any(|change| change.name == "gateway.request.beforeSend" && change.change == "added"));
        assert!(diff.permission_changes.is_empty());
        assert!(diff.contribution_changes.iter().any(|change| {
            change.kind == "gatewayHook"
                && change.name == "gateway.request.beforeSend"
                && change.change == "added"
        }));
        assert!(diff.blocking_reasons.is_empty());
    }

    #[test]
    fn plugin_local_update_preview_reports_prerelease_version_direction() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let rc_package = dir.path().join("prerelease-rc.aio-plugin");
        write_local_package(
            &rc_package,
            local_package_manifest("local.prerelease", "1.0.0-rc.1"),
        );
        install_plugin_from_local_package_with_policy(
            &db,
            &rc_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let release_package = dir.path().join("prerelease-release.aio-plugin");
        write_local_package(
            &release_package,
            local_package_manifest("local.prerelease", "1.0.0"),
        );
        let release_diff = preview_plugin_update_from_local_package(
            &db,
            &release_package,
            &cache_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(release_diff.from_version, "1.0.0-rc.1");
        assert_eq!(release_diff.to_version, "1.0.0");
        assert_eq!(release_diff.version_direction, "upgrade");
        assert!(release_diff
            .warnings
            .iter()
            .all(|notice| notice.code != "PLUGIN_UPDATE_DOWNGRADE"));

        update_plugin_from_local_package(
            &db,
            &release_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let rc_diff = preview_plugin_update_from_local_package(
            &db,
            &rc_package,
            &cache_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(rc_diff.from_version, "1.0.0");
        assert_eq!(rc_diff.to_version, "1.0.0-rc.1");
        assert_eq!(rc_diff.version_direction, "downgrade");
        assert!(rc_diff
            .warnings
            .iter()
            .any(|notice| notice.code == "PLUGIN_UPDATE_DOWNGRADE"));
    }

    #[test]
    fn plugin_local_update_preview_reports_rollback_unavailable_when_current_install_dir_is_missing(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let v1_package = dir.path().join("rollback-v1.aio-plugin");
        let v2_package = dir.path().join("rollback-v2.aio-plugin");
        let v3_package = dir.path().join("rollback-v3.aio-plugin");
        write_local_package(
            &v1_package,
            local_package_manifest("local.rollback-preview", "1.0.0"),
        );
        write_local_package(
            &v2_package,
            local_package_manifest("local.rollback-preview", "1.1.0"),
        );
        write_local_package(
            &v3_package,
            local_package_manifest("local.rollback-preview", "1.2.0"),
        );
        install_plugin_from_local_package_with_policy(
            &db,
            &v1_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();
        update_plugin_from_local_package(
            &db,
            &v2_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let diff_with_current_dir = preview_plugin_update_from_local_package(
            &db,
            &v3_package,
            &cache_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert!(diff_with_current_dir.rollback_available);

        let v2_installed_dir = installed_dir.join("local.rollback-preview").join("1.1.0");
        assert!(v2_installed_dir.is_dir());
        std::fs::remove_dir_all(&v2_installed_dir).unwrap();

        let diff_without_current_dir = preview_plugin_update_from_local_package(
            &db,
            &v3_package,
            &cache_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert!(!diff_without_current_dir.rollback_available);
    }

    fn signed_package_policy(
        package_path: &Path,
        key_seed: u8,
    ) -> (String, LocalPackageInstallPolicy) {
        use base64::Engine;
        use ed25519_dalek::Signer;
        use sha2::{Digest, Sha256};

        let package_bytes = std::fs::read(package_path).unwrap();
        let expected_checksum = crate::infra::plugins::signing::verify_checksum(
            &package_bytes,
            &format!("sha256:{:x}", Sha256::digest(&package_bytes)),
        )
        .unwrap();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[key_seed; 32]);
        let signature = signing_key.sign(&package_bytes);
        let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        let public_key_b64 = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().to_bytes());

        (
            expected_checksum,
            LocalPackageInstallPolicy {
                signature: Some(signature_b64),
                public_key: Some(public_key_b64),
                developer_mode: false,
                ..LocalPackageInstallPolicy::default()
            },
        )
    }

    fn hex_to_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }

    #[test]
    fn plugin_local_install_installs_package_into_cache_and_installed_dir() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("local-safe.aio-plugin");
        write_local_package(&package_path, local_package_manifest("local.safe", "1.0.0"));
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");

        let detail = install_plugin_from_local_package(
            &db,
            &package_path,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();

        assert_eq!(detail.summary.plugin_id, "local.safe");
        assert_eq!(detail.install_source, PluginInstallSource::Local);
        assert_eq!(detail.summary.status, PluginStatus::Disabled);
        assert!(detail
            .installed_dir
            .as_deref()
            .is_some_and(|path| path.ends_with("plugins/installed/local.safe/1.0.0")));
        assert!(installed_dir
            .join("local.safe")
            .join("1.0.0")
            .join("rules/main.json")
            .exists());
        let cached_packages: Vec<_> = std::fs::read_dir(&cache_dir).unwrap().collect();
        assert_eq!(cached_packages.len(), 1);
        assert!(detail
            .audit_logs
            .iter()
            .any(|log| log.event_type == "plugin.installed"));
    }

    #[test]
    fn plugin_local_install_rejects_legacy_wasm_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("legacy-wasm-install.aio-plugin");
        write_local_package(
            &package_path,
            legacy_wasm_package_manifest("local.legacy-install", "1.0.0"),
        );
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");

        let err = install_plugin_from_local_package(
            &db,
            &package_path,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap_err();

        assert_eq!(err.code(), "PLUGIN_UNSUPPORTED_RUNTIME");
        assert!(repository::get_plugin(&db, "local.legacy-install").is_err());
        assert!(!installed_dir.join("local.legacy-install").exists());
    }

    #[test]
    fn plugin_local_install_rolls_back_invalid_package_without_db_row_or_install_dir() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("invalid.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.bad", "not-semver"),
        );
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");

        let err = install_plugin_from_local_package(
            &db,
            &package_path,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_INVALID_VERSION:"));
        assert!(repository::get_plugin(&db, "local.bad").is_err());
        assert!(!installed_dir.join("local.bad").exists());
        assert!(!cache_dir.join("staging").exists());
    }

    #[test]
    fn plugin_local_install_rejects_reserved_official_privacy_filter_package() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("fake-official-privacy-filter.aio-plugin");
        let manifest = local_package_manifest("official.privacy-filter", "1.0.0");
        write_local_package(&package_path, manifest);

        let err = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_RESERVED_OFFICIAL_ID:"));
        assert!(repository::get_plugin(&db, "official.privacy-filter").is_err());
    }

    #[test]
    fn plugin_local_install_preview_rejects_fake_official_native_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir
            .path()
            .join("fake-official-privacy-filter-preview.aio-plugin");
        let mut manifest = local_package_manifest("official.privacy-filter", "1.0.0");
        manifest["runtime"] = serde_json::json!({
            "kind": "native",
            "engine": "hostPrivateRedactor"
        });
        manifest["hooks"] = serde_json::json!([
            {
                "name": "gateway.request.afterBodyRead",
                "priority": 10,
                "failurePolicy": "fail-open"
            },
            {
                "name": "log.beforePersist",
                "priority": 20,
                "failurePolicy": "fail-closed"
            }
        ]);
        manifest["permissions"] =
            serde_json::json!(["request.body.read", "request.body.write", "log.redact"]);
        write_local_package(&package_path, manifest);

        let err = preview_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap_err();

        assert_eq!(err.code(), "PLUGIN_UNSUPPORTED_RUNTIME");
        assert!(repository::get_plugin(&db, "official.privacy-filter").is_err());
    }

    #[test]
    fn plugin_package_security_rejects_checksum_mismatch_without_installing() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("checksum-mismatch.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.checksum", "1.0.0"),
        );
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");

        let err = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                expected_checksum: Some(invalid_checksum()),
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_CHECKSUM_MISMATCH:"));
        assert!(repository::get_plugin(&db, "local.checksum").is_err());
        assert!(!installed_dir.join("local.checksum").exists());
        assert!(!cache_dir.join("staging").exists());
    }

    #[test]
    fn plugin_signature_verification_accepts_signed_local_install() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("signed-valid.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.signed-valid", "1.0.0"),
        );
        let (expected_checksum, mut policy) = signed_package_policy(&package_path, 7);
        policy.expected_checksum = Some(expected_checksum);

        let detail = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            policy,
        )
        .unwrap();

        assert_eq!(detail.summary.plugin_id, "local.signed-valid");
        assert_eq!(detail.summary.status, PluginStatus::Disabled);
        assert!(detail
            .installed_dir
            .as_deref()
            .is_some_and(|path| path.ends_with("plugins/installed/local.signed-valid/1.0.0")));
    }

    #[test]
    fn plugin_signature_verification_records_signed_local_install() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("signed-extension.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.signed-extension", "1.0.0"),
        );
        let (expected_checksum, mut policy) = signed_package_policy(&package_path, 8);
        policy.expected_checksum = Some(expected_checksum);

        let detail = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            policy,
        )
        .unwrap();

        assert_eq!(detail.summary.plugin_id, "local.signed-extension");
        let install_audit = detail
            .audit_logs
            .iter()
            .find(|log| log.event_type == "plugin.installed")
            .unwrap();
        assert_eq!(install_audit.details["unsigned"], false);
    }

    #[test]
    fn plugin_local_package_install_records_no_pending_permissions_for_extension_host() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("signed-extension-host.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.extension-host", "1.0.0"),
        );
        let (expected_checksum, mut policy) = signed_package_policy(&package_path, 9);
        policy.expected_checksum = Some(expected_checksum);

        let detail = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            policy,
        )
        .unwrap();

        assert_eq!(detail.granted_permissions, Vec::<String>::new());
        assert!(detail.pending_permissions.is_empty());
        assert_eq!(
            detail.manifest.capabilities,
            vec!["gateway.hooks".to_string()]
        );
    }

    #[test]
    fn plugin_market_revoked_quarantines_installed_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("revoked-v1.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("community.revoked", "1.0.0"),
        );
        install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let detail =
            quarantine_revoked_plugin(&db, "community.revoked", "Plugin revoked by market index")
                .unwrap();

        assert_eq!(detail.summary.status, PluginStatus::Quarantined);
        assert_eq!(
            detail.summary.last_error.as_deref(),
            Some("Plugin revoked by market index")
        );
        assert!(detail
            .audit_logs
            .iter()
            .any(|log| log.event_type == "plugin.quarantined"));
    }

    #[test]
    fn github_release_plugin_install_installs_verified_artifact_bytes() {
        use sha2::Digest;

        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("github-release.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("github.release", "1.0.0"),
        );
        let package_bytes = std::fs::read(&package_path).unwrap();
        let checksum = format!("sha256:{:x}", sha2::Sha256::digest(&package_bytes));
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");

        let detail = install_plugin_from_remote_package_bytes(
            &db,
            package_bytes.clone(),
            "https://github.com/acme/release/releases/download/v1/plugin.aio-plugin",
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            RemotePackageInstallPolicy {
                install_source: PluginInstallSource::GithubRelease,
                expected_plugin_id: "github.release".to_string(),
                expected_checksum: checksum,
                signature: None,
                public_key: None,
                market_source_url: None,
            },
        )
        .unwrap();

        assert_eq!(detail.install_source, PluginInstallSource::GithubRelease);
        assert_eq!(detail.summary.plugin_id, "github.release");
        assert!(installed_dir
            .join("github.release")
            .join("1.0.0")
            .join("plugin.json")
            .exists());
    }

    #[test]
    fn github_release_plugin_install_ignores_market_source_url_in_audit() {
        use sha2::Digest;

        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("github-release-market-url.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("github.market-url", "1.0.0"),
        );
        let package_bytes = std::fs::read(&package_path).unwrap();
        let checksum = format!("sha256:{:x}", sha2::Sha256::digest(&package_bytes));

        let detail = install_plugin_from_remote_package_bytes(
            &db,
            package_bytes,
            "https://github.com/acme/release/releases/download/v1/plugin.aio-plugin",
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            RemotePackageInstallPolicy {
                install_source: PluginInstallSource::GithubRelease,
                expected_plugin_id: "github.market-url".to_string(),
                expected_checksum: checksum,
                signature: None,
                public_key: None,
                market_source_url: Some("https://plugins.example.test/index.json".to_string()),
            },
        )
        .unwrap();

        let install_audit = detail
            .audit_logs
            .iter()
            .find(|log| log.event_type == "plugin.remote.installed")
            .unwrap();
        assert_eq!(install_audit.details["source"], "github_release");
        assert!(install_audit.details.get("marketSourceUrl").is_none());
    }

    #[test]
    fn github_release_plugin_install_rejects_checksum_mismatch_without_installing() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("github-release-bad.aio-plugin");
        write_local_package(&package_path, local_package_manifest("github.bad", "1.0.0"));
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");

        let err = install_plugin_from_remote_package_bytes(
            &db,
            std::fs::read(&package_path).unwrap(),
            "https://github.com/acme/release/releases/download/v1/plugin.aio-plugin",
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            RemotePackageInstallPolicy {
                install_source: PluginInstallSource::GithubRelease,
                expected_plugin_id: "github.bad".to_string(),
                expected_checksum: invalid_checksum(),
                signature: None,
                public_key: None,
                market_source_url: None,
            },
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_CHECKSUM_MISMATCH:"));
        assert!(repository::get_plugin(&db, "github.bad").is_err());
        assert!(!installed_dir.join("github.bad").exists());
        assert!(!cache_dir.join("staging").exists());
    }

    #[test]
    fn plugin_remote_install_uses_trusted_market_source_public_key() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("market-signed-extension.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("market.signed-extension", "1.0.0"),
        );
        let package_bytes = std::fs::read(&package_path).unwrap();
        let (expected_checksum, trusted_policy) = signed_package_policy(&package_path, 9);
        let trusted_public_key = trusted_policy.public_key.clone().unwrap();
        let caller_public_key = signed_package_policy(&package_path, 10)
            .1
            .public_key
            .unwrap();
        let conn = db.open_connection().unwrap();
        conn.execute(
            r#"
INSERT INTO plugin_market_sources(
  name,
  index_url,
  enabled,
  trusted_public_key,
  created_at,
  updated_at
) VALUES (?1, ?2, 1, ?3, 1, 1)
"#,
            rusqlite::params![
                "Community",
                "https://plugins.example.test/index.json",
                trusted_public_key
            ],
        )
        .unwrap();
        drop(conn);

        let detail = install_plugin_from_remote_package_bytes(
            &db,
            package_bytes,
            "https://plugins.example.test/download/market-signed-risky.aio-plugin",
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            RemotePackageInstallPolicy {
                install_source: PluginInstallSource::Market,
                expected_plugin_id: "market.signed-extension".to_string(),
                expected_checksum,
                signature: trusted_policy.signature,
                public_key: Some(caller_public_key),
                market_source_url: None,
            },
        )
        .unwrap();

        assert_eq!(detail.summary.plugin_id, "market.signed-extension");
        assert_eq!(detail.install_source, PluginInstallSource::Market);
        assert_eq!(detail.granted_permissions, Vec::<String>::new());
        assert!(detail.pending_permissions.is_empty());
        let install_audit = detail
            .audit_logs
            .iter()
            .find(|log| log.event_type == "plugin.remote.installed")
            .unwrap();
        assert_eq!(install_audit.details["source"], "market");
        assert_eq!(
            install_audit.details["sourceUrl"],
            "https://plugins.example.test/download/market-signed-risky.aio-plugin"
        );
        assert_eq!(install_audit.details["unsigned"], false);
        assert_eq!(install_audit.details["signatureVerified"], true);
    }

    #[test]
    fn plugin_remote_install_uses_market_source_url_when_download_host_differs() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("market-cdn-signed.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("market.cdn-signed", "1.0.0"),
        );
        let package_bytes = std::fs::read(&package_path).unwrap();
        let (expected_checksum, trusted_policy) = signed_package_policy(&package_path, 19);
        let trusted_public_key = trusted_policy.public_key.clone().unwrap();

        let conn = db.open_connection().unwrap();
        conn.execute(
            r#"
INSERT INTO plugin_market_sources(
  name,
  index_url,
  enabled,
  trusted_public_key,
  created_at,
  updated_at
) VALUES (?1, ?2, 1, ?3, 1, 1)
"#,
            rusqlite::params![
                "Community CDN",
                "https://plugins.example.test/index.json",
                trusted_public_key
            ],
        )
        .unwrap();
        drop(conn);

        let detail = install_plugin_from_remote_package_bytes(
            &db,
            package_bytes,
            "https://cdn.example.test/download/market-cdn-signed.aio-plugin",
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            RemotePackageInstallPolicy {
                install_source: PluginInstallSource::Market,
                expected_plugin_id: "market.cdn-signed".to_string(),
                expected_checksum,
                signature: trusted_policy.signature,
                public_key: None,
                market_source_url: Some("https://plugins.example.test/index.json".to_string()),
            },
        )
        .unwrap();

        assert_eq!(detail.summary.plugin_id, "market.cdn-signed");
        assert_eq!(detail.install_source, PluginInstallSource::Market);
        let install_audit = detail
            .audit_logs
            .iter()
            .find(|log| log.event_type == "plugin.remote.installed")
            .unwrap();
        assert_eq!(
            install_audit.details["sourceUrl"],
            "https://cdn.example.test/download/market-cdn-signed.aio-plugin"
        );
        assert_eq!(
            install_audit.details["marketSourceUrl"],
            "https://plugins.example.test/index.json"
        );
        assert_eq!(install_audit.details["signatureVerified"], true);
    }

    #[test]
    fn plugin_remote_install_rejects_signed_market_package_without_trusted_source() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("market-untrusted.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("market.untrusted", "1.0.0"),
        );
        let package_bytes = std::fs::read(&package_path).unwrap();
        let (expected_checksum, policy) = signed_package_policy(&package_path, 11);

        let err = install_plugin_from_remote_package_bytes(
            &db,
            package_bytes,
            "https://untrusted.example.test/download/market-untrusted.aio-plugin",
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            RemotePackageInstallPolicy {
                install_source: PluginInstallSource::Market,
                expected_plugin_id: "market.untrusted".to_string(),
                expected_checksum,
                signature: policy.signature,
                public_key: policy.public_key,
                market_source_url: None,
            },
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_MARKET_TRUSTED_PUBLIC_KEY_REQUIRED:"));
        assert!(repository::get_plugin(&db, "market.untrusted").is_err());
    }

    #[test]
    fn plugin_unsigned_offline_install_allows_extension_host_without_legacy_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("unsigned-extension.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.unsigned-extension", "1.0.0"),
        );

        let detail = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: false,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(detail.summary.plugin_id, "local.unsigned-extension");
        assert!(detail.pending_permissions.is_empty());
        assert!(detail
            .audit_logs
            .iter()
            .any(|log| log.details["unsigned"] == true));
    }

    #[test]
    fn plugin_unsigned_offline_install_allows_low_risk_in_developer_mode_as_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("unsigned-low-risk.aio-plugin");
        write_local_package(
            &package_path,
            local_package_manifest("local.low-risk", "1.0.0"),
        );

        let detail = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(detail.summary.status, PluginStatus::Disabled);
        assert!(detail
            .audit_logs
            .iter()
            .any(|log| log.details["unsigned"] == true));
    }

    #[test]
    fn plugin_update_keeps_existing_config_without_legacy_permission_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let v1_package = dir.path().join("plugin-v1.aio-plugin");
        write_local_package(
            &v1_package,
            local_package_manifest("local.updatable", "1.0.0"),
        );
        install_plugin_from_local_package_with_policy(
            &db,
            &v1_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();
        save_plugin_config(&db, "local.updatable", serde_json::json!({"enabled": true})).unwrap();

        let v2_package = dir.path().join("plugin-v2.aio-plugin");
        write_local_package(
            &v2_package,
            local_package_manifest("local.updatable", "1.1.0"),
        );

        let updated = update_plugin_from_local_package(
            &db,
            &v2_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(updated.summary.current_version.as_deref(), Some("1.1.0"));
        assert_eq!(updated.config["enabled"], true);
        assert!(updated.granted_permissions.is_empty());
        assert!(updated.pending_permissions.is_empty());
    }

    #[test]
    fn plugin_update_rollback_keeps_old_version_when_new_package_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let v1_package = dir.path().join("plugin-v1.aio-plugin");
        write_local_package(
            &v1_package,
            local_package_manifest("local.rollback", "1.0.0"),
        );
        install_plugin_from_local_package_with_policy(
            &db,
            &v1_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let bad_package = dir.path().join("plugin-bad.aio-plugin");
        write_local_package(
            &bad_package,
            local_package_manifest("local.rollback", "not-semver"),
        );

        let err = update_plugin_from_local_package(
            &db,
            &bad_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_INVALID_VERSION:"));
        let current = get_plugin_detail(&db, "local.rollback").unwrap();
        assert_eq!(current.summary.current_version.as_deref(), Some("1.0.0"));
        assert!(installed_dir
            .join("local.rollback")
            .join("1.0.0")
            .join("plugin.json")
            .exists());
        assert!(!installed_dir
            .join("local.rollback")
            .join("not-semver")
            .exists());
    }

    #[test]
    fn plugin_update_rollback_rejects_invalid_signature_and_keeps_old_version() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let v1_package = dir.path().join("signed-plugin-v1.aio-plugin");
        write_local_package(&v1_package, local_package_manifest("local.signed", "1.0.0"));
        install_plugin_from_local_package_with_policy(
            &db,
            &v1_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let v2_package = dir.path().join("signed-plugin-v2.aio-plugin");
        write_local_package(&v2_package, local_package_manifest("local.signed", "1.1.0"));
        let signature_for_empty_payload = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            hex_to_bytes("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b").as_slice(),
        );

        let err = update_plugin_from_local_package(
            &db,
            &v2_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                signature: Some(signature_for_empty_payload),
                public_key: Some("11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=".to_string()),
                developer_mode: false,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_SIGNATURE_INVALID:"));
        let current = get_plugin_detail(&db, "local.signed").unwrap();
        assert_eq!(current.summary.current_version.as_deref(), Some("1.0.0"));
        assert!(installed_dir
            .join("local.signed")
            .join("1.0.0")
            .join("plugin.json")
            .exists());
        assert!(!installed_dir.join("local.signed").join("1.1.0").exists());
    }

    #[test]
    fn plugin_update_rollback_can_manually_restore_previous_version() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let v1_package = dir.path().join("plugin-v1.aio-plugin");
        let v2_package = dir.path().join("plugin-v2.aio-plugin");
        write_local_package(&v1_package, local_package_manifest("local.manual", "1.0.0"));
        write_local_package(&v2_package, local_package_manifest("local.manual", "1.1.0"));
        install_plugin_from_local_package_with_policy(
            &db,
            &v1_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();
        update_plugin_from_local_package(
            &db,
            &v2_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let rolled_back = rollback_plugin_to_version(&db, "local.manual", "1.0.0").unwrap();

        assert_eq!(
            rolled_back.summary.current_version.as_deref(),
            Some("1.0.0")
        );
        assert!(rolled_back
            .installed_dir
            .as_deref()
            .is_some_and(|path| path.ends_with("plugins/installed/local.manual/1.0.0")));
    }

    #[test]
    fn plugin_update_rollback_reconciles_config_version_without_legacy_permission_state() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let v1_package = dir.path().join("plugin-v1.aio-plugin");
        let v2_package = dir.path().join("plugin-v2.aio-plugin");

        let mut v1_manifest = local_package_manifest("local.rollback-state", "1.0.0");
        v1_manifest["configVersion"] = serde_json::json!(1);
        write_local_package(&v1_package, v1_manifest);

        let mut v2_manifest = local_package_manifest("local.rollback-state", "1.1.0");
        v2_manifest["configVersion"] = serde_json::json!(2);
        write_local_package(&v2_package, v2_manifest);

        install_plugin_from_local_package_with_policy(
            &db,
            &v1_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();
        save_plugin_config(
            &db,
            "local.rollback-state",
            serde_json::json!({"enabled": true, "extra": "kept"}),
        )
        .unwrap();

        let updated = update_plugin_from_local_package(
            &db,
            &v2_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();
        assert!(updated.pending_permissions.is_empty());

        let rolled_back = rollback_plugin_to_version(&db, "local.rollback-state", "1.0.0").unwrap();

        assert_eq!(
            rolled_back.summary.current_version.as_deref(),
            Some("1.0.0")
        );
        assert!(rolled_back.granted_permissions.is_empty());
        assert!(rolled_back.pending_permissions.is_empty());
        assert_eq!(rolled_back.config["enabled"], true);
        assert_eq!(
            repository::plugin_config_version(&db, "local.rollback-state").unwrap(),
            Some(1)
        );
        assert!(enable_plugin(&db, "local.rollback-state", env!("CARGO_PKG_VERSION")).is_ok());
    }

    #[test]
    fn plugin_update_rollback_rejects_missing_historical_install_dir() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let cache_dir = dir.path().join("plugins/cache");
        let installed_dir = dir.path().join("plugins/installed");
        let v1_package = dir.path().join("plugin-v1.aio-plugin");
        let v2_package = dir.path().join("plugin-v2.aio-plugin");
        write_local_package(
            &v1_package,
            local_package_manifest("local.missing-rollback", "1.0.0"),
        );
        write_local_package(
            &v2_package,
            local_package_manifest("local.missing-rollback", "1.1.0"),
        );
        install_plugin_from_local_package_with_policy(
            &db,
            &v1_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();
        update_plugin_from_local_package(
            &db,
            &v2_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        let v1_installed_dir = installed_dir.join("local.missing-rollback").join("1.0.0");
        assert!(v1_installed_dir.is_dir());
        std::fs::remove_dir_all(&v1_installed_dir).unwrap();

        let err = rollback_plugin_to_version(&db, "local.missing-rollback", "1.0.0").unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_ROLLBACK_UNAVAILABLE:"));
        let current = get_plugin_detail(&db, "local.missing-rollback").unwrap();
        assert_eq!(current.summary.current_version.as_deref(), Some("1.1.0"));
    }
}
