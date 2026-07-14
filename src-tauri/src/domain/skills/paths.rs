use crate::app_paths;
use crate::codex_paths;
use crate::domain::skills::types::SkillsPaths;
use std::path::PathBuf;

pub(super) fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn home_dir<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    crate::app_paths::home_dir(app)
}

pub(super) fn ssot_skills_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?.join("skills"))
}

pub(super) fn repos_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?.join("skill-repos"))
}

pub(super) fn cli_skills_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    validate_cli_key(cli_key)?;
    let home = home_dir(app)?;
    match cli_key {
        "claude" => Ok(home.join(".claude").join("skills")),
        "codex" => codex_paths::codex_skills_dir(app),
        "gemini" => Ok(home.join(".gemini").join("skills")),
        "grok" => crate::grok_config::skills_dir(app),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}").into()),
    }
}

pub(super) fn ensure_skills_roots<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<()> {
    std::fs::create_dir_all(ssot_skills_root(app)?)
        .map_err(|e| format!("failed to create ssot skills dir: {e}"))?;
    std::fs::create_dir_all(repos_root(app)?)
        .map_err(|e| format!("failed to create repos dir: {e}"))?;
    Ok(())
}

pub fn paths_get<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<SkillsPaths> {
    validate_cli_key(cli_key)?;
    let ssot = ssot_skills_root(app)?;
    let repos = repos_root(app)?;
    let cli = cli_skills_root(app, cli_key)?;

    Ok(SkillsPaths {
        ssot_dir: ssot.to_string_lossy().to_string(),
        repos_dir: repos.to_string_lossy().to_string(),
        cli_dir: cli.to_string_lossy().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    struct GrokHomeRestore(Option<OsString>);

    impl Drop for GrokHomeRestore {
        fn drop(&mut self) {
            match self.0.take() {
                Some(value) => std::env::set_var("GROK_HOME", value),
                None => std::env::remove_var("GROK_HOME"),
            }
        }
    }

    #[test]
    fn grok_skills_root_uses_grok_home() {
        let _lock = crate::test_support::test_env_lock();
        let previous = GrokHomeRestore(std::env::var_os("GROK_HOME"));
        let temp = tempfile::tempdir().expect("tempdir");
        let grok_home = temp.path().join("custom-grok");
        std::env::set_var("GROK_HOME", &grok_home);
        let app = tauri::test::mock_app();

        let path = cli_skills_root(app.handle(), "grok").expect("Grok skills root");

        assert_eq!(path, grok_home.join("skills"));
        drop(previous);
    }
}
