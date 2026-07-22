//! Durable local-task completion binding for CLI output consumers.

use std::sync::Arc;

use homecli_proto::{AgentToServer, CliCompletionEnvelope};
use tokio_tungstenite::tungstenite::Message;

use crate::NodeRuntime;

pub(crate) fn spawn_local_output_consumer(
    runtime: Arc<NodeRuntime>,
    owner_user_id: String,
    task_id: String,
    mut out_rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
) {
    tokio::spawn(async move {
        while let Some(message) = out_rx.recv().await {
            let Message::Text(text) = message else {
                continue;
            };
            let Ok(message) = serde_json::from_str::<AgentToServer>(&text) else {
                continue;
            };
            match message {
                AgentToServer::CliDone { req_id, .. } if req_id == task_id => {
                    let completion = match durable_completion_for_local_display(
                        &runtime.completion_outbox,
                        &req_id,
                    ) {
                        Ok(Some(completion)) => completion,
                        Ok(None) => {
                            tracing::warn!(
                                %task_id,
                                "received local CliDone without durable outbox row; leaving terminal state to durable producer/startup repair"
                            );
                            break;
                        }
                        Err(error) => {
                            tracing::warn!(%task_id, %error, "failed to read durable local completion");
                            break;
                        }
                    };
                    if let Err(error) = runtime.local_tasks.finish(&owner_user_id, &completion) {
                        tracing::warn!(%task_id, %error, "failed to persist local task completion");
                    }
                    break;
                }
                _ => {}
            }
        }
    });
}

pub(super) fn durable_completion_for_local_display(
    outbox: &crate::node_agent_completion_outbox::CliCompletionOutbox,
    req_id: &str,
) -> anyhow::Result<Option<CliCompletionEnvelope>> {
    outbox.latest_for_req_id(req_id)
}
