//! HTTP adapter for the V1 open-commerce service layer.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_directory_model::SetDirectoryPublicationRequest,
    open_commerce_directory_service,
    open_commerce_integration_model::{
        CreateIntegrationRequest, RecordSyncReceiptRequest, SetIntegrationEnabledRequest,
    },
    open_commerce_model::{
        normalize_app_id, CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest,
        InvokeCapabilityRequest, UpdateCapabilityRequest, UpdateMerchantRequest,
    },
    open_commerce_runtime_model::UpsertRuntimeBindingRequest,
    open_commerce_runtime_service,
    open_commerce_service::{self, OpenCommerceActor},
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

const DEFAULT_HTTP_APP_ID: &str = "pc-web";

#[derive(Debug, Deserialize)]
struct MerchantSearchQuery {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    capability: Option<String>,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    #[serde(default = "default_audit_limit")]
    limit: usize,
}

struct ProjectCaller {
    user_id: String,
    role: String,
    app_id: String,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/open-commerce/connector-contract",
            get(connector_contract),
        )
        .route(
            "/api/projects/:project_id/open-commerce/overview",
            get(project_overview),
        )
        .route(
            "/api/projects/:project_id/open-commerce/development-context",
            get(project_development_context),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants",
            post(create_merchant),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id",
            patch(update_merchant),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/directory-publication",
            put(set_directory_publication),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/capabilities",
            post(publish_capability),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/runtime",
            put(upsert_runtime_binding),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/runtime/verify",
            post(verify_runtime_binding),
        )
        .route(
            "/api/projects/:project_id/open-commerce/capabilities/:capability_id",
            patch(update_capability),
        )
        .route(
            "/api/projects/:project_id/open-commerce/grants",
            post(create_grant),
        )
        .route(
            "/api/projects/:project_id/open-commerce/grants/:grant_id/revoke",
            post(revoke_grant),
        )
        .route(
            "/api/projects/:project_id/open-commerce/audit",
            get(project_audit),
        )
        .route(
            "/api/projects/:project_id/open-commerce/integrations",
            post(create_integration),
        )
        .route(
            "/api/projects/:project_id/open-commerce/integrations/:integration_id/enabled",
            patch(set_integration_enabled),
        )
        .route(
            "/api/projects/:project_id/open-commerce/sync-receipts",
            post(record_sync_receipt),
        )
        .route("/api/open-commerce/merchants", get(search_merchants))
        .route(
            "/api/open-commerce/merchants/:merchant_id",
            get(get_merchant),
        )
        .route("/api/open-commerce/invoke", post(invoke_capability))
}

async fn connector_contract() -> Response {
    Json(crate::open_commerce_connector_contract::contract()).into_response()
}

async fn project_development_context(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(open_commerce_service::development_context(
        &state.store,
        &project_id,
    ))
}

async fn project_overview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(open_commerce_service::overview(&state.store, &project_id))
}

async fn create_merchant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateMerchantRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let actor = actor(&caller);
    service_response(open_commerce_service::create_merchant(
        &state.store,
        &project_id,
        &actor,
        request,
    ))
}

async fn update_merchant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
    Json(request): Json<UpdateMerchantRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_service::update_merchant(
        &state.store,
        &project_id,
        &merchant_id,
        &actor(&caller),
        request,
    ))
}

async fn set_directory_publication(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
    Json(request): Json<SetDirectoryPublicationRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_directory_service::set_publication(
        &state.store,
        &project_id,
        &merchant_id,
        &actor(&caller),
        request.published,
    ))
}

async fn publish_capability(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
    Json(request): Json<CreateCapabilityRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_service::publish_capability(
        &state.store,
        &project_id,
        &merchant_id,
        &actor(&caller),
        request,
    ))
}

async fn update_capability(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, capability_id)): Path<(String, String)>,
    Json(request): Json<UpdateCapabilityRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_service::update_capability(
        &state.store,
        &project_id,
        &capability_id,
        &actor(&caller),
        request,
    ))
}

async fn upsert_runtime_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
    Json(request): Json<UpsertRuntimeBindingRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_runtime_service::upsert_binding(
        &state.store,
        &project_id,
        &merchant_id,
        &actor(&caller),
        request,
    ))
}

async fn verify_runtime_binding(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_runtime_service::verify_binding(
            &state.store,
            &project_id,
            &merchant_id,
            &actor(&caller),
        )
        .await,
    )
}

async fn create_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateGrantRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_service::create_grant(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn revoke_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, grant_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_service::revoke_grant(
        &state.store,
        &project_id,
        &grant_id,
        &actor(&caller),
    ))
}

async fn project_audit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<AuditQuery>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(
        state
            .store
            .list_project_open_commerce_audit(&project_id, query.limit)
            .map(|events| json!({"schema": "open_commerce.audit.v1", "events": events})),
    )
}

async fn create_integration(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateIntegrationRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_service::create_integration(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn set_integration_enabled(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, integration_id)): Path<(String, String)>,
    Json(request): Json<SetIntegrationEnabledRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_service::set_integration_enabled(
        &state.store,
        &project_id,
        &integration_id,
        &actor(&caller),
        request.enabled,
    ))
}

async fn record_sync_receipt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<RecordSyncReceiptRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_service::record_sync_receipt(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn search_merchants(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MerchantSearchQuery>,
) -> Response {
    service_response(
        open_commerce_directory_service::discover_merchants(
            &state.store,
            query.query.as_deref(),
            query.capability.as_deref(),
            query.limit,
        )
        .map(|merchants| json!({"schema": "open_commerce.discovery.v1", "merchants": merchants})),
    )
}

async fn get_merchant(
    State(state): State<Arc<AppState>>,
    Path(merchant_id): Path<String>,
) -> Response {
    service_response(open_commerce_directory_service::discover_merchant(
        &state.store,
        &merchant_id,
    ))
}

async fn invoke_capability(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<InvokeCapabilityRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let app_id = match app_id_from_headers(&headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if request.requester_app_id != app_id {
        return json_error(
            StatusCode::BAD_REQUEST,
            "requester_app_id 必须与 x-elon-app-id 一致",
        );
    }
    let merchant = match state.store.open_commerce_merchant(&request.merchant_id) {
        Ok(value) => value,
        Err(error) => return service_error(error),
    };
    let role = project_access(&state, &user_id, &merchant.project_id)
        .ok()
        .map(|access| access.role);
    let actor = OpenCommerceActor {
        user_id: &user_id,
        app_id: &app_id,
        project_role: role.as_deref(),
    };
    service_response(open_commerce_service::invoke(&state.store, &actor, request).await)
}

fn project_caller(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<ProjectCaller, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    let app_id = app_id_from_headers(headers)?;
    Ok(ProjectCaller {
        user_id: user.id,
        role: access.role,
        app_id,
    })
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn app_id_from_headers(headers: &HeaderMap) -> Result<String, Response> {
    let value = headers
        .get("x-elon-app-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(DEFAULT_HTTP_APP_ID);
    normalize_app_id(value).map_err(|error| json_error(StatusCode::BAD_REQUEST, error))
}

fn actor(caller: &ProjectCaller) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id: &caller.user_id,
        app_id: &caller.app_id,
        project_role: Some(&caller.role),
    }
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => service_error(error),
    }
}

fn service_error(error: anyhow::Error) -> Response {
    let rate_limited =
        error.is::<crate::open_commerce_rate_limit_model::OpenCommerceRateLimitExceeded>();
    let app_blocked = error.is::<crate::open_commerce_app_block_model::OpenCommerceAppBlocked>();
    let message = format!("{error:#}");
    let status = if rate_limited {
        StatusCode::TOO_MANY_REQUESTS
    } else if app_blocked {
        StatusCode::FORBIDDEN
    } else if message.contains("权限") || message.contains("授权") {
        StatusCode::FORBIDDEN
    } else if message.contains("不存在") || message.contains("未发布") {
        StatusCode::NOT_FOUND
    } else if message.contains("相同") || message.contains("已经") || message.contains("冲突")
    {
        StatusCode::CONFLICT
    } else {
        StatusCode::BAD_REQUEST
    };
    json_error(status, message)
}

fn default_search_limit() -> usize {
    20
}

fn default_audit_limit() -> usize {
    50
}

#[cfg(test)]
mod tests {
    use super::service_error;
    use axum::http::StatusCode;

    #[test]
    fn service_errors_keep_access_and_conflict_semantics() {
        assert_eq!(
            service_error(anyhow::anyhow!("当前调用方没有项目编辑权限")).status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            service_error(anyhow::anyhow!("相同幂等键不能用于不同输入")).status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            service_error(anyhow::anyhow!(
                crate::open_commerce_rate_limit_model::OpenCommerceRateLimitExceeded {
                    retry_after_seconds: 3,
                    max_requests: 2,
                    window_seconds: 60,
                }
            ))
            .status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            service_error(anyhow::anyhow!(
                crate::open_commerce_app_block_model::OpenCommerceAppBlocked {
                    requester_app_id: "consumer.blocked".to_string(),
                }
            ))
            .status(),
            StatusCode::FORBIDDEN
        );
    }
}
