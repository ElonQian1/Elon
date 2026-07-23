use anyhow::{anyhow, bail, Result};
use homecli_proto::CliProjectContext;

use super::{FullAccessGrantIdentity, FullAccessGrantState};

pub(crate) async fn require_route_a_full_access_grant_for_resolved_resume(
    grants: &FullAccessGrantState,
    identity: &FullAccessGrantIdentity,
    cli_name: &str,
    runtime_permission: Option<&str>,
    project_context: Option<&CliProjectContext>,
    cwd: Option<&str>,
    parent: &crate::node_agent_local_task_store::LocalTaskRecord,
    resolved: &crate::node_agent_local_task_resume::ResolvedResumeWorkspace,
) -> Result<()> {
    let authorization_record = resolved_resume_authorization_record(parent, resolved)?;
    super::require_route_a_full_access_grant(
        grants,
        identity,
        cli_name,
        runtime_permission,
        project_context,
        cwd,
        false,
        Some(&authorization_record),
    )
    .await
}

fn resolved_resume_authorization_record(
    parent: &crate::node_agent_local_task_store::LocalTaskRecord,
    resolved: &crate::node_agent_local_task_resume::ResolvedResumeWorkspace,
) -> Result<crate::node_agent_local_task_store::LocalTaskRecord> {
    let status = parent
        .workspace_status
        .as_ref()
        .ok_or_else(|| anyhow!("Resume 父任务缺少平台 workspace_status，已拒绝继承授权。"))?;
    let recorded_base = status
        .get("base_workspace_path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("Resume 父任务 workspace_status 缺少基础工作区。"))?;
    let recorded_active = status
        .get("active_workspace_path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("Resume 父任务 workspace_status 缺少活动 worktree。"))?;
    let resolved_base = resolved
        .inherited_workspace
        .base_workspace_path
        .as_deref()
        .ok_or_else(|| anyhow!("Resolved Resume 缺少基础工作区身份。"))?;
    let resolved_branch = resolved
        .inherited_workspace
        .branch
        .as_deref()
        .ok_or_else(|| anyhow!("Resolved Resume 缺少隔离分支身份。"))?;
    let resolved_root = resolved
        .inherited_workspace
        .supervision_root_task_id
        .as_deref()
        .ok_or_else(|| anyhow!("Resolved Resume 缺少监督根 lease 身份。"))?;
    if !resolved.inherited_workspace.isolated
        || !super::same_existing_path(recorded_base, &resolved.authorized_workspace_path)
        || !super::same_existing_path(recorded_base, resolved_base)
        || !super::same_existing_path(
            recorded_active,
            &resolved.inherited_workspace.workspace_path,
        )
        || status.get("branch").and_then(serde_json::Value::as_str) != Some(resolved_branch)
        || status
            .get("root_task_id")
            .and_then(serde_json::Value::as_str)
            != Some(resolved_root)
        || resolved.git_head.trim().is_empty()
    {
        bail!(
            "ISOLATED_WORKTREE_AUTH_MISSING: Resolved Resume 与父任务记录的工作区、分支或监督根身份不一致。"
        )
    }

    // The resolver has already validated lineage, receipt consensus and the
    // current worktree identity; the route retains its occupancy and lease
    // admission checks. Preserve every immutable parent provenance field and
    // carry only the strictly resolved current HEAD into the existing proof.
    let mut authorization_record = parent.clone();
    authorization_record
        .workspace_status
        .as_mut()
        .expect("workspace_status checked above")["git_head"] =
        serde_json::Value::String(resolved.git_head.clone());
    Ok(authorization_record)
}
