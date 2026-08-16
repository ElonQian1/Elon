use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{project_auth::auth_from_headers, types::AppState};

use super::{auth, gc_contract::*, json_error, require_node_owner};

#[derive(Debug, Serialize)]
struct GcNodeCommand {
    schema: &'static str,
    command: &'static str,
    request_id: String,
    node_id: String,
    expires_at: String,
    options: crate::store::rust_cache::gc_requests::RustCacheGcOptions,
    plan_id: Option<String>,
    plan_digest: Option<String>,
}

pub(crate) async fn create_request(
    Path(node_id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateGcRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(response) = require_node_owner(&state, &node_id, &user.id) {
        return response;
    }
    if !body.acknowledge_remote_gc {
        return json_error(
            StatusCode::BAD_REQUEST,
            "必须明确确认在目标电脑重新预演后执行缓存回收",
        );
    }
    match state
        .store
        .create_rust_cache_gc_request(&user.id, &node_id, body.options)
    {
        Ok(request) => request_response(&request),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn latest_request(
    Path(node_id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(response) = require_node_owner(&state, &node_id, &user.id) {
        return response;
    }
    match state.store.latest_rust_cache_gc_request(&user.id, &node_id) {
        Ok(Some(request)) => request_response(&request),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "该节点尚无缓存回收请求"),
        Err(error) => internal_error(&node_id, error),
    }
}

pub(crate) async fn approve_request(
    Path((node_id, request_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ApproveGcRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(response) = require_node_owner(&state, &node_id, &user.id) {
        return response;
    }
    if body.acknowledgement != "APPROVE_EXACT_GC_PLAN" {
        return json_error(StatusCode::BAD_REQUEST, "审批确认文本不匹配");
    }
    match state
        .store
        .rust_cache_gc_request_for_node(&node_id, &request_id)
    {
        Ok(Some(request)) if request.owner_user_id == user.id => {}
        Ok(Some(_)) => return json_error(StatusCode::FORBIDDEN, "缓存回收请求不属于当前用户"),
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "缓存回收请求不属于该节点"),
        Err(error) => return internal_error(&node_id, error),
    }
    match state.store.approve_rust_cache_gc_request(
        &user.id,
        &request_id,
        &body.plan_id,
        &body.plan_digest,
    ) {
        Ok(request) => request_response(&request),
        Err(error) => json_error(StatusCode::CONFLICT, error.to_string()),
    }
}

pub(crate) async fn reject_request(
    Path((node_id, request_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(response) = require_node_owner(&state, &node_id, &user.id) {
        return response;
    }
    match state
        .store
        .rust_cache_gc_request_for_node(&node_id, &request_id)
    {
        Ok(Some(request)) if request.owner_user_id == user.id => {}
        Ok(Some(_)) => return json_error(StatusCode::FORBIDDEN, "缓存回收请求不属于当前用户"),
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "缓存回收请求不属于该节点"),
        Err(error) => return internal_error(&node_id, error),
    }
    match state
        .store
        .reject_rust_cache_gc_request(&user.id, &request_id)
    {
        Ok(request) => request_response(&request),
        Err(error) => json_error(StatusCode::CONFLICT, error.to_string()),
    }
}

pub(crate) async fn next_node_command(
    Path(node_id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = auth::authenticate_node_bearer(&state, &headers, &node_id) {
        return response;
    }
    match state.store.next_rust_cache_gc_request_for_node(&node_id) {
        Ok(Some(request)) => {
            let apply = request.status == "executing";
            Json(GcNodeCommand {
                schema: "elon.rust_cache.gc_node_command.v1",
                command: if apply { "apply_plan" } else { "generate_plan" },
                request_id: request.request_id,
                node_id: request.node_id,
                expires_at: request.expires_at,
                options: request.options,
                plan_id: request.plan_id,
                plan_digest: request.plan_digest,
            })
            .into_response()
        }
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => internal_error(&node_id, error),
    }
}

pub(crate) async fn upload_node_plan(
    Path((node_id, request_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<NodeGcPlanUpload>,
) -> Response {
    if let Err(response) = auth::authenticate_node_bearer(&state, &headers, &node_id) {
        return response;
    }
    let request = match state
        .store
        .rust_cache_gc_request_for_node(&node_id, &request_id)
    {
        Ok(Some(request)) => request,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "缓存回收请求不存在"),
        Err(error) => return internal_error(&node_id, error),
    };
    if let Err(error) = validate_plan_summary(&body.summary, &node_id, &request_id, request.options)
    {
        return json_error(StatusCode::BAD_REQUEST, error.to_string());
    }
    let summary_json = match serde_json::to_string(&body.summary) {
        Ok(json) => json,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    match state.store.record_rust_cache_gc_plan(
        &node_id,
        &request_id,
        &body.summary.plan_id,
        &body.summary.plan_digest,
        &summary_json,
    ) {
        Ok(request) => Json(serde_json::json!({
            "schema": "elon.rust_cache.gc_plan_ack.v1", "accepted": true,
            "request_id": request.request_id, "plan_id": request.plan_id,
            "plan_digest": request.plan_digest, "status": request.status
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::CONFLICT, error.to_string()),
    }
}

pub(crate) async fn upload_node_result(
    Path((node_id, request_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<NodeGcResultUpload>,
) -> Response {
    if let Err(response) = auth::authenticate_node_bearer(&state, &headers, &node_id) {
        return response;
    }
    let request = match state
        .store
        .rust_cache_gc_request_for_node(&node_id, &request_id)
    {
        Ok(Some(request)) => request,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "缓存回收请求不存在"),
        Err(error) => return internal_error(&node_id, error),
    };
    if let Err(error) = validate_result(
        &body,
        &node_id,
        &request_id,
        request.plan_id.as_deref(),
        request.plan_digest.as_deref(),
    ) {
        return json_error(StatusCode::BAD_REQUEST, error.to_string());
    }
    let result_json = match serde_json::to_string(&body) {
        Ok(json) => json,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    match state.store.finish_rust_cache_gc_request(
        &node_id,
        &request_id,
        &body.status,
        &result_json,
        body.failure_code.as_deref(),
    ) {
        Ok(request) => Json(serde_json::json!({
            "schema": "elon.rust_cache.gc_result_ack.v1", "accepted": true,
            "request_id": request.request_id, "status": request.status
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::CONFLICT, error.to_string()),
    }
}

fn request_response(
    request: &crate::store::rust_cache::gc_requests::NodeRustCacheGcRequest,
) -> Response {
    let plan = request
        .plan_summary_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok());
    let result = request
        .result_summary_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok());
    Json(serde_json::json!({
        "schema": "elon.rust_cache.gc_request.v1", "request_id": request.request_id,
        "node_id": request.node_id, "status": request.status, "options": request.options,
        "plan": plan, "result": result, "failure_code": request.failure_code,
        "created_at": request.created_at, "updated_at": request.updated_at,
        "expires_at": request.expires_at, "server_has_absolute_paths": false
    }))
    .into_response()
}

fn internal_error(node_id: &str, error: anyhow::Error) -> Response {
    tracing::warn!(node_id, %error, "Rust cache GC approval operation failed");
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "缓存回收审批操作失败")
}
