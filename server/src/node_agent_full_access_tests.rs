use super::*;
use uuid::Uuid;

fn task_record(
    project_id: &str,
    conversation_id: &str,
    base: &Path,
    active: &Path,
) -> crate::node_agent_local_task_store::LocalTaskRecord {
    crate::node_agent_local_task_store::LocalTaskRecord {
        task_id: "local-proof".into(),
        owner_user_id: "owner-1".into(),
        agent_id: "agent-1".into(),
        install_id: "install-1".into(),
        project_id: project_id.into(),
        channel_id: None,
        conversation_id: conversation_id.into(),
        workspace_path: base.to_string_lossy().into_owned(),
        prompt: "test".into(),
        cli: "codex".into(),
        runtime_permission: "full_access".into(),
        execution_origin: "local_offline".into(),
        billing_source: "own_codex".into(),
        status: "running".into(),
        error: None,
        final_reply: None,
        model: None,
        codex_session_id: None,
        input_tokens: None,
        cached_input_tokens: None,
        output_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        workspace_status: Some(serde_json::json!({
            "base_workspace_path": base.to_string_lossy(),
            "active_workspace_path": active.to_string_lossy(),
            "isolated": true
        })),
        sync_state: "local_only".into(),
        completion_event_id: None,
        started_at_ms: 1,
        finished_at_ms: None,
        server_ack_at_ms: None,
    }
}

fn temp_workspace(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "elon_full_access_{label}_{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&path).expect("create temp workspace");
    path
}

fn grant_file(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon_full_access_grants_{label}_{}.json",
        Uuid::new_v4().simple()
    ))
}

#[test]
fn platform_managed_workspace_allows_project_repo_and_conversation_worktree() {
    let root = temp_workspace("managed_root");
    let project_id = "prj_abc123";
    let project_part = safe_path_part(project_id, "project", 80);
    let repo = root.join("usr_1").join(&project_part).join("repo");
    let worktree = root
        .join("conversation-worktrees")
        .join(&project_part)
        .join("conv_1");
    std::fs::create_dir_all(&repo).expect("create managed repo");
    std::fs::create_dir_all(&worktree).expect("create managed worktree");

    assert!(platform_managed_workspace_matches_under(
        project_id,
        repo.to_string_lossy().as_ref(),
        &root
    ));
    assert!(platform_managed_workspace_matches_under(
        project_id,
        worktree.to_string_lossy().as_ref(),
        &root
    ));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn platform_managed_workspace_rejects_other_project_paths() {
    let root = temp_workspace("managed_mismatch");
    let repo = root.join("usr_1").join("prj_other").join("repo");
    std::fs::create_dir_all(&repo).expect("create other repo");

    assert!(!platform_managed_workspace_matches_under(
        "prj_expected",
        repo.to_string_lossy().as_ref(),
        &root
    ));

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn task_record_allows_only_the_exact_legacy_conversation_worktree() {
    let legacy_root = temp_workspace("legacy_task_record").join("workspaces");
    let base = legacy_root.join("elon-self");
    let active = legacy_root
        .join("conversation-worktrees")
        .join("elon-self")
        .join("conv-proof");
    let sibling = legacy_root
        .join("conversation-worktrees")
        .join("elon-self")
        .join("conv-other");
    std::fs::create_dir_all(&base).unwrap();
    std::fs::create_dir_all(&active).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    let record = task_record("elon-project", "conv-proof", &base, &active);
    let state = FullAccessGrantState::load_from_path(grant_file("legacy-proof"));
    let identity = identity("owner-1", "agent-1", "install-1");

    require_route_a_full_access_grant(
        &state,
        &identity,
        "codex",
        Some("full_access"),
        Some(&CliProjectContext {
            project_id: "elon-self".into(),
            conversation_id: "conv-proof".into(),
            runtime_permission: Some("full_access".into()),
        }),
        Some(active.to_string_lossy().as_ref()),
        false,
        Some(&record),
    )
    .await
    .expect_err("legacy path shape without durable provenance must fail closed");

    assert!(task_record_proves_legacy_managed_workspace(
        &record,
        &identity,
        &CliProjectContext {
            project_id: "elon-self".into(),
            conversation_id: "conv-proof".into(),
            runtime_permission: Some("full_access".into()),
        },
        base.to_string_lossy().as_ref(),
    ));

    assert!(!task_record_proves_legacy_managed_workspace(
        &record,
        &identity,
        &context("elon-self"),
        sibling.to_string_lossy().as_ref(),
    ));
    let mut foreign = task_record("elon-self", "conv-proof", &base, &active);
    foreign.install_id = "install-other".into();
    assert!(!task_record_proves_legacy_managed_workspace(
        &foreign,
        &identity,
        &CliProjectContext {
            project_id: "elon-self".into(),
            conversation_id: "conv-proof".into(),
            runtime_permission: Some("full_access".into()),
        },
        active.to_string_lossy().as_ref(),
    ));

    let _ = std::fs::remove_dir_all(legacy_root.parent().unwrap());
}

fn context(project_id: &str) -> CliProjectContext {
    CliProjectContext {
        project_id: project_id.to_string(),
        conversation_id: "conv".to_string(),
        runtime_permission: Some("full_access".to_string()),
    }
}

fn identity(owner_user_id: &str, agent_id: &str, install_id: &str) -> FullAccessGrantIdentity {
    FullAccessGrantIdentity::new(owner_user_id, agent_id, install_id).expect("valid grant identity")
}

#[tokio::test]
async fn grant_and_require_full_access_for_same_project_path() {
    let workspace = temp_workspace("ok");
    let state = FullAccessGrantState::load_from_path(grant_file("ok"));
    let identity = identity("owner-1", "agent-1", "install-1");
    state
        .grant_project(&identity, "project_1", workspace.to_string_lossy().as_ref())
        .await
        .expect("grant project");

    require_route_a_full_access_grant(
        &state,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context("project_1")),
        Some(workspace.to_string_lossy().as_ref()),
        true,
        None,
    )
    .await
    .expect("grant should authorize matching project path");
}

#[tokio::test]
async fn historical_self_project_alias_reuses_only_the_same_builtin_project_grant() {
    let workspace = temp_workspace("self-alias");
    let state = FullAccessGrantState::load_from_path(grant_file("self-alias"));
    let identity = identity("owner-1", "agent-1", "install-1");
    state
        .grant_project(
            &identity,
            "elon-project",
            workspace.to_string_lossy().as_ref(),
        )
        .await
        .expect("grant historical self-project id");

    require_route_a_full_access_grant(
        &state,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context("elon-self")),
        Some(workspace.to_string_lossy().as_ref()),
        false,
        None,
    )
    .await
    .expect("the durable self-project id should reuse the historical alias grant");

    assert!(!project_ids_equivalent("project-a", "project-b"));
}

#[tokio::test]
async fn route_a_full_access_requires_local_grant() {
    let workspace = temp_workspace("missing");
    let state = FullAccessGrantState::load_from_path(grant_file("missing"));
    let identity = identity("owner-1", "agent-1", "install-1");
    let error = require_route_a_full_access_grant(
        &state,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context("project_1")),
        Some(workspace.to_string_lossy().as_ref()),
        true,
        None,
    )
    .await
    .expect_err("missing grant should reject full access");

    assert!(
        error.to_string().contains("PROJECT_FULL_ACCESS_DISABLED"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn project_write_and_builtin_runtime_do_not_need_full_access_grant() {
    let workspace = temp_workspace("bypass");
    let state = FullAccessGrantState::load_from_path(grant_file("bypass"));
    let identity = identity("owner-1", "agent-1", "install-1");

    require_route_a_full_access_grant(
        &state,
        &identity,
        "codex",
        Some("project_write"),
        Some(&context("project_1")),
        Some(workspace.to_string_lossy().as_ref()),
        true,
        None,
    )
    .await
    .expect("project_write route A should not require full-access grant");

    require_route_a_full_access_grant(
        &state,
        &identity,
        "api-runtime",
        Some("full_access"),
        Some(&context("project_1")),
        Some(workspace.to_string_lossy().as_ref()),
        true,
        None,
    )
    .await
    .expect("built-in runtime keeps its own sandbox guard");
}

#[tokio::test]
async fn local_offline_policy_never_uses_personal_chat_bypass() {
    let workspace = temp_workspace("chat-policy");
    let state = FullAccessGrantState::load_from_path(grant_file("chat-policy"));
    let identity = identity("owner-1", "agent-1", "install-1");

    require_route_a_full_access_grant(
        &state,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context("chat")),
        Some(workspace.to_string_lossy().as_ref()),
        true,
        None,
    )
    .await
    .expect("cloud personal chat keeps its historical bypass");

    let error = require_route_a_full_access_grant(
        &state,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context("chat")),
        Some(workspace.to_string_lossy().as_ref()),
        false,
        None,
    )
    .await
    .expect_err("local offline chat must require an explicit project grant");
    assert!(error.to_string().contains("PROJECT_FULL_ACCESS_DISABLED"));
}

#[tokio::test]
async fn full_access_grants_are_isolated_by_owner_agent_and_install() {
    let workspace = temp_workspace("identity-isolation");
    let grant_path = grant_file("identity-isolation");
    let state = FullAccessGrantState::load_from_path(grant_path.clone());
    let granted = identity("owner-a", "agent-a", "install-a");
    state
        .grant_project(&granted, "project_1", workspace.to_string_lossy().as_ref())
        .await
        .expect("grant project for original identity");
    let state = FullAccessGrantState::load_from_path(grant_path.clone());

    for foreign in [
        identity("owner-b", "agent-a", "install-a"),
        identity("owner-a", "agent-b", "install-a"),
        identity("owner-a", "agent-a", "install-b"),
    ] {
        assert!(state.list(&foreign).await.is_empty());
        let error = require_route_a_full_access_grant(
            &state,
            &foreign,
            "codex",
            Some("full_access"),
            Some(&context("project_1")),
            Some(workspace.to_string_lossy().as_ref()),
            false,
            None,
        )
        .await
        .expect_err("foreign runtime identity must not reuse grant");
        assert!(error.to_string().contains("PROJECT_FULL_ACCESS_DISABLED"));
    }

    assert_eq!(state.list(&granted).await.len(), 1);
    let _ = std::fs::remove_file(grant_path);
    let _ = std::fs::remove_dir_all(workspace);
}

#[tokio::test]
async fn legacy_identityless_grants_fail_closed() {
    let workspace = temp_workspace("legacy-grant");
    let grant_path = grant_file("legacy-grant");
    let canonical = std::fs::canonicalize(&workspace).unwrap();
    std::fs::write(
        &grant_path,
        serde_json::json!({
            "grants": [{
                "project_id": "project_1",
                "workspace_path": canonical.to_string_lossy(),
                "granted_at_ms": 1
            }]
        })
        .to_string(),
    )
    .unwrap();
    let state = FullAccessGrantState::load_from_path(grant_path.clone());
    let current = identity("owner-a", "agent-a", "install-a");

    assert!(state.list(&current).await.is_empty());
    assert!(state
        .require_project(&current, "project_1", workspace.to_string_lossy().as_ref())
        .await
        .is_err());

    let _ = std::fs::remove_file(grant_path);
    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn runtime_policy_summary_exposes_route_bc_safety_limits() {
    let summary = runtime_policy_summary();

    assert_eq!(summary["schema"], "elon.pc_node.runtime_policy.v1");
    assert_eq!(summary["fullAccess"]["routeAInstalledCliOnly"], true);
    assert_eq!(
        summary["fullAccess"]["routeBCFullAccessEffect"],
        "keeps_workspace_path_checks_command_allowlist_and_tool_approvals"
    );
    assert_eq!(
        summary["fullAccess"]["routeBCDangerFullAccessEffect"],
        "danger_full_access_allows_absolute_paths_arbitrary_shell_and_skips_tool_approvals"
    );
    assert_eq!(
        summary["operatorVisibility"]["policyField"],
        "runtime_policy"
    );

    let approval_tools = summary["routeBC"]["approvalRequiredTools"]
        .as_array()
        .expect("approvalRequiredTools should be an array");
    for tool in ["write_file", "apply_patch", "run_command"] {
        assert!(
            approval_tools
                .iter()
                .any(|item| item.as_str() == Some(tool)),
            "missing approval tool {tool}"
        );
    }

    let denied = summary["routeBC"]["highRiskGitPushDenied"]
        .as_array()
        .expect("highRiskGitPushDenied should be an array");
    for arg in ["--force*", "--delete", "--mirror", "+refspec", ":branch"] {
        assert!(
            denied.iter().any(|item| item.as_str() == Some(arg)),
            "missing high-risk git push marker {arg}"
        );
    }
}
