//! Usage: CLI config snapshots for MCP sync rollback (best-effort restore on failure).

use crate::mcp_sync;

use super::cli_specs::MCP_CLI_KEYS;

#[derive(Debug, Clone)]
pub(crate) struct SingleCliBackup {
    target: Option<Vec<u8>>,
    legacy: Option<(std::path::PathBuf, Vec<u8>)>,
    manifest: Option<Vec<u8>>,
}

impl SingleCliBackup {
    pub(crate) fn capture<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        cli_key: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            legacy: if cli_key == "gemini" {
                mcp_sync::legacy_antigravity_snapshot(app)?
            } else {
                None
            },
            target: mcp_sync::read_target_bytes(app, cli_key)?,
            manifest: mcp_sync::read_manifest_bytes(app, cli_key)?,
        })
    }

    pub(crate) fn capture_for_shared<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        cli_key: &str,
    ) -> Result<Option<Self>, String> {
        match Self::capture(app, cli_key) {
            Ok(backup) => Ok(Some(backup)),
            Err(_) if cli_key == "gemini" => {
                warn_skipped(app);
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn sync_shared<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        conn: &rusqlite::Connection,
        cli_key: &str,
        backup: Option<&Self>,
    ) -> crate::shared::error::AppResult<()> {
        let Some(backup) = backup else { return Ok(()) };
        let servers = super::sync::list_enabled_for_cli(conn, cli_key)?;
        match mcp_sync::sync_cli(app, cli_key, &servers) {
            Err(_) if cli_key == "gemini" => {
                backup.clone().restore(app, cli_key);
                warn_skipped(app);
                Ok(())
            }
            result => Ok(result?),
        }
    }

    pub(crate) fn restore<R: tauri::Runtime>(self, app: &tauri::AppHandle<R>, cli_key: &str) {
        for result in [
            mcp_sync::restore_target_bytes(app, cli_key, self.target),
            self.legacy.map_or(Ok(()), |(path, bytes)| {
                crate::shared::fs::write_private_file_atomic(&path, &bytes).map_err(String::from)
            }),
            mcp_sync::restore_manifest_bytes(app, cli_key, self.manifest).map_err(String::from),
        ] {
            if let Err(error) = result {
                tracing::warn!(cli_key, %error, "failed to restore MCP sync snapshot");
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct CliBackupSnapshots(Vec<(&'static str, Option<SingleCliBackup>)>);

impl CliBackupSnapshots {
    pub(super) fn capture_all<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
    ) -> Result<Self, String> {
        let mut out = Vec::with_capacity(MCP_CLI_KEYS.len());
        for cli_key in MCP_CLI_KEYS.iter().copied() {
            out.push((cli_key, SingleCliBackup::capture_for_shared(app, cli_key)?));
        }
        Ok(Self(out))
    }

    pub(super) fn sync_all<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        conn: &rusqlite::Connection,
    ) -> crate::shared::error::AppResult<()> {
        for (key, backup) in &self.0 {
            SingleCliBackup::sync_shared(app, conn, key, backup.as_ref())?;
        }
        Ok(())
    }

    pub(super) fn restore_all<R: tauri::Runtime>(self, app: &tauri::AppHandle<R>) {
        for (cli_key, backup) in self.0 {
            if let Some(backup) = backup {
                backup.restore(app, cli_key);
            }
        }
    }
}

fn warn_skipped<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    mcp_sync::warn_antigravity_mcp(app, "Antigravity 的 MCP 配置未同步。请检查 ~/.gemini/config/mcp_config.json 是否为可写的有效 JSON，以及 AIO 的 MCP 同步清单能否读取，然后重新同步。".into());
}
