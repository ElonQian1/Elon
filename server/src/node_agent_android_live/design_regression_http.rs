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
            "/api/android-live/design/regressions/baselines",
            post(create_baseline),
        )
        .route(
            "/api/android-live/design/regressions/baselines/:baseline_id",
            post(get_baseline),
        )
        .route(
            "/api/android-live/design/regressions/comparisons/plan",
            post(plan_comparison),
        )
        .route(
            "/api/android-live/design/regressions/comparisons/:comparison_id",
            post(get_comparison),
        )
        .route(
            "/api/android-live/design/regressions/comparisons/:comparison_id/complete",
            post(complete_comparison),
        )
}

async fn create_baseline(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    super::design_http::call(&runtime, "ui_create_design_regression_baseline", arguments).await
}

async fn get_baseline(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(baseline_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["baselineId"] = json!(baseline_id);
    super::design_http::call(&runtime, "ui_get_design_regression_baseline", arguments).await
}

async fn plan_comparison(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    super::design_http::call(&runtime, "ui_plan_design_regression_comparison", arguments).await
}

async fn get_comparison(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(comparison_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["comparisonId"] = json!(comparison_id);
    super::design_http::call(&runtime, "ui_get_design_regression_comparison", arguments).await
}

async fn complete_comparison(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(comparison_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["comparisonId"] = json!(comparison_id);
    super::design_http::call(
        &runtime,
        "ui_complete_design_regression_comparison",
        arguments,
    )
    .await
}
