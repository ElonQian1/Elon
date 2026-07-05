use std::sync::Arc;

use anyhow::{anyhow, Result};
use homecli_proto::{AgentToServer, CliProjectContext};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::{
    pc_prompt_acceptance::{pc_cli_accept_timeout, wait_for_pc_cli_prompt_acceptance},
    AiCliRequestMode, NativeSessionScope,
};
use crate::{
    homecli_agent::{CliPromptCancelHandle, CliPromptDispatch},
    pc_node_display::pc_node_progress_name,
    types::{AppState, WsMessage},
};

const PC_AGENT_CLI_ACCEPT_RETRY_LIMIT: usize = 1;
const PC_AGENT_CLI_ACCEPT_RECONNECT_WAIT_SECS: u64 = 45;

pub(crate) struct PcCliPromptDispatchRequest<'a> {
    pub(crate) state: &'a Arc<AppState>,
    pub(crate) tx: &'a UnboundedSender<String>,
    pub(crate) agent_id: &'a str,
    pub(crate) cli_name: &'a str,
    pub(crate) extra_args: &'a [String],
    pub(crate) cwd: Option<&'a str>,
    pub(crate) prompt: &'a str,
    pub(crate) request_mode: AiCliRequestMode,
    pub(crate) native_session_scope: Option<&'a NativeSessionScope>,
    pub(crate) lightweight_pc_chat: bool,
}

pub(crate) struct PcAcceptedCliPrompt {
    pub(crate) pc_req_id: String,
    pub(crate) rx: UnboundedReceiver<AgentToServer>,
    pub(crate) cancel_handle: CliPromptCancelHandle,
    pub(crate) first_cli_event: Option<AgentToServer>,
}

pub(crate) async fn dispatch_pc_cli_prompt_until_accepted(
    request: PcCliPromptDispatchRequest<'_>,
) -> Result<PcAcceptedCliPrompt> {
    let mut accept_retry_count = 0usize;
    let node_progress_name = pc_node_progress_name(request.state.as_ref(), request.agent_id).await;
    loop {
        let previous_connected_at = agent_connected_at(request.state, request.agent_id).await;
        let dispatch = dispatch_pc_cli_prompt_once(&request).await?;
        let (pc_req_id, mut rx, cancel_handle) = dispatch.into_parts();
        match wait_for_pc_cli_prompt_acceptance(
            request.state,
            request.agent_id,
            &node_progress_name,
            &pc_req_id,
            request.cli_name,
            &mut rx,
        )
        .await
        {
            Ok(first_cli_event) => {
                return Ok(PcAcceptedCliPrompt {
                    pc_req_id,
                    rx,
                    cancel_handle,
                    first_cli_event,
                });
            }
            Err(error)
                if pc_cli_accept_timeout(&error).is_some()
                    && accept_retry_count < PC_AGENT_CLI_ACCEPT_RETRY_LIMIT =>
            {
                let timeout_secs = pc_cli_accept_timeout(&error)
                    .map(|timeout| timeout.timeout_secs())
                    .unwrap_or(0);
                accept_retry_count += 1;
                if !request.lightweight_pc_chat {
                    let _ = request.tx.send(
                        WsMessage::progress(format!(
                            "PC 节点 {node_progress_name} 连接疑似假在线：{} 秒内未确认接收请求，已关闭旧连接，等待节点重新注册后自动重派（{}/{}）。",
                            timeout_secs,
                            accept_retry_count,
                            PC_AGENT_CLI_ACCEPT_RETRY_LIMIT
                        ))
                        .to_json(),
                    );
                }
                if !wait_for_agent_reconnect(&request, previous_connected_at).await {
                    return Err(anyhow!(
                        "PC 节点 {} 在 {} 秒内没有重新连接；本轮已停止。请重启一龙 PC 节点客户端后重发。",
                        node_progress_name,
                        PC_AGENT_CLI_ACCEPT_RECONNECT_WAIT_SECS
                    ));
                }
                if !request.lightweight_pc_chat {
                    let _ = request.tx.send(
                        WsMessage::progress("PC 节点已重新连接，正在重派本轮请求。").to_json(),
                    );
                }
            }
            Err(error) => return Err(error),
        }
    }
}

async fn dispatch_pc_cli_prompt_once(
    request: &PcCliPromptDispatchRequest<'_>,
) -> Result<CliPromptDispatch> {
    let mut last_err = anyhow!("dispatch failed");
    let mut result = Err(last_err);
    let max_attempts = if request.lightweight_pc_chat { 3 } else { 25 };
    for attempt in 0..max_attempts {
        let project_context = request.native_session_scope.map(|scope| CliProjectContext {
            project_id: scope.project_id.clone(),
            conversation_id: scope.conversation_id.clone(),
            runtime_permission: Some(if request.request_mode.is_plan() {
                "read_only".to_string()
            } else {
                scope.runtime_permission.clone()
            }),
        });
        match request
            .state
            .agent_manager
            .dispatch_cli_prompt_with_context_control(
                request.agent_id,
                request.cli_name.to_string(),
                request.extra_args.to_vec(),
                request.cwd.map(ToOwned::to_owned),
                project_context,
                request.prompt.to_string(),
            )
            .await
        {
            Ok(dispatch) => {
                result = Ok(dispatch);
                break;
            }
            Err(e) => {
                last_err = e;
                let msg = last_err.to_string();
                let is_offline = msg.contains("agent not connected");
                if is_offline && attempt + 1 < max_attempts {
                    let wait = format!(
                        "PC 节点短暂离线，等待重连（{}/{}）…",
                        attempt + 1,
                        max_attempts
                    );
                    if !request.lightweight_pc_chat {
                        let _ = request.tx.send(WsMessage::progress(wait).to_json());
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                } else {
                    result = Err(last_err);
                    break;
                }
            }
        }
    }
    result
}

async fn agent_connected_at(state: &Arc<AppState>, agent_id: &str) -> Option<u64> {
    state
        .agent_manager
        .list()
        .await
        .into_iter()
        .find(|agent| agent.agent_id == agent_id)
        .map(|agent| agent.connected_at)
}

async fn wait_for_agent_reconnect(
    request: &PcCliPromptDispatchRequest<'_>,
    previous_connected_at: Option<u64>,
) -> bool {
    let mut waited_secs = 0u64;
    while waited_secs < PC_AGENT_CLI_ACCEPT_RECONNECT_WAIT_SECS {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        waited_secs += 2;
        if let Some(connected_at) = agent_connected_at(request.state, request.agent_id).await {
            if previous_connected_at.is_none_or(|previous| connected_at > previous) {
                return true;
            }
        }
        if !request.lightweight_pc_chat && waited_secs % 10 == 0 {
            let _ = request.tx.send(
                WsMessage::progress(format!(
                    "仍在等待 PC 节点重新连接（已等待 {}s / {}s）。",
                    waited_secs, PC_AGENT_CLI_ACCEPT_RECONNECT_WAIT_SECS
                ))
                .to_json(),
            );
        }
    }
    false
}
