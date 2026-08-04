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
        .route("/api/android-live/design/intents/plan", post(plan_intent))
        .route(
            "/api/android-live/design/intents/:plan_id",
            post(get_intent_plan),
        )
        .route(
            "/api/android-live/design/events/checkpoints/:consumer_id/:task_id",
            post(get_event_checkpoint),
        )
        .route(
            "/api/android-live/design/events/checkpoints/:consumer_id/:task_id/commit",
            post(commit_event_checkpoint),
        )
}

async fn plan_intent(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    super::design_http::call(&runtime, "ui_plan_design_intent", arguments).await
}

async fn get_intent_plan(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(plan_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["planId"] = json!(plan_id);
    super::design_http::call(&runtime, "ui_get_design_intent_plan", arguments).await
}

async fn get_event_checkpoint(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((consumer_id, task_id)): Path<(String, String)>,
    Json(mut arguments): Json<Value>,
) -> Response {
    inject_checkpoint(&mut arguments, consumer_id, task_id);
    super::design_http::call(&runtime, "ui_get_design_event_checkpoint", arguments).await
}

async fn commit_event_checkpoint(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((consumer_id, task_id)): Path<(String, String)>,
    Json(mut arguments): Json<Value>,
) -> Response {
    inject_checkpoint(&mut arguments, consumer_id, task_id);
    super::design_http::call(&runtime, "ui_commit_design_event_checkpoint", arguments).await
}

fn inject_checkpoint(arguments: &mut Value, consumer_id: String, task_id: String) {
    arguments["consumerId"] = json!(consumer_id);
    arguments["taskId"] = json!(task_id);
}
