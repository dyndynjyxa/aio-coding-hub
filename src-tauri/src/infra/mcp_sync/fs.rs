//! Usage: Filesystem helpers for MCP sync operations.

use std::path::Path;

use super::paths;

pub fn read_target_bytes(app: &tauri::AppHandle, cli_key: &str) -> Result<Option<Vec<u8>>, String> {
    let path = paths::mcp_target_path(app, cli_key)?;
    read_optional_file(&path)
}

pub fn restore_target_bytes(
    app: &tauri::AppHandle,
    cli_key: &str,
    bytes: Option<Vec<u8>>,
) -> Result<(), String> {
    let path = paths::mcp_target_path(app, cli_key)?;
    match bytes {
        Some(content) => write_file_atomic(&path, &content),
        None => {
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
            }
            Ok(())
        }
    }
}

pub(super) fn copy_dir_recursive_if_missing(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("failed to create {}: {e}", dst.display()))?;

    let entries =
        std::fs::read_dir(src).map_err(|e| format!("failed to read dir {}: {e}", src.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", src.display()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive_if_missing(&path, &dst_path)?;
            continue;
        }

        if dst_path.exists() {
            continue;
        }

        std::fs::copy(&path, &dst_path).map_err(|e| {
            format!(
                "failed to copy {} -> {}: {e}",
                path.display(),
                dst_path.display()
            )
        })?;
    }

    Ok(())
}

pub(super) fn copy_file_if_missing(src: &Path, dst: &Path) -> Result<bool, String> {
    if dst.exists() {
        return Ok(false);
    }

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    std::fs::copy(src, dst)
        .map_err(|e| format!("failed to copy {} -> {}: {e}", src.display(), dst.display()))?;
    Ok(true)
}

pub(super) fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>, String> {
    if !path.exists() {
        return Ok(None);
    }
    std::fs::read(path)
        .map(Some)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))
}

pub(super) fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create dir {}: {e}", parent.display()))?;
    }

    let file_name = path.file_name().and_then(|v| v.to_str()).unwrap_or("file");
    let tmp_path = path.with_file_name(format!("{file_name}.aio-tmp"));

    std::fs::write(&tmp_path, bytes)
        .map_err(|e| format!("failed to write temp file {}: {e}", tmp_path.display()))?;

    // Windows rename requires target not to exist.
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }

    std::fs::rename(&tmp_path, path)
        .map_err(|e| format!("failed to finalize file {}: {e}", path.display()))?;

    Ok(())
}

pub(super) fn write_file_atomic_if_changed(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    if let Ok(existing) = std::fs::read(path) {
        if existing == bytes {
            return Ok(false);
        }
    }
    write_file_atomic(path, bytes)?;
    Ok(true)
}
