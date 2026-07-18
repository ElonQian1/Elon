use std::{fs, path::Path};

use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::{
    git_command_error::git_command,
    node_agent_local_task_resume::{resolve_resume_workspace, ResumeWorkspaceMode},
    node_agent_update_recovery::{RecoveryTransport, WorkspaceGitFingerprint},
};

struct Fixture {
    root: PathBuf,
    base: PathBuf,
    active: PathBuf,
    head: String,
    parent: LocalTaskRecord,
    contract: SupervisionContract,
    receipt: UpdateRecoveryReceipt,
}

impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("elon-resume-rebuild-{}", Uuid::new_v4().simple()));
        let origin = root.join("origin.git");
        let base = root.join("user-a").join("project-a").join("repo");
        let active = root
            .join("conversation-worktrees")
            .join("project-a")
            .join("conversation-a");
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "--bare", path(&origin)]);
        fs::create_dir_all(base.parent().unwrap()).unwrap();
        git(&root, &["clone", path(&origin), path(&base)]);
        git(&base, &["checkout", "-b", "main"]);
        git(&base, &["config", "user.email", "ai@example.test"]);
        git(&base, &["config", "user.name", "AI Test"]);
        fs::write(base.join("README.md"), "seed\n").unwrap();
        git(&base, &["add", "README.md"]);
        git(&base, &["commit", "-m", "seed"]);
        git(&base, &["push", "-u", "origin", "main"]);
        fs::create_dir_all(active.parent().unwrap()).unwrap();
        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                "ai/session/project-a/conversation-a",
                path(&active),
                "HEAD",
            ],
        );
        let head = output(&active, &["rev-parse", "HEAD"]);
        git(&base, &["worktree", "remove", path(&active)]);

        let parent = task_record(&base, &active);
        let contract = SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.to_string(),
            supervisor: "codex_desktop".to_string(),
            task_role: "resume_original".to_string(),
            parent_task_id: Some(parent.task_id.clone()),
            root_task_id: Some("root-1".to_string()),
            acceptance_criteria: Vec::new(),
            improvement_policy: "after_task_only".to_string(),
        };
        let mut receipt = UpdateRecoveryReceipt::planned("update-1", "root-1", &parent.task_id);
        receipt.workspace = WorkspaceGitFingerprint {
            workspace_path: active.to_string_lossy().to_string(),
            git_head: Some(head.clone()),
            git_status_sha256: Some("clean-status".to_string()),
            git_status_clean: Some(true),
        };
        Self {
            root,
            base,
            active,
            head,
            parent,
            contract,
            receipt,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn clean_landed_receipt_recreates_recycled_worktree() {
    let fixture = Fixture::new();

    let resolved = resolve_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        path(&fixture.base),
        Some(&fixture.receipt),
        ResumeWorkspaceMode::Acquire,
    )
    .unwrap();

    assert!(fixture.active.is_dir());
    assert_eq!(resolved.git_head, fixture.head);
    assert_eq!(resolved.derivation, "platform_receipt_commit_rebuilt");
    assert!(!resolved.requires_recreation);
    assert_eq!(
        output(&fixture.active, &["branch", "--show-current"]),
        "ai/session/project-a/conversation-a"
    );
}

#[test]
fn inspect_reports_rebuild_without_mutating_path() {
    let fixture = Fixture::new();

    let resolved = resolve_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        path(&fixture.active),
        Some(&fixture.receipt),
        ResumeWorkspaceMode::Inspect,
    )
    .unwrap();

    assert!(resolved.requires_recreation);
    assert!(!fixture.active.exists());
}

#[test]
fn recycled_resume_of_resume_keeps_the_recorded_inherited_identity() {
    let mut fixture = Fixture::new();
    fixture.parent.conversation_id = "offline-resume-child".to_string();
    let parent_contract = SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "resume_original".to_string(),
        parent_task_id: Some("original-parent".to_string()),
        root_task_id: Some("root-1".to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: "after_task_only".to_string(),
    };

    let resolved = resolve_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&parent_contract),
        None,
        "project-a",
        path(&fixture.base),
        Some(&fixture.receipt),
        ResumeWorkspaceMode::Inspect,
    )
    .expect("a trusted receipt should preserve the inherited branch across generations");

    assert_eq!(
        resolved.inherited_workspace.branch.as_deref(),
        Some("ai/session/project-a/conversation-a")
    );
    assert_eq!(
        resolved.derivation,
        "platform_receipt_commit_rebuild_available"
    );
    assert!(resolved.requires_recreation);
    assert!(!fixture.active.exists());
}

#[test]
fn remote_v1_receipt_stays_fail_closed() {
    let mut fixture = Fixture::new();
    fixture.receipt.transport = RecoveryTransport::remote_v1();

    let error = resolve_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        path(&fixture.base),
        Some(&fixture.receipt),
        ResumeWorkspaceMode::Inspect,
    )
    .unwrap_err();

    assert!(error.to_string().contains("existing worktree unavailable"));
    assert!(format!("{error:#}").contains("not trusted"));
    assert!(!fixture.active.exists());
}

#[test]
fn rebuild_rejects_dirty_evidence_arbitrary_path_and_identity_drift() {
    let mut fixture = Fixture::new();
    fixture.receipt.workspace.git_status_clean = Some(false);
    assert!(resolve_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        path(&fixture.base),
        Some(&fixture.receipt),
        ResumeWorkspaceMode::Inspect,
    )
    .is_err());

    fixture.receipt.workspace.git_status_clean = Some(true);
    assert!(resolve_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        path(&fixture.root),
        Some(&fixture.receipt),
        ResumeWorkspaceMode::Inspect,
    )
    .is_err());

    fixture.receipt.root_task_id = "another-root".to_string();
    assert!(resolve_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        path(&fixture.base),
        Some(&fixture.receipt),
        ResumeWorkspaceMode::Inspect,
    )
    .is_err());
}

#[test]
fn rebuild_rejects_branch_occupied_by_another_worktree() {
    let fixture = Fixture::new();
    let occupied = fixture.root.join("occupied");
    git(
        &fixture.base,
        &[
            "worktree",
            "add",
            path(&occupied),
            "ai/session/project-a/conversation-a",
        ],
    );

    let error = resolve_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        path(&fixture.base),
        Some(&fixture.receipt),
        ResumeWorkspaceMode::Inspect,
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("occupied"));
    assert!(!fixture.active.exists());
}

fn task_record(base: &Path, active: &Path) -> LocalTaskRecord {
    LocalTaskRecord {
        task_id: "parent-1".to_string(),
        owner_user_id: "owner-1".to_string(),
        agent_id: "agent-1".to_string(),
        install_id: "install-1".to_string(),
        project_id: "project-a".to_string(),
        channel_id: None,
        conversation_id: "conversation-a".to_string(),
        workspace_path: base.to_string_lossy().to_string(),
        prompt: "task".to_string(),
        cli: "codex".to_string(),
        runtime_permission: "full_access".to_string(),
        execution_origin: "local_offline".to_string(),
        billing_source: "own_codex".to_string(),
        status: "done".to_string(),
        error: None,
        final_reply: Some("done".to_string()),
        model: None,
        codex_session_id: None,
        input_tokens: None,
        cached_input_tokens: None,
        output_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        workspace_status: Some(json!({
            "base_workspace_path": base.to_string_lossy(),
            "active_workspace_path": active.to_string_lossy(),
            "isolated": true,
            "branch": "ai/session/project-a/conversation-a",
        })),
        sync_state: "local_only".to_string(),
        completion_event_id: Some("event-1".to_string()),
        started_at_ms: 1,
        finished_at_ms: Some(2),
        server_ack_at_ms: None,
    }
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn output(cwd: &Path, args: &[&str]) -> String {
    let output = git_command().args(args).current_dir(cwd).output().unwrap();
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git(cwd: &Path, args: &[&str]) {
    let output = git_command().args(args).current_dir(cwd).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
