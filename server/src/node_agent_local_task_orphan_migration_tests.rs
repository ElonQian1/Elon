use std::path::{Path, PathBuf};

use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::{
    git_command_error::git_command,
    node_agent_local_task_store::LocalTaskStart,
    node_agent_local_task_supervision::{contract_payload, record_supervision_event},
};

struct Fixture {
    root: PathBuf,
    base: PathBuf,
    source: PathBuf,
    history: PathBuf,
    store: LocalTaskStore,
    journal: TaskJournal,
    parent: LocalTaskRecord,
    parent_contract: SupervisionContract,
    migration: OrphanedWorkspaceMigration,
}

impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("elon-orphan-occupancy-{}", Uuid::new_v4().simple()));
        let base = root.join("base");
        let source = root.join("conversation-worktrees/project/root-conversation");
        let history = root.join("conversation-worktrees/project/history-conversation");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        git(&base, &["init"]);
        git(&base, &["config", "user.email", "tests@example.invalid"]);
        git(&base, &["config", "user.name", "Tests"]);
        std::fs::write(base.join("seed.txt"), "seed\n").unwrap();
        git(&base, &["add", "seed.txt"]);
        git(&base, &["commit", "-m", "seed"]);
        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                "ai/session/project/history-conversation",
                history.to_str().unwrap(),
                "HEAD",
            ],
        );
        crate::node_agent_supervision_worktree_lease::acquire(&base, &history, "local-root")
            .unwrap();

        let store = LocalTaskStore::new(root.join("tasks.sqlite3"));
        let journal = TaskJournal::new(root.join("journal"));
        let parent_contract = contract("requirement", None);
        create_task(
            &store,
            &journal,
            "local-root",
            "owner",
            "agent",
            "install",
            "project",
            &source,
            &base,
            &parent_contract,
            true,
            "local-root",
        );
        let history_contract = contract("capability_repair", Some("local-root"));
        create_task(
            &store,
            &journal,
            "local-history",
            "owner",
            "agent",
            "install",
            "project",
            &history,
            &base,
            &history_contract,
            true,
            "local-root",
        );
        let parent = store.get("local-root").unwrap().unwrap();
        let head = output(&base, &["rev-parse", "HEAD"]);
        Self {
            root,
            base,
            source: source.clone(),
            history,
            store,
            journal,
            parent,
            parent_contract,
            migration: OrphanedWorkspaceMigration {
                source_path: source.to_string_lossy().into_owned(),
                source_branch: "ai/session/project/root-conversation".into(),
                recorded_head: head.clone(),
                target_head: head,
            },
        }
    }

    fn validate(&self) -> anyhow::Result<OrphanMigrationOccupancy> {
        validate_occupancy(
            &self.store,
            &self.journal,
            &self.parent,
            &self.parent_contract,
            &self.migration,
            &self.base,
        )
    }

    fn add_child(
        &self,
        id: &str,
        owner: &str,
        agent: &str,
        install: &str,
        project: &str,
        status_root: &str,
        terminal: bool,
    ) {
        let path = self
            .root
            .join(format!("conversation-worktrees/project/{id}"));
        std::fs::create_dir_all(&path).unwrap();
        create_task(
            &self.store,
            &self.journal,
            id,
            owner,
            agent,
            install,
            project,
            &path,
            &self.base,
            &contract("capability_repair", Some("local-root")),
            terminal,
            status_root,
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = crate::node_agent_supervision_worktree_lease::release(
            &self.base,
            &self.history,
            "local-root",
        );
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn terminal_history_is_reclaimable_without_touching_the_orphan_source() {
    let fixture = Fixture::new();
    std::fs::write(fixture.source.join("preserve.txt"), "preserve\n").unwrap();
    let plan = fixture
        .validate()
        .expect("terminal history should correlate");
    assert_eq!(plan.reclaimable_leases.len(), 1);
    reclaim_terminal_leases(&fixture.base, &fixture.source, &plan).unwrap();
    assert_eq!(
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
            &fixture.base,
            &fixture.history,
        )
        .unwrap(),
        None
    );
    assert_eq!(
        std::fs::read_to_string(fixture.source.join("preserve.txt")).unwrap(),
        "preserve\n"
    );
}

#[test]
fn nonterminal_same_root_task_is_strictly_rejected() {
    let fixture = Fixture::new();
    fixture.add_child(
        "local-running",
        "owner",
        "agent",
        "install",
        "project",
        "local-root",
        false,
    );
    let error = fixture.validate().unwrap_err();
    assert!(error.to_string().contains("非终态"));
}

#[test]
fn owner_install_project_and_workspace_root_drift_are_rejected() {
    for (label, owner, agent, install, project, status_root) in [
        (
            "owner",
            "foreign",
            "agent",
            "install",
            "project",
            "local-root",
        ),
        (
            "agent",
            "owner",
            "foreign",
            "install",
            "project",
            "local-root",
        ),
        (
            "install",
            "owner",
            "agent",
            "foreign",
            "project",
            "local-root",
        ),
        (
            "project",
            "owner",
            "agent",
            "install",
            "foreign",
            "local-root",
        ),
        (
            "root",
            "owner",
            "agent",
            "install",
            "project",
            "foreign-root",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.add_child(
            &format!("local-drift-{label}"),
            owner,
            agent,
            install,
            project,
            status_root,
            true,
        );
        assert!(
            fixture.validate().is_err(),
            "{label} drift must fail closed"
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn create_task(
    store: &LocalTaskStore,
    journal: &TaskJournal,
    id: &str,
    owner: &str,
    agent: &str,
    install: &str,
    project: &str,
    active: &Path,
    base: &Path,
    contract: &SupervisionContract,
    terminal: bool,
    status_root: &str,
) {
    store
        .create(LocalTaskStart {
            task_id: id,
            owner_user_id: owner,
            agent_id: agent,
            install_id: install,
            project_id: project,
            channel_id: None,
            conversation_id: id,
            workspace_path: active.to_str().unwrap(),
            prompt: "task",
            cli: "codex",
            runtime_permission: "full_access",
        })
        .unwrap();
    store
        .record_initial_workspace_status(
            id,
            &json!({
                "platform_provenance": "elon.conversation_worktree.v1",
                "project_id": project,
                "root_task_id": status_root,
                "base_workspace_path": base,
                "active_workspace_path": active,
                "isolated": true,
                "branch": format!("ai/session/{project}/{id}"),
            }),
        )
        .unwrap();
    record_supervision_event(
        journal,
        id,
        "supervision_contract",
        contract_payload(contract),
    )
    .unwrap();
    if terminal {
        assert!(store
            .mark_recovery_blocked(id, "terminal test fixture")
            .unwrap());
    }
}

fn contract(role: &str, parent: Option<&str>) -> SupervisionContract {
    SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.into(),
        supervisor: "codex_desktop".into(),
        task_role: role.into(),
        parent_task_id: parent.map(str::to_string),
        root_task_id: Some("local-root".into()),
        acceptance_criteria: Vec::new(),
        improvement_policy: "after_task_only".into(),
    }
}

fn git(cwd: &Path, args: &[&str]) {
    let output = git_command().args(args).current_dir(cwd).output().unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn output(cwd: &Path, args: &[&str]) -> String {
    let output = git_command().args(args).current_dir(cwd).output().unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
