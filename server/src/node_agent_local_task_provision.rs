//! Shared production provision -> journal -> local-record transaction boundary.

use anyhow::{Context, Result};
use homecli_proto::CliProjectContext;

use crate::{
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStart, LocalTaskStore},
    node_agent_local_task_supervision::{
        contract_payload, record_supervision_event, SupervisionContract,
    },
    node_agent_task_journal::TaskJournal,
    pc_workspace_provisioner::ConversationWorkspaceResult,
};

pub(crate) struct SupervisedLocalTaskProvision<'a> {
    pub task_id: &'a str,
    pub owner_user_id: &'a str,
    pub agent_id: &'a str,
    pub install_id: &'a str,
    pub project_id: &'a str,
    pub channel_id: Option<&'a str>,
    pub conversation_id: &'a str,
    pub base_workspace_path: &'a str,
    pub prompt: &'a str,
    pub runtime_permission: &'a str,
    pub root_task_id: &'a str,
    pub contract: &'a SupervisionContract,
}

pub(crate) fn provision_and_record_supervised_task(
    data_paths: Option<&elon_pc_dev_runtime::NodeDataPaths>,
    local_tasks: &LocalTaskStore,
    journal: &TaskJournal,
    request: SupervisedLocalTaskProvision<'_>,
) -> Result<(LocalTaskRecord, ConversationWorkspaceResult)> {
    let prepared = crate::node_agent_cli_runner::prepare_cli_prompt_cwd_in_with_supervision(
        data_paths,
        Some(request.base_workspace_path.to_string()),
        Some(CliProjectContext {
            project_id: request.project_id.to_string(),
            conversation_id: request.conversation_id.to_string(),
            runtime_permission: Some(request.runtime_permission.to_string()),
        }),
        Some(request.root_task_id),
    )?;
    let workspace = prepared
        .conversation_workspace
        .context("supervised task did not receive an isolated worktree")?;
    let record = record_provisioned_supervised_task(local_tasks, journal, &workspace, &request)?;
    Ok((record, workspace))
}

pub(crate) fn provision_record_and_dispatch_supervised_task(
    data_paths: Option<&elon_pc_dev_runtime::NodeDataPaths>,
    local_tasks: &LocalTaskStore,
    journal: &TaskJournal,
    request: SupervisedLocalTaskProvision<'_>,
    dispatch: impl FnOnce(&LocalTaskRecord, &ConversationWorkspaceResult) -> Result<()>,
) -> Result<(LocalTaskRecord, ConversationWorkspaceResult)> {
    let (record, workspace) =
        provision_and_record_supervised_task(data_paths, local_tasks, journal, request)?;
    dispatch(&record, &workspace)?;
    Ok((record, workspace))
}

pub(super) fn record_provisioned_supervised_task(
    local_tasks: &LocalTaskStore,
    journal: &TaskJournal,
    workspace: &ConversationWorkspaceResult,
    request: &SupervisedLocalTaskProvision<'_>,
) -> Result<LocalTaskRecord> {
    anyhow::ensure!(
        workspace.isolated,
        "supervised task worktree is not isolated"
    );
    anyhow::ensure!(
        !crate::node_agent_update_checkpoint::same_path(
            std::path::Path::new(request.base_workspace_path),
            std::path::Path::new(&workspace.workspace_path),
        ),
        "supervised task execution path equals its base workspace"
    );
    record_supervision_event(
        journal,
        request.task_id,
        "supervision_contract",
        contract_payload(request.contract),
    )?;
    let record = local_tasks.create(LocalTaskStart {
        task_id: request.task_id,
        owner_user_id: request.owner_user_id,
        agent_id: request.agent_id,
        install_id: request.install_id,
        project_id: request.project_id,
        channel_id: request.channel_id,
        conversation_id: request.conversation_id,
        workspace_path: &workspace.workspace_path,
        prompt: request.prompt,
        cli: "codex",
        runtime_permission: request.runtime_permission,
    })?;
    anyhow::ensure!(
        crate::node_agent_update_checkpoint::same_path(
            std::path::Path::new(&record.workspace_path),
            std::path::Path::new(&workspace.workspace_path),
        ),
        "local task record did not preserve the provisioned execution worktree"
    );
    Ok(record)
}
