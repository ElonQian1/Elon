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

#[test]
fn inspect_git_workspace_uses_first_remote_when_origin_is_absent() {
    let Some(dir) = temp_git_repo("first-remote") else {
        return;
    };
    run_git(
        &dir,
        &["remote", "add", "upstream", "https://example.com/demo.git"],
    );

    let status = inspect_project_workspace(dir.to_string_lossy().as_ref()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        status.git_remote_origin.as_deref(),
        Some("https://example.com/demo.git")
    );
}

#[test]
fn inspect_detached_head_uses_containing_branch_name() {
    let Some(dir) = temp_git_repo("detached-head") else {
        return;
    };
    run_git(&dir, &["checkout", "--detach", "HEAD"]);

    let status = inspect_project_workspace(dir.to_string_lossy().as_ref()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(status.git_branch.as_deref(), Some("main"));
}

fn temp_git_repo(label: &str) -> Option<PathBuf> {
    if !git_available() {
        return None;
    }
    let dir = std::env::temp_dir().join(format!(
        "elon-git-workspace-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    if !run_git_status(&dir, &["init", "-b", "main"]) {
        if !run_git_status(&dir, &["init"]) {
            let _ = std::fs::remove_dir_all(&dir);
            return None;
        }
        run_git(&dir, &["checkout", "-B", "main"]);
    }
    run_git(&dir, &["config", "user.email", "test@example.com"]);
    run_git(&dir, &["config", "user.name", "Elon Test"]);
    std::fs::write(dir.join("README.md"), "# Demo\n").unwrap();
    run_git(&dir, &["add", "README.md"]);
    run_git(&dir, &["commit", "-m", "init"]);
    Some(dir)
}

fn git_available() -> bool {
    crate::git_command_error::git_command()
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_git(cwd: &Path, args: &[&str]) {
    assert!(
        run_git_status(cwd, args),
        "git command failed: git {}",
        args.join(" ")
    );
}

fn run_git_status(cwd: &Path, args: &[&str]) -> bool {
    crate::git_command_error::git_command()
        .current_dir(cwd)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
