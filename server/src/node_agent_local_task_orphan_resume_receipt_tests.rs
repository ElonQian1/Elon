use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    node_agent_cli_sidecar::CliSidecarSessionRecord,
    node_agent_local_task_resume::{resolve_resume_workspace, ResumeWorkspaceMode},
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStart, LocalTaskStore},
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    node_agent_task_journal::TaskJournalStart,
    node_agent_update_recovery::{UpdateRecoveryReceipt, UpdateRecoveryState},
    NodeRuntime,
};

use super::super::reconcile_with_stale_after;

const TASK: &str = "local-terminal-head-advance";
const PROJECT: &str = "project";
const CONVERSATION: &str = "conversation";
const BRANCH: &str = "ai/session/project/conversation";

#[tokio::test]
async fn orphan_terminal_commit_persists_resume_proof_and_keeps_fail_closed_guards() {
    let fixture = Fixture::new();
    let recorded_head = git_output(&fixture.active, &["rev-parse", "HEAD"]);
    fixture.create_supervised_task(&recorded_head);
    let final_head = fixture.commit_and_push("terminal task commit");

    std::thread::sleep(Duration::from_millis(2));
    assert_eq!(
        reconcile_with_stale_after(&fixture.runtime, 0)
            .await
            .unwrap(),
        1
    );

    let parent = fixture.runtime.local_tasks.get(TASK).unwrap().unwrap();
    let journal = fixture.runtime.task_journal.record(TASK).unwrap().unwrap();
    let receipt = fixture
        .runtime
        .update_recovery
        .receipt_for_resume_parent(&parent)
        .unwrap()
        .expect("orphan terminal transition must persist a task-bound receipt");

    assert_eq!(parent.status, "resume_required");
    assert!(
        crate::node_agent_local_task_store::is_orphan_runtime_resume_required_reason(
            parent.error.as_deref()
        )
    );
    assert_eq!(journal.status, "resume_required");
    assert_eq!(receipt.state, UpdateRecoveryState::Applying);
    assert_eq!(receipt.active_task_id(), TASK);
    assert_eq!(
        receipt.workspace.git_head.as_deref(),
        Some(final_head.as_str())
    );
    assert_eq!(receipt.workspace.git_status_clean, Some(true));
    assert!(receipt.safety.evidence_complete);
    assert!(receipt.safety.pending_approval_ids.is_empty());
    assert!(receipt.safety.non_repeatable_action.is_none());

    let resolved = resolve(&fixture, &parent, &journal, Some(&receipt))
        .expect("the exact clean terminal successor on origin/main must resume");
    assert_eq!(resolved.git_head, final_head);

    let missing = resolve(&fixture, &parent, &journal, None)
        .expect_err("HEAD advance without its durable receipt must fail closed");
    assert!(format!("{missing:#}").contains("缺少可验证的终态更新恢复回执"));

    let mut journal_drift = journal.clone();
    journal_drift.status = "running".to_string();
    let _drift = resolve(&fixture, &parent, &journal_drift, Some(&receipt))
        .expect_err("terminal row and journal drift must fail closed");

    let mut approval = receipt.clone();
    approval
        .safety
        .pending_approval_ids
        .push("approval-pending".to_string());
    let _approval_error = resolve(&fixture, &parent, &journal, Some(&approval))
        .expect_err("pending approval must fail closed");

    let mut branch_drift = receipt.clone();
    branch_drift.workspace.branch = Some("ai/session/project/other".to_string());
    let _branch_error = resolve(&fixture, &parent, &journal, Some(&branch_drift))
        .expect_err("receipt branch drift must fail closed");

    fs::write(fixture.active.join("untracked.txt"), "dirty\n").unwrap();
    let _dirty = resolve(&fixture, &parent, &journal, Some(&receipt))
        .expect_err("dirty terminal worktree must fail closed");
    fs::remove_file(fixture.active.join("untracked.txt")).unwrap();

    fs::write(fixture.active.join("README.md"), "unpushed successor\n").unwrap();
    git(&fixture.active, &["add", "README.md"]);
    git(&fixture.active, &["commit", "-m", "unpushed successor"]);
    let unpushed_head = git_output(&fixture.active, &["rev-parse", "HEAD"]);
    let mut unpushed = receipt.clone();
    unpushed.workspace.git_head = Some(unpushed_head);
    let _not_landed = resolve(&fixture, &parent, &journal, Some(&unpushed))
        .expect_err("a successor outside origin/main must fail closed");

    git(&fixture.active, &["reset", "--hard", &recorded_head]);
    fs::write(fixture.active.join("README.md"), "divergent successor\n").unwrap();
    git(&fixture.active, &["add", "README.md"]);
    git(&fixture.active, &["commit", "-m", "divergent successor"]);
    let divergent_head = git_output(&fixture.active, &["rev-parse", "HEAD"]);
    let mut divergent_receipt = receipt.clone();
    divergent_receipt.workspace.git_head = Some(divergent_head);
    let mut divergent_parent = parent.clone();
    divergent_parent.workspace_status.as_mut().unwrap()["git_head"] =
        serde_json::Value::String(final_head);
    let _non_ancestor = resolve(
        &fixture,
        &divergent_parent,
        &journal,
        Some(&divergent_receipt),
    )
    .expect_err("a non-ancestor HEAD must fail closed");
}

fn resolve(
    fixture: &Fixture,
    parent: &LocalTaskRecord,
    journal: &crate::node_agent_task_journal::TaskJournalRecord,
    receipt: Option<&UpdateRecoveryReceipt>,
) -> anyhow::Result<crate::node_agent_local_task_resume::ResolvedResumeWorkspace> {
    resolve_resume_workspace(
        &resume_contract(),
        parent,
        Some(&requirement_contract()),
        Some(journal),
        PROJECT,
        fixture.base.to_string_lossy().as_ref(),
        receipt,
        ResumeWorkspaceMode::Inspect,
    )
}

struct Fixture {
    root: PathBuf,
    base: PathBuf,
    active: PathBuf,
    runtime: NodeRuntime,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "elon-orphan-terminal-head-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let remote = root.join("remote.git");
        let base = root.join("base");
        let active = root
            .join("conversation-worktrees")
            .join(PROJECT)
            .join(CONVERSATION);
        fs::create_dir_all(active.parent().unwrap()).unwrap();
        git(
            &root,
            &["init", "--bare", remote.to_string_lossy().as_ref()],
        );
        git(
            &root,
            &["init", "-b", "main", base.to_string_lossy().as_ref()],
        );
        git(&base, &["config", "user.email", "ai@example.test"]);
        git(&base, &["config", "user.name", "AI Test"]);
        fs::write(base.join("README.md"), "seed\n").unwrap();
        git(&base, &["add", "README.md"]);
        git(&base, &["commit", "-m", "seed"]);
        git(
            &base,
            &["remote", "add", "origin", remote.to_string_lossy().as_ref()],
        );
        git(&base, &["push", "-u", "origin", "main"]);
        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                BRANCH,
                active.to_string_lossy().as_ref(),
            ],
        );
        git(&active, &["config", "user.email", "ai@example.test"]);
        git(&active, &["config", "user.name", "AI Test"]);
        crate::node_agent_supervision_worktree_lease::acquire(&base, &active, TASK).unwrap();

        let mut runtime = test_runtime(&root);
        runtime.cli_sidecars =
            crate::node_agent_cli_sidecar::CliSidecarRegistry::new(root.join("sidecars"));
        Self {
            root,
            base,
            active,
            runtime,
        }
    }

    fn create_supervised_task(&self, recorded_head: &str) {
        self.runtime
            .local_tasks
            .create(LocalTaskStart {
                task_id: TASK,
                owner_user_id: "owner",
                agent_id: "agent",
                install_id: "install",
                project_id: PROJECT,
                channel_id: None,
                conversation_id: CONVERSATION,
                workspace_path: self.active.to_string_lossy().as_ref(),
                prompt: "work",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        let common = git_output(
            &self.active,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        );
        let origin = git_output(&self.base, &["config", "--get", "remote.origin.url"]);
        self.runtime
            .local_tasks
            .record_initial_workspace_status(
                TASK,
                &serde_json::json!({
                    "platform_provenance": "elon.conversation_worktree.v1",
                    "root_task_id": TASK,
                    "project_id": PROJECT,
                    "base_workspace_path": self.base,
                    "active_workspace_path": self.active,
                    "isolated": true,
                    "branch": BRANCH,
                    "git_head": recorded_head,
                    "git_common_dir": common,
                    "git_remote": origin,
                }),
            )
            .unwrap();
        crate::node_agent_local_task_supervision::record_supervision_event(
            &self.runtime.task_journal,
            TASK,
            "supervision_contract",
            crate::node_agent_local_task_supervision::contract_payload(&requirement_contract()),
        )
        .unwrap();
        self.runtime
            .task_journal
            .record_started(TaskJournalStart {
                req_id: TASK,
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some(TASK),
                cwd: self.active.to_str(),
                runtime_permission: Some("full_access"),
            })
            .unwrap();
        self.runtime
            .cli_sidecars
            .upsert_session(CliSidecarSessionRecord::managed_pipe_json(
                "sidecar-terminal-head",
                TASK,
                "codex",
                "route_a_external_cli",
                Some(self.active.to_string_lossy().to_string()),
                None,
                None,
                None,
                1,
            ))
            .unwrap();
    }

    fn commit_and_push(&self, content: &str) -> String {
        fs::write(self.active.join("README.md"), format!("{content}\n")).unwrap();
        git(&self.active, &["add", "README.md"]);
        git(&self.active, &["commit", "-m", content]);
        git(&self.active, &["push", "origin", "HEAD:main"]);
        git_output(&self.active, &["rev-parse", "HEAD"])
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ =
            crate::node_agent_supervision_worktree_lease::release(&self.base, &self.active, TASK);
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn requirement_contract() -> SupervisionContract {
    SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "requirement".to_string(),
        parent_task_id: None,
        root_task_id: Some(TASK.to_string()),
        acceptance_criteria: Vec::new(),
        improvement_policy: "after_task_only".to_string(),
    }
}

fn resume_contract() -> SupervisionContract {
    SupervisionContract {
        task_role: "resume_original".to_string(),
        parent_task_id: Some(TASK.to_string()),
        ..requirement_contract()
    }
}

fn test_runtime(root: &Path) -> NodeRuntime {
    let mut runtime = NodeRuntime::new(
        crate::node_agent_config::NodeConfig {
            cloud_url: "ws://127.0.0.1".into(),
            cloud_http_url: "http://127.0.0.1".into(),
            ollama_url: "http://127.0.0.1".into(),
            lm_studio_url: None,
            custom_url: None,
            price_per_1k: 0.0,
        },
        Some(crate::node_agent_config::Credentials {
            agent_id: "agent".into(),
            agent_secret: "unused".into(),
            owner_user_id: "owner".into(),
            user_token: None,
        }),
        crate::pc_storage_repo::StorageSettings::default(),
        crate::node_agent_data_root::resolve(None, None, None),
        "install".into(),
    );
    runtime.local_tasks = LocalTaskStore::new(root.join("tasks.sqlite3"));
    runtime.task_journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    runtime.completion_outbox =
        crate::node_agent_completion_outbox::CliCompletionOutbox::new(root.join("outbox.sqlite3"));
    runtime.update_recovery =
        crate::node_agent_update_recovery::UpdateRecoveryStore::new(root.join("recovery.json"));
    runtime
}

fn git(cwd: &Path, args: &[&str]) {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(cwd: &Path, args: &[&str]) -> String {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
