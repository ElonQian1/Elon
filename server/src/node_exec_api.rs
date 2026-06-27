//! POST /api/me/node/exec — 把用户消息直接分发给在线 PC 节点上的 AI CLI 执行，
//! 并把执行输出同步返回。这让 /pc/ai 聊天页能真正运行本机命令，而不只是文字引导。

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use homecli_proto::AgentToServer;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

const EXEC_TIMEOUT_SECS: u64 = 120;

#[derive(Deserialize)]
pub struct NodeExecRequest {
    /// 用户发送的自然语言消息（或 shell 命令）
    pub prompt: String,
    /// 可选：指定使用哪个 node_id；不传则自动选第一个在线且有 CLI 的节点
    pub node_id: Option<String>,
}

#[derive(Serialize)]
pub struct NodeExecResponse {
    pub output: String,
    pub req_id: String,
    pub node_id: String,
    pub node_display_name: String,
    pub exit_ok: bool,
    pub error: Option<String>,
}

/// POST /api/me/node/exec
pub async fn node_exec_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<NodeExecRequest>,
) -> impl IntoResponse {
    // ── 1. 鉴权 ──────────────────────────────────────────────────────────────
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    if req.prompt.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "prompt 不能为空");
    }

    // ── 2. 找到用户可用的在线 PC 节点 ────────────────────────────────────────
    let agents = state.agent_manager.list().await;

    // 找属于该用户且有 CLI 可用的在线 agent；取出我们需要的字段避免 move 问题
    struct PickedAgent {
        agent_id: String,
        device_name: Option<String>,
        allowed_clis: Vec<String>,
    }

    let agent = if let Some(ref node_id) = req.node_id {
        let found = agents.iter().find(|a| a.agent_id == *node_id);
        match found {
            Some(a) => {
                match state.store.get_node_credential_owner(&a.agent_id) {
                    Ok(Some(owner)) if owner == user.id => PickedAgent {
                        agent_id: a.agent_id.clone(),
                        device_name: a.device_name.clone(),
                        allowed_clis: a.allowed_clis.clone(),
                    },
                    _ => {
                        return json_error(
                            StatusCode::FORBIDDEN,
                            "指定的节点不属于当前账号或不在线",
                        )
                    }
                }
            }
            None => return json_error(StatusCode::NOT_FOUND, "指定的节点未在线"),
        }
    } else {
        // 自动选：找第一个属于当前用户且有 CLI 的在线 agent
        let mut picked: Option<PickedAgent> = None;
        for a in &agents {
            if a.allowed_clis.is_empty() {
                continue;
            }
            if let Ok(Some(owner)) = state.store.get_node_credential_owner(&a.agent_id) {
                if owner == user.id {
                    picked = Some(PickedAgent {
                        agent_id: a.agent_id.clone(),
                        device_name: a.device_name.clone(),
                        allowed_clis: a.allowed_clis.clone(),
                    });
                    break;
                }
            }
        }
        // 退而选第一个有 CLI 的公共 agent（向后兼容）
        if picked.is_none() {
            if let Some(a) = agents.iter().find(|a| !a.allowed_clis.is_empty()) {
                picked = Some(PickedAgent {
                    agent_id: a.agent_id.clone(),
                    device_name: a.device_name.clone(),
                    allowed_clis: a.allowed_clis.clone(),
                });
            }
        }
        match picked {
            Some(a) => a,
            None => {
                return json_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "没有可用的在线 PC 节点。请先在你的 Windows 电脑上启动「一龙 PC 节点」。",
                )
            }
        }
    };

    // ── 3. 选择 CLI（优先 copilot > codex > claude > gemini > 第一个）────────
    let cli = pick_best_cli(&agent.allowed_clis);

    let agent_id = agent.agent_id.clone();
    let display_name = agent
        .device_name
        .clone()
        .unwrap_or_else(|| agent_id[..8.min(agent_id.len())].to_string());

    // ── 4. 分发给 PC 节点执行 ──────────────────────────────────────────────
    let (req_id, mut rx) = match state
        .agent_manager
        .dispatch_cli_prompt(&agent_id, cli, vec![], req.prompt)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                format!("分发到 PC 节点失败：{e}"),
            )
        }
    };

    // ── 5. 收集流式输出 ────────────────────────────────────────────────────
    let mut output = String::new();
    let mut exit_ok = false;
    let mut exec_error: Option<String> = None;

    loop {
        match timeout(Duration::from_secs(EXEC_TIMEOUT_SECS), rx.recv()).await {
            Ok(Some(AgentToServer::CliChunk { text, .. })) => {
                output.push_str(&text);
            }
            Ok(Some(AgentToServer::CliDone {
                exit_ok: ok,
                error: e,
                ..
            })) => {
                exit_ok = ok;
                exec_error = e;
                break;
            }
            Ok(Some(_)) => { /* 忽略其他消息类型 */ }
            Ok(None) => {
                exec_error = Some("节点连接已关闭".to_string());
                break;
            }
            Err(_) => {
                exec_error = Some(format!("执行超时（{}s）", EXEC_TIMEOUT_SECS));
                break;
            }
        }
    }

    axum::Json(NodeExecResponse {
        output,
        req_id,
        node_id: agent_id,
        node_display_name: display_name,
        exit_ok,
        error: exec_error,
    })
    .into_response()
}

/// 从允许的 CLI 列表中选最优的
/// Codex 优先：它的 --dangerously-bypass-approvals-and-sandbox 专门为无 TTY 场景设计。
/// Copilot 的 shell 工具需要 TTY，在后台进程中即使有 --allow-all 也会被拒绝。
fn pick_best_cli(allowed_clis: &[String]) -> String {
    let priority = ["codex", "claude", "gemini", "copilot"];
    for preferred in priority {
        if allowed_clis
            .iter()
            .any(|c| c.eq_ignore_ascii_case(preferred))
        {
            return preferred.to_string();
        }
    }
    allowed_clis
        .first()
        .cloned()
        .unwrap_or_else(|| "codex".to_string())
}
