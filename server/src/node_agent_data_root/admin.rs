use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;

use crate::NodeRuntime;

pub(crate) async fn get(State(runtime): State<Arc<NodeRuntime>>) -> Json<serde_json::Value> {
    let state = runtime.node_data_root.read().await.clone();
    let mut payload = state.status_payload();
    if let Some(paths) = state.paths {
        if let Ok(build_cache) =
            tokio::task::spawn_blocking(move || crate::node_agent_build_runtime::status(&paths))
                .await
        {
            payload["build_cache"] = serde_json::json!(build_cache);
        }
    }
    Json(payload)
}

#[derive(Deserialize)]
pub(crate) struct SetRequest {
    root_path: String,
}

pub(crate) async fn set(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(request): Json<SetRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !runtime
        .active_cli_prompts
        .views_without_approvals()
        .await
        .is_empty()
    {
        return error(
            StatusCode::CONFLICT,
            "当前仍有 CLI 任务运行，不能切换节点数据根",
        );
    }
    let current = runtime.node_data_root.read().await.clone();
    if let Err(reason) = super::validate_no_root_overlap(&request.root_path, &current) {
        return error(StatusCode::BAD_REQUEST, reason);
    }
    let paths = match super::validate_and_prepare(&request.root_path, &runtime.install_id) {
        Ok(paths) => paths,
        Err(reason) => return error(StatusCode::BAD_REQUEST, reason),
    };
    match runtime.set_node_data_root(paths).await {
        Ok(state) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "data_root": state.status_payload(),
                "restart_recommended": true,
            })),
        ),
        Err(reason) => error(StatusCode::INTERNAL_SERVER_ERROR, reason),
    }
}

#[derive(Deserialize)]
pub(crate) struct CleanupRequest {
    #[serde(default)]
    apply: bool,
}

pub(crate) async fn cleanup(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(request): Json<CleanupRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if request.apply
        && !runtime
            .active_cli_prompts
            .views_without_approvals()
            .await
            .is_empty()
    {
        return error(
            StatusCode::CONFLICT,
            "当前仍有 CLI 任务运行，不能清理节点构建缓存",
        );
    }
    let state = runtime.node_data_root.read().await.clone();
    let Some(paths) = state.paths.clone() else {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "ok": false,
                "error": "尚未配置 ELON_NODE_DATA_ROOT，拒绝清理旧用户目录",
                "data_root": state.status_payload(),
            })),
        );
    };
    let apply = request.apply;
    match tokio::task::spawn_blocking(move || {
        crate::node_agent_build_runtime::cleanup_rebuildable(&paths, apply)
    })
    .await
    {
        Ok(Ok(result)) => (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "cleanup": result })),
        ),
        Ok(Err(reason)) => error(StatusCode::INTERNAL_SERVER_ERROR, reason),
        Err(reason) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("节点缓存清理任务异常结束: {reason}"),
        ),
    }
}

fn error(
    status: StatusCode,
    reason: impl std::fmt::Display,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "error": reason.to_string(),
        })),
    )
}
