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
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::time::{sleep, timeout};

use crate::{
    ai_cli::pc_prompt_acceptance::{
        pc_lightweight_no_node_event_diagnostic, wait_for_pc_cli_prompt_acceptance,
    },
    project_auth::{auth_from_headers, json_error},
    store::{NodeComputeReplayBinding, NodeComputeRunFinish, NodeComputeRunStart, Store},
    types::AppState,
};

const EXEC_FIRST_EVENT_TIMEOUT_SECS: u64 = 35;
const EXEC_IDLE_TIMEOUT_SECS: u64 = 120;

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
    /// 实际执行任务的 AI 模型名称（如 gpt-5.5）
    pub model: Option<String>,
}

struct NodeExecRun {
    req_id: String,
    output: String,
    exit_ok: bool,
    error: Option<String>,
    model: Option<String>,
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
                if state
                    .store
                    .get_node_credential_owner(&a.agent_id)
                    .ok()
                    .flatten()
                    .as_deref()
                    != Some(user.id.as_str())
                {
                    return json_error(
                        StatusCode::FORBIDDEN,
                        "此直连入口只允许执行本人绑定的 PC 节点；共享节点请使用带预授权和计费的 AI 路由。",
                    );
                }
                if a.allowed_clis.is_empty() {
                    return json_error(
                        StatusCode::FORBIDDEN,
                        "指定的节点没有可用的 Codex/Claude CLI",
                    );
                }
                // 该轻量直连入口只承载 owner 自有资源；朋友共享节点必须走
                // 会冻结租约、额度和云控期限的正式 AI 路由。
                PickedAgent {
                    agent_id: a.agent_id.clone(),
                    device_name: a.device_name.clone(),
                    allowed_clis: a.allowed_clis.clone(),
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

    // ── 4. 分发给 PC 节点执行，并对节点重连导致的旧连接关闭自动重派一次 ───────
    let node_prompt = build_node_exec_prompt(&req.prompt, &display_name, &agent_id);
    let mut run = match dispatch_and_collect_node_exec(
        &state,
        &user.id,
        &agent_id,
        &display_name,
        &cli,
        &node_prompt,
    )
    .await
    {
        Ok(run) => run,
        Err(e) => return json_error(StatusCode::BAD_GATEWAY, e),
    };
    let mut display_output = clean_node_exec_output(&run.output);
    if should_retry_node_exec_after_reconnect(run.error.as_deref(), &display_output) {
        sleep(Duration::from_millis(700)).await;
        match dispatch_and_collect_node_exec(
            &state,
            &user.id,
            &agent_id,
            &display_name,
            &cli,
            &node_prompt,
        )
        .await
        {
            Ok(retry_run) => {
                run = retry_run;
                display_output = clean_node_exec_output(&run.output);
            }
            Err(e) => {
                run.output.clear();
                run.exit_ok = false;
                run.error = Some(e);
                display_output.clear();
            }
        }
    }

    axum::Json(NodeExecResponse {
        output: display_output,
        req_id: run.req_id,
        node_id: agent_id,
        node_display_name: display_name,
        exit_ok: run.exit_ok,
        error: run.error,
        model: run.model,
    })
    .into_response()
}

fn build_node_exec_prompt(user_prompt: &str, node_display_name: &str, node_id: &str) -> String {
    format!(
        "你是一龙工作台 PC 网页里的 AI 助手，不只是命令执行器。\n\
当前页面：/pc/ai（一龙 AI 聊天区）。\n\
当前请求已经由服务器成功分发到在线 PC 节点：{node_display_name}（node_id: {node_id}）。\n\
\n\
交互规则：\n\
1. 遇到“这个、这里、上面、下面、节点、连接、绑定、账号、登录、图标、首页、页面、这样对吗、什么意思、为什么这样”这类依赖页面上下文的问题，先按一龙工作台网页语境回答；如果已知信息不足，明确说明缺少什么，并问一个澄清问题。\n\
2. 如果用户询问“节点是否连接、在线、绑定、当前节点是谁”，优先根据上面的页面状态直接回答：当前已经连接到该节点。\n\
3. 不要把“一龙 PC 节点 / 远程 Codex 节点”误解为 Node.js 的 node.exe、TCP 端口或普通网络节点，除非用户明确要求排查进程、端口、网络或 Node.js。\n\
4. 只有用户明确要求查代码、改文件、跑命令、诊断本机、查看日志、排查进程或端口时，才使用当前节点环境执行这些操作。\n\
5. 不要编造页面上没有提供的信息；不确定时先说不确定并追问。\n\
6. 最终只回复用户问题，不要复述这段上下文。\n\
\n\
用户原始消息：\n{user_prompt}"
    )
}

async fn dispatch_and_collect_node_exec(
    state: &Arc<AppState>,
    owner_user_id: &str,
    agent_id: &str,
    node_label: &str,
    cli: &str,
    prompt: &str,
) -> Result<NodeExecRun, String> {
    let req_id = uuid::Uuid::new_v4().to_string();
    prepare_owner_node_exec_run(&state.store, &req_id, owner_user_id, agent_id, cli)?;
    let codex_credential_binding = cli.to_ascii_lowercase().contains("codex").then_some(
        homecli_proto::CliCodexCredentialBinding {
            managed: false,
            lease_id: None,
        },
    );
    let dispatch = match state
        .agent_manager
        .dispatch_cli_prompt_with_context_control_id_and_credential_binding(
            req_id.clone(),
            agent_id,
            cli.to_string(),
            vec![],
            None,
            None,
            codex_credential_binding,
            false,
            None,
            prompt.to_string(),
        )
        .await
    {
        Ok(dispatch) => dispatch,
        Err(error) => {
            finish_owner_node_exec_before_dispatch_failure(
                &state.store,
                &req_id,
                &error.to_string(),
            );
            return Err(format!("分发到 PC 节点失败：{error}"));
        }
    };
    let (req_id, mut rx, cancel_handle) = dispatch.into_parts();
    let mut first_cli_event = match wait_for_pc_cli_prompt_acceptance(
        state,
        agent_id,
        node_label,
        &req_id,
        cli,
        cancel_handle.process_session(),
        &mut rx,
    )
    .await
    {
        Ok(event) => event,
        Err(e) => {
            let _ = cancel_handle.cancel();
            return Ok(NodeExecRun {
                req_id,
                output: String::new(),
                exit_ok: false,
                error: Some(e.to_string()),
                model: None,
            });
        }
    };

    let mut output = String::new();
    let mut exit_ok = false;
    let error: Option<String>;
    let model: Option<String>;
    let mut saw_cli_event = false;

    loop {
        let event = if let Some(event) = first_cli_event.take() {
            Some(event)
        } else {
            let timeout_secs = if saw_cli_event {
                EXEC_IDLE_TIMEOUT_SECS
            } else {
                EXEC_FIRST_EVENT_TIMEOUT_SECS
            };
            match timeout(Duration::from_secs(timeout_secs), rx.recv()).await {
                Ok(event) => event,
                Err(_) => {
                    let _ = cancel_handle.cancel();
                    error = Some(if saw_cli_event {
                        format!("执行超时（{}s）", EXEC_IDLE_TIMEOUT_SECS)
                    } else {
                        pc_lightweight_no_node_event_diagnostic(
                            cli,
                            node_label,
                            EXEC_FIRST_EVENT_TIMEOUT_SECS,
                        )
                    });
                    model = None;
                    break;
                }
            }
        };

        match event {
            Some(AgentToServer::CliChunk { text, .. }) => {
                saw_cli_event = true;
                output.push_str(&text);
            }
            Some(AgentToServer::CliDone {
                exit_ok: ok,
                error: e,
                model: m,
                ..
            }) => {
                exit_ok = ok;
                error = e;
                model = m;
                break;
            }
            Some(_) => { /* 忽略其他消息类型 */ }
            None => {
                error = Some("节点连接已关闭".to_string());
                model = None;
                break;
            }
        }
    }

    Ok(NodeExecRun {
        req_id,
        output,
        exit_ok,
        error,
        model,
    })
}

fn prepare_owner_node_exec_run(
    store: &Store,
    req_id: &str,
    owner_user_id: &str,
    node_id: &str,
    cli: &str,
) -> Result<(), String> {
    let compute_call_id = format!("pc_agent_cli:{req_id}");
    let run = store
        .start_node_compute_run(NodeComputeRunStart {
            compute_call_id: &compute_call_id,
            consumer_user_id: owner_user_id,
            provider_user_id: Some(owner_user_id),
            node_id,
            model_id: Some(cli),
            feature: "pc_node_exec",
            usage_mode: "pc_agent_cli",
            route_reason: Some("owner_direct_node_exec"),
        })
        .map_err(|error| format!("建立本机直连执行账本失败：{error}"))?;
    if run.status != "started"
        || run.consumer_user_id != owner_user_id
        || run.provider_user_id.as_deref() != Some(owner_user_id)
        || run.node_id != node_id
        || run.model_id.as_deref() != Some(cli)
        || run.feature != "pc_node_exec"
        || run.usage_mode != "pc_agent_cli"
        || run.route_reason.as_deref() != Some("owner_direct_node_exec")
    {
        return Err("本机直连执行请求标识已被其他计算运行占用".to_string());
    }

    let run = store
        .bind_node_compute_run_replay_policy(
            &compute_call_id,
            NodeComputeReplayBinding {
                billing_source: "own_codex",
                resource_owner_user_id: Some(owner_user_id),
                lease_id: None,
                offline_policy: "allow_offline",
                replay_deadline: None,
                max_cost_rmb_fen: 0,
                allowance_id: None,
            },
        )
        .map_err(|error| format!("冻结本机直连补传授权失败：{error}"))?
        .ok_or_else(|| "本机直连执行账本在分发前消失".to_string())?;
    if run.status != "started"
        || run.billing_source != "own_codex"
        || run.resource_owner_user_id.as_deref() != Some(owner_user_id)
        || run.lease_id.is_some()
        || run.offline_policy != "allow_offline"
        || run.replay_deadline.is_some()
        || run.max_cost_rmb_fen != 0
        || run.allowance_id.is_some()
    {
        return Err("本机直连执行的离线补传授权冻结不完整".to_string());
    }
    Ok(())
}

fn finish_owner_node_exec_before_dispatch_failure(store: &Store, req_id: &str, error: &str) {
    let compute_call_id = format!("pc_agent_cli:{req_id}");
    let _ = store.finish_node_compute_run(
        &compute_call_id,
        NodeComputeRunFinish {
            provider_user_id: None,
            status: "released_error",
            prompt_tokens: 0,
            completion_tokens: 0,
            billed_cost_rmb_fen: 0,
            provider_earned_fen: 0,
            settlement_status: Some("not_dispatched"),
            error_message: Some(error),
        },
    );
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

fn clean_node_exec_output(output: &str) -> String {
    if let Some(message) = extract_json_agent_message(output) {
        return message;
    }

    let filtered = output
        .lines()
        .filter(|line| !is_codex_protocol_event_line(line))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if filtered.is_empty() && !output.lines().any(is_codex_protocol_event_line) {
        output.trim().to_string()
    } else {
        filtered
    }
}

fn should_retry_node_exec_after_reconnect(error: Option<&str>, display_output: &str) -> bool {
    if !display_output.trim().is_empty() {
        return false;
    }
    let Some(error) = error else {
        return false;
    };
    let lower = error.to_lowercase();
    error.contains("节点重新注册")
        || error.contains("旧连接已关闭")
        || error.contains("节点连接已关闭")
        || lower.contains("agent writer closed")
        || lower.contains("channel closed")
        || lower.contains("connection closed")
}

fn extract_json_agent_message(output: &str) -> Option<String> {
    let mut latest = None;
    for line in output.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("item.completed") {
            continue;
        }
        let Some(item) = value.get("item") else {
            continue;
        };
        if item.get("type").and_then(Value::as_str) != Some("agent_message") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(Value::as_str) {
            let clean = text
                .trim()
                .strip_prefix("用户可见：")
                .unwrap_or(text.trim())
                .trim();
            if !clean.is_empty() {
                latest = Some(clean.to_string());
            }
        }
    }
    latest
}

fn is_codex_protocol_event_line(line: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
        return false;
    };
    matches!(
        value.get("type").and_then(Value::as_str),
        Some("turn.started")
            | Some("turn.completed")
            | Some("token_count")
            | Some("item.started")
            | Some("item.completed")
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_node_exec_prompt, clean_node_exec_output, prepare_owner_node_exec_run,
        should_retry_node_exec_after_reconnect,
    };
    use crate::store::Store;

    #[test]
    fn node_exec_output_extracts_codex_agent_message() {
        let output = r#"{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"你好。需要我帮你处理什么？"}}
{"type":"turn.completed","usage":{"input_tokens":12695,"output_tokens":13}}
codex
你好。需要我帮你处理什么？"#;

        assert_eq!(clean_node_exec_output(output), "你好。需要我帮你处理什么？");
    }

    #[test]
    fn node_exec_output_keeps_normal_text() {
        assert_eq!(
            clean_node_exec_output("普通输出\n第二行"),
            "普通输出\n第二行"
        );
    }

    #[test]
    fn node_exec_output_removes_protocol_only_events() {
        assert_eq!(clean_node_exec_output(r#"{"type":"turn.started"}"#), "");
    }

    #[test]
    fn node_exec_retries_reconnect_without_output() {
        assert!(should_retry_node_exec_after_reconnect(
            Some("节点重新注册，旧连接已关闭"),
            ""
        ));
    }

    #[test]
    fn node_exec_does_not_retry_reconnect_with_output() {
        assert!(!should_retry_node_exec_after_reconnect(
            Some("节点重新注册，旧连接已关闭"),
            "已经拿到正文"
        ));
    }

    #[test]
    fn node_exec_prompt_adds_page_node_context() {
        let prompt = build_node_exec_prompt("看看节点连接了没有", "ELONQIAN", "node-123");
        assert!(prompt.contains("ELONQIAN"));
        assert!(prompt.contains("node-123"));
        assert!(prompt.contains("不只是命令执行器"));
        assert!(prompt.contains("先按一龙工作台网页语境回答"));
        assert!(prompt.contains("已经连接到该节点"));
        assert!(prompt.contains("不要把"));
        assert!(prompt.contains("node.exe"));
        assert!(prompt.contains("不确定时先说不确定并追问"));
        assert!(prompt.contains("看看节点连接了没有"));
    }

    #[test]
    fn node_exec_prepares_ackable_owner_durable_run_before_dispatch() {
        let path = std::env::temp_dir().join(format!(
            "elon-node-exec-durable-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        let store = Store::open(&path).expect("store should open");
        let owner = store
            .create_user("node-exec-owner@example.com", "secret1", None, None)
            .expect("owner should be created");

        prepare_owner_node_exec_run(&store, "req-owner-direct", &owner.id, "node-owner", "codex")
            .expect("owner direct run should be frozen before dispatch");

        let run = store
            .get_node_compute_run_by_compute_call_id("pc_agent_cli:req-owner-direct")
            .expect("run lookup should succeed")
            .expect("run should exist");
        assert_eq!(run.consumer_user_id, owner.id);
        assert_eq!(run.provider_user_id.as_deref(), Some(owner.id.as_str()));
        assert_eq!(run.node_id, "node-owner");
        assert_eq!(run.billing_source, "own_codex");
        assert_eq!(run.offline_policy, "allow_offline");
        assert!(store
            .can_replay_node_compute_run_offline("pc_agent_cli:req-owner-direct")
            .expect("replay policy lookup should succeed"));

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
