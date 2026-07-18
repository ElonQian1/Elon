use std::path::{Path, PathBuf};

use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::{
    git_command_error::git_command,
    node_agent_local_task_resume::validate_resume_workspace,
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStart, LocalTaskStore},
    node_agent_local_task_supervision::{
        contract_payload, record_supervision_event, SupervisionContract,
    },
    node_agent_task_journal::TaskJournal,
};

struct MigrationFixture {
    root: PathBuf,
    base: PathBuf,
    active: PathBuf,
    store: LocalTaskStore,
    journal: TaskJournal,
    parent: LocalTaskRecord,
    parent_contract: SupervisionContract,
    resume_contract: SupervisionContract,
}

impl MigrationFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "elon-resume-lease-migration-{}",
            Uuid::new_v4().simple()
        ));
        let base = root.join("base");
        let active = root
            .join("conversation-worktrees")
            .join("project-a")
            .join("conversation-a");
        std::fs::create_dir_all(&base).unwrap();
        run_git(&base, &["init"]);
        run_git(&base, &["config", "user.email", "ai@example.test"]);
        run_git(&base, &["config", "user.name", "AI Test"]);
        std::fs::write(base.join("README.md"), "seed\n").unwrap();
        run_git(&base, &["add", "README.md"]);
        run_git(&base, &["commit", "-m", "seed"]);
        std::fs::create_dir_all(active.parent().unwrap()).unwrap();
        run_git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                "ai/session/project-a/conversation-a",
                active.to_string_lossy().as_ref(),
                "HEAD",
            ],
        );

        let store = LocalTaskStore::new(root.join("tasks.sqlite3"));
        let journal = TaskJournal::new(root.join("journal"));
        create_task(&store, "local-root", "root-conversation", &base);
        create_task(&store, "local-middle", "middle-conversation", &base);
        create_task(&store, "local-child", "conversation-a", &base);
        let root_contract = contract("requirement", None, None);
        let middle_contract = contract("capability_repair", Some("local-root"), Some("local-root"));
        let parent_contract = contract(
            "capability_repair",
            Some("local-middle"),
            Some("local-root"),
        );
        record_contract(&journal, "local-root", &root_contract);
        record_contract(&journal, "local-middle", &middle_contract);
        record_contract(&journal, "local-child", &parent_contract);

        let mut parent = store.get("local-child").unwrap().unwrap();
        parent.status = "failed".to_string();
        parent.finished_at_ms = Some(2);
        parent.workspace_status = Some(json!({
            "base_workspace_path": base.to_string_lossy(),
            "active_workspace_path": active.to_string_lossy(),
            "isolated": true,
            "branch": "ai/session/project-a/conversation-a",
            "git_head": git_output(&active, &["rev-parse", "HEAD"]),
        }));
        let resume_contract = contract("resume_original", Some("local-child"), Some("local-root"));
        crate::node_agent_supervision_worktree_lease::acquire(&base, &active, "local-child")
            .unwrap();
        Self {
            root,
            base,
            active,
            store,
            journal,
            parent,
            parent_contract,
            resume_contract,
        }
    }

    fn resolve(&self) -> crate::node_agent_local_task_resume::ResolvedResumeWorkspace {
        validate_resume_workspace(
            &self.resume_contract,
            &self.parent,
            Some(&self.parent_contract),
            None,
            "project-a",
            self.base.to_string_lossy().as_ref(),
        )
        .expect("trusted legacy child lease should become a migration candidate")
    }
}

impl Drop for MigrationFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn route_and_lease_modules_migrate_one_trusted_lineage_without_touching_dirty_state() {
    let fixture = MigrationFixture::new();
    std::fs::write(fixture.active.join("README.md"), "staged\n").unwrap();
    run_git(&fixture.active, &["add", "README.md"]);
    std::fs::write(fixture.active.join("README.md"), "staged plus unstaged\n").unwrap();
    std::fs::write(fixture.active.join("draft.txt"), "untracked\n").unwrap();
    let staged = git_output(&fixture.active, &["diff", "--cached"]);
    let unstaged = git_output(&fixture.active, &["diff"]);

    let resolved = fixture.resolve();
    let migration = resolved.lease_migration.as_ref().expect("migration marker");
    assert_eq!(migration.legacy_task_id, "local-child");
    assert_eq!(migration.root_task_id, "local-root");
    validate_full_lineage(
        &fixture.store,
        &fixture.journal,
        &fixture.parent,
        &fixture.parent_contract,
        migration,
    )
    .expect("the complete durable lineage should reach the requirement root");

    let admission =
        crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(&fixture.base)
            .unwrap();
    crate::node_agent_local_task_resume_routes::commit_validated_lease_migration(
        &resolved, &admission,
    )
    .unwrap();
    assert_eq!(
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
            &fixture.base,
            &fixture.active,
        )
        .unwrap()
        .as_deref(),
        Some("elon-supervision:local-root")
    );
    assert_eq!(git_output(&fixture.active, &["diff", "--cached"]), staged);
    assert_eq!(git_output(&fixture.active, &["diff"]), unstaged);
    assert_eq!(
        std::fs::read_to_string(fixture.active.join("draft.txt")).unwrap(),
        "untracked\n"
    );
    assert!(
        crate::node_agent_local_task_resume_routes::commit_validated_lease_migration(
            &resolved, &admission,
        )
        .is_err()
    );

    let mut next_parent = fixture.parent.clone();
    next_parent.task_id = "local-resume-generation-2".to_string();
    next_parent.conversation_id = "offline-resume-generation-2".to_string();
    let next_parent_contract = contract("resume_original", Some("local-child"), Some("local-root"));
    let next_contract = contract(
        "resume_original",
        Some("local-resume-generation-2"),
        Some("local-root"),
    );
    let next = validate_resume_workspace(
        &next_contract,
        &next_parent,
        Some(&next_parent_contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect("later Resume generations should reuse the migrated root lease");
    assert!(next.lease_migration.is_none());

    assert!(crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "local-child",
    )
    .is_err());
    assert!(crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "another-root",
    )
    .is_err());
    crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "local-root",
    )
    .expect("only an accepted review carrying the exact root may release the lease");
}

#[test]
fn generic_foreign_and_identity_unknown_leases_remain_fail_closed() {
    let fixture = MigrationFixture::new();
    crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "local-child",
    )
    .unwrap();
    run_git(
        &fixture.base,
        &[
            "worktree",
            "lock",
            "--reason",
            "active PC CLI task; Resume or successful finalization unlocks",
            fixture.active.to_string_lossy().as_ref(),
        ],
    );
    let generic = validate_resume_workspace(
        &fixture.resume_contract,
        &fixture.parent,
        Some(&fixture.parent_contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("a generic conversation lock must never be upgraded");
    assert!(generic.to_string().contains("root lease 身份不匹配"));
    assert!(crate::node_agent_supervision_worktree_lease::acquire(
        &fixture.base,
        &fixture.active,
        "local-root",
    )
    .is_err());

    run_git(
        &fixture.base,
        &[
            "worktree",
            "unlock",
            fixture.active.to_string_lossy().as_ref(),
        ],
    );
    crate::node_agent_supervision_worktree_lease::acquire(
        &fixture.base,
        &fixture.active,
        "foreign-task",
    )
    .unwrap();
    let foreign = validate_resume_workspace(
        &fixture.resume_contract,
        &fixture.parent,
        Some(&fixture.parent_contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("a foreign supervision lease must never migrate");
    assert!(foreign.to_string().contains("root lease 身份不匹配"));
}

#[test]
fn incomplete_or_drifting_durable_lineage_cannot_authorize_migration() {
    let fixture = MigrationFixture::new();
    let missing = assess_legacy_lease(
        Some("elon-supervision:local-child"),
        "elon-supervision:local-root",
        &fixture.resume_contract,
        &fixture.parent,
        None,
    )
    .expect_err("identity-unknown legacy lease must fail closed");
    assert!(missing.to_string().contains("缺少可验证"));

    let migration = fixture.resolve().lease_migration.unwrap();
    let drifting_middle = contract(
        "capability_repair",
        Some("local-root"),
        Some("another-root"),
    );
    record_contract(&fixture.journal, "local-middle", &drifting_middle);
    let error = validate_full_lineage(
        &fixture.store,
        &fixture.journal,
        &fixture.parent,
        &fixture.parent_contract,
        &migration,
    )
    .expect_err("a root drift anywhere in the durable ancestry must fail closed");
    assert!(error.to_string().contains("同一 root_task_id"));
    assert_eq!(
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
            &fixture.base,
            &fixture.active,
        )
        .unwrap()
        .as_deref(),
        Some("elon-supervision:local-child")
    );
}

fn create_task(store: &LocalTaskStore, task_id: &str, conversation_id: &str, base: &Path) {
    store
        .create(LocalTaskStart {
            task_id,
            owner_user_id: "owner-a",
            agent_id: "agent-a",
            install_id: "install-a",
            project_id: "project-a",
            channel_id: None,
            conversation_id,
            workspace_path: base.to_string_lossy().as_ref(),
            prompt: "task",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
}

fn contract(
    role: &str,
    parent_task_id: Option<&str>,
    root_task_id: Option<&str>,
) -> SupervisionContract {
    SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: role.to_string(),
        parent_task_id: parent_task_id.map(str::to_string),
        root_task_id: root_task_id.map(str::to_string),
        acceptance_criteria: vec!["safe resume".to_string()],
        improvement_policy: "after_task_only".to_string(),
    }
}

fn record_contract(journal: &TaskJournal, task_id: &str, contract: &SupervisionContract) {
    record_supervision_event(
        journal,
        task_id,
        "supervision_contract",
        contract_payload(contract),
    )
    .unwrap();
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = git_command().args(args).current_dir(cwd).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(cwd: &Path, args: &[&str]) -> String {
    let output = git_command().args(args).current_dir(cwd).output().unwrap();
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
