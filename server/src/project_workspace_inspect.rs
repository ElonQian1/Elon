use anyhow::Result;
use homecli_proto::ProjectWorkspaceInspectStatus;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn inspect_project_workspace(workspace_path: &str) -> Result<ProjectWorkspaceInspectStatus> {
    let path = PathBuf::from(workspace_path);
    let path_exists = path.exists();
    let is_dir = path.is_dir();

    let git_inside = if is_dir {
        git_output(&path, &["rev-parse", "--is-inside-work-tree"])
            .map(|value| value == "true")
            .unwrap_or(false)
    } else {
        false
    };
    let git_branch = if git_inside {
        git_output(&path, &["rev-parse", "--abbrev-ref", "HEAD"])
    } else {
        None
    };
    let git_head = if git_inside {
        git_output(&path, &["rev-parse", "--short", "HEAD"])
    } else {
        None
    };
    let git_remote_origin = if git_inside {
        git_output(&path, &["remote", "get-url", "origin"])
    } else {
        None
    };
    let porcelain = if git_inside {
        git_output(&path, &["status", "--porcelain"])
    } else {
        None
    };
    let uncommitted_count = porcelain
        .as_deref()
        .map(|value| value.lines().filter(|line| !line.trim().is_empty()).count() as u32);
    let has_uncommitted_changes = uncommitted_count.unwrap_or(0) > 0;

    Ok(ProjectWorkspaceInspectStatus {
        workspace_path: workspace_path.to_string(),
        path_exists,
        is_dir,
        is_git_worktree: git_inside,
        git_branch,
        git_head,
        git_remote_origin,
        has_uncommitted_changes,
        uncommitted_count,
        disk_free_bytes: disk_free_bytes(&path),
        codex_available: command_available("codex"),
        copilot_available: command_available("copilot"),
    })
}

fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

#[cfg(windows)]
fn command_available(name: &str) -> bool {
    Command::new("where")
        .arg(name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn command_available(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", "command -v \"$1\" >/dev/null 2>&1", "sh", name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn disk_free_bytes(path: &Path) -> Option<u64> {
    use std::path::{Component, Prefix};

    let drive = path.components().find_map(|component| match component {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                Some((letter as char).to_ascii_uppercase())
            }
            _ => None,
        },
        _ => None,
    })?;
    let script = format!("(Get-PSDrive -Name '{}').Free", drive);
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

#[cfg(not(windows))]
fn disk_free_bytes(path: &Path) -> Option<u64> {
    let target = if path.exists() {
        path
    } else {
        path.parent().unwrap_or_else(|| Path::new("/"))
    };
    let output = Command::new("df").args(["-Pk"]).arg(target).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let available_kb = line.split_whitespace().nth(3)?.parse::<u64>().ok()?;
    available_kb.checked_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn inspect_missing_path_reports_unavailable_workspace() {
        let missing = std::env::temp_dir().join(format!(
            "elon-missing-workspace-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let status = inspect_project_workspace(missing.to_string_lossy().as_ref()).unwrap();

        assert!(!status.path_exists);
        assert!(!status.is_dir);
        assert!(!status.is_git_worktree);
        assert!(!status.has_uncommitted_changes);
    }

    #[test]
    fn inspect_plain_directory_reports_non_git_workspace() {
        let dir = std::env::temp_dir().join(format!(
            "elon-plain-workspace-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let status = inspect_project_workspace(dir.to_string_lossy().as_ref()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert!(status.path_exists);
        assert!(status.is_dir);
        assert!(!status.is_git_worktree);
        assert_eq!(status.uncommitted_count, None);
    }
}
