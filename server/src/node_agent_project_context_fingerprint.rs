//! Bounded content fingerprint for dirty project-context workspaces.
//!
//! Only stable, regular files below fixed count and byte ceilings are hashed.
//! Unsafe, incomplete, oversized, or racing snapshots fail closed.

use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

const MAX_DIRTY_FILES: usize = 256;
const MAX_DIRTY_BYTES: u64 = 64 * 1024 * 1024;
const HASH_BUFFER_BYTES: usize = 64 * 1024;

pub(crate) struct WorkspaceFingerprint {
    pub(crate) digest: Option<String>,
    pub(crate) status: String,
    pub(crate) file_count: usize,
    pub(crate) total_bytes: u64,
    pub(crate) bypass_reason: Option<String>,
}

pub(crate) fn inspect(
    workspace: &Path,
    git_clean: Option<bool>,
    status: Option<&str>,
) -> WorkspaceFingerprint {
    match (git_clean, status) {
        (Some(true), _) => WorkspaceFingerprint::clean(),
        (Some(false), Some(status)) => fingerprint_dirty_workspace(workspace, status),
        _ => WorkspaceFingerprint::unavailable("git_status_unavailable"),
    }
}

impl WorkspaceFingerprint {
    fn clean() -> Self {
        Self {
            digest: None,
            status: "clean_head".to_string(),
            file_count: 0,
            total_bytes: 0,
            bypass_reason: None,
        }
    }

    fn unavailable(reason: &str) -> Self {
        Self::blocked("unavailable", reason, 0, 0)
    }

    fn blocked(status: &str, reason: &str, file_count: usize, total_bytes: u64) -> Self {
        Self {
            digest: None,
            status: status.to_string(),
            file_count,
            total_bytes,
            bypass_reason: Some(reason.to_string()),
        }
    }
}

fn fingerprint_dirty_workspace(workspace: &Path, status: &str) -> WorkspaceFingerprint {
    let Ok(canonical_workspace) = fs::canonicalize(workspace) else {
        return WorkspaceFingerprint::unavailable("workspace_canonicalization_failed");
    };
    let tracked = crate::node_agent_update_checkpoint::git_output(
        workspace,
        &["diff", "--name-only", "-z", "HEAD", "--"],
    );
    let untracked = crate::node_agent_update_checkpoint::git_output(
        workspace,
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    );
    let raw_delta = crate::node_agent_update_checkpoint::git_output(
        workspace,
        &["diff", "--raw", "-z", "HEAD", "--"],
    );
    let (Some(tracked), Some(untracked), Some(raw_delta)) = (tracked, untracked, raw_delta) else {
        return WorkspaceFingerprint::unavailable("git_delta_unavailable");
    };
    let mut paths = nul_paths(&tracked);
    paths.extend(nul_paths(&untracked));
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return WorkspaceFingerprint::unavailable("dirty_paths_unavailable");
    }
    if paths.len() > MAX_DIRTY_FILES {
        return WorkspaceFingerprint::blocked(
            "limit_exceeded",
            "dirty_file_limit_exceeded",
            paths.len(),
            0,
        );
    }

    let mut hasher = Sha256::new();
    hasher.update(b"elon.project_context.worktree.v1\0");
    hasher.update(status.as_bytes());
    hasher.update(b"\0");
    hasher.update(raw_delta.as_bytes());
    let mut total_bytes = 0u64;
    for relative in &paths {
        let remaining_bytes = MAX_DIRTY_BYTES.saturating_sub(total_bytes);
        let Ok((bytes, metadata_len)) = hash_changed_file(
            workspace,
            &canonical_workspace,
            relative,
            remaining_bytes,
            &mut hasher,
        ) else {
            return WorkspaceFingerprint::blocked(
                "incomplete",
                "changed_file_unreadable_or_unsafe",
                paths.len(),
                total_bytes,
            );
        };
        total_bytes = total_bytes.saturating_add(metadata_len.max(bytes));
        if total_bytes > MAX_DIRTY_BYTES {
            return WorkspaceFingerprint::blocked(
                "limit_exceeded",
                "dirty_byte_limit_exceeded",
                paths.len(),
                total_bytes,
            );
        }
    }
    let status_after = crate::node_agent_update_checkpoint::git_output(
        workspace,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    );
    let raw_delta_after = crate::node_agent_update_checkpoint::git_output(
        workspace,
        &["diff", "--raw", "-z", "HEAD", "--"],
    );
    if status_after.as_deref() != Some(status)
        || raw_delta_after.as_deref() != Some(raw_delta.as_str())
    {
        return WorkspaceFingerprint::blocked(
            "raced",
            "workspace_changed_during_fingerprint",
            paths.len(),
            total_bytes,
        );
    }
    WorkspaceFingerprint {
        digest: Some(hex::encode(hasher.finalize())),
        status: "content_hashed".to_string(),
        file_count: paths.len(),
        total_bytes,
        bypass_reason: None,
    }
}

fn nul_paths(raw: &str) -> Vec<PathBuf> {
    raw.split('\0')
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn hash_changed_file(
    workspace: &Path,
    canonical_workspace: &Path,
    relative: &Path,
    max_bytes: u64,
    hasher: &mut Sha256,
) -> Result<(u64, u64), ()> {
    if !safe_relative_path(relative) {
        return Err(());
    }
    let path = workspace.join(relative);
    hasher.update(relative.to_string_lossy().as_bytes());
    hasher.update(b"\0");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            hasher.update(b"deleted\0");
            return Ok((0, 0));
        }
        Err(_) => return Err(()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(());
    }
    let canonical_path = fs::canonicalize(&path).map_err(|_| ())?;
    if !canonical_path.starts_with(canonical_workspace) || metadata.len() > max_bytes {
        return Err(());
    }
    hasher.update(metadata.len().to_le_bytes());
    let modified_before = metadata.modified().ok();
    let mut file = fs::File::open(&path).map_err(|_| ())?;
    let mut buffer = vec![0u8; HASH_BUFFER_BYTES];
    let mut read_bytes = 0u64;
    loop {
        let read = file.read(&mut buffer).map_err(|_| ())?;
        if read == 0 {
            break;
        }
        read_bytes = read_bytes.saturating_add(read as u64);
        if read_bytes > max_bytes {
            return Err(());
        }
        hasher.update(&buffer[..read]);
    }
    let metadata_after = fs::symlink_metadata(&path).map_err(|_| ())?;
    if metadata_after.len() != metadata.len()
        || metadata_after.modified().ok() != modified_before
        || read_bytes != metadata.len()
    {
        return Err(());
    }
    hasher.update(b"\0");
    Ok((read_bytes, metadata.len()))
}

fn safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::safe_relative_path;
    use std::path::Path;

    #[test]
    fn fingerprint_paths_stay_inside_the_workspace() {
        assert!(safe_relative_path(Path::new("server/src/main.rs")));
        assert!(!safe_relative_path(Path::new("../outside")));
    }
}
