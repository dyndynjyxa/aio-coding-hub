use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, OnceLock};

use tempfile::TempDir;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
const TEST_APP_DOTDIR_PREFIX: &str = ".aio-coding-hub-test-";
const TEST_HOME_DIR_ENV: &str = "AIO_CODING_HUB_TEST_HOME";

fn env_lock() -> MutexGuard<'static, ()> {
    let mutex = ENV_LOCK.get_or_init(|| Mutex::new(()));
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn test_app_dotdir_name(home: &std::path::Path) -> String {
    let suffix = home
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| {
            value
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "isolated".to_string());

    format!("{TEST_APP_DOTDIR_PREFIX}{suffix}")
}

#[derive(Default)]
struct EnvRestore {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvRestore {
    fn save_once(&mut self, key: &'static str) {
        if self.saved.iter().any(|(k, _)| *k == key) {
            return;
        }
        self.saved.push((key, std::env::var_os(key)));
    }

    fn set_var(&mut self, key: &'static str, value: impl Into<OsString>) {
        self.save_once(key);
        std::env::set_var(key, value.into());
    }

    fn remove_var(&mut self, key: &'static str) {
        self.save_once(key);
        std::env::remove_var(key);
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..).rev() {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

pub struct TestApp {
    _env: EnvRestore,
    _lock: MutexGuard<'static, ()>,
    #[allow(dead_code)]
    home: TempDir,
    #[allow(dead_code)]
    app_dotdir_name: String,
    app: tauri::App<tauri::test::MockRuntime>,
}

impl TestApp {
    pub fn new() -> Self {
        let lock = env_lock();
        let home = tempfile::tempdir().expect("tempdir");

        let mut env = EnvRestore::default();
        let home_os = home.path().as_os_str().to_os_string();
        let app_dotdir_name = test_app_dotdir_name(home.path());

        env.set_var("HOME", home_os.clone());
        env.set_var("AIO_CODING_HUB_HOME_DIR", home_os.clone());
        env.set_var(
            "XDG_STATE_HOME",
            home.path().join(".state").as_os_str().to_os_string(),
        );
        // Windows fallback env for `dirs`/tauri path resolution.
        env.set_var("USERPROFILE", home_os);
        env.set_var(TEST_HOME_DIR_ENV, home.path().as_os_str().to_os_string());

        // Ensure app data stays within the isolated HOME.
        env.set_var("AIO_CODING_HUB_DOTDIR_NAME", app_dotdir_name.clone());

        // Default to ~/.codex for deterministic codex_paths behavior.
        env.remove_var("CODEX_HOME");

        // Flush the global settings cache so a fresh read hits the new temp dir.
        aio_coding_hub_lib::test_support::clear_settings_cache();

        let app = tauri::test::mock_app();

        Self {
            _lock: lock,
            _env: env,
            home,
            app_dotdir_name,
            app,
        }
    }

    pub fn handle(&self) -> tauri::AppHandle<tauri::test::MockRuntime> {
        self.app.handle().clone()
    }

    #[allow(dead_code)]
    pub fn home_dir(&self) -> &std::path::Path {
        self.home.path()
    }

    #[allow(dead_code)]
    pub fn app_dotdir_name(&self) -> &str {
        &self.app_dotdir_name
    }
}

impl Default for TestApp {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Shared JSON assertion helpers (previously duplicated across test files)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn json_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

#[allow(dead_code)]
pub fn json_i64(value: &serde_json::Value, key: &str) -> i64 {
    value.get(key).and_then(|v| v.as_i64()).unwrap_or_default()
}

#[allow(dead_code)]
pub fn json_bool(value: &serde_json::Value, key: &str) -> bool {
    value.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

#[allow(dead_code)]
pub fn json_f64(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|v| v.as_f64())
}

#[allow(dead_code)]
pub fn json_u64(value: &serde_json::Value, key: &str) -> u64 {
    value.get(key).and_then(|v| v.as_u64()).unwrap_or_default()
}

#[allow(dead_code)]
pub fn json_array(value: serde_json::Value) -> Vec<serde_json::Value> {
    value.as_array().cloned().unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Skills test fixture: shared workspace + skill + SSOT setup
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct SkillTestFixture {
    pub workspace_id: i64,
    pub skill_id: i64,
    pub skill_key: String,
    pub ssot_skill_dir: std::path::PathBuf,
    pub cli_skills_root: std::path::PathBuf,
    pub conn: rusqlite::Connection,
}

#[allow(dead_code)]
impl SkillTestFixture {
    /// Create a workspace (active) + skill row + SSOT dir for the given `cli_key`.
    pub fn new(
        app: &TestApp,
        handle: &tauri::AppHandle<tauri::test::MockRuntime>,
        cli_key: &str,
        workspace_name: &str,
    ) -> Self {
        use rusqlite::params;

        let created = aio_coding_hub_lib::test_support::workspace_create_json(
            handle,
            cli_key,
            workspace_name,
            false,
        )
        .expect("create workspace");
        let workspace_id = json_i64(&created, "id");
        assert!(workspace_id > 0);

        let db_path = aio_coding_hub_lib::test_support::db_path(handle).expect("db path");
        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        conn.execute(
            r#"
INSERT INTO workspace_active(cli_key, workspace_id, updated_at)
VALUES (?1, ?2, ?3)
ON CONFLICT(cli_key) DO UPDATE SET
  workspace_id = excluded.workspace_id,
  updated_at = excluded.updated_at
"#,
            params![cli_key, workspace_id, 1_i64],
        )
        .expect("set active workspace");

        let skill_key = "context7".to_string();
        conn.execute(
            r#"
INSERT INTO skills(
  skill_key,
  name,
  normalized_name,
  description,
  source_git_url,
  source_branch,
  source_subdir,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, '', ?4, 'main', ?5, 1, 1)
"#,
            params![
                &skill_key,
                "Context7",
                "context7",
                "https://example.com/repo.git",
                "skills/context7"
            ],
        )
        .expect("insert skill");
        let skill_id = conn.last_insert_rowid();
        assert!(skill_id > 0);

        let app_data_dir =
            aio_coding_hub_lib::test_support::app_data_dir(handle).expect("app_data_dir");
        let ssot_skill_dir = app_data_dir.join("skills").join(&skill_key);
        std::fs::create_dir_all(&ssot_skill_dir).expect("create ssot dir");
        std::fs::write(ssot_skill_dir.join("SKILL.md"), "name: Context7\n")
            .expect("write ssot skill");

        let cli_dot_dir = format!(".{cli_key}");
        let cli_skills_root = app.home_dir().join(&cli_dot_dir).join("skills");

        Self {
            workspace_id,
            skill_id,
            skill_key,
            ssot_skill_dir,
            cli_skills_root,
            conn,
        }
    }
}
