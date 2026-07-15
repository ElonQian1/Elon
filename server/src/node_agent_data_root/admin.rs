use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;

use crate::NodeRuntime;

pub(crate) async fn get(State(runtime): State<Arc<NodeRuntime>>) -> Json<serde_json::Value> {
    let state = runtime.node_data_root.read().await.clone();
    let mut payload = state.status_payload();
    payload["active_task_count"] = serde_json::json!(runtime
        .active_cli_prompts
        .views_without_approvals()
        .await
        .len());
    if let Some(paths) = state.paths {
        if let Ok(build_cache) =
            tokio::task::spawn_blocking(move || crate::node_agent_build_runtime::status(&paths))
                .await
        {
            payload["build_cache"] = serde_json::json!(build_cache);
        }
    }
    payload["cache_advisor"] = serde_json::json!(runtime
        .cache_advisor
        .report(runtime.node_data_root.read().await.paths.as_ref(), false));
    Json(payload)
}

pub(crate) async fn analyze(
    State(runtime): State<Arc<NodeRuntime>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let data_paths = runtime.node_data_root.read().await.paths.clone();
    let advisor = runtime.cache_advisor.clone();
    match tokio::task::spawn_blocking(move || advisor.report(data_paths.as_ref(), true)).await {
        Ok(report) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "cache_advisor": report,
            })),
        ),
        Err(reason) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("项目数据架构体检任务异常结束: {reason}"),
        ),
    }
}

#[derive(Deserialize)]
pub(crate) struct SetRequest {
    root_path: String,
}

pub(crate) async fn set(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(request): Json<SetRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Root changes, project-task admission, and manual cleanup share one gate.
    // Holding it through persistence closes the check-then-switch race: either
    // a task has already created its lease, or it observes the new root.
    let _transition = runtime.node_data_root_transition.lock().await;
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
    if let Some(paths) = current.paths.as_ref() {
        let active_leases = crate::node_agent_build_runtime::active_leases(paths);
        if active_leases > 0 {
            return error(
                StatusCode::CONFLICT,
                format!("当前仍有 {active_leases} 个项目任务运行，不能切换节点数据根"),
            );
        }
    }
    if let Err(reason) =
        super::validate_no_root_overlap(&request.root_path, &current, &runtime.install_id)
    {
        return error(StatusCode::BAD_REQUEST, reason);
    }
    let paths = match super::validate_and_prepare(&request.root_path, &runtime.install_id) {
        Ok(paths) => paths,
        Err(reason) => return error(StatusCode::BAD_REQUEST, reason),
    };
    if let Err(reason) =
        super::validate_no_canonical_root_overlap(paths.root(), &current, &runtime.install_id)
    {
        return error(StatusCode::BAD_REQUEST, reason);
    }
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
    // Preview is read-only and may be slightly stale; only an applying cleanup
    // blocks new project-task admission for the full deletion window.
    let _transition = if request.apply {
        Some(runtime.node_data_root_transition.clone().lock_owned().await)
    } else {
        None
    };
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
    if request.apply {
        let active_leases = crate::node_agent_build_runtime::active_leases(&paths);
        if active_leases > 0 {
            return error(
                StatusCode::CONFLICT,
                format!("当前仍有 {active_leases} 个项目任务运行，不能清理节点构建缓存"),
            );
        }
    }
    let apply = request.apply;
    let install_id = runtime.install_id.clone();
    match tokio::task::spawn_blocking(move || {
        // Keep the owned transition guard inside the blocking job. If the HTTP
        // client disconnects, Tokio may drop the JoinHandle while deletion
        // continues; the guard must survive until that deletion really ends.
        let _transition = _transition;
        crate::node_agent_build_runtime::cleanup_rebuildable(&paths, &install_id, apply)
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
