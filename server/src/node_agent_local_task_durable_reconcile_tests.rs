use std::{fs, path::Path};

use super::*;
use crate::{
    git_command_error::git_command,
    node_agent_full_access::FullAccessGrant,
    node_agent_local_task_store::{reconcile::RecoveredLocalTaskStart, LocalTaskStore},
    node_agent_update_recovery::WorkspaceGitFingerprint,
};

struct GitFixture {
    root: PathBuf,
    base: PathBuf,
    active: PathBuf,
    branch: String,
}

impl GitFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "durable-task-reconcile-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let base = root.join("base");
        let active = root
            .join("conversation-worktrees")
            .join("elon-self")
            .join("conversation-a");
        fs::create_dir_all(&base).unwrap();
        run(&base, &["init"]);
        run(&base, &["config", "user.email", "ai@example.test"]);
        run(&base, &["config", "user.name", "AI Test"]);
        run(
            &base,
            &[
                "config",
                "remote.origin.url",
                "https://example.test/elon.git",
            ],
        );
        fs::write(base.join("seed.txt"), "seed\n").unwrap();
        run(&base, &["add", "seed.txt"]);
        run(&base, &["commit", "-m", "seed"]);
        let branch = "ai/session/elon-self/conversation-a".to_string();
        run(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                active.to_str().unwrap(),
                "HEAD",
            ],
        );
        crate::node_agent_supervision_worktree_lease::acquire(&base, &active, "local-root")
            .unwrap();
        Self {
            root,
            base,
            active,
            branch,
        }
    }

    fn session(&self) -> CliSidecarSessionRecord {
        CliSidecarSessionRecord::managed_pipe_json(
            "sidecar-a",
            "local-root",
            "codex",
            "local_offline",
            Some(self.active.to_string_lossy().to_string()),
            Some(self.root.join("output.jsonl").to_string_lossy().to_string()),
            None,
            None,
            10,
        )
    }

    fn receipt(&self) -> UpdateRecoveryReceipt {
        let mut receipt = UpdateRecoveryReceipt::planned("update-a", "local-root", "local-root");
        receipt.sidecar_session_id = Some("sidecar-a".to_string());
        receipt.workspace = WorkspaceGitFingerprint {
            base_workspace_path: Some(self.base.to_string_lossy().to_string()),
            workspace_path: self.active.to_string_lossy().to_string(),
            isolated: true,
            branch: Some(self.branch.clone()),
            git_head: Some(output(&self.active, &["rev-parse", "HEAD"])),
            git_status_sha256: Some("checkpoint-status-sha".to_string()),
            git_status_clean: Some(true),
        };
        receipt
    }

    fn grant(&self) -> FullAccessGrant {
        FullAccessGrant {
            owner_user_id: "owner".to_string(),
            agent_id: "agent".to_string(),
            install_id: "install".to_string(),
            project_id: "elon-self".to_string(),
            workspace_path: self.base.to_string_lossy().to_string(),
            granted_at_ms: 1,
        }
    }
}

impl Drop for GitFixture {
    fn drop(&mut self) {
        let _ = crate::node_agent_supervision_worktree_lease::release(
            &self.base,
            &self.active,
            "local-root",
        );
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn trusted_contract_receipt_grant_workspace_and_root_lease_rebuild_identity() {
    let fixture = GitFixture::new();
    let identity = validate_workspace_identity(
        &fixture.session(),
        &fixture.receipt(),
        "local-root",
        &[fixture.grant()],
    )
    .unwrap();
    assert_eq!(identity.project_id, "elon-self");
    assert_eq!(identity.conversation_id, "conversation-a");
    assert_eq!(
        identity.status["durable_reconcile_provenance"],
        "journal_contract_workspace_grant_root_lease"
    );
    assert_eq!(identity.status["root_task_id"], "local-root");
}

#[test]
fn sidecar_cannot_claim_foreign_project_or_wrong_root_lease() {
    let fixture = GitFixture::new();
    let mut foreign = fixture.grant();
    foreign.project_id = "foreign".to_string();
    assert!(validate_workspace_identity(
        &fixture.session(),
        &fixture.receipt(),
        "local-root",
        &[foreign],
    )
    .unwrap_err()
    .to_string()
    .contains("not authorized"));

    crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "local-root",
    )
    .unwrap();
    run(
        &fixture.base,
        &[
            "worktree",
            "lock",
            "--reason",
            "elon-supervision:wrong-root",
            fixture.active.to_str().unwrap(),
        ],
    );
    assert!(validate_workspace_identity(
        &fixture.session(),
        &fixture.receipt(),
        "local-root",
        &[fixture.grant()],
    )
    .unwrap_err()
    .to_string()
    .contains("root supervision lease"));
}

#[test]
fn atomic_reconcile_is_idempotent_and_rejects_identity_takeover() {
    let fixture = GitFixture::new();
    let store = LocalTaskStore::new(fixture.root.join("tasks.sqlite3"));
    let identity = validate_workspace_identity(
        &fixture.session(),
        &fixture.receipt(),
        "local-root",
        &[fixture.grant()],
    )
    .unwrap();
    let insert = |owner: &'static str| RecoveredLocalTaskStart {
        task_id: "local-root",
        owner_user_id: owner,
        agent_id: "agent",
        install_id: "install",
        project_id: "elon-self",
        conversation_id: "conversation-a",
        workspace_path: fixture.active.to_str().unwrap(),
        prompt: RECOVERED_PROMPT,
        cli: "codex",
        runtime_permission: "full_access",
        status: "recovering",
        error: "recovered",
        workspace_status: &identity.status,
        started_at_ms: 1,
    };
    assert_eq!(
        store
            .reconcile_missing_supervised(insert("owner"))
            .unwrap()
            .status,
        "recovering"
    );
    store.reconcile_missing_supervised(insert("owner")).unwrap();
    assert!(store
        .reconcile_missing_supervised(insert("foreign-owner"))
        .unwrap_err()
        .to_string()
        .contains("different durable identity"));
}

#[test]
fn contract_and_receipt_root_must_be_immutable() {
    let contract = SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "requirement".to_string(),
        parent_task_id: None,
        root_task_id: None,
        acceptance_criteria: vec![],
        improvement_policy: "observe_only".to_string(),
    };
    let receipt = UpdateRecoveryReceipt::planned("update", "local-root", "local-root");
    assert_eq!(
        validate_contract_receipt("local-root", &contract, &receipt).unwrap(),
        "local-root"
    );
    let wrong = UpdateRecoveryReceipt::planned("update", "wrong-root", "local-root");
    assert!(validate_contract_receipt("local-root", &contract, &wrong).is_err());
}

fn run(cwd: &Path, args: &[&str]) {
    let output = git_command().args(args).current_dir(cwd).output().unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn output(cwd: &Path, args: &[&str]) -> String {
    let output = git_command().args(args).current_dir(cwd).output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
