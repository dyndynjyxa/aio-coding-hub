//! Replace a user-owned file without exposing its contents through a shared or predictable temp file.
use crate::shared::error::AppResult;
use std::fs::{self, Metadata};
use std::io::{ErrorKind, Write};
use std::path::Path;

pub(crate) fn regular_file_metadata(path: &Path) -> AppResult<Option<Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => Ok(Some(meta)),
        Ok(_) => Err(format!(
            "SEC_INVALID_INPUT: expected a regular file at {}",
            path.display()
        )
        .into()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("读取 {} 的文件信息失败：{e}", path.display()).into()),
    }
}

pub(crate) fn write_private_file_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let meta = regular_file_metadata(path)?;
    let parent = path
        .parent()
        .ok_or("SEC_INVALID_INPUT: file has no parent directory")?;
    let result = (|| -> std::io::Result<()> {
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(bytes)?;
        if let Some(meta) = meta {
            file.as_file().set_permissions(meta.permissions())?;
            #[cfg(windows)]
            return replace_windows_file(path, file);
        }
        file.persist(path)?;
        Ok(())
    })();
    result.map_err(|e| format!("写入 {} 失败：{e}", path.display()).into())
}

// ReplaceFile preserves the original Windows ACL. Moving the temp file over the file would inherit the directory ACL.
#[cfg(windows)]
fn replace_windows_file(path: &Path, file: tempfile::NamedTempFile) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;
    let temp_path = file.into_temp_path();
    let wide = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let dest = wide(path);
    let source = wide(&temp_path);
    // Both UTF-16 strings are NUL-terminated and live through the call.
    let replaced = unsafe {
        ReplaceFileW(
            dest.as_ptr(),
            source.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
