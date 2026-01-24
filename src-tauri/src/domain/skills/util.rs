use std::path::{Component, Path};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn now_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

pub(super) fn enabled_to_int(enabled: bool) -> i64 {
    if enabled {
        1
    } else {
        0
    }
}

pub(super) fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

pub(super) fn validate_relative_subdir(subdir: &str) -> Result<(), String> {
    let subdir = subdir.trim();
    if subdir.is_empty() {
        return Err("SEC_INVALID_INPUT: source_subdir is required".to_string());
    }

    let p = Path::new(subdir);
    if p.is_absolute() {
        return Err("SEC_INVALID_INPUT: source_subdir must be relative".to_string());
    }

    for comp in p.components() {
        match comp {
            Component::CurDir | Component::Normal(_) => {}
            Component::ParentDir => {
                return Err("SEC_INVALID_INPUT: source_subdir must not contain '..'".to_string())
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("SEC_INVALID_INPUT: source_subdir must be relative".to_string())
            }
        }
    }

    Ok(())
}

pub(super) fn validate_dir_name(dir_name: &str) -> Result<String, String> {
    let dir_name = dir_name.trim();
    if dir_name.is_empty() {
        return Err("SEC_INVALID_INPUT: dir_name is required".to_string());
    }

    let p = Path::new(dir_name);
    let mut count = 0;
    for comp in p.components() {
        count += 1;
        match comp {
            Component::Normal(_) => {}
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(
                    "SEC_INVALID_INPUT: dir_name must be a single directory name".to_string(),
                );
            }
        }
    }

    if count != 1 {
        return Err("SEC_INVALID_INPUT: dir_name must be a single directory name".to_string());
    }

    Ok(dir_name.to_string())
}
