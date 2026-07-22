//! Durable rejection for failures discovered before the CLI starts.

use homecli_proto::AgentToServer;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use crate::{node_agent_cli_done::CliCompletionContext, NodeRuntime};

pub(super) async fn send_preflight_failure(
    runtime: &NodeRuntime,
    completion_context: &CliCompletionContext,
    cli_name: &str,
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: String,
    error: String,
) {
    let message = AgentToServer::CliDone {
        req_id,
        exit_ok: false,
        error: Some(error),
        session_id: None,
        prompt_tokens: None,
        cached_input_tokens: None,
        completion_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        model: None,
        workspace_status: None,
    };
    if let Err(error) = crate::node_agent_cli_done::persist_and_send_cli_done(
        runtime,
        completion_context,
        cli_name,
        None,
        message,
        out_tx,
    )
    .await
    {
        warn!(%error, "failed to persist preflight CLI completion");
    }
}
