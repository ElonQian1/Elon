//! Dispatch assembly for an already persisted local task record.

use std::sync::Arc;

use homecli_proto::{CliCompletionProducerIdentity, CliProjectContext};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    node_agent_cli_task_dispatch::{spawn_cli_task, CliTaskDispatchRequest},
    NodeRuntime,
};

pub(crate) fn dispatch_local_task_record(
    runtime: Arc<NodeRuntime>,
    record: &crate::node_agent_local_task_store::LocalTaskRecord,
    executor_prompt: String,
    execution_workspace_path: String,
    supervision: Option<&crate::node_agent_local_task_supervision::SupervisionContract>,
    inherited_workspace: Option<crate::pc_workspace_provisioner::ConversationWorkspaceResult>,
    resume_admission: Option<crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard>,
    inherited_authorization_record: Option<crate::node_agent_local_task_store::LocalTaskRecord>,
    frozen_codex_home: crate::node_agent_codex_child_env::FrozenCodexHome,
) {
    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    super::spawn_local_output_consumer(
        runtime.clone(),
        record.owner_user_id.clone(),
        record.task_id.clone(),
        out_rx,
    );
    spawn_cli_task(
        runtime.clone(),
        out_tx,
        CliTaskDispatchRequest {
            req_id: record.task_id.clone(),
            cli: "codex".to_string(),
            extra_args: Vec::new(),
            cwd: Some(execution_workspace_path),
            project_context: Some(CliProjectContext {
                project_id: record.project_id.clone(),
                conversation_id: record.conversation_id.clone(),
                runtime_permission: Some(record.runtime_permission.clone()),
            }),
            codex_credential_binding: None,
            requires_cloud_control: false,
            cloud_control_deadline: None,
            cloud_control_issued_at: None,
            cloud_control_ttl_ms: None,
            prompt: executor_prompt,
            completion_context: crate::node_agent_cli_done::CliCompletionContext::local_offline(
                CliCompletionProducerIdentity {
                    owner_user_id: record.owner_user_id.clone(),
                    agent_id: record.agent_id.clone(),
                    install_id: record.install_id.clone(),
                },
                CliProjectContext {
                    project_id: record.project_id.clone(),
                    conversation_id: record.conversation_id.clone(),
                    runtime_permission: Some(record.runtime_permission.clone()),
                },
                record.channel_id.clone(),
                record.prompt.clone(),
                supervision.map(|contract| contract.protocol.clone()),
            ),
            inherited_workspace,
            resume_admission,
            inherited_authorization_record,
            allow_codex_auth_switch: false,
            frozen_codex_home: Some(frozen_codex_home),
        },
    );
}
