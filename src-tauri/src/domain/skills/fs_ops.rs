use std::path::Path;

const MANAGED_MARKER_FILE: &str = ".aio-coding-hub.managed";

pub(super) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
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
            copy_dir_recursive(&path, &dst_path)?;
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

pub(super) fn write_marker(dir: &Path) -> Result<(), String> {
    let path = dir.join(MANAGED_MARKER_FILE);
    std::fs::write(&path, "aio-coding-hub\n")
        .map_err(|e| format!("failed to write marker {}: {e}", path.display()))
}

pub(super) fn remove_marker(dir: &Path) {
    let path = dir.join(MANAGED_MARKER_FILE);
    let _ = std::fs::remove_file(path);
}

pub(super) fn is_managed_dir(dir: &Path) -> bool {
    dir.join(MANAGED_MARKER_FILE).exists()
}

pub(super) fn remove_managed_dir(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    if !is_managed_dir(dir) {
        return Err(format!(
            "SKILL_REMOVE_BLOCKED_UNMANAGED: target exists but is not managed: {}",
            dir.display()
        ));
    }
    std::fs::remove_dir_all(dir).map_err(|e| format!("failed to remove {}: {e}", dir.display()))?;
    Ok(())
}
