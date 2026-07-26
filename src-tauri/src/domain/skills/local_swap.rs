use crate::app_paths;
use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

use super::fs_ops::{has_skill_md, is_managed_link_to_ssot};
use super::local::managed_marker_belongs_to_installed_skill;
use super::paths::{cli_skills_root, ssot_skills_root};

fn stash_bucket_name(workspace_id: Option<i64>) -> String {
    workspace_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unassigned".to_string())
}

fn stash_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?
        .join("skills-local")
        .join(cli_key))
}

fn is_local_skill_dir(
    conn: &Connection,
    path: &Path,
    ssot_root: &Path,
) -> crate::shared::error::AppResult<bool> {
    if !path.is_dir() {
        return Ok(false);
    }
    if managed_marker_belongs_to_installed_skill(conn, path)?
        || is_managed_link_to_ssot(path, ssot_root)
    {
        return Ok(false);
    }
    Ok(has_skill_md(path))
}

fn rotate_existing_dir(dst: &Path) -> crate::shared::error::AppResult<()> {
    if !dst.exists() {
        return Ok(());
    }
    let Some(parent) = dst.parent() else {
        return Ok(());
    };
    let base = dst
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("skill")
        .to_string();

    let nonce = now_unix_seconds();
    let mut candidate = parent.join(format!(".{base}.old-{nonce}"));
    let mut idx = 2;
    while candidate.exists() && idx < 100 {
        candidate = parent.join(format!(".{base}.old-{nonce}-{idx}"));
        idx += 1;
    }

    std::fs::rename(dst, &candidate)
        .map_err(|e| format!("failed to rotate {}: {e}", dst.display()))?;
    Ok(())
}

fn move_dir(src: &Path, dst: &Path) -> crate::shared::error::AppResult<()> {
    let Some(parent) = dst.parent() else {
        return Err(format!("SEC_INVALID_INPUT: invalid dst path {}", dst.display()).into());
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;

    if dst.exists() {
        rotate_existing_dir(dst)?;
    }

    std::fs::rename(src, dst)
        .map_err(|e| format!("failed to move {} -> {}: {e}", src.display(), dst.display()).into())
}

#[derive(Debug)]
pub(crate) struct LocalSkillsSwap {
    cli_root: PathBuf,
    from_bucket: PathBuf,
    to_bucket: PathBuf,
    moved_from_cli: Vec<String>,
    moved_to_cli: Vec<String>,
}

impl LocalSkillsSwap {
    pub(crate) fn rollback(self) {
        // Best-effort: reverse order to avoid name collisions.
        for name in self.moved_to_cli.iter().rev() {
            let src = self.cli_root.join(name);
            let dst = self.to_bucket.join(name);
            if src.exists() {
                let _ = move_dir(&src, &dst);
            }
        }

        for name in self.moved_from_cli.iter().rev() {
            let src = self.from_bucket.join(name);
            let dst = self.cli_root.join(name);
            if src.exists() {
                let _ = move_dir(&src, &dst);
            }
        }
    }
}

pub(crate) fn swap_local_skills_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
    from_workspace_id: Option<i64>,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<LocalSkillsSwap> {
    let cli_root = cli_skills_root(app, cli_key)?;
    let ssot_root = ssot_skills_root(app)?;

    let stash_root = stash_root(app, cli_key)?;
    let from_bucket = stash_root.join(stash_bucket_name(from_workspace_id));
    let to_bucket = stash_root.join(to_workspace_id.to_string());

    std::fs::create_dir_all(&from_bucket)
        .map_err(|e| format!("failed to create {}: {e}", from_bucket.display()))?;
    std::fs::create_dir_all(&to_bucket)
        .map_err(|e| format!("failed to create {}: {e}", to_bucket.display()))?;

    let mut moved_from_cli = Vec::new();
    let mut moved_to_cli = Vec::new();

    if cli_root.exists() {
        let entries = std::fs::read_dir(&cli_root)
            .map_err(|e| format!("failed to read dir {}: {e}", cli_root.display()))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| format!("failed to read dir entry {}: {e}", cli_root.display()))?;
            let path = entry.path();
            if !is_local_skill_dir(conn, &path, &ssot_root)? {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_string();
            if dir_name.is_empty() {
                continue;
            }
            let dst = from_bucket.join(&dir_name);
            move_dir(&path, &dst)?;
            moved_from_cli.push(dir_name);
        }
    }

    if to_bucket.exists() {
        let entries = std::fs::read_dir(&to_bucket)
            .map_err(|e| format!("failed to read dir {}: {e}", to_bucket.display()))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| format!("failed to read dir entry {}: {e}", to_bucket.display()))?;
            let path = entry.path();
            if !is_local_skill_dir(conn, &path, &ssot_root)? {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_string();
            if dir_name.is_empty() {
                continue;
            }

            let dst = cli_root.join(&dir_name);
            if dst.exists() {
                tracing::warn!(
                    cli_key = %cli_key,
                    dir = %dir_name,
                    "本机 Skills 切换: 目标目录已存在，跳过恢复"
                );
                continue;
            }

            move_dir(&path, &dst)?;
            moved_to_cli.push(dir_name);
        }
    }

    Ok(LocalSkillsSwap {
        cli_root,
        from_bucket,
        to_bucket,
        moved_from_cli,
        moved_to_cli,
    })
}
