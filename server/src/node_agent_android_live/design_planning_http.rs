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
            "/api/android-live/design/intents/:plan_id/start",
            post(start_intent_plan),
        )
        .route(
            "/api/android-live/design/intents/:plan_id/transition",
            post(transition_intent_plan),
        )
        .route(
            "/api/android-live/design/intents/:plan_id/actions/:action_order",
            post(record_intent_action),
        )
        .route(
            "/api/android-live/design/intents/:plan_id/replan",
            post(replan_intent),
        )
        .route(
            "/api/android-live/design/events/checkpoints/:consumer_id/:task_id",
            post(get_event_checkpoint),
        )
        .route(
            "/api/android-live/design/events/checkpoints/:consumer_id/:task_id/commit",
            post(commit_event_checkpoint),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/source-binding/health",
            post(check_binding_health),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/writeback/plan",
            post(plan_writeback),
        )
        .route(
            "/api/android-live/design/writeback/plans/:plan_id",
            post(get_writeback_plan),
        )
        .route(
            "/api/android-live/design/writeback/plans/:plan_id/decision",
            post(decide_writeback_plan),
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

async fn start_intent_plan(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(plan_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["planId"] = json!(plan_id);
    super::design_http::call(&runtime, "ui_start_design_intent_plan", arguments).await
}

async fn transition_intent_plan(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(plan_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["planId"] = json!(plan_id);
    super::design_http::call(&runtime, "ui_transition_design_intent_plan", arguments).await
}

async fn record_intent_action(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((plan_id, action_order)): Path<(String, u32)>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["planId"] = json!(plan_id);
    arguments["actionOrder"] = json!(action_order);
    super::design_http::call(&runtime, "ui_record_design_intent_action", arguments).await
}

async fn replan_intent(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(plan_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["planId"] = json!(plan_id);
    super::design_http::call(&runtime, "ui_replan_design_intent", arguments).await
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

async fn check_binding_health(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    super::design_http::call(&runtime, "ui_check_design_source_binding", arguments).await
}

async fn plan_writeback(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    super::design_http::call(&runtime, "ui_plan_design_writeback", arguments).await
}

async fn get_writeback_plan(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(plan_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["planId"] = json!(plan_id);
    super::design_http::call(&runtime, "ui_get_design_writeback_plan", arguments).await
}

async fn decide_writeback_plan(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(plan_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["planId"] = json!(plan_id);
    super::design_http::call(&runtime, "ui_decide_design_writeback_plan", arguments).await
}
