use std::{fmt, sync::Arc};

use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::types::AppState;

const PC_AGENT_CLI_ACCEPT_TIMEOUT_ENV: &str = "ELON_PC_AGENT_CLI_ACCEPT_TIMEOUT_SECS";
const PC_AGENT_CLI_ACCEPT_TIMEOUT_DEFAULT_SECS: u64 = 15;

#[derive(Debug)]
pub(crate) struct PcCliPromptAcceptTimeout {
    agent_id: String,
    node_label: String,
    cli_name: String,
    timeout_secs: u64,
}

impl PcCliPromptAcceptTimeout {
    fn new(agent_id: &str, node_label: &str, cli_name: &str, timeout_secs: u64) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            node_label: node_label.to_string(),
            cli_name: cli_name.to_string(),
            timeout_secs,
        }
    }

    pub(crate) fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }
}

impl fmt::Display for PcCliPromptAcceptTimeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node_label = self.node_label.trim();
        let node_label = if node_label.is_empty() {
            self.agent_id.as_str()
        } else {
            node_label
        };
        write!(
            f,
            "PC 节点 {} 在 {} 秒内没有确认接收 {} 请求；本轮已停止。通常是节点连接假在线、旧节点进程未更新、节点正在重连，或 WebSocket 写通道没有真正送达本机。请重启一龙 PC 节点客户端后重发。",
            node_label, self.timeout_secs, self.cli_name
        )
    }
}

impl std::error::Error for PcCliPromptAcceptTimeout {}

pub(crate) fn pc_cli_accept_timeout(error: &anyhow::Error) -> Option<&PcCliPromptAcceptTimeout> {
    error.downcast_ref::<PcCliPromptAcceptTimeout>()
}

pub(crate) async fn wait_for_pc_cli_prompt_acceptance(
    state: &Arc<AppState>,
    agent_id: &str,
    node_label: &str,
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
        Ok(None) => {
            let node_label = node_label.trim();
            let node_label = if node_label.is_empty() {
                agent_id
            } else {
                node_label
            };
            Err(anyhow!(
                "PC 节点 {node_label} 的 CLI 通道在确认接收 {cli_name} 请求前已断开；请重启一龙 PC 节点客户端后重发。"
            ))
        }
        Err(_) => {
            let _ = state
                .agent_manager
                .close_agent_session(agent_id, "CLI prompt accept timeout")
                .await;
            Err(PcCliPromptAcceptTimeout::new(agent_id, node_label, cli_name, timeout_secs).into())
        }
    }
}

pub(crate) fn pc_lightweight_no_node_event_diagnostic(
    cli_name: &str,
    node_label: &str,
    timeout_secs: u64,
) -> String {
    let node_label = node_label.trim();
    let node_label = if node_label.is_empty() {
        "未知节点"
    } else {
        node_label
    };
    format!(
        "PC 节点 {} 已确认接收 {} 请求，但 {} 秒内没有返回任何 CLI 输出或完成事件；本轮已停止。请检查本机 Codex 是否卡在登录、网络、代理或插件同步阶段。",
        node_label,
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

#[cfg(test)]
mod tests {
    use super::{pc_cli_accept_timeout, PcCliPromptAcceptTimeout};

    #[test]
    fn accept_timeout_error_can_be_downcast_for_retry() {
        let error: anyhow::Error =
            PcCliPromptAcceptTimeout::new("node-a", "一龙4060（node-a）", "codex", 15).into();

        let timeout = pc_cli_accept_timeout(&error).expect("typed timeout");

        assert_eq!(timeout.timeout_secs(), 15);
        assert!(error
            .to_string()
            .contains("PC 节点 一龙4060（node-a） 在 15 秒内没有确认接收 codex 请求"));
    }
}
