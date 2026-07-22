use std::path::{Path, PathBuf};

use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::{
    git_command_error::git_command,
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
};

struct Fixture {
    root: PathBuf,
    base: PathBuf,
    active: PathBuf,
    parent: LocalTaskRecord,
    contract: SupervisionContract,
}

impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("elon-orphan-git-{}", Uuid::new_v4().simple()));
        let base = root.join("base");
        let active = root.join("conversation-worktrees/project-a/conversation-a");
        std::fs::create_dir_all(&base).unwrap();
        git(&base, &["init"]);
        git(&base, &["config", "user.email", "tests@example.invalid"]);
        git(&base, &["config", "user.name", "Tests"]);
        git(
            &base,
            &[
                "config",
                "remote.origin.url",
                "https://example.test/elon.git",
            ],
        );
        std::fs::write(base.join("README.md"), "seed\n").unwrap();
        git(&base, &["add", "README.md"]);
        git(&base, &["commit", "-m", "seed"]);
        std::fs::create_dir_all(active.parent().unwrap()).unwrap();
        git(
            &base,
            &[
                "worktree",
                "add",
                "-b",
                "ai/session/project-a/conversation-a",
                active.to_str().unwrap(),
                "HEAD",
            ],
        );
        let head = output(&active, &["rev-parse", "HEAD"]);
        git(&base, &["update-ref", "refs/remotes/origin/main", &head]);
        crate::node_agent_supervision_worktree_lease::acquire(&base, &active, "local-parent")
            .unwrap();
        let parent = LocalTaskRecord {
            task_id: "local-parent".into(),
            owner_user_id: "owner".into(),
            agent_id: "agent".into(),
            install_id: "install".into(),
            project_id: "project-a".into(),
            channel_id: None,
            conversation_id: "conversation-a".into(),
            workspace_path: active.to_string_lossy().into_owned(),
            prompt: "task".into(),
            cli: "codex".into(),
            runtime_permission: "full_access".into(),
            execution_origin: "local_offline".into(),
            billing_source: "own_codex".into(),
            status: "failed".into(),
            error: None,
            final_reply: None,
            model: None,
            codex_session_id: None,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            workspace_status: Some(json!({
                "platform_provenance":"elon.conversation_worktree.v1", "project_id":"project-a", "root_task_id":"local-parent",
                "base_workspace_path":base, "active_workspace_path":active, "isolated":true,
                "branch":"ai/session/project-a/conversation-a", "git_head":head,
                "git_common_dir":output(&base, &["rev-parse", "--path-format=absolute", "--git-common-dir"]),
                "git_remote":"https://example.test/elon.git"
            })),
            sync_state: "local_only".into(),
            completion_event_id: Some("event".into()),
            started_at_ms: 1,
            finished_at_ms: Some(2),
            server_ack_at_ms: None,
        };
        let contract = SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: "resume_original".into(),
            parent_task_id: Some("local-parent".into()),
            root_task_id: Some("local-parent".into()),
            acceptance_criteria: Vec::new(),
            improvement_policy: "after_task_only".into(),
        };
        Self {
            root,
            base,
            active,
            parent,
            contract,
        }
    }

    fn orphanize(&self, name: &str) {
        crate::node_agent_supervision_worktree_lease::release(
            &self.base,
            &self.active,
            "local-parent",
        )
        .unwrap();
        let parked = self.root.join(name);
        std::fs::rename(&self.active, &parked).unwrap();
        git(&self.base, &["worktree", "prune", "--expire", "now"]);
        std::fs::rename(parked, &self.active).unwrap();
    }

    fn inspect(&self) -> anyhow::Result<ResolvedResumeWorkspace> {
        inspect_resume_workspace(
            &self.contract,
            &self.parent,
            None,
            None,
            "project-a",
            self.base.to_str().unwrap(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn orphaned_source_is_read_only_and_becomes_a_controlled_migration_candidate() {
    let fixture = Fixture::new();
    fixture.orphanize("parked-clean");
    let marker = std::fs::read_to_string(fixture.active.join(".git")).unwrap();
    let resolved = fixture.inspect().unwrap();
    assert_eq!(
        resolved.derivation,
        "orphaned_workspace_controlled_migration_ready_branch_head"
    );
    assert!(resolved.requires_recreation && resolved.orphaned_migration.is_some());
    assert!(!recovery::is_git_worktree(&fixture.active));
    assert_eq!(
        std::fs::read_to_string(fixture.active.join(".git")).unwrap(),
        marker
    );
}

#[test]
fn orphaned_content_drift_is_rejected_without_mutating_source() {
    let fixture = Fixture::new();
    fixture.orphanize("parked-dirty");
    std::fs::write(fixture.active.join("README.md"), "drifted\n").unwrap();
    assert!(format!("{:#}", fixture.inspect().unwrap_err()).contains("未保存差异"));
    assert_eq!(
        std::fs::read_to_string(fixture.active.join("README.md")).unwrap(),
        "drifted\n"
    );
}

#[test]
fn missing_platform_control_files_do_not_count_as_business_drift() {
    let fixture = Fixture::new();
    let control = fixture.active.join(".elon/recovery-state.json");
    std::fs::create_dir_all(control.parent().unwrap()).unwrap();
    std::fs::write(&control, "platform\n").unwrap();
    git(&fixture.active, &["add", ".elon/recovery-state.json"]);
    git(
        &fixture.active,
        &["commit", "-m", "platform control successor"],
    );
    let successor = output(&fixture.active, &["rev-parse", "HEAD"]);
    git(
        &fixture.base,
        &["update-ref", "refs/remotes/origin/main", &successor],
    );
    std::fs::remove_file(&control).unwrap();
    fixture.orphanize("parked-platform-control-gap");

    assert_eq!(fixture.inspect().unwrap().git_head, successor);
    assert!(!control.exists());
}

#[test]
fn legacy_workspace_status_without_new_provenance_fields_can_be_proved() {
    let mut fixture = Fixture::new();
    let status = fixture.parent.workspace_status.as_mut().unwrap();
    for field in [
        "platform_provenance",
        "project_id",
        "root_task_id",
        "git_common_dir",
        "git_remote",
    ] {
        status.as_object_mut().unwrap().remove(field);
    }
    fixture.orphanize("parked-legacy");

    let resolved = fixture.inspect().unwrap();
    assert_eq!(
        resolved.derivation,
        "orphaned_workspace_controlled_migration_ready_branch_head"
    );
    assert_eq!(
        resolved.git_head,
        output(&fixture.base, &["rev-parse", "HEAD"])
    );
}

#[test]
fn recorded_common_dir_and_remote_drift_fail_closed() {
    for field in ["git_common_dir", "git_remote"] {
        let mut fixture = Fixture::new();
        let value = if field == "git_common_dir" {
            fixture.root.to_string_lossy().into_owned()
        } else {
            "https://example.test/unrelated.git".to_string()
        };
        fixture.parent.workspace_status.as_mut().unwrap()[field] = json!(value);
        fixture.orphanize(&format!("parked-drift-{field}"));

        let error = format!("{:#}", fixture.inspect().unwrap_err());
        assert!(error.contains("漂移"), "{field}: {error}");
    }
}

#[test]
fn branch_head_must_be_a_landed_successor_of_the_recorded_head() {
    for landed in [true, false] {
        let fixture = Fixture::new();
        std::fs::write(fixture.active.join("successor.txt"), "successor\n").unwrap();
        git(&fixture.active, &["add", "successor.txt"]);
        git(&fixture.active, &["commit", "-m", "successor"]);
        let successor = output(&fixture.active, &["rev-parse", "HEAD"]);
        if landed {
            git(
                &fixture.base,
                &["update-ref", "refs/remotes/origin/main", &successor],
            );
        }
        fixture.orphanize(&format!("parked-landed-{landed}"));
        if landed {
            assert_eq!(fixture.inspect().unwrap().git_head, successor);
        } else {
            assert!(
                format!("{:#}", fixture.inspect().unwrap_err()).contains("尚未进入 origin/main")
            );
        }
    }

    let fixture = Fixture::new();
    fixture.orphanize("parked-unrelated");
    let tree = output(&fixture.base, &["rev-parse", "HEAD^{tree}"]);
    let unrelated = output(&fixture.base, &["commit-tree", &tree, "-m", "unrelated"]);
    git(
        &fixture.base,
        &[
            "update-ref",
            "refs/heads/ai/session/project-a/conversation-a",
            &unrelated,
        ],
    );
    git(
        &fixture.base,
        &["update-ref", "refs/remotes/origin/main", &unrelated],
    );
    assert!(format!("{:#}", fixture.inspect().unwrap_err()).contains("不是分支 HEAD 的祖先"));
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
