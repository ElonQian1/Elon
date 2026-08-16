use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

mod auth;
mod contract;

use crate::{project_auth::auth_from_headers, types::AppState};
use contract::{types::FleetEnvelopeV1, validate_envelope};

#[derive(Debug, Serialize)]
struct FleetReportAck {
    schema: &'static str,
    accepted: bool,
    deduplicated: bool,
    envelope_id: String,
    node_id: String,
    report_sha256: String,
    received_at: String,
    destructive_actions_authorized: bool,
}

pub(crate) async fn upload_report(
    Path(node_id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(envelope): Json<FleetEnvelopeV1>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(response) = require_node_owner(&state, &node_id, &user.id) {
        return response;
    }
    persist_report(&state, &user.id, node_id, envelope).await
}

pub(crate) async fn upload_node_report(
    Path(node_id): Path<String>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(envelope): Json<FleetEnvelopeV1>,
) -> Response {
    let owner_user_id = match auth::authenticate_node_bearer(&state, &headers, &node_id) {
        Ok(owner_user_id) => owner_user_id,
        Err(response) => return response,
    };
    persist_report(&state, &owner_user_id, node_id, envelope).await
}

async fn persist_report(
    state: &AppState,
    owner_user_id: &str,
    node_id: String,
    envelope: FleetEnvelopeV1,
) -> Response {
    let validated = match validate_envelope(&node_id, envelope) {
        Ok(validated) => validated,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    let requested_envelope_id = validated.input.envelope_id.clone();
    let requested_report_hash = validated.input.report_sha256.clone();
    let write = match state
        .store
        .record_rust_cache_fleet_report(owner_user_id, validated.input)
    {
        Ok(write) => write,
        Err(error) => {
            tracing::warn!(node_id = %node_id, error = %error, "failed to persist Rust cache fleet report");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "缓存健康报告保存失败");
        }
    };

    Json(FleetReportAck {
        schema: "elon.rust_cache.fleet_ack.v1",
        accepted: true,
        deduplicated: write.deduplicated,
        envelope_id: requested_envelope_id,
        node_id,
        report_sha256: requested_report_hash,
        received_at: write.report.received_at,
        destructive_actions_authorized: false,
    })
    .into_response()
}

pub(crate) async fn latest_report(
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
    let report = match state
        .store
        .latest_rust_cache_fleet_report(&user.id, &node_id)
    {
        Ok(Some(report)) => report,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "该节点尚未上报缓存健康报告"),
        Err(error) => {
            tracing::warn!(node_id = %node_id, error = %error, "failed to read Rust cache fleet report");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "缓存健康报告读取失败");
        }
    };
    let payload = match serde_json::from_str::<serde_json::Value>(&report.report_json) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::error!(node_id = %node_id, error = %error, "stored Rust cache fleet report is invalid");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "缓存健康报告存储损坏");
        }
    };
    Json(serde_json::json!({
        "schema": "elon.rust_cache.fleet_latest.v1",
        "node_id": node_id,
        "envelope_id": report.envelope_id,
        "report_sha256": report.report_sha256,
        "platform_health": report.platform_health,
        "gc_review_recommended": report.gc_review_recommended,
        "active_writer_count": report.active_writer_count,
        "managed_size_bytes": report.managed_size_bytes,
        "generated_at": report.generated_at,
        "received_at": report.received_at,
        "report": payload,
        "destructive_actions_authorized": false
    }))
    .into_response()
}

fn require_node_owner(state: &AppState, node_id: &str, user_id: &str) -> Result<(), Response> {
    match state.store.get_node_credential_owner(node_id.trim()) {
        Ok(Some(owner)) if owner == user_id => Ok(()),
        Ok(Some(_)) => Err(json_error(
            StatusCode::FORBIDDEN,
            "无权访问其他用户的 PC 节点缓存报告",
        )),
        Ok(None) => Err(json_error(StatusCode::NOT_FOUND, "PC 节点不存在")),
        Err(error) => {
            tracing::warn!(node_id = %node_id, error = %error, "failed to verify Rust cache fleet report owner");
            Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PC 节点归属校验失败",
            ))
        }
    }
}

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(serde_json::json!({ "error": message.into() }))).into_response()
}
