// server/src/node_agent_workspace_match.rs

use std::path::{Path, PathBuf};

pub(crate) fn cwd_matches_workspace(cwd: &str, workspace: &Path) -> bool {
    let cwd = cwd.trim();
    if cwd.is_empty() {
        return false;
    }
    canonical_or_original(Path::new(cwd)).starts_with(workspace)
}

pub(crate) fn record_cwd_matches_workspace(record_cwd: Option<&str>, workspace: &Path) -> bool {
    record_cwd.is_some_and(|cwd| cwd_matches_workspace(cwd, workspace))
}

pub(crate) fn canonical_or_original(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
