use anyhow::Result;
use homecli_proto::ProjectWorkspaceInspectStatus;
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
use std::process::Command;

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
    let output = elon_pc_dev_runtime::command_output("git", args, Some(cwd)).ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn command_available(name: &str) -> bool {
    elon_pc_dev_runtime::command_path(name).is_some()
}

#[cfg(windows)]
fn disk_free_bytes(path: &Path) -> Option<u64> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Component, Prefix};

    let drive_root = path.components().find_map(|component| match component {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                Some(format!("{}:\\", (letter as char).to_ascii_uppercase()))
            }
            _ => None,
        },
        _ => None,
    })?;
    let wide_path = OsStr::new(&drive_root)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let mut free_available = 0u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide_path.as_ptr(),
            &mut free_available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free_available)
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetDiskFreeSpaceExW(
        lpDirectoryName: *const u16,
        lpFreeBytesAvailableToCaller: *mut u64,
        lpTotalNumberOfBytes: *mut u64,
        lpTotalNumberOfFreeBytes: *mut u64,
    ) -> i32;
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
