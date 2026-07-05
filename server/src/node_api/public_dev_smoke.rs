mod support;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{admin, types::AppState};

use support::{
    clean, default_smoke_prompt, resolve_side, run_smoke_direction, PublicDevMutualSmokeResponse,
};

const DEFAULT_LEFT_OWNER: &str = "钱一龙";
const DEFAULT_LEFT_NODE: &str = "一龙4060";
const DEFAULT_RIGHT_OWNER: &str = "夜云";
const DEFAULT_RIGHT_NODE: &str = "志伟4060";
const DEFAULT_CLI: &str = "codex";

#[derive(Debug, Deserialize)]
pub struct PublicDevMutualSmokeRequest {
    pub execute: Option<bool>,
    pub left_owner: Option<String>,
    pub left_node: Option<String>,
    pub right_owner: Option<String>,
    pub right_node: Option<String>,
    pub cli_name: Option<String>,
    pub prompt: Option<String>,
}

impl Default for PublicDevMutualSmokeRequest {
    fn default() -> Self {
        Self {
            execute: Some(false),
            left_owner: Some(DEFAULT_LEFT_OWNER.to_string()),
            left_node: Some(DEFAULT_LEFT_NODE.to_string()),
            right_owner: Some(DEFAULT_RIGHT_OWNER.to_string()),
            right_node: Some(DEFAULT_RIGHT_NODE.to_string()),
            cli_name: Some(DEFAULT_CLI.to_string()),
            prompt: None,
        }
    }
}

pub async fn admin_public_dev_mutual_smoke_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    run_public_dev_mutual_smoke(state, headers, PublicDevMutualSmokeRequest::default()).await
}

pub async fn admin_public_dev_mutual_smoke_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<PublicDevMutualSmokeRequest>,
) -> impl IntoResponse {
    run_public_dev_mutual_smoke(state, headers, req).await
}

async fn run_public_dev_mutual_smoke(
    state: Arc<AppState>,
    headers: HeaderMap,
    req: PublicDevMutualSmokeRequest,
) -> axum::response::Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"admin token required"})),
        )
            .into_response();
    }

    let execute = req.execute.unwrap_or(false);
    let cli_name = clean(req.cli_name.as_deref()).unwrap_or_else(|| DEFAULT_CLI.to_string());
    let prompt = req
        .prompt
        .unwrap_or_else(|| default_smoke_prompt(&cli_name));
    let left = match resolve_side(
        &state,
        req.left_owner.as_deref().unwrap_or(DEFAULT_LEFT_OWNER),
        req.left_node.as_deref().unwrap_or(DEFAULT_LEFT_NODE),
    )
    .await
    {
        Ok(side) => side,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let right = match resolve_side(
        &state,
        req.right_owner.as_deref().unwrap_or(DEFAULT_RIGHT_OWNER),
        req.right_node.as_deref().unwrap_or(DEFAULT_RIGHT_NODE),
    )
    .await
    {
        Ok(side) => side,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };

    let directions = vec![
        run_smoke_direction(
            &state,
            "钱一龙使用志伟4060",
            &left.owner,
            &right,
            &cli_name,
            &prompt,
            execute,
        )
        .await,
        run_smoke_direction(
            &state,
            "夜云使用一龙4060",
            &right.owner,
            &left,
            &cli_name,
            &prompt,
            execute,
        )
        .await,
    ];
    let expected_status = if execute { "passed" } else { "ready" };
    let ok = directions
        .iter()
        .all(|direction| direction.status == expected_status);

    Json(PublicDevMutualSmokeResponse {
        ok,
        execute,
        cli_name,
        generated_at: chrono::Utc::now().to_rfc3339(),
        left,
        right,
        directions,
    })
    .into_response()
}

fn json_error(status: StatusCode, error: anyhow::Error) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({ "error": error.to_string() })),
    )
        .into_response()
}
