//! Usage: Small filesystem helpers shared across infra adapters (atomic writes, optional reads).

use std::path::Path;

const WRITE_IF_CHANGED_COMPARE_MAX_BYTES: usize = 16 * 1024 * 1024;

/// Check whether the given path is a symbolic link.
/// Returns an error when metadata cannot be read so callers can fail-closed.
pub(crate) fn is_symlink(path: &Path) -> crate::shared::error::AppResult<bool> {
    std::fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .map_err(|e| format!("failed to read metadata {}: {e}", path.display()).into())
}

pub(crate) fn copy_dir_recursive_if_missing(
    src: &Path,
    dst: &Path,
) -> crate::shared::error::AppResult<()> {
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

pub(crate) fn copy_file_if_missing(
    src: &Path,
    dst: &Path,
) -> crate::shared::error::AppResult<bool> {
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

pub(crate) fn read_optional_file_with_max_len(
    path: &Path,
    max_len: usize,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    if !path.exists() {
        return Ok(None);
    }

    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("failed to read metadata {}: {e}", path.display()))?;
    if metadata.len() > max_len as u64 {
        return Err(format!(
            "SEC_INVALID_INPUT: file {} too large (max {max_len} bytes)",
            path.display()
        )
        .into());
    }

    let bytes =
        std::fs::read(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    if bytes.len() > max_len {
        return Err(format!(
            "SEC_INVALID_INPUT: file {} too large (max {max_len} bytes)",
            path.display()
        )
        .into());
    }
    Ok(Some(bytes))
}

pub(crate) fn read_file_with_max_len(
    path: &Path,
    max_len: usize,
) -> crate::shared::error::AppResult<Vec<u8>> {
    read_optional_file_with_max_len(path, max_len)?.ok_or_else(|| {
        crate::shared::error::AppError::from(format!(
            "failed to read {}: not found",
            path.display()
        ))
    })
}

pub(crate) fn write_file_atomic(path: &Path, bytes: &[u8]) -> crate::shared::error::AppResult<()> {
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

pub(crate) fn write_file_atomic_if_changed(
    path: &Path,
    bytes: &[u8],
) -> crate::shared::error::AppResult<bool> {
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() == bytes.len() as u64 && bytes.len() <= WRITE_IF_CHANGED_COMPARE_MAX_BYTES
        {
            if let Ok(existing) = std::fs::read(path) {
                if existing == bytes {
                    return Ok(false);
                }
            }
        }
    }
    write_file_atomic(path, bytes)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TMP_DIR_SEQ: AtomicUsize = AtomicUsize::new(0);

    fn unique_tmp_dir() -> std::path::PathBuf {
        let seq = TMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "aio_coding_hub_fs_test_{nanos}_{}_{}",
            std::process::id(),
            seq
        ));
        std::fs::create_dir_all(&dir).expect("create tmp dir");
        dir
    }

    #[test]
    fn unique_tmp_dir_is_unique_across_calls() {
        let a = unique_tmp_dir();
        let b = unique_tmp_dir();
        assert_ne!(a, b);
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }

    #[test]
    fn read_optional_file_with_max_len_rejects_oversized_files() {
        let dir = unique_tmp_dir();
        let path = dir.join("large.txt");
        std::fs::write(&path, b"hello").expect("write large");

        let err = read_optional_file_with_max_len(&path, 4).expect_err("oversized file fails");

        assert!(err.to_string().contains("too large"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_file_with_max_len_missing_is_error() {
        let dir = unique_tmp_dir();
        let path = dir.join("missing.txt");

        let err = read_file_with_max_len(&path, 4).expect_err("missing file fails");

        assert!(err.to_string().contains("not found"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_file_atomic_creates_parent_and_writes_bytes() {
        let dir = unique_tmp_dir();
        let path = dir.join("a").join("b").join("file.txt");
        write_file_atomic(&path, b"hello").expect("write_file_atomic");
        let got = read_file_with_max_len(&path, 16).expect("read_file_with_max_len");
        assert_eq!(got, b"hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_file_atomic_if_changed_is_false_when_unchanged() {
        let dir = unique_tmp_dir();
        let path = dir.join("file.txt");
        assert!(write_file_atomic_if_changed(&path, b"v1").expect("write"));
        assert!(!write_file_atomic_if_changed(&path, b"v1").expect("write"));
        assert!(write_file_atomic_if_changed(&path, b"v2").expect("write"));
        let got = read_file_with_max_len(&path, 16).expect("read_file_with_max_len");
        assert_eq!(got, b"v2");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_file_atomic_if_changed_skips_compare_for_oversized_inputs() {
        let dir = unique_tmp_dir();
        let path = dir.join("large.txt");
        let bytes = vec![b'x'; WRITE_IF_CHANGED_COMPARE_MAX_BYTES + 1];
        write_file_atomic(&path, &bytes).expect("write initial large file");

        assert!(
            write_file_atomic_if_changed(&path, &bytes).expect("rewrite oversized file"),
            "oversized inputs should skip full-file equality compare"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_file_if_missing_copies_once() {
        let dir = unique_tmp_dir();
        let src = dir.join("src.txt");
        let dst = dir.join("nested").join("dst.txt");

        std::fs::write(&src, "content").expect("write src");
        assert!(copy_file_if_missing(&src, &dst).expect("copy"));
        assert_eq!(std::fs::read_to_string(&dst).expect("read dst"), "content");

        assert!(!copy_file_if_missing(&src, &dst).expect("copy"));
        assert_eq!(std::fs::read_to_string(&dst).expect("read dst"), "content");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_dir_recursive_if_missing_skips_existing_files() {
        let dir = unique_tmp_dir();
        let src_dir = dir.join("src");
        let dst_dir = dir.join("dst");

        std::fs::create_dir_all(src_dir.join("sub")).expect("create src dir");
        std::fs::write(src_dir.join("a.txt"), "src-a").expect("write");
        std::fs::write(src_dir.join("sub").join("b.txt"), "src-b").expect("write");

        std::fs::create_dir_all(&dst_dir).expect("create dst dir");
        std::fs::write(dst_dir.join("a.txt"), "dst-a").expect("write dst override");

        copy_dir_recursive_if_missing(&src_dir, &dst_dir).expect("copy dir");
        assert_eq!(
            std::fs::read_to_string(dst_dir.join("a.txt")).expect("read"),
            "dst-a"
        );
        assert_eq!(
            std::fs::read_to_string(dst_dir.join("sub").join("b.txt")).expect("read"),
            "src-b"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
