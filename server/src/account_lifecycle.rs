use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    account_security::{authenticated_account, coded_error},
    auth_request_guard::validate_request_id,
    types::AppState,
};

#[derive(Debug, Deserialize)]
struct DeletionPreflightRequest {
    request_id: String,
    #[serde(default)]
    confirm: bool,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/auth/account-export/manifest",
            get(account_export_manifest),
        )
        .route(
            "/api/auth/account-deletion/preflight",
            post(account_deletion_preflight),
        )
        .route(
            "/api/auth/safety/capabilities",
            get(auth_safety_capabilities),
        )
}

async fn auth_safety_capabilities() -> Json<serde_json::Value> {
    Json(crate::auth_safety_store::auth_safety_capabilities())
}

async fn account_export_manifest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let (user_id, token) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match lifecycle_snapshot(&state, &user_id, &token) {
        Ok(snapshot) => Json(serde_json::json!({
            "schema": "elon.account_export_manifest.v1",
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "account_id": user_id,
            "summary": snapshot,
            "available_exports": [
                {"category":"projects_and_workspaces", "endpoint":"/api/me/archive", "format":"json"},
                {"category":"security_events", "endpoint":"/api/auth/security/events", "format":"json"}
            ],
            "excluded_secrets": [
                "password_hash", "session_tokens", "recovery_code_hashes",
                "oauth_tokens", "provider_credentials", "codex_vault_ciphertext"
            ],
            "full_portability_bundle_available": false,
            "next": "download_each_available_export_until_versioned_bundle_is_implemented"
        }))
        .into_response(),
        Err(error) => lifecycle_error(error),
    }
}

async fn account_deletion_preflight(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<DeletionPreflightRequest>,
) -> Response {
    if !request.confirm || !validate_request_id(&request.request_id) {
        return coded_error(
            StatusCode::BAD_REQUEST,
            "invalid_deletion_preflight_request",
            "必须明确确认并提供有效 request_id",
        );
    }
    let (user_id, token) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let snapshot = match lifecycle_snapshot(&state, &user_id, &token) {
        Ok(snapshot) => snapshot,
        Err(error) => return lifecycle_error(error),
    };
    let mut blockers = Vec::new();
    if snapshot.owned_project_count > 0 {
        blockers.push(serde_json::json!({
            "code": "owned_projects_require_transfer_or_deletion",
            "count": snapshot.owned_project_count
        }));
    }
    if snapshot.owned_project_with_other_members_count > 0 {
        blockers.push(serde_json::json!({
            "code": "project_ownership_transfer_required",
            "count": snapshot.owned_project_with_other_members_count
        }));
    }
    if snapshot.codex_vault_slot_count > 0 {
        blockers.push(serde_json::json!({
            "code": "codex_vault_delete_required",
            "count": snapshot.codex_vault_slot_count
        }));
    }
    Json(serde_json::json!({
        "schema": "elon.account_deletion_preflight.v1",
        "preflight_passed": blockers.is_empty(),
        "deletion_execution_available": false,
        "blockers": blockers,
        "summary": snapshot,
        "required_final_controls": [
            "recent_reauthentication", "cooling_off_period", "cancel_window",
            "ownership_transfer", "provider_credential_purge_receipt", "append_only_tombstone"
        ],
        "message": "当前只提供注销预检，不会停用账号或删除任何数据。"
    }))
    .into_response()
}

#[derive(serde::Serialize)]
struct LifecycleSnapshot {
    linked_identity_count: usize,
    active_session_count: usize,
    security_event_count: u64,
    owned_project_count: usize,
    owned_project_with_other_members_count: usize,
    shared_project_count: usize,
    codex_vault_slot_count: usize,
}

fn lifecycle_snapshot(
    state: &AppState,
    user_id: &str,
    token: &str,
) -> anyhow::Result<LifecycleSnapshot> {
    let identities = state.store.list_linked_identities(user_id)?;
    let sessions = state.store.list_account_sessions(user_id, token)?;
    let projects = state.store.list_archive_projects_for_user(user_id)?;
    let vault_slots = state.store.list_user_codex_credential_slots(user_id)?;
    let owned = projects
        .iter()
        .filter(|project| project.project_origin_type == "self" && project.system_key.is_none())
        .collect::<Vec<_>>();
    Ok(LifecycleSnapshot {
        linked_identity_count: identities.len(),
        active_session_count: sessions.len(),
        security_event_count: state.store.count_account_security_events(user_id)?,
        owned_project_count: owned.len(),
        owned_project_with_other_members_count: owned
            .iter()
            .filter(|project| project.project.member_count > 1)
            .count(),
        shared_project_count: projects
            .iter()
            .filter(|project| project.project_origin_type == "member")
            .count(),
        codex_vault_slot_count: vault_slots.len(),
    })
}

fn lifecycle_error(error: anyhow::Error) -> Response {
    tracing::warn!(error = %error, "读取账号生命周期清单失败");
    coded_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "account_lifecycle_unavailable",
        "账号生命周期服务暂时不可用",
    )
}
