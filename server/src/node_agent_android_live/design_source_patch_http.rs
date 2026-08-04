use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::Response,
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};

use crate::NodeRuntime;

pub(super) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route(
            "/api/android-live/design/source-patches/propose",
            post(propose),
        )
        .route(
            "/api/android-live/design/source-patches/:proposal_id",
            post(get),
        )
        .route(
            "/api/android-live/design/source-patches/:proposal_id/decision",
            post(decide),
        )
        .route(
            "/api/android-live/design/source-patches/:proposal_id/apply",
            post(apply),
        )
        .route(
            "/api/android-live/design/source-patches/:proposal_id/rollback/plan",
            post(plan_rollback),
        )
}

async fn propose(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    super::design_http::call(&runtime, "ui_propose_design_source_patch", arguments).await
}

async fn get(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(proposal_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["proposalId"] = json!(proposal_id);
    super::design_http::call(&runtime, "ui_get_design_source_patch", arguments).await
}

async fn decide(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(proposal_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["proposalId"] = json!(proposal_id);
    super::design_http::call(&runtime, "ui_decide_design_source_patch", arguments).await
}

async fn apply(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(proposal_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["proposalId"] = json!(proposal_id);
    super::design_http::call(&runtime, "ui_apply_design_source_patch", arguments).await
}

async fn plan_rollback(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(proposal_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["proposalId"] = json!(proposal_id);
    super::design_http::call(&runtime, "ui_plan_design_source_rollback", arguments).await
}
