use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::NodeRuntime;

use super::model::{CreateFitRunRequest, FitCommand, FitSessionContext};
use super::service::BatchAcceptRequest;
use super::workspace_revision::workspace_fingerprint;
use crate::node_agent_android_live::fit_learning::learning_summary;

/// 薄路由层。根模块只需将它 merge 到现有受保护 Android Live Router。
///
/// 接线约定：`NodeRuntime` 增加 `ui_fit_runs: Arc<FitRunService>`。
pub(crate) fn protected_routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route(
            "/api/android-live/sessions/:session_id/fit-runs",
            get(list_runs_handler).post(create_run_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/fit-runs/batch-accept",
            post(batch_accept_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/fit-learning",
            get(fit_learning_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/fit-runs/:run_id",
            get(get_run_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/fit-runs/:run_id/commands",
            post(command_handler),
        )
}

async fn fit_learning_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    let context = match session_context(&runtime, &session_id).await {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match learning_summary(&context.project_root) {
        Ok(summary) => Json(json!({ "ok": true, "summary": summary })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn batch_accept_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<BatchAcceptRequest>,
) -> Response {
    let context = match session_context(&runtime, &session_id).await {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match runtime.ui_fit_runs.accept_batch(context, request).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::CONFLICT, format!("{error:#}")),
    }
}

async fn create_run_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<CreateFitRunRequest>,
) -> Response {
    let context = match session_context(&runtime, &session_id).await {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match runtime.ui_fit_runs.create_run(context, request).await {
        Ok(run) => Json(json!({ "ok": true, "run": run })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn list_runs_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    let context = match session_context(&runtime, &session_id).await {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match runtime.ui_fit_runs.list_runs(&context) {
        Ok(runs) => Json(json!({ "ok": true, "runs": runs })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn get_run_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((session_id, run_id)): Path<(String, String)>,
) -> Response {
    let context = match session_context(&runtime, &session_id).await {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match runtime.ui_fit_runs.get_run(&context, &run_id) {
        Ok(run) => Json(json!({ "ok": true, "run": run })).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    }
}

async fn command_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((session_id, run_id)): Path<(String, String)>,
    Json(command): Json<FitCommand>,
) -> Response {
    let context = match session_context(&runtime, &session_id).await {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match runtime.ui_fit_runs.command(context, &run_id, command).await {
        Ok(result) => Json(json!({
            "ok": true,
            "run": result.run,
            "idempotent": result.idempotent,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::CONFLICT, format!("{error:#}")),
    }
}

async fn session_context(
    runtime: &NodeRuntime,
    session_id: &str,
) -> anyhow::Result<FitSessionContext> {
    let session = runtime.live_ui.session(session_id).await?;
    let view = session.view().await;
    let project_root = session
        .project_root
        .clone()
        .ok_or_else(|| anyhow::anyhow!("FitRun 需要本机项目目录"))?;
    let source_revision = workspace_fingerprint(&project_root)?;
    Ok(FitSessionContext {
        session_id: session.id.clone(),
        project_root,
        package_name: session.package_name.clone(),
        device_id: session.device_id.clone(),
        runtime_build_id: view.runtime_build_id,
        tree_revision: view.tree_revision,
        source_revision,
    })
}

fn json_error(status: StatusCode, error: impl Into<String>) -> Response {
    (status, Json(json!({ "ok": false, "error": error.into() }))).into_response()
}
