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
        run_git(
            &base,
            &[
                "config",
                "remote.origin.url",
                "https://example.test/elon.git",
            ],
        );
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
        let git_head = git_output(&active, &["rev-parse", "--verify", "HEAD^{commit}"]);
        run_git(
            &base,
            &["update-ref", "refs/remotes/origin/main", &git_head],
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
                "platform_provenance": "elon.conversation_worktree.v1",
                "project_id": "project-a",
                "root_task_id": "local-parent",
                "base_workspace_path": base.to_string_lossy(),
                "active_workspace_path": active.to_string_lossy(),
                "isolated": true,
                "branch": "ai/session/project-a/conversation-a",
                "git_head": git_head,
                "git_common_dir": git_output(&base, &["rev-parse", "--path-format=absolute", "--git-common-dir"]),
                "git_remote": "https://example.test/elon.git",
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
        crate::node_agent_supervision_worktree_lease::acquire(
            &base,
            &active,
            contract.root_task_id.as_deref().unwrap(),
        )
        .expect("fixture should hold the supervision root lease");
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
fn current_platform_resume_rejects_provenance_common_dir_and_remote_drift() {
    for drift in ["provenance", "common", "remote"] {
        let mut fixture = ResumeFixture::new();
        match drift {
            "provenance" => {
                fixture.parent.workspace_status.as_mut().unwrap()["platform_provenance"] =
                    "malformed".into();
            }
            "common" => {
                fixture.parent.workspace_status.as_mut().unwrap()["git_common_dir"] = fixture
                    .root
                    .join("foreign.git")
                    .to_string_lossy()
                    .into_owned()
                    .into();
            }
            "remote" => run_git(
                &fixture.base,
                &["config", "remote.origin.url", "https://evil.test/elon.git"],
            ),
            _ => unreachable!(),
        }
        assert!(
            validate_resume_workspace(
                &fixture.contract,
                &fixture.parent,
                None,
                None,
                "project-a",
                fixture.active.to_string_lossy().as_ref(),
            )
            .is_err(),
            "{drift} must fail closed"
        );
    }
}

#[test]
fn resume_of_resume_reuses_the_platform_recorded_inherited_workspace() {
    let mut fixture = ResumeFixture::new();
    fixture.parent.conversation_id = "offline-resume-child".to_string();
    crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "local-parent",
    )
    .unwrap();
    crate::node_agent_supervision_worktree_lease::acquire(
        &fixture.base,
        &fixture.active,
        "local-root",
    )
    .unwrap();
    fixture.contract.root_task_id = Some("local-root".to_string());
    let parent_contract = SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "resume_original".to_string(),
        parent_task_id: Some("local-original".to_string()),
        root_task_id: Some("local-root".to_string()),
        acceptance_criteria: vec!["resume safely".to_string()],
        improvement_policy: "after_task_or_unblock".to_string(),
    };

    let previous_error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("the former single-generation rule should reproduce the rejection");
    assert!(previous_error
        .to_string()
        .contains("缺少可验证的继承监督契约"));

    let resolved = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&parent_contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect("a recorded resume_original lineage should keep its inherited workspace eligible");
    assert_eq!(resolved.derivation, "inherited_workspace_status");
    assert_eq!(
        resolved.inherited_workspace.branch.as_deref(),
        Some("ai/session/project-a/conversation-a")
    );
    assert!(same_path(
        Path::new(&resolved.inherited_workspace.workspace_path),
        &std::fs::canonicalize(&fixture.active).unwrap()
    ));
}

#[test]
fn inherited_workspace_rejects_untrusted_lineage_or_root_drift() {
    let mut fixture = ResumeFixture::new();
    fixture.parent.conversation_id = "offline-resume-child".to_string();
    let mut parent_contract = fixture.contract.clone();
    parent_contract.parent_task_id = Some("local-original".to_string());

    parent_contract.task_role = "requirement".to_string();
    let role_error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&parent_contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("an ordinary supervised parent must not claim inherited identity");
    assert!(role_error.to_string().contains("resume_original 父任务"));

    parent_contract.task_role = "resume_original".to_string();
    parent_contract.root_task_id = Some("another-root".to_string());
    let root_error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&parent_contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("a different supervision root must fail closed");
    assert!(root_error.to_string().contains("root_task_id"));
}

#[test]
fn resume_rejects_non_isolated_workspace_status() {
    let mut fixture = ResumeFixture::new();
    fixture.parent.workspace_status.as_mut().unwrap()["isolated"] = json!(false);
    let error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("a shared workspace must never become resumable by inheritance");
    assert!(error.to_string().contains("不是平台生成的隔离 worktree"));
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
        phase: "finished".to_string(),
        current_command: None,
        last_progress_ms: None,
        heartbeat_at_ms: None,
        timeout_policy: None,
        started_at_ms: 1,
        updated_at_ms: 2,
        cancel_requested_at_ms: None,
        cancel_intent: None,
    };
    let resolved = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
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
        None,
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
        None,
        "project-b",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("cross-project resume must fail");
    assert!(error.to_string().contains("不能跨项目"));
}

#[test]
fn second_generation_resume_inherits_the_original_root_lease() {
    let mut fixture = ResumeFixture::new();
    fixture.parent.task_id = "local-resume-generation-1".to_string();
    fixture.contract.parent_task_id = Some(fixture.parent.task_id.clone());
    fixture.contract.root_task_id = Some("local-parent".to_string());
    let resolved = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect("a later generation under the same root must reuse the root lease");
    assert_eq!(
        resolved
            .inherited_workspace
            .supervision_root_task_id
            .as_deref(),
        Some("local-parent")
    );
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
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("active parent must fail");
    assert!(error.to_string().contains("父任务仍在运行"));
}

#[test]
fn deleted_active_workspace_requires_snapshot_continue_from_recorded_head() {
    let mut fixture = ResumeFixture::new();
    // Real isolated records authorize the base via workspace_status while the
    // top-level workspace_path points at the active conversation worktree.
    fixture.parent.workspace_path = fixture.active.to_string_lossy().to_string();
    let recorded_head = git_output(&fixture.base, &["rev-parse", "HEAD"]);
    run_git(
        &fixture.base,
        &["update-ref", "refs/remotes/origin/main", &recorded_head],
    );
    crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "local-parent",
    )
    .unwrap();
    run_git(
        &fixture.base,
        &[
            "worktree",
            "remove",
            "--force",
            fixture.active.to_string_lossy().as_ref(),
        ],
    );

    let resolved = inspect_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&fixture.contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect("a deleted active directory should become a snapshot-continue candidate");
    assert!(resolved.snapshot_continue_required);
    assert_eq!(resolved.git_head, recorded_head);
    assert_eq!(resolved.derivation, "missing_active_snapshot_continue");

    inspect_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&fixture.contract),
        None,
        "project-a",
        fixture.active.to_string_lossy().as_ref(),
    )
    .expect("the recorded deleted active path should remain an allowed request identity");

    let recreated =
        crate::pc_workspace_provisioner::prepare_conversation_workspace_in_with_supervision_at_ref(
            &fixture.root,
            fixture.base.to_string_lossy().as_ref(),
            "project-a",
            "conversation-resume",
            Some("local-parent"),
            &resolved.git_head,
        )
        .expect("snapshot continue should create a new platform worktree");
    assert_ne!(Path::new(&recreated.workspace_path), fixture.active);
    assert_eq!(
        git_output(Path::new(&recreated.workspace_path), &["rev-parse", "HEAD"]),
        recorded_head
    );
    assert_eq!(
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
            &fixture.base,
            Path::new(&recreated.workspace_path)
        )
        .unwrap()
        .as_deref(),
        Some("elon-supervision:local-parent")
    );
}

#[test]
fn deleted_active_workspace_fails_closed_for_missing_or_drifted_git_head() {
    let mut fixture = ResumeFixture::new();
    let recorded_head = git_output(&fixture.base, &["rev-parse", "HEAD"]);
    run_git(
        &fixture.base,
        &["update-ref", "refs/remotes/origin/main", &recorded_head],
    );
    crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "local-parent",
    )
    .unwrap();
    run_git(
        &fixture.base,
        &[
            "worktree",
            "remove",
            "--force",
            fixture.active.to_string_lossy().as_ref(),
        ],
    );
    let mut wrong_root = fixture.contract.clone();
    wrong_root.root_task_id = Some("another-root".to_string());
    let root_error = inspect_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&wrong_root),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("a deleted worktree must not weaken supervision root identity");
    assert!(root_error.to_string().contains("root_task_id"));

    fixture.parent.workspace_status.as_mut().unwrap()["git_head"] = serde_json::Value::Null;
    let missing = inspect_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&fixture.contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("missing snapshot identity must fail closed");
    assert!(missing.to_string().contains("缺少 git_head"));

    fixture.parent.workspace_status.as_mut().unwrap()["git_head"] =
        json!("0000000000000000000000000000000000000000");
    let drifted = inspect_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        Some(&fixture.contract),
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("an unknown commit must fail closed");
    assert!(drifted.to_string().contains("rev-parse"));
}

#[test]
fn dirty_exclusive_resume_preserves_staged_unstaged_and_untracked_changes() {
    let fixture = ResumeFixture::new();
    std::fs::write(fixture.active.join("README.md"), "staged edit\n").unwrap();
    run_git(&fixture.active, &["add", "README.md"]);
    std::fs::write(
        fixture.active.join("README.md"),
        "staged edit plus unstaged edit\n",
    )
    .unwrap();
    std::fs::write(fixture.active.join("draft.txt"), "untracked draft\n").unwrap();
    let staged_before = git_output(&fixture.active, &["diff", "--cached"]);
    let unstaged_before = git_output(&fixture.active, &["diff"]);

    let resolved = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect("trusted exclusive dirty worktree should be resumable");

    assert_eq!(
        resolved
            .inherited_workspace
            .supervision_root_task_id
            .as_deref(),
        Some("local-parent")
    );
    assert_eq!(
        git_output(&fixture.active, &["diff", "--cached"]),
        staged_before
    );
    assert_eq!(git_output(&fixture.active, &["diff"]), unstaged_before);
    assert_eq!(
        std::fs::read_to_string(fixture.active.join("draft.txt")).unwrap(),
        "untracked draft\n"
    );
    let status = git_output(&fixture.active, &["status", "--short"]);
    assert!(status.contains("MM README.md"), "{status}");
    assert!(status.contains("?? draft.txt"), "{status}");
}

#[test]
fn resume_rejects_a_nonmatching_root_lease_identity() {
    let fixture = ResumeFixture::new();
    crate::node_agent_supervision_worktree_lease::release(
        &fixture.base,
        &fixture.active,
        "local-parent",
    )
    .unwrap();
    crate::node_agent_supervision_worktree_lease::acquire(
        &fixture.base,
        &fixture.active,
        "another-root",
    )
    .unwrap();
    let error = validate_resume_workspace(
        &fixture.contract,
        &fixture.parent,
        None,
        None,
        "project-a",
        fixture.base.to_string_lossy().as_ref(),
    )
    .expect_err("mismatched root lease must fail closed");
    assert!(error.to_string().contains("root lease 身份不匹配"));
}

#[test]
fn locked_worktree_survives_cleanup_while_a_spawned_process_is_alive() {
    let fixture = ResumeFixture::new();
    let mut child = spawn_waiting_process(&fixture.active);

    let output = git_command()
        .args([
            "worktree",
            "remove",
            "--force",
            fixture.active.to_string_lossy().as_ref(),
        ])
        .current_dir(&fixture.base)
        .output()
        .expect("cleanup command should start");
    assert!(
        !output.status.success(),
        "single-force cleanup must honor lock"
    );
    assert!(recovery::is_git_worktree(&fixture.active));

    let _ = child.kill();
    let _ = child.wait();
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

fn git_output(cwd: &Path, args: &[&str]) -> String {
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
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[cfg(windows)]
fn spawn_waiting_process(cwd: &Path) -> std::process::Child {
    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
        .current_dir(cwd)
        .spawn()
        .expect("spawn waiting Windows process")
}

#[cfg(not(windows))]
fn spawn_waiting_process(cwd: &Path) -> std::process::Child {
    std::process::Command::new("sh")
        .args(["-c", "sleep 30"])
        .current_dir(cwd)
        .spawn()
        .expect("spawn waiting Unix process")
}
