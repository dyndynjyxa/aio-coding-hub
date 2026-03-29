//! Import rollback infrastructure: backup/restore CLI runtime, skill FS guard, settings recovery.

use crate::shared::error::AppResult;
use crate::{db, settings};
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;

use super::skill_fs::{
    build_local_skill_source_metadata, cli_skills_root, local_skill_dirs, remove_dir_if_exists,
    ssot_skills_root, validate_local_dir_name, write_skill_files_to_dir,
};
use super::{InstalledSkillExport, LocalSkillExport};

#[derive(Debug, Clone)]
pub(super) struct CliRuntimeBackup {
    pub(super) cli_key: &'static str,
    pub(super) prompt_target: Option<Vec<u8>>,
    pub(super) prompt_manifest: Option<Vec<u8>>,
    pub(super) mcp_target: Option<Vec<u8>>,
    pub(super) mcp_manifest: Option<Vec<u8>>,
}

#[derive(Debug)]
struct LocalSkillBackup {
    original_path: PathBuf,
    backup_path: PathBuf,
}

#[derive(Debug, Default)]
pub(super) struct SkillFsImportGuard {
    ssot_root: Option<PathBuf>,
    ssot_backup_dir: Option<PathBuf>,
    local_backup_roots: Vec<PathBuf>,
    local_backups: Vec<LocalSkillBackup>,
    imported_local_dirs: Vec<PathBuf>,
}

impl SkillFsImportGuard {
    pub(super) fn rollback(&mut self) {
        for path in self.imported_local_dirs.iter().rev() {
            let _ = remove_dir_if_exists(path);
        }

        for backup in self.local_backups.iter().rev() {
            let _ = remove_dir_if_exists(&backup.original_path);
            let _ = std::fs::rename(&backup.backup_path, &backup.original_path);
        }

        if let Some(ssot_root) = &self.ssot_root {
            let _ = remove_dir_if_exists(ssot_root);
            if let Some(backup_dir) = &self.ssot_backup_dir {
                let _ = std::fs::rename(backup_dir, ssot_root);
            }
        }

        for backup_root in self.local_backup_roots.iter().rev() {
            let _ = remove_dir_if_exists(backup_root);
        }
    }

    pub(super) fn finish(self) {
        if let Some(backup_dir) = self.ssot_backup_dir {
            let _ = remove_dir_if_exists(&backup_dir);
        }
        for backup_root in self.local_backup_roots {
            let _ = remove_dir_if_exists(&backup_root);
        }
    }
}

pub(super) fn capture_cli_runtime_backups<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<Vec<CliRuntimeBackup>> {
    let mut backups = Vec::new();
    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        backups.push(CliRuntimeBackup {
            cli_key,
            prompt_target: crate::prompt_sync::read_target_bytes(app, cli_key)?,
            prompt_manifest: crate::prompt_sync::read_manifest_bytes(app, cli_key)?,
            mcp_target: crate::mcp_sync::read_target_bytes(app, cli_key)?,
            mcp_manifest: crate::mcp_sync::read_manifest_bytes(app, cli_key)?,
        });
    }
    Ok(backups)
}

fn restore_cli_runtime_backups<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    backups: Vec<CliRuntimeBackup>,
) {
    for backup in backups {
        if let Err(err) =
            crate::prompt_sync::restore_target_bytes(app, backup.cli_key, backup.prompt_target)
        {
            tracing::warn!(cli_key = backup.cli_key, error = %err, "config import rollback: failed to restore prompt target");
        }
        if let Err(err) =
            crate::prompt_sync::restore_manifest_bytes(app, backup.cli_key, backup.prompt_manifest)
        {
            tracing::warn!(cli_key = backup.cli_key, error = %err, "config import rollback: failed to restore prompt manifest");
        }
        if let Err(err) =
            crate::mcp_sync::restore_target_bytes(app, backup.cli_key, backup.mcp_target)
        {
            tracing::warn!(cli_key = backup.cli_key, error = %err, "config import rollback: failed to restore mcp target");
        }
        if let Err(err) =
            crate::mcp_sync::restore_manifest_bytes(app, backup.cli_key, backup.mcp_manifest)
        {
            tracing::warn!(cli_key = backup.cli_key, error = %err, "config import rollback: failed to restore mcp manifest");
        }
    }
}

pub(super) fn sync_all_cli_runtime<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
) -> AppResult<()> {
    for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
        crate::prompts::sync_one_cli(app, conn, cli_key)?;
        crate::mcp::sync_one_cli(app, conn, cli_key)?;
        crate::skills::sync_one_cli(app, conn, cli_key)?;
    }
    Ok(())
}

fn restore_settings_after_failed_import<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    previous_settings: &settings::AppSettings,
) {
    if let Err(err) = settings::write(app, previous_settings) {
        tracing::warn!(error = %err, "config import rollback: failed to restore settings");
    }
}

pub(super) fn rollback_after_failed_import<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    previous_settings: &settings::AppSettings,
    runtime_backups: Vec<CliRuntimeBackup>,
    skill_fs_guard: Option<&mut SkillFsImportGuard>,
) {
    restore_settings_after_failed_import(app, previous_settings);
    crate::app::autostart::restore_auto_start_best_effort(app, previous_settings.auto_start);

    if let Some(guard) = skill_fs_guard {
        guard.rollback();
    }

    match db.open_connection() {
        Ok(conn) => {
            if let Err(err) = sync_all_cli_runtime(app, &conn) {
                tracing::warn!(error = %err, "config import rollback: failed to resync runtime from restored db state");
                restore_cli_runtime_backups(app, runtime_backups);
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "config import rollback: failed to reopen database");
            restore_cli_runtime_backups(app, runtime_backups);
        }
    }
}

pub(super) fn apply_skill_fs_import<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    installed_skills: &[InstalledSkillExport],
    local_skills: &[LocalSkillExport],
) -> AppResult<SkillFsImportGuard> {
    let app_data_dir = crate::app_paths::app_data_dir(app)?;
    let import_id = crate::shared::time::now_unix_seconds();
    let ssot_root = ssot_skills_root(app)?;
    let ssot_stage_dir = app_data_dir.join(format!("config-import-skills-stage-{import_id}"));
    let ssot_backup_dir = app_data_dir.join(format!("config-import-skills-backup-{import_id}"));
    let mut guard = SkillFsImportGuard {
        ssot_root: Some(ssot_root.clone()),
        ..SkillFsImportGuard::default()
    };

    let apply_result = (|| -> AppResult<()> {
        remove_dir_if_exists(&ssot_stage_dir)?;
        remove_dir_if_exists(&ssot_backup_dir)?;
        std::fs::create_dir_all(&ssot_stage_dir)
            .map_err(|e| format!("failed to create {}: {e}", ssot_stage_dir.display()))?;

        let mut seen_skill_keys = HashSet::new();
        for skill in installed_skills {
            let skill_key = skill.skill_key.trim();
            if skill_key.is_empty() {
                return Err("SEC_INVALID_INPUT: installed skill_key is required"
                    .to_string()
                    .into());
            }
            if !seen_skill_keys.insert(skill_key.to_string()) {
                return Err(format!(
                    "SEC_INVALID_INPUT: duplicate installed skill_key={skill_key}"
                )
                .into());
            }

            let skill_dir = ssot_stage_dir.join(skill_key);
            write_skill_files_to_dir(&skill_dir, &skill.files, None)?;
            if !skill_dir.join("SKILL.md").exists() {
                return Err(format!(
                    "SEC_INVALID_INPUT: installed skill missing SKILL.md: {skill_key}"
                )
                .into());
            }
        }

        if ssot_root.exists() {
            std::fs::rename(&ssot_root, &ssot_backup_dir).map_err(|e| {
                format!(
                    "failed to backup installed skills dir {} -> {}: {e}",
                    ssot_root.display(),
                    ssot_backup_dir.display()
                )
            })?;
            guard.ssot_backup_dir = Some(ssot_backup_dir.clone());
        }

        std::fs::rename(&ssot_stage_dir, &ssot_root).map_err(|e| {
            format!(
                "failed to activate installed skills dir {} -> {}: {e}",
                ssot_stage_dir.display(),
                ssot_root.display()
            )
        })?;

        for cli_key in crate::shared::cli_key::SUPPORTED_CLI_KEYS {
            let root = cli_skills_root(app, cli_key)?;
            std::fs::create_dir_all(&root)
                .map_err(|e| format!("failed to create {}: {e}", root.display()))?;

            let existing_local_dirs = local_skill_dirs(&root)?;
            let backup_root =
                app_data_dir.join(format!("config-import-local-backup-{cli_key}-{import_id}"));
            if !existing_local_dirs.is_empty() {
                remove_dir_if_exists(&backup_root)?;
                std::fs::create_dir_all(&backup_root)
                    .map_err(|e| format!("failed to create {}: {e}", backup_root.display()))?;
                guard.local_backup_roots.push(backup_root.clone());
            }

            for dir in existing_local_dirs {
                let dir_name = dir
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| {
                        format!(
                            "SKILL_IMPORT_INVALID_LOCAL_DIR_NAME: local skill dir name invalid: {}",
                            dir.display()
                        )
                    })?;
                let backup_path = backup_root.join(dir_name);
                std::fs::rename(&dir, &backup_path).map_err(|e| {
                    format!(
                        "failed to backup local skill dir {} -> {}: {e}",
                        dir.display(),
                        backup_path.display()
                    )
                })?;
                guard.local_backups.push(LocalSkillBackup {
                    original_path: dir,
                    backup_path,
                });
            }

            let mut seen_dir_names = HashSet::new();
            for local_skill in local_skills.iter().filter(|value| value.cli_key == cli_key) {
                let dir_name = validate_local_dir_name(&local_skill.dir_name)?;
                if !seen_dir_names.insert(dir_name.clone()) {
                    return Err(format!(
                        "SEC_INVALID_INPUT: duplicate local skill dir_name for cli_key={cli_key}: {dir_name}"
                    )
                    .into());
                }

                let target_dir = root.join(&dir_name);
                if target_dir.exists() {
                    return Err(format!(
                        "SKILL_IMPORT_LOCAL_CONFLICT: target local skill dir already exists: {}",
                        target_dir.display()
                    )
                    .into());
                }

                let source_metadata = build_local_skill_source_metadata(local_skill)?;
                write_skill_files_to_dir(
                    &target_dir,
                    &local_skill.files,
                    source_metadata.as_ref(),
                )?;
                if !target_dir.join("SKILL.md").exists() {
                    return Err(format!(
                        "SEC_INVALID_INPUT: local skill missing SKILL.md: cli_key={cli_key}, dir_name={dir_name}"
                    )
                    .into());
                }
                guard.imported_local_dirs.push(target_dir);
            }
        }

        Ok(())
    })();

    if let Err(err) = apply_result {
        let _ = remove_dir_if_exists(&ssot_stage_dir);
        guard.rollback();
        return Err(err);
    }

    Ok(guard)
}
