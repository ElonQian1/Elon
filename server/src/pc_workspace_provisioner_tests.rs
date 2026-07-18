use super::{
    cleanup_project_workspace_in, conversation_workspace_head_landed,
    ensure_conversation_workspace_committed, git_output, git_path_arg, is_git_work_tree,
    is_retryable_push_rejection, merge_conversation_workspace, prepare_conversation_workspace_in,
    prepare_conversation_workspace_in_with_supervision, recover_stale_conversation_worktree_path,
    worktree_clean,
};
use elon_pc_dev_runtime::safe_path_part;
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[test]
fn safe_path_part_removes_path_separators() {
    assert_eq!(
        safe_path_part("../usr:abc\\project", "fallback", 80),
        "usrabcproject"
    );
}

#[test]
fn safe_path_part_uses_fallback_when_empty() {
    assert_eq!(safe_path_part("///", "fallback", 80), "fallback");
}

#[test]
fn retryable_push_rejection_only_matches_remote_race_errors() {
    assert!(is_retryable_push_rejection(
        "! [rejected] HEAD -> main (fetch first)"
    ));
    assert!(is_retryable_push_rejection(
        "error: failed to push some refs to 'origin'"
    ));
    assert!(is_retryable_push_rejection(
        "Updates were rejected because the remote contains work that you do not have locally"
    ));
    assert!(!is_retryable_push_rejection(
        "Permission denied (publickey). fatal: Could not read from remote repository."
    ));
    assert!(!is_retryable_push_rejection(
        "remote: error: GH006: Protected branch update failed for refs/heads/main."
    ));
}

#[cfg(windows)]
#[test]
fn git_path_arg_strips_windows_verbatim_prefixes() {
    assert_eq!(
        git_path_arg(Path::new(r"\\?\C:\Users\Administrator\repo")),
        r"C:\Users\Administrator\repo"
    );
    assert_eq!(
        git_path_arg(Path::new(r"\\?\UNC\server\share\repo")),
        r"\\server\share\repo"
    );
}

#[test]
fn non_git_conversation_workspace_uses_base_path() {
    let base = std::env::temp_dir().join(format!(
        "elon_non_git_conversation_{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&base).expect("base should create");

    let workspace = prepare_conversation_workspace_in(
        &base.join("managed-workspaces"),
        &base.to_string_lossy(),
        "project-a",
        "conversation-a",
    )
    .expect("non-git workspace should resolve");

    assert!(!workspace.isolated);
    assert_eq!(workspace.branch, None);
    assert_eq!(workspace.workspace_path, base.to_string_lossy().to_string());
    let _ = fs::remove_dir_all(base);
}

#[test]
fn stale_conversation_worktree_path_is_archived() {
    let root = std::env::temp_dir().join(format!(
        "elon_stale_conversation_{}",
        Uuid::new_v4().simple()
    ));
    let worktree_root = root.join("conversation-worktrees").join("project-a");
    let worktree_path = worktree_root.join("conversation-a");
    fs::create_dir_all(&worktree_path).expect("stale path should create");
    fs::write(worktree_path.join("leftover.txt"), "partial output\n")
        .expect("leftover file should write");

    recover_stale_conversation_worktree_path(&root, &worktree_root, &worktree_path)
        .expect("stale path should be recovered");

    assert!(!worktree_path.exists());
    let archived = fs::read_dir(&worktree_root)
        .expect("worktree root should be readable")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("conversation-a.stale-"))
                .unwrap_or(false)
        })
        .expect("stale directory should be archived");
    assert!(archived.join("leftover.txt").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn prepare_conversation_workspace_recovers_stale_path() {
    let root = std::env::temp_dir().join(format!(
        "elon_prepare_stale_conversation_root_{}",
        Uuid::new_v4().simple()
    ));
    let base = std::env::temp_dir().join(format!(
        "elon_prepare_stale_conversation_repo_{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&base).expect("base repo should create");
    run_git(&base, &["init"]);
    run_git(&base, &["config", "user.email", "ai@example.test"]);
    run_git(&base, &["config", "user.name", "AI Test"]);
    fs::write(base.join("README.md"), "seed\n").expect("seed file should write");
    run_git(&base, &["add", "README.md"]);
    run_git(&base, &["commit", "-m", "seed"]);

    let worktree_root = root.join("conversation-worktrees").join("project-a");
    let stale_path = worktree_root.join("conversation-a");
    fs::create_dir_all(&stale_path).expect("stale path should create");
    fs::write(stale_path.join("leftover.txt"), "partial output\n")
        .expect("leftover file should write");

    let workspace = prepare_conversation_workspace_in(
        &root,
        &base.to_string_lossy(),
        "project-a",
        "conversation-a",
    )
    .expect("stale path should be recovered");

    assert!(workspace.isolated);
    let active = std::path::PathBuf::from(&workspace.workspace_path);
    assert!(is_git_work_tree(&active));
    assert!(fs::read_dir(&worktree_root)
        .expect("worktree root should be readable")
        .filter_map(|entry| entry.ok())
        .any(|entry| entry
            .file_name()
            .to_str()
            .map(|value| value.starts_with("conversation-a.stale-"))
            .unwrap_or(false)));

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(base);
}

#[test]
fn merge_missing_conversation_git_metadata_is_blocked_message() {
    let base = std::env::temp_dir().join(format!(
        "elon_missing_conversation_git_base_{}",
        Uuid::new_v4().simple()
    ));
    let active = std::env::temp_dir().join(format!(
        "elon_missing_conversation_git_active_{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&base).expect("base repo should create");
    fs::create_dir_all(&active).expect("active path should create");
    run_git(&base, &["init"]);
    run_git(&base, &["config", "user.email", "ai@example.test"]);
    run_git(&base, &["config", "user.name", "AI Test"]);
    fs::write(base.join("README.md"), "seed\n").expect("seed file should write");
    run_git(&base, &["add", "README.md"]);
    run_git(&base, &["commit", "-m", "seed"]);

    let workspace = super::ConversationWorkspaceResult {
        base_workspace_path: Some(base.to_string_lossy().to_string()),
        workspace_path: active.to_string_lossy().to_string(),
        isolated: true,
        branch: Some("ai/session/project-a/conversation-a".into()),
        supervision_root_task_id: None,
    };
    let message = merge_conversation_workspace(&workspace)
        .expect("missing git metadata should return a blocked merge message");
    assert!(message.starts_with("conversation worktree missing git metadata"));

    let _ = fs::remove_dir_all(base);
    let _ = fs::remove_dir_all(active);
}

#[test]
fn dirty_conversation_workspace_is_auto_committed() {
    let repo = std::env::temp_dir().join(format!(
        "elon_dirty_conversation_{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&repo).expect("repo should create");
    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "ai@example.test"]);
    run_git(&repo, &["config", "user.name", "AI Test"]);
    fs::write(repo.join("README.md"), "seed\n").expect("seed file should write");
    run_git(&repo, &["add", "README.md"]);
    run_git(&repo, &["commit", "-m", "seed"]);

    fs::write(repo.join("README.md"), "changed\n").expect("dirty file should write");
    let commit = ensure_conversation_workspace_committed(&repo)
        .expect("auto commit should succeed")
        .expect("dirty workspace should create a commit");

    assert!(!commit.is_empty());
    assert!(worktree_clean(&repo).expect("status should be readable"));
    let subject = git_output(&repo, &["log", "-1", "--pretty=%s"])
        .expect("commit subject should be readable");
    assert_eq!(subject, "chore(ai): 保存会话工作区改动");
    let _ = fs::remove_dir_all(repo);
}

#[test]
fn conversation_workspace_head_landed_checks_origin_main() {
    let root = std::env::temp_dir().join(format!(
        "elon_landed_conversation_{}",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("root should create");
    let origin = root.join("origin.git");
    let base = root.join("base");
    let active = root.join("active");
    let unlanded = root.join("unlanded");

    fs::create_dir_all(&origin).expect("origin should create");
    run_git(&origin, &["init", "--bare"]);
    let origin_arg = origin.to_string_lossy().to_string();
    run_git(&root, &["clone", &origin_arg, "base"]);
    run_git(&base, &["checkout", "-b", "main"]);
    run_git(&base, &["config", "user.email", "ai@example.test"]);
    run_git(&base, &["config", "user.name", "AI Test"]);
    fs::write(base.join("README.md"), "seed\n").expect("seed file should write");
    run_git(&base, &["add", "README.md"]);
    run_git(&base, &["commit", "-m", "seed"]);
    run_git(&base, &["push", "-u", "origin", "main"]);

    let active_arg = active.to_string_lossy().to_string();
    run_git(
        &base,
        &[
            "worktree",
            "add",
            "-b",
            "ai/session/project-a/conversation-a",
            &active_arg,
            "main",
        ],
    );
    fs::write(active.join("README.md"), "landed\n").expect("landed file should write");
    run_git(&active, &["add", "README.md"]);
    run_git(&active, &["commit", "-m", "landed"]);
    run_git(&active, &["push", "origin", "HEAD:main"]);
    let landed_workspace = super::ConversationWorkspaceResult {
        base_workspace_path: Some(base.to_string_lossy().to_string()),
        workspace_path: active.to_string_lossy().to_string(),
        isolated: true,
        branch: Some("ai/session/project-a/conversation-a".into()),
        supervision_root_task_id: None,
    };
    assert!(
        conversation_workspace_head_landed(&landed_workspace).expect("landed probe should succeed")
    );

    let unlanded_arg = unlanded.to_string_lossy().to_string();
    run_git(
        &base,
        &[
            "worktree",
            "add",
            "-b",
            "ai/session/project-a/conversation-b",
            &unlanded_arg,
            "main",
        ],
    );
    fs::write(unlanded.join("README.md"), "local only\n").expect("unlanded file should write");
    run_git(&unlanded, &["add", "README.md"]);
    run_git(&unlanded, &["commit", "-m", "local only"]);
    let unlanded_workspace = super::ConversationWorkspaceResult {
        base_workspace_path: Some(base.to_string_lossy().to_string()),
        workspace_path: unlanded.to_string_lossy().to_string(),
        isolated: true,
        branch: Some("ai/session/project-a/conversation-b".into()),
        supervision_root_task_id: None,
    };
    assert!(!conversation_workspace_head_landed(&unlanded_workspace)
        .expect("unlanded probe should succeed"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_cleanup_preserves_supervision_leased_worktree_and_base_repo() {
    let temp = std::env::temp_dir().join(format!(
        "elon_supervision_cleanup_{}",
        Uuid::new_v4().simple()
    ));
    let project_dir = temp.join("user-a").join("project-a");
    let base = project_dir.join("repo");
    let active = temp
        .join("conversation-worktrees")
        .join("project-a")
        .join("conversation-a");
    fs::create_dir_all(&base).expect("base should create");
    run_git(&base, &["init"]);
    run_git(&base, &["config", "user.email", "ai@example.test"]);
    run_git(&base, &["config", "user.name", "AI Test"]);
    fs::write(base.join("README.md"), "seed\n").expect("seed should write");
    run_git(&base, &["add", "README.md"]);
    run_git(&base, &["commit", "-m", "seed"]);
    fs::create_dir_all(active.parent().unwrap()).expect("worktree root should create");
    let active_arg = active.to_string_lossy().to_string();
    run_git(
        &base,
        &[
            "worktree",
            "add",
            "-b",
            "ai/session/project-a/conversation-a",
            &active_arg,
            "HEAD",
        ],
    );
    crate::node_agent_supervision_worktree_lease::acquire(&base, &active, "root-1")
        .expect("lease should persist");

    let protected =
        cleanup_project_workspace_in(&temp, "project-a", base.to_string_lossy().as_ref())
            .expect("cleanup should defer safely");
    assert!(base.exists());
    assert!(active.exists());
    assert!(protected
        .skipped_paths
        .iter()
        .any(|message| message.contains("persistent lease")));

    crate::node_agent_supervision_worktree_lease::release(&base, &active, "root-1")
        .expect("accepted review should release lease");
    cleanup_project_workspace_in(&temp, "project-a", base.to_string_lossy().as_ref())
        .expect("cleanup should complete after acceptance");
    assert!(!project_dir.exists());
    assert!(!active.exists());
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn supervised_prepare_merge_review_release_and_cleanup_use_one_root_lease() {
    let temp = std::env::temp_dir().join(format!(
        "elon_supervision_lifecycle_{}",
        Uuid::new_v4().simple()
    ));
    let project_dir = temp.join("user-a").join("project-a");
    let base = project_dir.join("repo");
    init_repo(&base);

    let workspace = prepare_conversation_workspace_in_with_supervision(
        &temp,
        base.to_string_lossy().as_ref(),
        "project-a",
        "conversation-a",
        Some("root-task-1"),
    )
    .expect("supervised prepare should acquire the authoritative root lease");
    let active = std::path::PathBuf::from(&workspace.workspace_path);
    assert_eq!(
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(&base, &active)
            .unwrap()
            .as_deref(),
        Some("elon-supervision:root-task-1")
    );

    fs::write(active.join("README.md"), "supervised change\n").unwrap();
    run_git(&active, &["add", "README.md"]);
    run_git(&active, &["commit", "-m", "supervised change"]);
    let merge = merge_conversation_workspace(&workspace).unwrap();
    assert!(merge.contains("retained until accepted review"));
    assert!(
        active.exists(),
        "completion/merge must not delete a supervised worktree"
    );
    assert_eq!(
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(&base, &active)
            .unwrap()
            .as_deref(),
        Some("elon-supervision:root-task-1")
    );
    assert!(crate::node_agent_supervision_worktree_lease::release(
        &base,
        &active,
        "different-root"
    )
    .is_err());

    crate::node_agent_supervision_worktree_lease::release(&base, &active, "root-task-1")
        .expect("matching accepted review should release exactly the root lease");
    cleanup_project_workspace_in(&temp, "project-a", base.to_string_lossy().as_ref())
        .expect("cleanup should reclaim the reviewed worktree");
    assert!(!active.exists());
    assert!(!project_dir.exists());
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn ordinary_conversation_worktree_still_merges_and_cleans_immediately() {
    let temp = std::env::temp_dir().join(format!(
        "elon_ordinary_lifecycle_{}",
        Uuid::new_v4().simple()
    ));
    let base = temp.join("base");
    init_repo(&base);
    let workspace = prepare_conversation_workspace_in(
        &temp,
        base.to_string_lossy().as_ref(),
        "project-a",
        "conversation-a",
    )
    .unwrap();
    let active = std::path::PathBuf::from(&workspace.workspace_path);
    assert!(workspace.supervision_root_task_id.is_none());
    fs::write(active.join("README.md"), "ordinary change\n").unwrap();
    run_git(&active, &["add", "README.md"]);
    run_git(&active, &["commit", "-m", "ordinary change"]);

    let merge = merge_conversation_workspace(&workspace).unwrap();
    assert!(merge.starts_with("conversation branch merged"));
    assert!(
        !active.exists(),
        "ordinary lifecycle should keep its existing cleanup behavior"
    );
    assert_eq!(
        fs::read_to_string(base.join("README.md"))
            .unwrap()
            .replace("\r\n", "\n"),
        "ordinary change\n"
    );
    let _ = fs::remove_dir_all(temp);
}

fn init_repo(repo: &Path) {
    fs::create_dir_all(repo).expect("base should create");
    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.email", "ai@example.test"]);
    run_git(repo, &["config", "user.name", "AI Test"]);
    fs::write(repo.join("README.md"), "seed\n").expect("seed should write");
    run_git(repo, &["add", "README.md"]);
    run_git(repo, &["commit", "-m", "seed"]);
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git should start");
    assert!(
        output.status.success(),
        "git {:?} failed (status={:?}, stdout={}, stderr={})",
        args,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
