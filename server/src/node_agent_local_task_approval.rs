use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::NodeRuntime;

#[derive(Debug, Deserialize)]
pub(super) struct ApprovalDecisionRequest {
    decision: String,
}

pub(super) async fn decide_approval(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((task_id, approval_id)): Path<(String, String)>,
    Json(request): Json<ApprovalDecisionRequest>,
) -> Response {
    let creds = match super::support::bound_credentials(&runtime).await {
        Ok(creds) => creds,
        Err(response) => return response,
    };
    match runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, task_id.trim())
    {
        Ok(Some(record)) if record.status == "running" => {}
        Ok(Some(_)) => {
            return super::support::json_error(StatusCode::CONFLICT, "任务已结束，审批不再可操作。")
        }
        Ok(None) => return super::support::json_error(StatusCode::NOT_FOUND, "本机任务不存在。"),
        Err(error) => return super::support::internal_error(error),
    }
    let decision = match request.decision.trim() {
        "approve" => "approve",
        "deny" => "deny",
        _ => {
            return super::support::json_error(
                StatusCode::BAD_REQUEST,
                "decision 只能是 approve 或 deny。",
            )
        }
    };
    if !runtime
        .decide_tool_approval(task_id.trim(), approval_id.trim(), decision)
        .await
    {
        return super::support::json_error(
            StatusCode::CONFLICT,
            "审批已失效，或运行时已不存在对应等待项。",
        );
    }
    Json(json!({
        "ok": true,
        "task_id": task_id,
        "approval_id": approval_id,
        "decision": decision,
    }))
    .into_response()
}
