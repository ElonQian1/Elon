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
    if supervision_protocol != Some(crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL)
    {
        return Ok(());
    }
    let contract =
        crate::node_agent_local_task_supervision::load_supervision_contract(task_journal, task_id)?
            .ok_or_else(|| {
                anyhow!("desktop-supervised task is missing its durable supervision contract")
            })?;
    let Some(workspace) = workspace.filter(|workspace| workspace.isolated) else {
        return Ok(());
    };
    let root_task_id = contract
        .root_task_id
        .as_deref()
        .or(contract.parent_task_id.as_deref())
        .unwrap_or(task_id);
    let base = workspace
        .base_workspace_path
        .as_deref()
        .map(Path::new)
        .ok_or_else(|| anyhow!("isolated supervised worktree is missing its base repository"))?;
    crate::node_agent_supervision_worktree_lease::acquire(
        base,
        Path::new(&workspace.workspace_path),
        root_task_id,
    )
}
