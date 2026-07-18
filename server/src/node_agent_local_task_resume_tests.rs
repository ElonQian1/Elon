use super::*;
use crate::git_command_error::git_command;
use serde_json::json;
use uuid::Uuid;

struct ResumeFixture {
    root: PathBuf,
    base: PathBuf,
    active: PathBuf,
    parent: LocalTaskRecord,
    contract: SupervisionContract,
}

impl ResumeFixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("elon_resume_workspace_{}", Uuid::new_v4().simple()));
        let base = root.join("base");
        let active = root
            .join("conversation-worktrees")
            .join("project-a")
            .join("conversation-a");
        std::fs::create_dir_all(&base).expect("create base repo");
        run_git(&base, &["init"]);
        run_git(&base, &["config", "user.email", "ai@example.test"]);
        run_git(&base, &["config", "user.name", "AI Test"]);
        std::fs::write(base.join("README.md"), "seed\n").expect("write seed");
        run_git(&base, &["add", "README.md"]);
        run_git(&base, &["commit", "-m", "seed"]);
        std::fs::create_dir_all(active.parent().unwrap()).expect("create worktree root");
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

        let parent = LocalTaskRecord {
            task_id: "local-parent".to_string(),
            owner_user_id: "owner-a".to_string(),
            agent_id: "agent-a".to_string(),
            install_id: "install-a".to_string(),
            project_id: "project-a".to_string(),
            channel_id: None,
            conversation_id: "conversation-a".to_string(),
            workspace_path: base.to_string_lossy().to_string(),
            prompt: "original request".to_string(),
            cli: "codex".to_string(),
            runtime_permission: "full_access".to_string(),
            execution_origin: "local_offline".to_string(),
            billing_source: "own_codex".to_string(),
            status: "failed".to_string(),
            error: Some("sidecar timeout".to_string()),
            final_reply: None,
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
                "prepare_status": "prepared",
                "merge_status": "skipped"
            })),
            sync_state: "pending".to_string(),
            completion_event_id: Some("completion-parent".to_string()),
            started_at_ms: 1,
            finished_at_ms: Some(2),
            server_ack_at_ms: None,
        };
        let contract = SupervisionContract {
            protocol: SUPERVISION_PROTOCOL.to_string(),
            supervisor: "codex_desktop".to_string(),
            task_role: "resume_original".to_string(),
            parent_task_id: Some(parent.task_id.clone()),
            root_task_id: Some(parent.task_id.clone()),
            acceptance_criteria: vec!["resume safely".to_string()],
            improvement_policy: "after_task_or_unblock".to_string(),
        };
        Self {
            root,
            base,
            active,
            parent,
            contract,
        }
    }
}

impl Drop for ResumeFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn valid_resume_inherits_recorded_platform_worktree_from_base_or_active_request() {
    let fixture = ResumeFixture::new();
    for requested in [&fixture.base, &fixture.active] {
        let resolved = validate_resume_workspace(
            &fixture.contract,
            &fixture.parent,
            None,
            "project-a",
            requested.to_string_lossy().as_ref(),
        )
        .expect("valid parent worktree should be inherited");
        assert!(same_path(
            Path::new(&resolved.authorized_workspace_path),
            &std::fs::canonicalize(&fixture.base).unwrap()
        ));
        assert!(same_path(
            Path::new(&resolved.inherited_workspace.workspace_path),
            &std::fs::canonicalize(&fixture.active).unwrap()
        ));
    }
}

#[test]
fn legacy_resume_derives_only_from_started_cwd_and_git_registry() {
    let mut fixture = ResumeFixture::new();
    fixture.parent.workspace_status = None;
    let journal = crate::node_agent_task_journal::TaskJournalRecord {
        req_id: fixture.parent.task_id.clone(),
        cli_name: "codex".to_string(),
        route: Some("local_offline".to_string()),
        run_handle_id: None,
        cwd: Some(fixture.active.to_string_lossy().to_string()),
        runtime_permission: Some("full_access".to_string()),
        os_pid: None,
        process_started_at_ms: None,
        codex_session_id: None,
        codex_session_scope_key: None,
        codex_session_updated_at_ms: None,
        status: "finished".to_string(),
        started_at_ms: 1,
        updated_at_ms: 2,
        cancel_requested_at_ms: None,
    };
    let resolved = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&journal),
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect("legacy resume should use the latest durable started.cwd");
    assert_eq!(resolved.derivation, "legacy_started_cwd_git_registry");
    assert!(!resolved.git_head.is_empty());

    let arbitrary = fixture.root.join("arbitrary");
    std::fs::create_dir_all(&arbitrary).unwrap();
    let error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&journal),
        "project-a",
        arbitrary.to_string_lossy().as_ref(),
    )
    .expect_err("caller supplied arbitrary path must remain rejected");
    assert!(error.to_string().contains("只能引用"));
}

#[test]
fn resume_rejects_cross_project_parent() {
    let fixture = ResumeFixture::new();
    let error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        "project-b",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("cross-project resume must fail");
    assert!(error.to_string().contains("不能跨项目"));
}

#[test]
fn resume_rejects_forged_active_path() {
    let mut fixture = ResumeFixture::new();
    let forged = fixture.root.join("forged-worktree");
    std::fs::create_dir_all(&forged).expect("create forged path");
    fixture.parent.workspace_status = Some(json!({
        "base_workspace_path": fixture.base.to_string_lossy(),
        "active_workspace_path": forged.to_string_lossy(),
        "isolated": true,
        "branch": "ai/session/project-a/conversation-a"
    }));
    let error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("forged worktree path must fail");
    assert!(error.to_string().contains("平台生成 worktree"));
}

#[test]
fn resume_rejects_active_parent_task() {
    let mut fixture = ResumeFixture::new();
    fixture.parent.status = "running".to_string();
    fixture.parent.finished_at_ms = None;
    let error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("active parent must fail");
    assert!(error.to_string().contains("父任务仍在运行"));
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git should start");
    assert!(
        output.status.success(),
        "git {:?} failed: stdout={} stderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
