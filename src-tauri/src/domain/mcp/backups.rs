//! Usage: CLI config snapshots for MCP sync rollback (best-effort restore on failure).

use crate::mcp_sync;

use super::cli_specs::MCP_CLI_KEYS;

#[derive(Debug)]
pub(super) struct SingleCliBackup {
    target: Option<Vec<u8>>,
    manifest: Option<Vec<u8>>,
}

impl SingleCliBackup {
    pub(super) fn capture<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        cli_key: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            target: mcp_sync::read_target_bytes(app, cli_key)?,
            manifest: mcp_sync::read_manifest_bytes(app, cli_key)?,
        })
    }

    pub(super) fn restore<R: tauri::Runtime>(self, app: &tauri::AppHandle<R>, cli_key: &str) {
        let _ = mcp_sync::restore_target_bytes(app, cli_key, self.target);
        let _ = mcp_sync::restore_manifest_bytes(app, cli_key, self.manifest);
    }
}

#[derive(Debug)]
pub(super) struct CliBackupSnapshots(Vec<(&'static str, SingleCliBackup)>);

impl CliBackupSnapshots {
    pub(super) fn capture_all<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
    ) -> Result<Self, String> {
        let mut out = Vec::with_capacity(MCP_CLI_KEYS.len());
        for cli_key in MCP_CLI_KEYS.iter().copied() {
            out.push((cli_key, SingleCliBackup::capture(app, cli_key)?));
        }
        Ok(Self(out))
    }

    pub(super) fn restore_all<R: tauri::Runtime>(self, app: &tauri::AppHandle<R>) {
        for (cli_key, backup) in self.0 {
            backup.restore(app, cli_key);
        }
    }
}
