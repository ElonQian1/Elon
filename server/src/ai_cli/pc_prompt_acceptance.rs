use std::sync::Arc;

use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::types::AppState;

const PC_AGENT_CLI_ACCEPT_TIMEOUT_ENV: &str = "ELON_PC_AGENT_CLI_ACCEPT_TIMEOUT_SECS";
const PC_AGENT_CLI_ACCEPT_TIMEOUT_DEFAULT_SECS: u64 = 15;

pub(crate) async fn wait_for_pc_cli_prompt_acceptance(
    state: &Arc<AppState>,
    agent_id: &str,
    pc_req_id: &str,
    cli_name: &str,
    rx: &mut UnboundedReceiver<AgentToServer>,
) -> Result<Option<AgentToServer>> {
    let timeout_secs = pc_agent_cli_accept_timeout_secs();
    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx.recv()).await {
        Ok(Some(AgentToServer::CliPromptAccepted { req_id, .. })) if req_id == pc_req_id => {
            Ok(None)
        }
        Ok(Some(AgentToServer::CliPromptAccepted { req_id, .. })) => Err(anyhow!(
            "PC 节点返回了不匹配的 CLI 接收确认：期望 {pc_req_id}，实际 {req_id}"
        )),
        Ok(Some(event)) => Ok(Some(event)),
        Ok(None) => Err(anyhow!(
            "PC 节点 {agent_id} 的 CLI 通道在确认接收 {cli_name} 请求前已断开；请重启一龙 PC 节点客户端后重发。"
        )),
        Err(_) => {
            let _ = state
                .agent_manager
                .close_agent_session(agent_id, "CLI prompt accept timeout")
                .await;
            Err(anyhow!(
                "PC 节点 {agent_id} 在 {timeout_secs} 秒内没有确认接收 {cli_name} 请求；本轮已停止。通常是节点连接假在线、旧节点进程未更新、节点正在重连，或 WebSocket 写通道没有真正送达本机。请重启一龙 PC 节点客户端后重发。"
            ))
        }
    }
}

pub(crate) fn pc_lightweight_no_node_event_diagnostic(
    cli_name: &str,
    agent_id: &str,
    timeout_secs: u64,
) -> String {
    format!(
        "PC 节点 {} 已确认接收 {} 请求，但 {} 秒内没有返回任何 CLI 输出或完成事件；本轮已停止。请检查本机 Codex 是否卡在登录、网络、代理或插件同步阶段。",
        agent_id,
        pc_cli_progress_label(cli_name),
        timeout_secs
    )
}

fn pc_agent_cli_accept_timeout_secs() -> u64 {
    std::env::var(PC_AGENT_CLI_ACCEPT_TIMEOUT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(PC_AGENT_CLI_ACCEPT_TIMEOUT_DEFAULT_SECS)
        .clamp(5, 120)
}

fn pc_cli_progress_label(cli_name: &str) -> &'static str {
    match cli_name {
        "codex" => "Codex",
        "copilot" => "Copilot",
        "claude" => "Claude",
        "gemini" => "Gemini",
        "api-runtime" => "Route B",
        "server-runtime" => "Route C",
        _ => "PC AI",
    }
}
