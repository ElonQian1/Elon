// server/src/node_agent_cli_sidecar_admin.rs

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

use crate::{
    node_agent_cli_sidecar::sidecar_status_view,
    node_agent_cli_sidecar_io::{read_output_records_from, CliSidecarOutputRecord},
    NodeRuntime,
};

#[derive(Debug, Deserialize)]
struct AttachQuery {
    since: Option<u64>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct InputRequest {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ResizeRequest {
    cols: u16,
    rows: u16,
}

#[derive(Debug, Serialize)]
struct SidecarAttachResponse {
    ok: bool,
    task_id: String,
    attached: bool,
    session: serde_json::Value,
    output_records: Vec<CliSidecarOutputRecord>,
    next_offset: u64,
    transport: String,
    protocol: serde_json::Value,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/cli-sidecars/:task_id/attach", get(attach_handler))
        .route("/api/cli-sidecars/:task_id/input", post(input_handler))
        .route("/api/cli-sidecars/:task_id/resize", post(resize_handler))
}

async fn attach_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Query(query): Query<AttachQuery>,
) -> Response {
    let task_id = task_id.trim().to_string();
    if let Err(message) = validate_task_id(&task_id) {
        return bad_request(message);
    }
    let Some(session) = session_for_task(&runtime, &task_id) else {
        return not_found(&task_id, "没有可重接的 sidecar 会话。");
    };
    if !session.can_replay_output_at(crate::node_agent_cli_sidecar::now_ms()) {
        return not_found(&task_id, "sidecar 会话已结束、心跳过期或不支持输出回放。");
    }
    let Some(output_path) = session.endpoint.as_deref().map(PathBuf::from) else {
        return not_found(&task_id, "sidecar 会话缺少输出流路径。");
    };
    let mut offset = query.since.unwrap_or(0);
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    match read_output_records_from(&output_path, &mut offset, limit) {
        Ok(records) => Json(SidecarAttachResponse {
            ok: true,
            task_id: task_id.clone(),
            attached: true,
            session: sidecar_status_view(&session),
            output_records: records,
            next_offset: offset,
            transport: session.transport.clone(),
            protocol: json!({
                "read": format!("/api/cli-sidecars/{task_id}/attach?since=<offset>"),
                "write": session.capabilities.terminal_input.then(|| format!("/api/cli-sidecars/{task_id}/input")),
                "resize": session.capabilities.terminal_resize.then(|| format!("/api/cli-sidecars/{task_id}/resize")),
                "cancel": session.capabilities.cancel,
                "output_stream_replay": session.capabilities.output_stream_replay,
                "terminal_attach": session.capabilities.terminal_attach,
                "input_encoding": if session.capabilities.terminal_input { "utf8_terminal_bytes" } else { "not_supported" },
                "resize_units": if session.capabilities.terminal_resize { "terminal_cells" } else { "not_supported" }
            }),
        })
        .into_response(),
        Err(error) => internal_error(error),
    }
}

async fn input_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Json(req): Json<InputRequest>,
) -> Response {
    let task_id = task_id.trim().to_string();
    if let Err(message) = validate_task_id(&task_id) {
        return bad_request(message);
    }
    if req.text.is_empty() {
        return bad_request("输入不能为空。");
    }
    if req.text.len() > 64 * 1024 {
        return bad_request("单次终端输入过长。");
    }
    match runtime
        .cli_sidecars
        .record_terminal_input(&task_id, &req.text)
    {
        Ok(true) => {
            Json(json!({ "ok": true, "task_id": task_id, "status": "queued" })).into_response()
        }
        Ok(false) => not_found(&task_id, "sidecar 会话不可写入或已过期。"),
        Err(error) => internal_error(error),
    }
}

async fn resize_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Json(req): Json<ResizeRequest>,
) -> Response {
    let task_id = task_id.trim().to_string();
    if let Err(message) = validate_task_id(&task_id) {
        return bad_request(message);
    }
    if !(20..=500).contains(&req.cols) || !(5..=200).contains(&req.rows) {
        return bad_request("终端尺寸超出允许范围。");
    }
    match runtime
        .cli_sidecars
        .record_terminal_resize(&task_id, req.cols, req.rows)
    {
        Ok(true) => Json(json!({
            "ok": true,
            "task_id": task_id,
            "status": "queued",
            "cols": req.cols,
            "rows": req.rows
        }))
        .into_response(),
        Ok(false) => not_found(&task_id, "sidecar 会话不可调整尺寸或已过期。"),
        Err(error) => internal_error(error),
    }
}

fn session_for_task(
    runtime: &NodeRuntime,
    task_id: &str,
) -> Option<crate::node_agent_cli_sidecar::CliSidecarSessionRecord> {
    runtime
        .cli_sidecars
        .session_for_task(task_id)
        .unwrap_or_else(|error| {
            tracing::warn!("读取 CLI sidecar 会话失败: {error}");
            None
        })
}

fn validate_task_id(task_id: &str) -> Result<(), &'static str> {
    if task_id.is_empty() {
        return Err("任务 ID 不能为空。");
    }
    if task_id.len() > 160 {
        return Err("任务 ID 过长。");
    }
    if task_id
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':')))
    {
        return Err("任务 ID 包含非法字符。");
    }
    Ok(())
}

fn bad_request(message: &'static str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "ok": false, "error": message })),
    )
        .into_response()
}

fn not_found(task_id: &str, message: &'static str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "ok": false,
            "task_id": task_id,
            "error": message,
        })),
    )
        .into_response()
}

fn internal_error(error: anyhow::Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "ok": false, "error": error.to_string() })),
    )
        .into_response()
}
