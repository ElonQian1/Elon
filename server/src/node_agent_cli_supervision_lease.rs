//! Persistent worktree lease admission for Desktop-supervised CLI tasks.

use std::path::Path;

use anyhow::{anyhow, Result};

use crate::{
    node_agent_task_journal::TaskJournal, pc_workspace_provisioner::ConversationWorkspaceResult,
};

pub(crate) fn acquire_for_task(
    task_journal: &TaskJournal,
    task_id: &str,
    supervision_protocol: Option<&str>,
    workspace: Option<&ConversationWorkspaceResult>,
) -> Result<()> {
    let Some(root_task_id) = root_task_id_for_task(task_journal, task_id, supervision_protocol)?
    else {
        return Ok(());
    };
    let Some(workspace) = workspace.filter(|workspace| workspace.isolated) else {
        return Ok(());
    };
    anyhow::ensure!(
        workspace.supervision_root_task_id.as_deref() == Some(root_task_id.as_str()),
        "prepared worktree supervision root identity does not match the durable contract"
    );
    let base = workspace
        .base_workspace_path
        .as_deref()
        .map(Path::new)
        .ok_or_else(|| anyhow!("isolated supervised worktree is missing its base repository"))?;
    crate::node_agent_supervision_worktree_lease::acquire(
        base,
        Path::new(&workspace.workspace_path),
        &root_task_id,
    )
}

pub(crate) fn root_task_id_for_task(
    task_journal: &TaskJournal,
    task_id: &str,
    supervision_protocol: Option<&str>,
) -> Result<Option<String>> {
    if supervision_protocol != Some(crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL)
    {
        return Ok(None);
    }
    let contract =
        crate::node_agent_local_task_supervision::load_supervision_contract(task_journal, task_id)?
            .ok_or_else(|| {
                anyhow!("desktop-supervised task is missing its durable supervision contract")
            })?;
    Ok(Some(
        contract
            .root_task_id
            .as_deref()
            .or(contract.parent_task_id.as_deref())
            .unwrap_or(task_id)
            .to_string(),
    ))
}
