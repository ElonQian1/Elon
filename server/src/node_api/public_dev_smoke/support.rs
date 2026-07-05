use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    ai_cli,
    node_runtime::{node_runtime_by_id, NodeRuntime},
    store::{AdminUserSummary, NodeComputeRun},
    types::AppState,
};

use super::super::public_dev::public_dev_handshake_state_for_runtime;

#[derive(Debug, Serialize)]
pub(super) struct PublicDevMutualSmokeResponse {
    pub(super) ok: bool,
    pub(super) execute: bool,
    pub(super) cli_name: String,
    pub(super) generated_at: String,
    pub(super) left: SmokeSide,
    pub(super) right: SmokeSide,
    pub(super) directions: Vec<SmokeDirection>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SmokeSide {
    pub(super) owner: SmokeUser,
    pub(super) node: SmokeNode,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SmokeUser {
    pub(super) id: String,
    pub(super) account: String,
    pub(super) nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SmokeNode {
    pub(super) node_id: String,
    pub(super) display_name: String,
    pub(super) device_name: Option<String>,
    pub(super) short_id: String,
    pub(super) public_dev_enabled: bool,
    pub(super) public_dev_handshake_ready: bool,
    pub(super) public_dev_handshake_status: String,
    pub(super) online: bool,
    pub(super) cli_connected: bool,
    pub(super) allowed_clis: Vec<String>,
    pub(super) last_handshake_allowed_clis: Vec<String>,
    pub(super) last_handshake_at: Option<String>,
    pub(super) agent_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct SmokeDirection {
    pub(super) label: String,
    pub(super) status: String,
    pub(super) consumer: SmokeUser,
    pub(super) provider: SmokeUser,
    pub(super) node: SmokeNode,
    pub(super) preflight: SmokePreflight,
    pub(super) result: Option<SmokeRunResult>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct SmokePreflight {
    authorized: bool,
    ready: bool,
    cli_allowed_by_share: bool,
    cli_reported_by_node: bool,
    route: String,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct SmokeRunResult {
    outcome: String,
    done_message: Option<String>,
    model_used: Option<String>,
    done_node_id: Option<String>,
    event_count: usize,
    event_preview: Vec<SmokeEventPreview>,
    compute_run: Option<NodeComputeRun>,
}

#[derive(Debug, Serialize)]
struct SmokeEventPreview {
    event_type: String,
    text: String,
}

pub(super) async fn resolve_side(
    state: &Arc<AppState>,
    owner_match: &str,
    node_match: &str,
) -> anyhow::Result<SmokeSide> {
    let users = state.store.list_users()?;
    let user = users
        .into_iter()
        .filter(|user| matches_user(user, owner_match))
        .max_by_key(|user| user_match_score(user, owner_match))
        .ok_or_else(|| anyhow::anyhow!("找不到匹配账号：{owner_match}"))?;

    let mut candidates = Vec::new();
    for credential in state.store.list_all_node_credentials()? {
        if credential.owner_user_id != user.id {
            continue;
        }
        if let Some(runtime) = node_runtime_by_id(state, &credential.agent_id).await? {
            if matches_node(&runtime, node_match) {
                candidates.push(runtime);
            }
        }
    }
    let runtime = candidates
        .into_iter()
        .max_by_key(|runtime| node_match_score(runtime, node_match))
        .ok_or_else(|| {
            anyhow::anyhow!("账号 {} 下找不到匹配节点：{node_match}", user_label(&user))
        })?;

    Ok(SmokeSide {
        owner: smoke_user(user),
        node: smoke_node(runtime),
    })
}

pub(super) async fn run_smoke_direction(
    state: &Arc<AppState>,
    label: &str,
    consumer: &SmokeUser,
    provider_side: &SmokeSide,
    cli_name: &str,
    prompt: &str,
    execute: bool,
) -> SmokeDirection {
    let preflight = build_preflight(consumer, provider_side, cli_name);
    if !preflight.ready {
        return smoke_direction(
            label,
            "blocked",
            consumer,
            provider_side,
            preflight,
            None,
            Some("预检未通过，未执行 CLI smoke test。"),
        );
    }
    if !execute {
        return smoke_direction(
            label,
            "ready",
            consumer,
            provider_side,
            preflight,
            None,
            None,
        );
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let outcome = ai_cli::run_with_pc_agent_chat(
        &provider_side.node.node_id,
        &consumer.id,
        prompt,
        None,
        Some(cli_name),
        None,
        Some("low"),
        Some(cli_name),
        state,
        &tx,
    )
    .await;
    drop(tx);

    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    let mut run_result = summarize_run_result(&events);
    run_result.compute_run = latest_compute_run_for_direction(
        state,
        &consumer.id,
        &provider_side.owner.id,
        &provider_side.node.node_id,
    );

    match outcome {
        Ok(ai_cli::PcAgentChatOutcome::Answered) => smoke_direction(
            label,
            "passed",
            consumer,
            provider_side,
            preflight,
            Some(run_result),
            None,
        ),
        Ok(ai_cli::PcAgentChatOutcome::NoReadableReply { diagnostic }) => smoke_direction(
            label,
            "failed",
            consumer,
            provider_side,
            preflight,
            Some(run_result),
            Some(diagnostic.as_deref().unwrap_or("CLI 未返回可读结果")),
        ),
        Err(error) => smoke_direction(
            label,
            "failed",
            consumer,
            provider_side,
            preflight,
            Some(run_result),
            Some(&error.to_string()),
        ),
    }
}

pub(super) fn default_smoke_prompt(cli_name: &str) -> String {
    format!(
        "请只回复一行：public-dev-smoke-ok cli={}。不要改文件，不要运行命令。",
        cli_name
    )
}

pub(super) fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn smoke_direction(
    label: &str,
    status: &str,
    consumer: &SmokeUser,
    provider_side: &SmokeSide,
    preflight: SmokePreflight,
    result: Option<SmokeRunResult>,
    error: Option<&str>,
) -> SmokeDirection {
    SmokeDirection {
        label: label.to_string(),
        status: status.to_string(),
        consumer: consumer.clone(),
        provider: provider_side.owner.clone(),
        node: provider_side.node.clone(),
        preflight,
        result,
        error: error.map(str::to_string),
    }
}

fn build_preflight(
    consumer: &SmokeUser,
    provider_side: &SmokeSide,
    cli_name: &str,
) -> SmokePreflight {
    let node = &provider_side.node;
    let cli_allowed_by_share = cli_list_contains(&node.last_handshake_allowed_clis, cli_name)
        || cli_list_contains(&node.allowed_clis, cli_name);
    let cli_reported_by_node = cli_list_contains(&node.allowed_clis, cli_name);
    let authorized = consumer.id != provider_side.owner.id && node.public_dev_enabled;
    let ready = authorized
        && node.public_dev_handshake_ready
        && node.online
        && node.cli_connected
        && cli_allowed_by_share
        && cli_reported_by_node;
    let mut notes = Vec::new();
    if !authorized {
        notes.push("不是跨账号公开开发授权".to_string());
    }
    if !node.public_dev_handshake_ready {
        notes.push(format!("握手状态：{}", node.public_dev_handshake_status));
    }
    if !node.online || !node.cli_connected {
        notes.push("节点不在线或 CLI 通道未连接".to_string());
    }
    if !cli_reported_by_node {
        notes.push(format!("节点未上报 {cli_name}"));
    }
    SmokePreflight {
        authorized,
        ready,
        cli_allowed_by_share,
        cli_reported_by_node,
        route: "RouteC3/public-dev-lightweight-codex".to_string(),
        notes,
    }
}

fn summarize_run_result(events: &[String]) -> SmokeRunResult {
    let mut done_message = None;
    let mut model_used = None;
    let mut done_node_id = None;
    let mut preview = Vec::new();
    for raw in events {
        if let Ok(value) = serde_json::from_str::<Value>(raw) {
            let event_type = value["type"].as_str().unwrap_or("unknown").to_string();
            let text = value["message"]
                .as_str()
                .or_else(|| value["text"].as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if event_type == "done" {
                done_message = value["message"].as_str().map(trim_preview);
                model_used = value["model_used"].as_str().map(str::to_string);
                done_node_id = value["node_id"].as_str().map(str::to_string);
            }
            if preview.len() < 12 && (!text.is_empty() || event_type == "done") {
                preview.push(SmokeEventPreview {
                    event_type,
                    text: trim_preview(&text),
                });
            }
        }
    }
    SmokeRunResult {
        outcome: if done_message.is_some() {
            "done"
        } else {
            "no_done_event"
        }
        .to_string(),
        done_message,
        model_used,
        done_node_id,
        event_count: events.len(),
        event_preview: preview,
        compute_run: None,
    }
}

fn latest_compute_run_for_direction(
    state: &AppState,
    consumer_user_id: &str,
    provider_user_id: &str,
    node_id: &str,
) -> Option<NodeComputeRun> {
    state
        .store
        .list_node_compute_runs_for_consumer(consumer_user_id, 20)
        .ok()?
        .into_iter()
        .find(|run| {
            run.provider_user_id.as_deref() == Some(provider_user_id) && run.node_id == node_id
        })
}

fn smoke_user(user: AdminUserSummary) -> SmokeUser {
    SmokeUser {
        id: user.id,
        account: user.account,
        nickname: user.nickname,
    }
}

fn smoke_node(runtime: NodeRuntime) -> SmokeNode {
    let (ready, status) = public_dev_handshake_state_for_runtime(&runtime);
    SmokeNode {
        node_id: runtime.node_id,
        display_name: runtime.display_name,
        device_name: runtime.device_name,
        short_id: runtime.short_id,
        public_dev_enabled: runtime.public_dev_enabled,
        public_dev_handshake_ready: ready,
        public_dev_handshake_status: status,
        online: runtime.online,
        cli_connected: runtime.cli_connected,
        allowed_clis: runtime.allowed_clis,
        last_handshake_allowed_clis: runtime.last_handshake_allowed_clis,
        last_handshake_at: runtime.last_handshake_at,
        agent_version: runtime.agent_version,
    }
}

fn matches_user(user: &AdminUserSummary, needle: &str) -> bool {
    let needle = normalize_match(needle);
    normalize_match(&user.id).contains(&needle)
        || normalize_match(&user.account).contains(&needle)
        || user
            .nickname
            .as_deref()
            .map(|name| normalize_match(name).contains(&needle))
            .unwrap_or(false)
}

fn user_match_score(user: &AdminUserSummary, needle: &str) -> i32 {
    let needle = normalize_match(needle);
    let mut score = 0;
    if normalize_match(&user.account) == needle {
        score += 20;
    }
    if user
        .nickname
        .as_deref()
        .map(|name| normalize_match(name) == needle)
        .unwrap_or(false)
    {
        score += 30;
    }
    if user.status == "active" {
        score += 5;
    }
    score
}

fn matches_node(runtime: &NodeRuntime, needle: &str) -> bool {
    let needle = normalize_match(needle);
    normalize_match(&runtime.node_id).contains(&needle)
        || normalize_match(&runtime.display_name).contains(&needle)
        || normalize_match(&runtime.label).contains(&needle)
        || runtime
            .device_name
            .as_deref()
            .map(|name| normalize_match(name).contains(&needle))
            .unwrap_or(false)
}

fn node_match_score(runtime: &NodeRuntime, needle: &str) -> i32 {
    let needle = normalize_match(needle);
    let mut score = 0;
    if normalize_match(&runtime.display_name) == needle {
        score += 30;
    }
    if runtime.online {
        score += 20;
    }
    if public_dev_handshake_state_for_runtime(runtime).0 {
        score += 20;
    }
    score
}

fn cli_list_contains(values: &[String], cli_name: &str) -> bool {
    values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(cli_name))
}

fn normalize_match(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn user_label(user: &AdminUserSummary) -> String {
    user.nickname
        .clone()
        .unwrap_or_else(|| user.account.clone())
}

fn trim_preview(value: &str) -> String {
    let value = value.trim();
    const MAX: usize = 360;
    if value.chars().count() <= MAX {
        value.to_string()
    } else {
        format!("{}…", value.chars().take(MAX).collect::<String>())
    }
}
