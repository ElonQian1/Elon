use super::*;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::NodeRuntime;

#[derive(Debug, Deserialize)]
struct EventInput {
    #[serde(default)]
    trace_id: String,
    source: String,
    #[serde(default)]
    level: String,
    kind: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    fields: Value,
}

#[derive(Debug, Deserialize)]
struct ActionInput {
    #[serde(default)]
    trace_id: String,
    kind: String,
    #[serde(default)]
    route: Option<String>,
    #[serde(default)]
    requested_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TimelineQuery {
    #[serde(default)]
    since: u64,
    limit: Option<usize>,
    sources: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PendingQuery {
    limit: Option<usize>,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/codex-control/capabilities", get(capabilities_handler))
        .route(
            "/api/codex-control/events",
            get(timeline_handler).post(event_handler),
        )
        .route("/api/codex-control/actions", post(action_handler))
        .route(
            "/api/codex-control/actions/pending",
            get(pending_actions_handler),
        )
        .route(
            "/api/codex-control/actions/:action_id/receipt",
            post(receipt_handler),
        )
        .route(
            "/api/codex-control/actions/:action_id/claim",
            post(claim_handler),
        )
        .route("/api/codex-control/diagnostics", get(diagnostics_handler))
        .route("/api/codex-control/export", post(export_handler))
}

async fn capabilities_handler(State(runtime): State<Arc<NodeRuntime>>) -> Json<Value> {
    Json(json!({"ok": true, "capabilities": runtime.win_codex_control.capabilities()}))
}

async fn event_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(input): Json<EventInput>,
) -> Response {
    if !is_client_source(&input.source) {
        return bad_request("页面只允许写入 frontend、network 或 tauri 来源事件。");
    }
    let event = runtime.win_codex_control.record(
        &input.trace_id,
        &input.source,
        &input.level,
        &input.kind,
        &input.summary,
        input.fields,
    );
    Json(json!({"ok": true, "event": event})).into_response()
}

async fn timeline_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Query(query): Query<TimelineQuery>,
) -> Json<Value> {
    Json(timeline_payload(
        &runtime,
        None,
        query.since,
        query.limit.unwrap_or(200),
        &parse_sources(query.sources.as_deref()),
    ))
}

async fn action_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(input): Json<ActionInput>,
) -> Response {
    match runtime.win_codex_control.enqueue_action(
        &input.trace_id,
        &input.kind,
        input.route.as_deref(),
        input.requested_by.as_deref().unwrap_or("pc_ui"),
    ) {
        Ok(action) => Json(json!({"ok": true, "action": action})).into_response(),
        Err(error) => bad_request(error),
    }
}

async fn pending_actions_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Query(query): Query<PendingQuery>,
) -> Json<Value> {
    Json(
        json!({"ok": true, "actions": runtime.win_codex_control.pending_actions(query.limit.unwrap_or(20))}),
    )
}

async fn receipt_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    AxumPath(action_id): AxumPath<String>,
    Json(receipt): Json<WinControlReceipt>,
) -> Response {
    match runtime
        .win_codex_control
        .record_receipt(action_id.trim(), receipt)
    {
        Ok(action) => Json(json!({"ok": true, "action": action})).into_response(),
        Err(error) => bad_request(error),
    }
}

async fn claim_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    AxumPath(action_id): AxumPath<String>,
) -> Response {
    match runtime.win_codex_control.claim_action(action_id.trim()) {
        Ok(action) => Json(json!({"ok": true, "action": action})).into_response(),
        Err(error) => bad_request(error),
    }
}

async fn diagnostics_handler() -> Json<Value> {
    Json(
        json!({"ok": true, "diagnostics": crate::node_agent_client_diagnostics::diagnostic_snapshot()}),
    )
}

async fn export_handler(State(runtime): State<Arc<NodeRuntime>>) -> Response {
    match export_bundle(&runtime) {
        Ok(path) => Json(json!({
            "ok": true, "path": path.to_string_lossy(), "message": "已生成脱敏 Win Codex 诊断包。",
        }))
        .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "error": error})),
        )
            .into_response(),
    }
}

pub(crate) fn timeline_payload(
    runtime: &NodeRuntime,
    workspace: Option<&Path>,
    since: u64,
    limit: usize,
    sources: &HashSet<String>,
) -> Value {
    let limit = limit.clamp(1, 500);
    let mut events = runtime
        .win_codex_control
        .events(since, limit, sources)
        .into_iter()
        .map(|event| serde_json::to_value(event).unwrap_or(Value::Null))
        .collect::<Vec<_>>();
    if since == 0 && (sources.is_empty() || sources.contains("rust")) {
        events.extend(rust_timeline());
    }
    if since == 0 && (sources.is_empty() || sources.contains("cli")) {
        events.extend(cli_timeline(runtime, workspace, 8, 60));
    }
    events.sort_by_key(|event| {
        event
            .get("at_ms")
            .and_then(Value::as_u64)
            .unwrap_or_default()
    });
    if events.len() > limit {
        events.drain(0..events.len() - limit);
    }
    let next_since = events
        .iter()
        .filter_map(|event| event.get("seq").and_then(Value::as_u64))
        .max()
        .unwrap_or(since);
    json!({
        "ok": true, "schema": "elon.win_codex_timeline.v1", "since": since,
        "next_since": next_since, "events": events,
        "capabilities": runtime.win_codex_control.capabilities(),
    })
}

fn cli_timeline(
    runtime: &NodeRuntime,
    workspace: Option<&Path>,
    task_limit: usize,
    event_limit: usize,
) -> Vec<Value> {
    let records = match workspace {
        Some(workspace) => runtime
            .task_journal
            .latest_records_for_workspace(workspace, task_limit),
        None => runtime.task_journal.latest_records(task_limit),
    }
    .unwrap_or_default();
    let ids = records
        .iter()
        .map(|record| record.req_id.clone())
        .collect::<HashSet<_>>();
    let recent_events = runtime
        .task_journal
        .recent_events_for_tasks(&ids, event_limit)
        .unwrap_or_default();
    let mut output = Vec::new();
    for record in records {
        output.push(json!({
            "seq": 0, "event_id": format!("cli_task:{}:{}", record.req_id, record.updated_at_ms),
            "trace_id": record.req_id, "source": "cli",
            "level": if record.status == "failed" {"error"} else {"info"},
            "kind": "cli.task_state", "summary": format!("{} · {} · {}", record.cli_name, record.status, record.phase),
            "at_ms": u128_to_u64(record.updated_at_ms),
            "fields": {"task_id": record.req_id, "route": record.route, "phase": record.phase, "status": record.status, "current_command": record.current_command.map(|value| redact_text(&value))},
        }));
        if let Some(task_events) = recent_events.get(&record.req_id) {
            for event in task_events {
                let event_type = event
                    .event
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("event");
                output.push(json!({
                    "seq": 0, "event_id": format!("cli_event:{}:{}", record.req_id, event.seq),
                    "trace_id": record.req_id, "source": "cli",
                    "level": if event_type.contains("error") || event_type.contains("failed") {"error"} else {"info"},
                    "kind": format!("cli.{event_type}"), "summary": cli_event_summary(event_type, &event.event),
                    "at_ms": event.event.get("at_ms").and_then(Value::as_u64).unwrap_or_default(),
                    "fields": {"task_id": record.req_id, "event_seq": event.seq, "event_type": event_type, "stream": event.event.get("stream")},
                }));
            }
        }
    }
    output
}

fn rust_timeline() -> Vec<Value> {
    let diagnostics = crate::node_agent_client_diagnostics::diagnostic_snapshot();
    let runtime = diagnostics
        .get("runtime")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let maintenance = diagnostics
        .pointer("/logs/maintenance")
        .cloned()
        .unwrap_or_else(|| json!({}));
    vec![json!({
        "seq": 0,
        "event_id": format!("rust_health:{}", runtime.get("pid").and_then(Value::as_u64).unwrap_or_default()),
        "trace_id": "node_runtime",
        "source": "rust",
        "level": if maintenance.get("parse_errors").and_then(Value::as_u64).unwrap_or_default() > 0 {"warn"} else {"info"},
        "kind": "rust.node_health",
        "summary": "Win 节点 Rust 控制域正在运行",
        "at_ms": diagnostics.get("generated_at_ms").and_then(Value::as_u64).unwrap_or_default(),
        "fields": {
            "version": runtime.get("version"),
            "pid": runtime.get("pid"),
            "maintenance_line_count": maintenance.get("line_count"),
            "maintenance_parse_errors": maintenance.get("parse_errors"),
        },
    })]
}

fn cli_event_summary(event_type: &str, event: &Value) -> String {
    match event_type {
        "cli_chunk" => format!(
            "Codex {} 有新输出（正文请在任务 journal 查看）",
            event
                .get("stream")
                .and_then(Value::as_str)
                .unwrap_or("stdout")
        ),
        "tool_event" => "Codex 工具事件（正文请在任务 journal 查看）".to_string(),
        "process_started" => "Codex 执行进程已启动".to_string(),
        "codex_session" => "Codex session 已绑定".to_string(),
        value => format!("Codex journal: {}", clean_kind(value)),
    }
}

fn export_bundle(runtime: &NodeRuntime) -> Result<PathBuf, String> {
    let base = crate::state_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("diagnostics");
    fs::create_dir_all(&dir).map_err(|error| format!("创建诊断目录失败: {error}"))?;
    let path = dir.join(format!("win-codex-diagnostics-{}.json", now_ms()));
    let payload = json!({
        "schema": "elon.win_codex_diagnostics.v1", "generated_at_ms": now_ms(),
        "privacy": {"cookies": false, "tokens": false, "request_bodies": false, "prompt_bodies": false, "raw_cli_output": false},
        "timeline": timeline_payload(runtime, None, 0, 500, &HashSet::new()),
        "node": crate::node_agent_client_diagnostics::diagnostic_snapshot(),
    });
    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|error| format!("生成诊断 JSON 失败: {error}"))?;
    crate::node_agent_atomic_file::write(&path, &bytes)
        .map_err(|error| format!("写入诊断包失败: {error:#}"))?;
    Ok(path)
}

fn parse_sources(raw: Option<&str>) -> HashSet<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| {
            matches!(
                *value,
                "frontend" | "rust" | "cli" | "network" | "tauri" | "control"
            )
        })
        .map(str::to_string)
        .collect()
}

fn bad_request(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"ok": false, "error": message.into()})),
    )
        .into_response()
}
