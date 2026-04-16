use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SkillRepoSummary {
    pub id: i64,
    pub git_url: String,
    pub branch: String,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledSkillSummary {
    pub id: i64,
    pub skill_key: String,
    pub name: String,
    pub description: String,
    pub source_git_url: String,
    pub source_branch: String,
    pub source_subdir: String,
    pub installed_commit: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableSkillSummary {
    pub name: String,
    pub description: String,
    pub source_git_url: String,
    pub source_branch: String,
    pub source_subdir: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillsPaths {
    pub ssot_dir: String,
    pub repos_dir: String,
    pub cli_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSkillSummary {
    pub dir_name: String,
    pub path: String,
    pub name: String,
    pub description: String,
    pub source_git_url: Option<String>,
    pub source_branch: Option<String>,
    pub source_subdir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillImportIssue {
    pub dir_name: String,
    pub error_code: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillImportLocalBatchReport {
    pub imported: Vec<InstalledSkillSummary>,
    pub skipped: Vec<SkillImportIssue>,
    pub failed: Vec<SkillImportIssue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillUpdateInfo {
    pub skill_id: i64,
    pub has_update: bool,
    pub installed_commit: Option<String>,
    pub latest_commit: Option<String>,
}
