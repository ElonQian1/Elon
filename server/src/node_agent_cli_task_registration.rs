//! Admission-coordinated active handle registration for CLI dispatch.

use anyhow::Result;
use homecli_proto::AgentToServer;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

use crate::{
    node_agent_active_task::ActiveCliPromptHandle,
    node_agent_active_task_registry::CliPromptRegistration, NodeRuntime,
};

pub(crate) async fn register(
    runtime: &NodeRuntime,
    req_id: &str,
    cli_name: &str,
    cwd: Option<String>,
    runtime_permission: Option<String>,
    cancel_tx: watch::Sender<bool>,
    requires_cloud_control: bool,
    supervised: bool,
    true_workspace_resume: bool,
    admission: Option<&crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard>,
) -> Result<CliPromptRegistration> {
    let handle = ActiveCliPromptHandle::new(
        req_id,
        cli_name,
        crate::node_agent_active_task::route_for_cli(cli_name),
        cwd,
        runtime_permission,
        cancel_tx,
    )
    .with_requires_cloud_control(requires_cloud_control)
    .with_exclusive_workspace(supervised || true_workspace_resume);
    if supervised {
        runtime
            .try_register_supervised_cli_prompt(handle, admission)
            .await
    } else {
        Ok(runtime.try_register_cli_prompt(handle).await)
    }
}

pub(crate) fn emit_ui_design_route(
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: &str,
    routed: bool,
    workspace_ready: bool,
    route_status: &str,
) {
    if !routed {
        return;
    }
    let status = if workspace_ready {
        route_status
    } else {
        "DEGRADED"
    };
    let event = serde_json::json!({ "type": "elon.ui_design.route", "status": status });
    let _ = out_tx.send(crate::node_agent_cli_prompt_runner::ws_text(
        &AgentToServer::CliChunk {
            req_id: req_id.to_string(),
            text: format!("{event}\n"),
        },
    ));
}
