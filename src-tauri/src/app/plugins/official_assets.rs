//! Usage: Materialize packaged official plugin assets into the install root.

use crate::shared::error::AppResult;
use std::path::{Path, PathBuf};

pub(crate) fn materialize_official_plugin(
    plugin_id: &str,
    source_root: &Path,
    installed_root: &Path,
    version: &str,
) -> AppResult<PathBuf> {
    let plugin_segment = crate::app_paths::plugin_id_path_segment(plugin_id)?;
    let version_segment = crate::app_paths::plugin_id_path_segment(version)?;
    let target = installed_root.join(plugin_segment).join(version_segment);
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|e| {
            format!(
                "failed to clear official plugin install dir {}: {e}",
                target.display()
            )
        })?;
    }
    copy_dir_recursive(source_root, &target)?;
    Ok(target)
}

fn copy_dir_recursive(source: &Path, target: &Path) -> AppResult<()> {
    std::fs::create_dir_all(target).map_err(|e| {
        format!(
            "failed to create official plugin dir {}: {e}",
            target.display()
        )
    })?;
    for entry in std::fs::read_dir(source).map_err(|e| {
        format!(
            "failed to read official plugin source {}: {e}",
            source.display()
        )
    })? {
        let entry = entry.map_err(|e| format!("failed to read official plugin entry: {e}"))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to read official plugin entry type: {e}"))?;
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &destination).map_err(|e| {
                format!(
                    "failed to copy official plugin resource to {}: {e}",
                    destination.display()
                )
            })?;
        }
    }
    Ok(())
}
