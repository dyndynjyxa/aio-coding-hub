use crate::domain::plugins::{
    permission_risk, validate_manifest, PluginDetail, PluginInstallSource, PluginManifest,
    PluginPermissionRisk, PluginRuntime, PluginStatus,
};
use crate::infra::plugins::{package, repository, signing};
use crate::shared::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

const OFFICIAL_PRIVACY_FILTER_ID: &str = "official.privacy-filter";

pub(crate) fn list_plugins(db: &crate::db::Db) -> AppResult<Vec<crate::plugins::PluginSummary>> {
    repository::list_plugins(db)
}

pub(crate) fn get_plugin_detail(db: &crate::db::Db, plugin_id: &str) -> AppResult<PluginDetail> {
    detail_with_config_defaults_for_db(db, repository::get_plugin(db, plugin_id)?)
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
    for summary in repository::list_plugins(db)? {
        if summary.status != PluginStatus::Enabled {
            continue;
        }
        out.push(detail_with_config_defaults_for_db(
            db,
            repository::get_plugin(db, &summary.plugin_id)?,
        )?);
    }
    Ok(out)
}

fn is_missing_plugin_table_error(err: &AppError) -> bool {
    let message = err.to_string();
    message.contains("no such table: plugins") || message.contains("no such table: plugin_")
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
    let detail =
        repository::save_plugin_permissions(db, plugin_id, &fixture.manifest.permissions, &[])?;
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
    validate_manifest(&manifest, host_version)?;
    validate_reserved_official_source(&manifest, install_source)?;
    let plugin_id = manifest.id.clone();
    let requested_permissions = manifest.permissions.clone();
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
        repository::save_plugin_permissions(db, &plugin_id, &[], &requested_permissions)?
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

#[derive(Debug, Clone, Default)]
pub(crate) struct LocalPackageInstallPolicy {
    pub(crate) expected_plugin_id: Option<String>,
    pub(crate) expected_checksum: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) public_key: Option<String>,
    pub(crate) allow_unsigned: bool,
    pub(crate) developer_mode: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RemotePackageInstallPolicy {
    pub(crate) install_source: PluginInstallSource,
    pub(crate) expected_plugin_id: String,
    pub(crate) expected_checksum: String,
    pub(crate) signature: Option<String>,
    pub(crate) public_key: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct PackageTrust {
    signature_verified: bool,
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
    let staging_dir =
        staging_root.join(format!("local-{}", crate::shared::time::now_unix_seconds()));
    let extracted = match package::extract_plugin_package(
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
    let cache_package_path = cache_dir.join(format!(
        "{}-{}-{}.aio-plugin",
        plugin_id,
        version,
        crate::shared::time::now_unix_seconds()
    ));

    let result = (|| -> AppResult<PluginDetail> {
        std::fs::copy(package_path, &cache_package_path).map_err(|e| {
            format!(
                "failed to copy plugin package {} -> {}: {e}",
                package_path.display(),
                cache_package_path.display()
            )
        })?;

        replace_dir(&extracted.root_dir, &installed_dir)?;
        let requested_permissions = extracted.manifest.permissions.clone();
        repository::insert_plugin(
            db,
            repository::InsertPluginInput {
                manifest: extracted.manifest.clone(),
                install_source: PluginInstallSource::Local,
                status: PluginStatus::Disabled,
                installed_dir: Some(installed_dir.to_string_lossy().to_string()),
            },
        )?;
        let detail =
            repository::save_plugin_permissions(db, &plugin_id, &[], &requested_permissions)?;
        append_audit(
            db,
            Some(plugin_id.clone()),
            "plugin.installed",
            "medium",
            "Local plugin package installed",
            serde_json::json!({
                "source": "local",
                "packageChecksum": extracted.checksum,
                "cachedPackage": cache_package_path.to_string_lossy(),
                "unsigned": !trust.signature_verified,
                "developerMode": policy.developer_mode,
            }),
        )?;
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
    let package_path = cache_dir.join(format!(
        "remote-{}-{}.aio-plugin",
        crate::app_paths::plugin_id_path_segment(&policy.expected_plugin_id)?,
        crate::shared::time::now_unix_seconds()
    ));
    std::fs::write(&package_path, &package_bytes).map_err(|e| {
        format!(
            "failed to write remote plugin package cache {}: {e}",
            package_path.display()
        )
    })?;

    let signature = policy.signature.clone();
    let public_key = remote_package_trusted_public_key(db, source_url, &policy)?;
    let result = install_plugin_from_local_package_with_policy(
        db,
        &package_path,
        cache_dir,
        installed_root,
        host_version,
        LocalPackageInstallPolicy {
            expected_plugin_id: Some(policy.expected_plugin_id),
            expected_checksum: Some(policy.expected_checksum),
            signature,
            public_key,
            allow_unsigned: false,
            developer_mode: false,
        },
    )
    .and_then(|detail| {
        let plugin_id = detail.summary.plugin_id.clone();
        let detail = repository::insert_plugin(
            db,
            repository::InsertPluginInput {
                manifest: detail.manifest.clone(),
                install_source: policy.install_source,
                status: PluginStatus::Disabled,
                installed_dir: detail.installed_dir.clone(),
            },
        )?;
        append_audit(
            db,
            Some(plugin_id.clone()),
            "plugin.remote.installed",
            "medium",
            "Remote plugin package installed",
            serde_json::json!({
                "sourceUrl": source_url,
                "source": policy.install_source.as_str(),
            }),
        )?;
        let next = repository::get_plugin(db, &plugin_id).unwrap_or(detail);
        tracing::info!(
            plugin_id,
            source = policy.install_source.as_str(),
            "remote plugin package installed"
        );
        Ok(next)
    });

    let _ = std::fs::remove_file(&package_path);
    result
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
            repository::trusted_market_public_key_for_url(db, source_url)?
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
    let staging_dir = staging_root.join(format!(
        "update-{}",
        crate::shared::time::now_unix_seconds()
    ));
    let extracted = match package::extract_plugin_package(
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
        let granted: Vec<String> = current
            .granted_permissions
            .iter()
            .filter(|permission| extracted.manifest.permissions.contains(*permission))
            .cloned()
            .collect();
        let pending: Vec<String> = extracted
            .manifest
            .permissions
            .iter()
            .filter(|permission| !granted.contains(permission))
            .cloned()
            .collect();
        repository::update_plugin_manifest(
            db,
            extracted.manifest.clone(),
            Some(installed_dir.to_string_lossy().to_string()),
        )?;
        repository::save_plugin_config(
            db,
            &plugin_id,
            extracted.manifest.config_version.unwrap_or(1),
            &current.config,
            &[],
        )?;
        let detail = repository::save_plugin_permissions(db, &plugin_id, &granted, &pending)?;
        append_audit(
            db,
            Some(plugin_id.clone()),
            "plugin.updated",
            "high",
            "Plugin updated from local package",
            serde_json::json!({
                "fromVersion": current.summary.current_version,
                "toVersion": extracted.manifest.version,
                "pendingPermissions": pending,
                "unsigned": !trust.signature_verified,
                "developerMode": policy.developer_mode,
            }),
        )?;
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
    let detail = repository::update_plugin_manifest(db, manifest, installed_dir)?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.rollback",
        "high",
        "Plugin rolled back",
        serde_json::json!({ "version": version }),
    )?;
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
    manifest: &PluginManifest,
    policy: &LocalPackageInstallPolicy,
    trust: PackageTrust,
) -> AppResult<()> {
    if trust.signature_verified {
        return Ok(());
    }
    if !policy.allow_unsigned || !policy.developer_mode {
        for permission in &manifest.permissions {
            if matches!(
                permission_risk(permission),
                Some(PluginPermissionRisk::High | PluginPermissionRisk::Critical)
            ) {
                return Err(AppError::new(
                    "PLUGIN_UNSIGNED_HIGH_RISK_PERMISSION",
                    format!("unsigned plugin cannot request high-risk permission: {permission}"),
                ));
            }
        }
    }
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
    validate_manifest(&extracted.manifest, host_version)?;
    validate_reserved_official_source(&extracted.manifest, PluginInstallSource::Local)?;
    let trust = verify_local_package(extracted, policy)?;
    enforce_unsigned_install_policy(&extracted.manifest, policy, trust)?;
    Ok(trust)
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
    validate_manifest(&detail.manifest, host_version)?;
    ensure_runtime_enabled(&detail.manifest)?;
    ensure_required_permissions_granted(&detail)?;
    validate_config_against_schema(detail.manifest.config_schema.as_ref(), &detail.config)?;
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
    permissions: Vec<String>,
) -> AppResult<PluginDetail> {
    let detail = repository::get_plugin(db, plugin_id)?;
    let mut granted = detail.granted_permissions;
    for permission in permissions {
        if !detail.manifest.permissions.contains(&permission) {
            return Err(AppError::new(
                "PLUGIN_PERMISSION_NOT_REQUESTED",
                format!("plugin did not request permission: {permission}"),
            ));
        }
        if !granted.contains(&permission) {
            granted.push(permission);
        }
    }
    granted.sort();
    let next = repository::save_plugin_permissions(db, plugin_id, &granted, &[])?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.permissions.granted",
        "high",
        "Plugin permissions granted",
        serde_json::json!({ "permissions": granted }),
    )?;
    Ok(next)
}

pub(crate) fn revoke_plugin_permission(
    db: &crate::db::Db,
    plugin_id: &str,
    permission: &str,
) -> AppResult<PluginDetail> {
    let detail = repository::get_plugin(db, plugin_id)?;
    let granted: Vec<String> = detail
        .granted_permissions
        .into_iter()
        .filter(|item| item != permission)
        .collect();
    let next = repository::save_plugin_permissions(db, plugin_id, &granted, &[])?;
    append_audit(
        db,
        Some(plugin_id.to_string()),
        "plugin.permissions.revoked",
        "medium",
        "Plugin permission revoked",
        serde_json::json!({ "permission": permission }),
    )?;
    repository::get_plugin(db, plugin_id).or(Ok(next))
}

fn ensure_required_permissions_granted(detail: &PluginDetail) -> AppResult<()> {
    let missing: Vec<&str> = detail
        .manifest
        .permissions
        .iter()
        .map(String::as_str)
        .filter(|permission| {
            !detail
                .granted_permissions
                .iter()
                .any(|item| item == permission)
        })
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(AppError::new(
            "PLUGIN_PERMISSION_REQUIRED",
            format!(
                "missing required plugin permissions: {}",
                missing.join(", ")
            ),
        ))
    }
}

fn ensure_runtime_enabled(manifest: &PluginManifest) -> AppResult<()> {
    match &manifest.runtime {
        PluginRuntime::DeclarativeRules { .. } => Ok(()),
        PluginRuntime::Native { engine }
            if manifest.id == "official.privacy-filter" && engine == "privacyFilter" =>
        {
            Ok(())
        }
        PluginRuntime::Wasm { .. } => Err(AppError::new(
            "PLUGIN_RUNTIME_DISABLED",
            "wasm runtime execution is disabled by host policy",
        )),
        PluginRuntime::Native { .. } => Err(AppError::new(
            "PLUGIN_UNSUPPORTED_RUNTIME",
            "native runtime is reserved for official plugins",
        )),
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
    use std::path::Path;
    use std::sync::Arc;

    fn manifest() -> PluginManifest {
        serde_json::from_value(serde_json::json!({
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

    fn wasm_manifest(plugin_id: &str) -> PluginManifest {
        serde_json::from_value(serde_json::json!({
            "id": plugin_id,
            "name": "WASM Policy Plugin",
            "version": "1.0.0",
            "apiVersion": "1.0.0",
            "runtime": {
                "kind": "wasm",
                "abiVersion": "1.0.0",
                "memoryLimitBytes": 16777216
            },
            "entry": "plugin.wasm",
            "hooks": [
                {
                    "name": "gateway.request.afterBodyRead",
                    "priority": 100,
                    "failurePolicy": "fail-open"
                }
            ],
            "permissions": ["request.body.read"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            }
        }))
        .unwrap()
    }

    #[test]
    fn service_requires_permissions_before_enable_and_preserves_config_on_disable() {
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

        let err =
            enable_plugin(&db, "community.prompt-helper", env!("CARGO_PKG_VERSION")).unwrap_err();
        assert!(err.to_string().starts_with("PLUGIN_PERMISSION_REQUIRED:"));

        save_plugin_config(
            &db,
            "community.prompt-helper",
            serde_json::json!({"mode": "append_instruction"}),
        )
        .unwrap();
        grant_plugin_permissions(
            &db,
            "community.prompt-helper",
            vec![
                "request.body.read".to_string(),
                "request.body.write".to_string(),
            ],
        )
        .unwrap();
        let enabled =
            enable_plugin(&db, "community.prompt-helper", env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(enabled.summary.status, PluginStatus::Enabled);

        let disabled = disable_plugin(&db, "community.prompt-helper").unwrap();
        assert_eq!(disabled.summary.status, PluginStatus::Disabled);
        assert_eq!(disabled.config["mode"], "append_instruction");
    }

    #[test]
    fn local_plugin_install_records_manifest_permissions_as_pending() {
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
        assert_eq!(
            installed.pending_permissions,
            vec!["request.body.read", "request.body.write"]
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
        grant_plugin_permissions(
            &db,
            "community.prompt-helper",
            vec![
                "request.body.read".to_string(),
                "request.body.write".to_string(),
            ],
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
        grant_plugin_permissions(
            &db,
            "community.prompt-helper",
            vec![
                "request.body.read".to_string(),
                "request.body.write".to_string(),
            ],
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
    fn enable_plugin_rejects_wasm_when_host_policy_disables_execution() {
        let dir = tempfile::tempdir().expect("db dir");
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).expect("db");
        install_plugin_manifest(
            &db,
            wasm_manifest("acme.wasm-policy"),
            PluginInstallSource::Local,
            Some(dir.path().to_string_lossy().to_string()),
            env!("CARGO_PKG_VERSION"),
        )
        .expect("install");
        grant_plugin_permissions(
            &db,
            "acme.wasm-policy",
            vec!["request.body.read".to_string()],
        )
        .expect("grant");

        let err = enable_plugin(&db, "acme.wasm-policy", env!("CARGO_PKG_VERSION"))
            .expect_err("wasm should not enable without policy");

        assert_eq!(err.code(), "PLUGIN_RUNTIME_DISABLED");
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
    fn revoke_plugin_permission_removes_grant_and_records_audit() {
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
        grant_plugin_permissions(
            &db,
            "community.prompt-helper",
            vec![
                "request.body.read".to_string(),
                "request.body.write".to_string(),
            ],
        )
        .unwrap();

        let detail =
            revoke_plugin_permission(&db, "community.prompt-helper", "request.body.write").unwrap();

        assert_eq!(detail.granted_permissions, vec!["request.body.read"]);
        assert!(detail
            .audit_logs
            .iter()
            .any(|log| log.event_type == "plugin.permissions.revoked"));
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
        assert!(installed
            .granted_permissions
            .contains(&"log.redact".to_string()));
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
        assert_eq!(installed.summary.runtime, "native:privacyFilter");
        assert_eq!(
            installed.manifest.runtime,
            crate::plugins::PluginRuntime::Native {
                engine: "privacyFilter".to_string()
            }
        );
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
        assert!(installed
            .granted_permissions
            .contains(&"request.body.write".to_string()));
        assert!(installed
            .granted_permissions
            .contains(&"log.redact".to_string()));
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
                crate::app::plugins::runtime_executor::RuntimeGatewayPluginExecutor::default(),
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
                crate::app::plugins::runtime_executor::RuntimeGatewayPluginExecutor::default(),
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
            .hooks
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
            .hooks
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
            .hooks
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
                .hooks
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
                "kind": "declarativeRules",
                "rules": ["rules/main.json"]
            },
            "hooks": [
                {
                    "name": "gateway.request.afterBodyRead",
                    "priority": 10,
                    "failurePolicy": "fail-open"
                }
            ],
            "permissions": ["request.meta.read"],
            "hostCompatibility": {
                "app": ">=0.56.0 <1.0.0",
                "pluginApi": "^1.0.0",
                "platforms": ["macos", "windows", "linux"]
            }
        })
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
        zip.finish().expect("finish package");
    }

    fn invalid_checksum() -> String {
        "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string()
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
                allow_unsigned: false,
                developer_mode: false,
                ..LocalPackageInstallPolicy::default()
            },
        )
    }

    fn hex_to_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
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
    fn plugin_local_install_rejects_reserved_official_privacy_filter_native_package() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("fake-official-privacy-filter.aio-plugin");
        let mut manifest = local_package_manifest("official.privacy-filter", "1.0.0");
        manifest["runtime"] = serde_json::json!({
            "kind": "native",
            "engine": "privacyFilter"
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

        let err = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                allow_unsigned: true,
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap_err();

        assert!(err.to_string().starts_with("PLUGIN_RESERVED_OFFICIAL_ID:"));
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
                allow_unsigned: true,
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
    fn plugin_signature_verification_allows_high_risk_signed_local_install() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("signed-risky.aio-plugin");
        let mut manifest = local_package_manifest("local.signed-risky", "1.0.0");
        manifest["permissions"] = serde_json::json!(["request.body.read"]);
        write_local_package(&package_path, manifest);
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

        assert_eq!(detail.summary.plugin_id, "local.signed-risky");
        let install_audit = detail
            .audit_logs
            .iter()
            .find(|log| log.event_type == "plugin.installed")
            .unwrap();
        assert_eq!(install_audit.details["unsigned"], false);
    }

    #[test]
    fn plugin_local_package_install_records_manifest_permissions_as_pending() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("signed-pending-permissions.aio-plugin");
        let mut manifest = local_package_manifest("local.pending-permissions", "1.0.0");
        manifest["permissions"] = serde_json::json!(["request.body.read", "request.body.write"]);
        write_local_package(&package_path, manifest);
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
        assert_eq!(
            detail.pending_permissions,
            vec!["request.body.read", "request.body.write"]
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
                allow_unsigned: true,
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
        let package_path = dir.path().join("market-signed-risky.aio-plugin");
        let mut manifest = local_package_manifest("market.signed-risky", "1.0.0");
        manifest["permissions"] = serde_json::json!(["request.body.read"]);
        write_local_package(&package_path, manifest);
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
                expected_plugin_id: "market.signed-risky".to_string(),
                expected_checksum,
                signature: trusted_policy.signature,
                public_key: Some(caller_public_key),
            },
        )
        .unwrap();

        assert_eq!(detail.summary.plugin_id, "market.signed-risky");
        assert_eq!(detail.granted_permissions, Vec::<String>::new());
        assert_eq!(detail.pending_permissions, vec!["request.body.read"]);
        let install_audit = detail
            .audit_logs
            .iter()
            .find(|log| log.event_type == "plugin.installed")
            .unwrap();
        assert_eq!(install_audit.details["unsigned"], false);
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
            },
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_MARKET_TRUSTED_PUBLIC_KEY_REQUIRED:"));
        assert!(repository::get_plugin(&db, "market.untrusted").is_err());
    }

    #[test]
    fn plugin_unsigned_offline_install_rejects_high_risk_permissions_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&dir.path().join("plugins.db")).unwrap();
        let package_path = dir.path().join("unsigned-risky.aio-plugin");
        let mut manifest = local_package_manifest("local.risky", "1.0.0");
        manifest["permissions"] = serde_json::json!(["request.header.readSensitive"]);
        write_local_package(&package_path, manifest);

        let err = install_plugin_from_local_package_with_policy(
            &db,
            &package_path,
            &dir.path().join("plugins/cache"),
            &dir.path().join("plugins/installed"),
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                allow_unsigned: false,
                developer_mode: false,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .starts_with("PLUGIN_UNSIGNED_HIGH_RISK_PERMISSION:"));
        assert!(repository::get_plugin(&db, "local.risky").is_err());
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
                allow_unsigned: true,
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
    fn plugin_update_rollback_marks_new_permissions_pending_and_keeps_existing_config() {
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
                allow_unsigned: true,
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();
        save_plugin_config(&db, "local.updatable", serde_json::json!({"enabled": true})).unwrap();
        grant_plugin_permissions(
            &db,
            "local.updatable",
            vec!["request.meta.read".to_string()],
        )
        .unwrap();

        let v2_package = dir.path().join("plugin-v2.aio-plugin");
        let mut v2_manifest = local_package_manifest("local.updatable", "1.1.0");
        v2_manifest["permissions"] =
            serde_json::json!(["request.meta.read", "request.header.read"]);
        write_local_package(&v2_package, v2_manifest);

        let updated = update_plugin_from_local_package(
            &db,
            &v2_package,
            &cache_dir,
            &installed_dir,
            env!("CARGO_PKG_VERSION"),
            LocalPackageInstallPolicy {
                allow_unsigned: true,
                developer_mode: true,
                ..LocalPackageInstallPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(updated.summary.current_version.as_deref(), Some("1.1.0"));
        assert_eq!(updated.config["enabled"], true);
        assert_eq!(updated.granted_permissions, vec!["request.meta.read"]);
        assert_eq!(updated.pending_permissions, vec!["request.header.read"]);
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
                allow_unsigned: true,
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
                allow_unsigned: true,
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
                allow_unsigned: true,
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
                allow_unsigned: false,
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
                allow_unsigned: true,
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
                allow_unsigned: true,
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
}
