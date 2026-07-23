use super::tests::{git, grant_file, identity, managed_workspace_record, temp_workspace};
use super::*;

#[tokio::test]
async fn resolved_resume_authorizes_current_head_but_rejects_an_arbitrary_worktree() {
    let (base, active, stale_parent) = managed_workspace_record("resolved-resume-stale-head");
    let state = FullAccessGrantState::load_from_path(grant_file("resolved-resume-stale-head"));
    let identity = identity("owner-1", "agent-1", "install-1");
    state
        .grant_project(&identity, "project_1", base.to_string_lossy().as_ref())
        .await
        .unwrap();

    let stale_head = stale_parent.workspace_status.as_ref().unwrap()["git_head"]
        .as_str()
        .unwrap()
        .to_string();
    std::fs::write(active.join("resume-current.txt"), "trusted receipt head").unwrap();
    git(&active, &["add", "resume-current.txt"]);
    git(&active, &["commit", "-m", "advance trusted resume head"]);
    let current_head = git(&active, &["rev-parse", "HEAD"]);
    assert_ne!(stale_head, current_head);

    let context = CliProjectContext {
        project_id: "project_1".into(),
        conversation_id: stale_parent.conversation_id.clone(),
        runtime_permission: Some("full_access".into()),
    };
    let mut resolved = crate::node_agent_local_task_resume::ResolvedResumeWorkspace {
        authorized_workspace_path: base.to_string_lossy().into_owned(),
        inherited_workspace: crate::pc_workspace_provisioner::ConversationWorkspaceResult {
            base_workspace_path: Some(base.to_string_lossy().into_owned()),
            workspace_path: active.to_string_lossy().into_owned(),
            isolated: true,
            branch: Some("ai/session/project/root".into()),
            supervision_root_task_id: Some("local-root".into()),
        },
        derivation: "workspace_status".into(),
        git_head: current_head,
        requires_recreation: false,
        snapshot_continue_required: false,
        orphaned_migration: None,
        lease_migration: None,
        resume_admission: None,
    };

    let foreign_base = temp_workspace("resolved-resume-foreign-grant");
    let wrong_grant =
        FullAccessGrantState::load_from_path(grant_file("resolved-resume-foreign-grant"));
    wrong_grant
        .grant_project(
            &identity,
            "project_1",
            foreign_base.to_string_lossy().as_ref(),
        )
        .await
        .unwrap();
    require_route_a_full_access_grant_for_resolved_resume(
        &wrong_grant,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context),
        Some(active.to_string_lossy().as_ref()),
        &stale_parent,
        &resolved,
    )
    .await
    .expect_err("the resolved worktree must still require its authoritative base grant");

    require_route_a_full_access_grant_for_resolved_resume(
        &state,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context),
        Some(active.to_string_lossy().as_ref()),
        &stale_parent,
        &resolved,
    )
    .await
    .expect("strictly resolved current HEAD should authorize the recorded worktree");

    let arbitrary = base.parent().unwrap().join("arbitrary-worktree");
    git(
        &base,
        &[
            "worktree",
            "add",
            "-b",
            "ai/session/project/arbitrary",
            arbitrary.to_string_lossy().as_ref(),
        ],
    );
    resolved.inherited_workspace.workspace_path = arbitrary.to_string_lossy().into_owned();
    let error = require_route_a_full_access_grant_for_resolved_resume(
        &state,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context),
        Some(arbitrary.to_string_lossy().as_ref()),
        &stale_parent,
        &resolved,
    )
    .await
    .expect_err("an arbitrary worktree must not inherit the base project grant");
    assert!(
        error.to_string().contains("ISOLATED_WORKTREE_AUTH_MISSING"),
        "{error:#}"
    );

    let _ = std::fs::remove_dir_all(foreign_base);
    let _ = std::fs::remove_dir_all(base.parent().unwrap());
}
