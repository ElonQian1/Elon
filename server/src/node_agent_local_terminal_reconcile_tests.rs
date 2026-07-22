use std::{
    fs,
    path::{Path, PathBuf},
};

use homecli_proto::{CliCompletionEnvelope, CliCompletionProducerIdentity, CliProjectContext};
use sha2::{Digest, Sha256};

use super::*;
use crate::{
    node_agent_local_task_store::{LocalTaskStart, LocalTaskStore},
    node_agent_local_task_supervision::{record_supervision_event, SupervisionContract},
    node_agent_update_recovery::{UpdateRecoveryReceipt, UpdateRecoveryState},
};

const TASK: &str = "supervised-terminal-task";
const ROOT: &str = "supervised-terminal-root";
const EVENT: &str = "supervised-terminal-event";

#[tokio::test]
async fn completed_done_with_missing_lease_persists_trusted_snapshot_and_replays() {
    let fixture = Fixture::new("done").await;
    fixture.write_completed_receipt();
    fixture.unlock();
    let completion = fixture.completion(true, None);
    fixture
        .runtime
        .completion_outbox
        .enqueue(&completion)
        .unwrap();

    fixture.reconcile(&completion).await.unwrap();
    let bound = fs::read(fixture.receipt_path()).unwrap();
    fixture.reconcile(&completion).await.unwrap();
    assert_eq!(fs::read(fixture.receipt_path()).unwrap(), bound);
    let task = fixture.task();
    assert_eq!(task.status, "done");
    assert_eq!(task.completion_event_id.as_deref(), Some(EVENT));
    assert_eq!(
        task.workspace_status.as_ref().unwrap()["terminal_snapshot_status"],
        "trusted"
    );
    assert_eq!(
        task.workspace_status.as_ref().unwrap()["git_head"],
        fixture.head()
    );
    assert_eq!(
        fixture.runtime.completion_outbox.pending_count().unwrap(),
        1
    );
}

#[tokio::test]
async fn failed_and_canceled_require_receipt_null_and_exact_lease() {
    for (label, error, expected) in [
        ("failed", "business failure", "failed"),
        ("canceled", "用户已停止 PC CLI 任务", "canceled"),
    ] {
        let fixture = Fixture::new(label).await;
        let completion = fixture.completion(false, Some(error));
        fixture
            .runtime
            .completion_outbox
            .enqueue(&completion)
            .unwrap();
        fixture.reconcile(&completion).await.unwrap();
        assert_eq!(fixture.task().status, expected);
        assert!(!fixture.receipt_path().exists());
        assert_eq!(
            fixture.lease().as_deref(),
            Some(format!("elon-supervision:{ROOT}").as_str())
        );
        assert_eq!(
            fixture.runtime.completion_outbox.pending_count().unwrap(),
            1
        );
    }
}

#[tokio::test]
async fn done_rejects_reacquired_lease_without_mutating_any_terminal_state() {
    let fixture = Fixture::new("reacquired").await;
    fixture.write_completed_receipt();
    fixture.unlock();
    crate::node_agent_supervision_worktree_lease::acquire(&fixture.base, &fixture.active, ROOT)
        .unwrap();
    let completion = fixture.completion(true, None);
    fixture
        .runtime
        .completion_outbox
        .enqueue(&completion)
        .unwrap();
    let before = fs::read(fixture.receipt_path()).unwrap();

    let error = fixture
        .reconcile(&completion)
        .await
        .expect_err("reacquire must fail");
    assert!(format!("{error:#}").contains("retained or reacquired"));
    assert_eq!(fs::read(fixture.receipt_path()).unwrap(), before);
    assert_eq!(fixture.task().status, "running");
    assert_eq!(
        fixture.runtime.completion_outbox.pending_count().unwrap(),
        1
    );
}

#[tokio::test]
async fn recovery_missing_multiple_and_conflict_fail_before_all_terminal_writes() {
    let missing = Fixture::new("recovery-missing").await;
    missing.write_completed_receipt();
    missing.unlock();
    missing
        .runtime
        .local_tasks
        .mark_recovering(TASK, "fixture recovery")
        .unwrap();
    missing
        .assert_failed_unchanged(missing.completion(true, None), "no unique durable")
        .await;

    let multiple = Fixture::new("recovery-multiple").await;
    for update in ["update-a", "update-b"] {
        multiple.install_recovery(update, None);
    }
    multiple
        .assert_failed_unchanged(multiple.completion(false, Some("failed")), "multiple")
        .await;

    let conflict = Fixture::new("recovery-conflict").await;
    conflict.install_recovery("update-conflict", Some("other-event"));
    conflict
        .assert_failed_unchanged(conflict.completion(false, Some("failed")), "conflicts")
        .await;
}

#[tokio::test]
async fn workspace_identity_and_receipt_conflicts_fail_closed() {
    let workspace = Fixture::new("workspace-drift").await;
    workspace.write_completed_receipt();
    workspace.unlock();
    let mut status = workspace.task().workspace_status.unwrap();
    status["active_workspace_path"] = serde_json::json!(workspace.base);
    workspace
        .runtime
        .local_tasks
        .replace_workspace_status_for_test(TASK, &status)
        .unwrap();
    workspace
        .assert_failed_unchanged(workspace.completion(true, None), "shared base workspace")
        .await;

    let receipt = Fixture::new("receipt-conflict").await;
    receipt.write_completed_receipt();
    receipt.unlock();
    let path = receipt.receipt_path();
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["taskId"] = serde_json::json!(TASK);
    value["completionEventId"] = serde_json::json!(EVENT);
    value["terminalStatus"] = serde_json::json!("failed");
    value["boundAtUtc"] = serde_json::json!("2026-07-22T01:02:00Z");
    crate::node_agent_atomic_file::write(&path, &serde_json::to_vec(&value).unwrap()).unwrap();
    receipt
        .assert_failed_unchanged(receipt.completion(true, None), "bind only done")
        .await;
}

struct Fixture {
    root: PathBuf,
    base: PathBuf,
    active: PathBuf,
    contracts: PathBuf,
    branch: String,
    runtime: NodeRuntime,
}

impl Fixture {
    async fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "elon-supervised-terminal-{label}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let origin = root.join("origin.git");
        let base = root.join("base");
        let conversation = format!("terminal-{label}");
        let branch = format!("ai/session/elon-self/{conversation}");
        let active = root
            .join("conversation-worktrees/elon-self")
            .join(&conversation);
        let contracts = root.join("contracts");
        fs::create_dir_all(&contracts).unwrap();
        git(&root, &["init", "--bare", origin.to_str().unwrap()]);
        git(&root, &["init", "-b", "main", base.to_str().unwrap()]);
        git(&base, &["config", "user.email", "ai@example.test"]);
        git(&base, &["config", "user.name", "AI Test"]);
        fs::write(base.join("README.md"), "seed\n").unwrap();
        git(&base, &["add", "README.md"]);
        git(&base, &["commit", "-m", "seed"]);
        git(
            &base,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        git(&base, &["push", "-u", "origin", "main"]);
        fs::create_dir_all(active.parent().unwrap()).unwrap();
        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                active.to_str().unwrap(),
                "origin/main",
            ],
        );
        crate::node_agent_supervision_worktree_lease::acquire(&base, &active, ROOT).unwrap();

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
        runtime.task_journal =
            crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
        runtime.completion_outbox = crate::node_agent_completion_outbox::CliCompletionOutbox::new(
            root.join("outbox.sqlite3"),
        );
        runtime.update_recovery =
            crate::node_agent_update_recovery::UpdateRecoveryStore::new(root.join("recovery.json"));
        runtime.full_access_grants =
            crate::node_agent_full_access::FullAccessGrantState::load_from_path(
                root.join("grants.json"),
            );
        let grant_identity = crate::node_agent_full_access::FullAccessGrantIdentity::new(
            "owner", "agent", "install",
        )
        .unwrap();
        runtime
            .full_access_grants
            .grant_project(&grant_identity, "elon-self", base.to_str().unwrap())
            .await
            .unwrap();
        runtime
            .local_tasks
            .create(LocalTaskStart {
                task_id: TASK,
                owner_user_id: "owner",
                agent_id: "agent",
                install_id: "install",
                project_id: "elon-self",
                channel_id: None,
                conversation_id: &conversation,
                workspace_path: active.to_str().unwrap(),
                prompt: "finish",
                cli: "codex",
                runtime_permission: "full_access",
            })
            .unwrap();
        let status = serde_json::json!({
            "platform_provenance":"elon.conversation_worktree.v1", "root_task_id":ROOT,
            "active_workspace_path":active, "base_workspace_path":base,
            "project_id":"elon-self", "isolated":true, "branch":branch,
            "git_common_dir":git_output(&active, &["rev-parse", "--path-format=absolute", "--git-common-dir"]),
            "git_remote":git_output(&active, &["remote", "get-url", "origin"]),
        });
        runtime
            .local_tasks
            .record_initial_workspace_status(TASK, &status)
            .unwrap();
        let contract = SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: "capability_repair".into(),
            parent_task_id: Some("parent".into()),
            root_task_id: Some(ROOT.into()),
            acceptance_criteria: vec![],
            improvement_policy: "after_task_only".into(),
        };
        record_supervision_event(
            &runtime.task_journal,
            TASK,
            "supervision_contract",
            crate::node_agent_local_task_supervision::contract_payload(&contract),
        )
        .unwrap();
        Self {
            root,
            base,
            active,
            contracts,
            branch,
            runtime,
        }
    }

    async fn reconcile(&self, completion: &CliCompletionEnvelope) -> anyhow::Result<()> {
        LocalTerminalReconciler::for_test(&self.runtime, self.contracts.clone())
            .reconcile(completion)
            .await
    }

    async fn assert_failed_unchanged(&self, completion: CliCompletionEnvelope, needle: &str) {
        let before_task = self.task();
        let before_receipt = fs::read(self.receipt_path()).ok();
        let recovery_path = self.root.join("recovery.json");
        let before_recovery = fs::read(&recovery_path).ok();
        let before_journal = serde_json::to_value(
            self.runtime
                .task_journal
                .snapshot(TASK, 0, 200)
                .unwrap()
                .events,
        )
        .unwrap();
        let before_outbox = self.runtime.completion_outbox.pending_count().unwrap();
        self.runtime.completion_outbox.enqueue(&completion).unwrap();
        let error = self
            .reconcile(&completion)
            .await
            .expect_err("preflight must fail");
        assert!(
            format!("{error:#}").contains(needle),
            "unexpected error: {error:#}"
        );
        let after_task = self.task();
        assert_eq!(after_task.status, before_task.status);
        assert_eq!(after_task.error, before_task.error);
        assert_eq!(after_task.final_reply, before_task.final_reply);
        assert_eq!(
            after_task.completion_event_id,
            before_task.completion_event_id
        );
        assert_eq!(after_task.finished_at_ms, before_task.finished_at_ms);
        assert_eq!(after_task.workspace_status, before_task.workspace_status);
        assert_eq!(fs::read(self.receipt_path()).ok(), before_receipt);
        assert_eq!(fs::read(&recovery_path).ok(), before_recovery);
        assert_eq!(
            serde_json::to_value(
                self.runtime
                    .task_journal
                    .snapshot(TASK, 0, 200)
                    .unwrap()
                    .events
            )
            .unwrap(),
            before_journal
        );
        assert_eq!(
            self.runtime.completion_outbox.pending_count().unwrap(),
            before_outbox + 1
        );
    }

    fn install_recovery(&self, update: &str, bound_event: Option<&str>) {
        let mut receipt = UpdateRecoveryReceipt::planned(update, ROOT, "parent");
        receipt.resume_task_id = Some(TASK.into());
        receipt.state = UpdateRecoveryState::Resumed;
        if let Some(event) = bound_event {
            receipt.completion_event_id = Some(event.into());
            receipt.terminal_task_status = Some("failed".into());
            receipt.terminal_finished_at_ms = Some(20);
            receipt.terminal_success = Some(false);
            receipt.terminal_outcome = Some("failed".into());
        }
        self.runtime.update_recovery.upsert(receipt).unwrap();
    }

    fn completion(&self, exit_ok: bool, error: Option<&str>) -> CliCompletionEnvelope {
        CliCompletionEnvelope {
            event_id: EVENT.into(),
            req_id: TASK.into(),
            cli: "codex".into(),
            origin: crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN.into(),
            producer_identity: Some(CliCompletionProducerIdentity {
                owner_user_id: "owner".into(),
                agent_id: "agent".into(),
                install_id: "install".into(),
            }),
            project_context: Some(CliProjectContext {
                project_id: "elon-self".into(),
                conversation_id: self
                    .active
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                runtime_permission: Some("full_access".into()),
            }),
            channel_id: None,
            prompt: Some("finish".into()),
            final_output: "terminal output".into(),
            exit_ok,
            error: error.map(str::to_string),
            session_id: Some("session".into()),
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            model: None,
            workspace_status: None,
            created_at_ms: 20,
        }
    }

    fn write_completed_receipt(&self) {
        let contract = serde_json::json!({
            "schema":"elon.ai_finish_contract.v1", "worktree":normalized(&self.active),
            "branch":self.branch, "baseCommit":self.head(),
            "origin":git_output(&self.active, &["remote", "get-url", "origin"]),
            "issuedAtUtc":"2026-07-22T01:00:00Z", "nonce":"d".repeat(32),
            "platformProvenance":"elon.conversation_worktree.v1",
            "supervisionRootTaskId":ROOT, "leaseReason":format!("elon-supervision:{ROOT}"),
            "gitCommonDir":normalized(Path::new(&self.git_common())),
        });
        let bytes = serde_json::to_vec(&contract).unwrap();
        let id = format!("{:x}", Sha256::digest(&bytes));
        fs::write(self.contracts.join(format!("{id}.json")), bytes).unwrap();
        let git_dir = PathBuf::from(git_output(
            &self.active,
            &["rev-parse", "--path-format=absolute", "--git-dir"],
        ));
        let fields = format!(
            "taskContractId={id}\nsupervisionRootTaskId={ROOT}\nworktree={}\nbaseWorkspace={}\ngitDir={}\ngitCommonDir={}\nbranch={}\norigin={}\nfinalHead={}\nleaseMarkerFingerprint={}",
            normalized(&self.active), normalized(&self.base), normalized(&git_dir), normalized(Path::new(&self.git_common())),
            self.branch, git_output(&self.active, &["remote", "get-url", "origin"]), self.head(), "a".repeat(64),
        );
        let receipt = serde_json::json!({
            "schema":"elon.terminal_finalization.v1", "state":"completed", "finalizationId":"c".repeat(32),
            "taskId":null, "completionEventId":null, "terminalStatus":null, "taskContractId":id,
            "supervisionRootTaskId":ROOT, "worktree":normalized(&self.active), "baseWorkspace":normalized(&self.base),
            "gitDir":normalized(&git_dir), "gitCommonDir":normalized(Path::new(&self.git_common())), "branch":self.branch,
            "origin":git_output(&self.active, &["remote", "get-url", "origin"]), "finalHead":self.head(),
            "leaseMarkerFingerprint":"a".repeat(64), "fingerprint":format!("{:x}", Sha256::digest(fields.as_bytes())),
            "preparedAtUtc":"2026-07-22T01:00:00Z", "completedAtUtc":"2026-07-22T01:01:00Z", "boundAtUtc":null,
        });
        crate::node_agent_atomic_file::write(
            &self.receipt_path(),
            &serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
    }

    fn unlock(&self) {
        crate::node_agent_supervision_worktree_lease::release(&self.base, &self.active, ROOT)
            .unwrap();
    }
    fn lease(&self) -> Option<String> {
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(&self.base, &self.active)
            .unwrap()
    }
    fn task(&self) -> LocalTaskRecord {
        self.runtime.local_tasks.get(TASK).unwrap().unwrap()
    }
    fn head(&self) -> String {
        git_output(&self.active, &["rev-parse", "HEAD^{commit}"])
    }
    fn git_common(&self) -> String {
        git_output(
            &self.active,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
    }
    fn receipt_path(&self) -> PathBuf {
        PathBuf::from(git_output(
            &self.active,
            &["rev-parse", "--path-format=absolute", "--git-dir"],
        ))
        .join("elon-terminal-finalization-v1.json")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn normalized(path: &Path) -> String {
    path.canonicalize()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}
fn git_output(cwd: &Path, args: &[&str]) -> String {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
fn git(cwd: &Path, args: &[&str]) {
    let _ = git_output(cwd, args);
}
