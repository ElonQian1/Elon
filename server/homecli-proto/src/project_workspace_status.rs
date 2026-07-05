use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWorkspaceInspectStatus {
    pub workspace_path: String,
    pub path_exists: bool,
    pub is_dir: bool,
    pub is_git_worktree: bool,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub git_head: Option<String>,
    #[serde(default)]
    pub git_remote_origin: Option<String>,
    pub has_uncommitted_changes: bool,
    #[serde(default)]
    pub uncommitted_count: Option<u32>,
    #[serde(default)]
    pub disk_free_bytes: Option<u64>,
    pub codex_available: bool,
    pub copilot_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGitWorktreeAudit {
    pub workspace_path: String,
    #[serde(default)]
    pub git_root: Option<String>,
    #[serde(default)]
    pub worktrees: Vec<ProjectGitWorktreeEntry>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGitWorktreeEntry {
    pub path: String,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub detached: bool,
    #[serde(default)]
    pub bare: bool,
    #[serde(default)]
    pub current: bool,
    pub has_uncommitted_changes: bool,
    pub uncommitted_count: u32,
    pub untracked_count: u32,
    pub modified_count: u32,
    pub staged_count: u32,
    #[serde(default)]
    pub status_preview: Vec<String>,
    #[serde(default)]
    pub status_truncated: bool,
    #[serde(default)]
    pub status_error: Option<String>,
}
