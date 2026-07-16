use anyhow::{Context, Result};
use homecli_proto::{AgentToServer, ProjectGitWorktreeAudit, ProjectGitWorktreeEntry};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::ws_client_transport::try_send_json;

const STATUS_PREVIEW_LIMIT: usize = 40;

pub fn audit_project_git_worktrees(workspace_path: &str) -> Result<ProjectGitWorktreeAudit> {
    let workspace = PathBuf::from(workspace_path);
    if !workspace.exists() {
        anyhow::bail!("工作区目录不存在: {workspace_path}");
    }
    if !workspace.is_dir() {
        anyhow::bail!("workspace_path 必须指向目录: {workspace_path}");
    }
    let is_worktree = git_output(&workspace, &["rev-parse", "--is-inside-work-tree"])
        .map(|value| value == "true")
        .unwrap_or(false);
    if !is_worktree {
        anyhow::bail!("工作区不是 Git worktree: {workspace_path}");
    }

    let git_root = git_output(&workspace, &["rev-parse", "--show-toplevel"]);
    let list = git_output(&workspace, &["worktree", "list", "--porcelain"])
        .context("读取 git worktree 列表失败")?;
    let current_key = path_key(&workspace);
    let mut warnings = Vec::new();
    let mut entries = parse_worktree_porcelain(&list);
    if entries.is_empty() {
        warnings.push("git worktree list 未返回任何工作树，已退回当前工作区状态。".to_string());
        entries.push(ProjectGitWorktreeEntry {
            path: workspace.to_string_lossy().to_string(),
            branch: git_output(&workspace, &["rev-parse", "--abbrev-ref", "HEAD"])
                .filter(|value| value != "HEAD"),
            head: git_output(&workspace, &["rev-parse", "--short", "HEAD"]),
            detached: false,
            bare: false,
            current: true,
            has_uncommitted_changes: false,
            uncommitted_count: 0,
            untracked_count: 0,
            modified_count: 0,
            staged_count: 0,
            status_preview: Vec::new(),
            status_truncated: false,
            status_error: None,
        });
    }

    for entry in &mut entries {
        entry.current = path_key(Path::new(&entry.path)) == current_key;
        fill_status(entry);
    }

    Ok(ProjectGitWorktreeAudit {
        workspace_path: workspace_path.to_string(),
        git_root,
        worktrees: entries,
        warnings,
    })
}

pub fn spawn_git_worktree_audit_response(
    req_id: String,
    workspace_path: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    tracing::info!("AuditProjectGitWorktrees: {}", req_id);
    tokio::spawn(async move {
        let response = git_worktree_audit_response(req_id, &workspace_path);
        let _ = try_send_json(&tx, &response);
    });
}

fn git_worktree_audit_response(req_id: String, workspace_path: &str) -> AgentToServer {
    match audit_project_git_worktrees(workspace_path) {
        Ok(audit) => AgentToServer::ProjectGitWorktreesAudited { req_id, audit },
        Err(e) => AgentToServer::ProjectGitWorktreeAuditError {
            req_id,
            message: e.to_string(),
        },
    }
}

fn parse_worktree_porcelain(text: &str) -> Vec<ProjectGitWorktreeEntry> {
    let mut entries = Vec::new();
    let mut current: Option<ProjectGitWorktreeEntry> = None;

    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(ProjectGitWorktreeEntry {
                path: path.to_string(),
                branch: None,
                head: None,
                detached: false,
                bare: false,
                current: false,
                has_uncommitted_changes: false,
                uncommitted_count: 0,
                untracked_count: 0,
                modified_count: 0,
                staged_count: 0,
                status_preview: Vec::new(),
                status_truncated: false,
                status_error: None,
            });
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(head) = line.strip_prefix("HEAD ") {
            entry.head = Some(short_head(head));
        } else if let Some(branch) = line.strip_prefix("branch ") {
            entry.branch = Some(branch.trim_start_matches("refs/heads/").to_string());
        } else if line == "detached" {
            entry.detached = true;
        } else if line == "bare" {
            entry.bare = true;
        }
    }

    if let Some(entry) = current {
        entries.push(entry);
    }
    entries
}

fn fill_status(entry: &mut ProjectGitWorktreeEntry) {
    let path = PathBuf::from(&entry.path);
    let Some(status) = git_output(&path, &["status", "--porcelain"]) else {
        entry.status_error = Some("无法读取 git status".to_string());
        return;
    };
    let lines = status
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let counts = status_counts(&lines);
    entry.uncommitted_count = counts.uncommitted;
    entry.untracked_count = counts.untracked;
    entry.modified_count = counts.modified;
    entry.staged_count = counts.staged;
    entry.has_uncommitted_changes = counts.uncommitted > 0;
    entry.status_truncated = lines.len() > STATUS_PREVIEW_LIMIT;
    entry.status_preview = lines.into_iter().take(STATUS_PREVIEW_LIMIT).collect();

    if entry.branch.is_none() {
        entry.branch = git_output(&path, &["rev-parse", "--abbrev-ref", "HEAD"])
            .filter(|value| value != "HEAD");
    }
    if entry.head.is_none() {
        entry.head = git_output(&path, &["rev-parse", "--short", "HEAD"]);
    }
}

#[derive(Debug, Default)]
struct StatusCounts {
    uncommitted: u32,
    untracked: u32,
    modified: u32,
    staged: u32,
}

fn status_counts(lines: &[String]) -> StatusCounts {
    let mut counts = StatusCounts::default();
    for line in lines {
        counts.uncommitted += 1;
        if line.starts_with("??") {
            counts.untracked += 1;
            continue;
        }
        let mut chars = line.chars();
        let index = chars.next().unwrap_or(' ');
        let worktree = chars.next().unwrap_or(' ');
        if index != ' ' {
            counts.staged += 1;
        }
        if worktree != ' ' {
            counts.modified += 1;
        }
    }
    counts
}

fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = elon_pc_dev_runtime::command_output("git", args, Some(cwd)).ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn short_head(head: &str) -> String {
    head.chars().take(12).collect()
}

fn path_key(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{parse_worktree_porcelain, status_counts};

    #[test]
    fn parses_worktree_porcelain_entries() {
        let entries = parse_worktree_porcelain(
            "worktree D:/repo\nHEAD 1234567890abcdef\nbranch refs/heads/main\n\nworktree D:/repo-wt\nHEAD abcdef123456\nbranch refs/heads/ai/session/prj/conv\n\n",
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "D:/repo");
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert_eq!(entries[0].head.as_deref(), Some("1234567890ab"));
        assert_eq!(entries[1].branch.as_deref(), Some("ai/session/prj/conv"));
    }

    #[test]
    fn counts_porcelain_status_kinds() {
        let lines = vec![
            " M src/lib.rs".to_string(),
            "A  src/new.rs".to_string(),
            "?? tmp.txt".to_string(),
        ];
        let counts = status_counts(&lines);
        assert_eq!(counts.uncommitted, 3);
        assert_eq!(counts.modified, 1);
        assert_eq!(counts.staged, 1);
        assert_eq!(counts.untracked, 1);
    }
}
