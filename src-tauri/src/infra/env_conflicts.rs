//! Usage: Detect environment variables that may override local CLI configuration.
//!
//! This module is intentionally read-only: it reports potential conflicts but does not attempt to
//! modify system settings or shell configuration files.

use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct EnvConflict {
    pub var_name: String,
    pub source_type: String, // "system" | "file"
    pub source_path: String, // "Process Environment" | "<path>:<line>"
}

fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn keywords_for_cli(cli_key: &str) -> Vec<&'static str> {
    match cli_key {
        "claude" => vec!["ANTHROPIC"],
        "codex" => vec!["OPENAI"],
        "gemini" => vec!["GEMINI", "GOOGLE_GEMINI"],
        _ => Vec::new(),
    }
}

fn variable_matches_cli(cli_key: &str, var_name_upper: &str) -> bool {
    if cli_key == "grok" {
        return matches!(
            var_name_upper,
            "GROK_MODELS_BASE_URL"
                | "XAI_API_KEY"
                | "GROK_CODE_XAI_API_KEY"
                | "GROK_WEB_SEARCH_MODEL"
        );
    }

    keywords_for_cli(cli_key)
        .iter()
        .any(|keyword| var_name_upper.contains(keyword))
}

fn conflict_dedupe_key(conflict: &EnvConflict) -> String {
    let var_name = conflict.var_name.trim().to_ascii_uppercase();
    let source_path = if conflict.source_type == "file" {
        strip_trailing_line_number(&conflict.source_path)
    } else {
        conflict.source_path.as_str()
    };
    format!("{}|{}|{}", var_name, conflict.source_type, source_path)
}

fn strip_trailing_line_number(source_path: &str) -> &str {
    let Some((path, suffix)) = source_path.rsplit_once(':') else {
        return source_path;
    };
    if suffix.chars().all(|ch| ch.is_ascii_digit()) {
        path
    } else {
        source_path
    }
}

#[cfg(not(target_os = "windows"))]
fn check_shell_configs<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    out: &mut Vec<EnvConflict>,
    seen: &mut HashSet<String>,
) -> crate::shared::error::AppResult<()> {
    use std::fs;
    use std::path::PathBuf;

    let home_dir = crate::shared::user_home::home_dir(app)?;

    let config_files: Vec<PathBuf> = vec![
        home_dir.join(".bashrc"),
        home_dir.join(".bash_profile"),
        home_dir.join(".bash_login"),
        home_dir.join(".zshrc"),
        home_dir.join(".zshenv"),
        home_dir.join(".zprofile"),
        home_dir.join(".profile"),
        PathBuf::from("/etc/profile"),
        PathBuf::from("/etc/bashrc"),
        PathBuf::from("/etc/bash.bashrc"),
        PathBuf::from("/etc/zshrc"),
        PathBuf::from("/etc/zshenv"),
        PathBuf::from("/etc/zprofile"),
        PathBuf::from("/etc/zsh/zshrc"),
        PathBuf::from("/etc/zsh/zshenv"),
        PathBuf::from("/etc/zsh/zprofile"),
    ];

    for file_path in config_files {
        let bytes = match fs::read(&file_path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let content = String::from_utf8_lossy(&bytes);

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let export_line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
            let Some(eq_pos) = export_line.find('=') else {
                continue;
            };

            let var_name = export_line[..eq_pos].trim();
            if var_name.is_empty() {
                continue;
            }

            let var_name_upper = var_name.to_ascii_uppercase();
            if !variable_matches_cli(cli_key, &var_name_upper) {
                continue;
            }

            let conflict = EnvConflict {
                var_name: var_name_upper,
                source_type: "file".to_string(),
                source_path: format!("{}:{}", file_path.display(), line_num + 1),
            };

            if seen.insert(conflict_dedupe_key(&conflict)) {
                out.push(conflict);
            }
        }
    }

    Ok(())
}

pub fn check_env_conflicts<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<EnvConflict>> {
    validate_cli_key(cli_key)?;

    let mut out = Vec::new();
    let mut seen = HashSet::<String>::new();

    for (key, _value) in std::env::vars_os() {
        let key = key.to_string_lossy().into_owned();
        let key_upper = key.to_ascii_uppercase();
        if !variable_matches_cli(cli_key, &key_upper) {
            continue;
        }

        let conflict = EnvConflict {
            var_name: key_upper,
            source_type: "system".to_string(),
            source_path: "Process Environment".to_string(),
        };
        if seen.insert(conflict_dedupe_key(&conflict)) {
            out.push(conflict);
        }
    }

    // `app` is only consumed by `check_shell_configs` (non-Windows).
    let _ = &app;

    #[cfg(not(target_os = "windows"))]
    check_shell_configs(app, cli_key, &mut out, &mut seen)?;

    out.sort_by(|a, b| {
        let a_type = a.source_type.as_str();
        let b_type = b.source_type.as_str();
        match (a_type, b_type) {
            ("system", "file") => std::cmp::Ordering::Greater,
            ("file", "system") => std::cmp::Ordering::Less,
            _ => {
                let name_cmp = a.var_name.cmp(&b.var_name);
                if name_cmp != std::cmp::Ordering::Equal {
                    return name_cmp;
                }
                a.source_path.cmp(&b.source_path)
            }
        }
    });

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_env_lock;
    use std::ffi::OsString;

    struct EnvRestore(Vec<(&'static str, Option<OsString>)>);

    impl EnvRestore {
        fn set(keys: &[&'static str]) -> Self {
            let saved = keys
                .iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect();
            for key in keys {
                std::env::set_var(key, "test-value");
            }
            Self(saved)
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, value) in self.0.drain(..).rev() {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn dedupe_key_for_file_strips_line_number() {
        let conflict = EnvConflict {
            var_name: "OpenAI_API_KEY".to_string(),
            source_type: "file".to_string(),
            source_path: "/Users/example/.zshrc:123".to_string(),
        };

        assert_eq!(
            conflict_dedupe_key(&conflict),
            "OPENAI_API_KEY|file|/Users/example/.zshrc"
        );
    }

    #[test]
    fn dedupe_key_for_system_keeps_source_path() {
        let conflict = EnvConflict {
            var_name: "ANTHROPIC_API_KEY".to_string(),
            source_type: "system".to_string(),
            source_path: "Process Environment".to_string(),
        };

        assert_eq!(
            conflict_dedupe_key(&conflict),
            "ANTHROPIC_API_KEY|system|Process Environment"
        );
    }

    #[test]
    fn grok_conflicts_match_only_inference_relevant_exact_variable_names() {
        let _lock = test_env_lock();
        let expected = [
            "GROK_MODELS_BASE_URL",
            "XAI_API_KEY",
            "GROK_CODE_XAI_API_KEY",
            "GROK_WEB_SEARCH_MODEL",
        ];
        let ignored = [
            "GROK_HOME",
            "GROK_TELEMETRY_ENABLED",
            "GROK_LOG_FILE",
            "MY_XAI_API_KEY_BACKUP",
            "GROK_MODELS_BASE_URL_BACKUP",
        ];
        let keys = expected
            .iter()
            .chain(ignored.iter())
            .copied()
            .collect::<Vec<_>>();
        let _restore = EnvRestore::set(&keys);
        let app = tauri::test::mock_app();

        let conflicts = check_env_conflicts(app.handle(), "grok").expect("check conflicts");
        let process_vars = conflicts
            .iter()
            .filter(|conflict| conflict.source_type == "system")
            .map(|conflict| conflict.var_name.as_str())
            .collect::<HashSet<_>>();

        for key in expected {
            assert!(
                process_vars.contains(key),
                "missing expected conflict {key}"
            );
        }
        for key in ignored {
            assert!(!process_vars.contains(key), "unexpected conflict {key}");
        }
    }
}
