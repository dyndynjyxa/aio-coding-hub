//! Usage: Skills domain (repositories, installed skills, local import, and CLI integration).

mod discover;
mod fs_ops;
mod git_url;
mod installed;
mod local;
mod local_swap;
mod ops;
mod paths;
mod repo_cache;
mod repos;
mod skill_md;
mod types;
mod util;

pub use discover::discover_available;
pub use installed::installed_list_for_workspace;
pub use local::{delete_local, import_local, import_local_batch, install_to_local, local_list};
pub(crate) use local_swap::swap_local_skills_for_workspace_switch;
pub(crate) use ops::sync_one_cli;
pub use ops::{install, return_to_local, set_enabled, sync_cli_for_workspace, uninstall};
pub use paths::paths_get;
pub use repos::{repo_delete, repo_upsert, repos_list};
pub use types::{
    AvailableSkillSummary, InstalledSkillSummary, LocalSkillSummary, SkillImportLocalBatchReport,
    SkillRepoSummary, SkillsPaths,
};

#[cfg(test)]
mod tests;
