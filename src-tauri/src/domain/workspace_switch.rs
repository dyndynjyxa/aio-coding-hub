//! Usage: Workspace (profile) preview/apply orchestration.

use crate::claude_plugins;
use crate::db;
use crate::mcp_sync;
use crate::prompt_sync;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use crate::{mcp, prompts, skills, workspaces};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceEnabledPromptPreview {
    pub name: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspacePromptsPreview {
    pub from_enabled: Option<WorkspaceEnabledPromptPreview>,
    pub to_enabled: Option<WorkspaceEnabledPromptPreview>,
    pub will_change: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceItemsPreview {
    pub from_enabled: Vec<String>,
    pub to_enabled: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspacePreview {
    pub cli_key: String,
    pub from_workspace_id: Option<i64>,
    pub to_workspace_id: i64,
    pub prompts: WorkspacePromptsPreview,
    pub mcp: WorkspaceItemsPreview,
    pub skills: WorkspaceItemsPreview,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceApplyReport {
    pub cli_key: String,
    pub from_workspace_id: Option<i64>,
    pub to_workspace_id: i64,
    pub applied_at: i64,
}

fn excerpt(content: &str) -> String {
    const MAX_CHARS: usize = 160;
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut cutoff = normalized.len();
    for (idx, (byte_idx, _)) in normalized.char_indices().enumerate() {
        if idx == MAX_CHARS {
            cutoff = byte_idx;
            break;
        }
    }
    if cutoff == normalized.len() {
        return normalized;
    }
    format!("{}…", &normalized[..cutoff])
}

fn enabled_prompt_raw(
    conn: &Connection,
    workspace_id: i64,
) -> Result<Option<(String, String)>, String> {
    conn.query_row(
        r#"
SELECT name, content
FROM prompts
WHERE workspace_id = ?1 AND enabled = 1
ORDER BY updated_at DESC, id DESC
LIMIT 1
"#,
        params![workspace_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query enabled prompt: {e}"))
}

fn enabled_prompt_preview(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Option<WorkspaceEnabledPromptPreview>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(None);
    };
    let Some((name, content)) = enabled_prompt_raw(conn, workspace_id)? else {
        return Ok(None);
    };
    Ok(Some(WorkspaceEnabledPromptPreview {
        name,
        excerpt: excerpt(&content),
    }))
}

fn list_enabled_mcp_keys(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Vec<String>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT s.server_key
    FROM mcp_servers s
    JOIN workspace_mcp_enabled e
      ON e.server_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.server_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled mcp query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to query enabled mcp servers: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read enabled mcp row: {e}"))?);
    }
    Ok(out)
}

fn list_enabled_skill_keys(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Vec<String>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT s.skill_key
    FROM skills s
    JOIN workspace_skill_enabled e
      ON e.skill_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.skill_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled skills query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to query enabled skills: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read enabled skill row: {e}"))?);
    }
    Ok(out)
}

fn diff(from_enabled: &[String], to_enabled: &[String]) -> (Vec<String>, Vec<String>) {
    let from_set: HashSet<&str> = from_enabled.iter().map(String::as_str).collect();
    let to_set: HashSet<&str> = to_enabled.iter().map(String::as_str).collect();

    let mut added: Vec<String> = to_set
        .difference(&from_set)
        .map(|v| v.to_string())
        .collect();
    let mut removed: Vec<String> = from_set
        .difference(&to_set)
        .map(|v| v.to_string())
        .collect();

    added.sort();
    removed.sort();
    (added, removed)
}

pub fn preview(
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<WorkspacePreview> {
    let conn = db.open_connection()?;

    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    let from_workspace_id = workspaces::active_id_by_cli(&conn, &cli_key)?;

    let from_enabled_prompt = enabled_prompt_preview(&conn, from_workspace_id)?;
    let to_enabled_prompt = enabled_prompt_preview(&conn, Some(workspace_id))?;

    let will_change = match (from_workspace_id, Some(workspace_id)) {
        (None, _) => to_enabled_prompt.is_some(),
        (Some(from_id), Some(to_id)) => {
            let from_raw = enabled_prompt_raw(&conn, from_id)?;
            let to_raw = enabled_prompt_raw(&conn, to_id)?;
            from_raw.map(|v| v.1).unwrap_or_default() != to_raw.map(|v| v.1).unwrap_or_default()
        }
        _ => false,
    };

    let from_mcp = list_enabled_mcp_keys(&conn, from_workspace_id)?;
    let to_mcp = list_enabled_mcp_keys(&conn, Some(workspace_id))?;
    let (mcp_added, mcp_removed) = diff(&from_mcp, &to_mcp);

    let from_skills = list_enabled_skill_keys(&conn, from_workspace_id)?;
    let to_skills = list_enabled_skill_keys(&conn, Some(workspace_id))?;
    let (skills_added, skills_removed) = diff(&from_skills, &to_skills);

    Ok(WorkspacePreview {
        cli_key,
        from_workspace_id,
        to_workspace_id: workspace_id,
        prompts: WorkspacePromptsPreview {
            from_enabled: from_enabled_prompt,
            to_enabled: to_enabled_prompt,
            will_change,
        },
        mcp: WorkspaceItemsPreview {
            from_enabled: from_mcp,
            to_enabled: to_mcp,
            added: mcp_added,
            removed: mcp_removed,
        },
        skills: WorkspaceItemsPreview {
            from_enabled: from_skills,
            to_enabled: to_skills,
            added: skills_added,
            removed: skills_removed,
        },
    })
}

pub fn apply(
    app: &tauri::AppHandle,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<WorkspaceApplyReport> {
    let conn = db.open_connection()?;

    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    let from_workspace_id = workspaces::active_id_by_cli(&conn, &cli_key)?;

    if from_workspace_id == Some(workspace_id) {
        return Ok(WorkspaceApplyReport {
            cli_key,
            from_workspace_id,
            to_workspace_id: workspace_id,
            applied_at: now_unix_seconds(),
        });
    }

    let prev_prompt_target = prompt_sync::read_target_bytes(app, &cli_key)?;
    let prev_prompt_manifest = prompt_sync::read_manifest_bytes(app, &cli_key)?;
    let prev_mcp_target = mcp_sync::read_target_bytes(app, &cli_key)?;
    let prev_mcp_manifest = mcp_sync::read_manifest_bytes(app, &cli_key)?;
    let managed_mcp_server_keys: HashSet<String> = list_enabled_mcp_keys(&conn, from_workspace_id)?
        .into_iter()
        .collect();

    if let Err(err) = prompts::sync_cli_for_workspace(app, &conn, workspace_id) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        return Err(err);
    }

    if let Err(err) = mcp::swap_local_mcp_servers_for_workspace_switch(
        app,
        &cli_key,
        &managed_mcp_server_keys,
        from_workspace_id,
        workspace_id,
    ) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
        let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);
        return Err(err);
    }

    if let Err(err) = mcp::sync_cli_for_workspace(app, &conn, workspace_id) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
        let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);
        return Err(err);
    }

    let mut local_plugins_swap = if cli_key == "claude" {
        match claude_plugins::swap_local_plugins_for_workspace_switch(
            app,
            &cli_key,
            from_workspace_id,
            workspace_id,
        ) {
            Ok(swap) => Some(swap),
            Err(err) => {
                let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
                let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
                let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
                let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);
                return Err(err);
            }
        }
    } else {
        None
    };

    if let Err(err) = skills::sync_cli_for_workspace(app, &conn, workspace_id) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
        let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);

        if let Some(swap) = local_plugins_swap.take() {
            swap.rollback();
        }

        if let Some(from_id) = from_workspace_id {
            let _ = skills::sync_cli_for_workspace(app, &conn, from_id);
        }

        return Err(err);
    }

    let local_skills_swap = match skills::swap_local_skills_for_workspace_switch(
        app,
        &cli_key,
        from_workspace_id,
        workspace_id,
    ) {
        Ok(swap) => swap,
        Err(err) => {
            let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
            let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
            let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
            let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);

            if let Some(swap) = local_plugins_swap.take() {
                swap.rollback();
            }

            if let Some(from_id) = from_workspace_id {
                let _ = skills::sync_cli_for_workspace(app, &conn, from_id);
            }

            return Err(err);
        }
    };

    if let Err(err) = workspaces::set_active(db, workspace_id) {
        let _ = prompt_sync::restore_target_bytes(app, &cli_key, prev_prompt_target);
        let _ = prompt_sync::restore_manifest_bytes(app, &cli_key, prev_prompt_manifest);
        let _ = mcp_sync::restore_target_bytes(app, &cli_key, prev_mcp_target);
        let _ = mcp_sync::restore_manifest_bytes(app, &cli_key, prev_mcp_manifest);

        local_skills_swap.rollback();

        if let Some(swap) = local_plugins_swap.take() {
            swap.rollback();
        }

        if let Some(from_id) = from_workspace_id {
            let _ = skills::sync_cli_for_workspace(app, &conn, from_id);
        }

        return Err(err);
    }

    Ok(WorkspaceApplyReport {
        cli_key,
        from_workspace_id,
        to_workspace_id: workspace_id,
        applied_at: now_unix_seconds(),
    })
}
