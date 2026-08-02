use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_authorization_decision::grant_request_for_authorization,
    open_commerce_consumer,
    open_commerce_consumer_model::ConsumerDiscoveryRequest,
    open_commerce_developer_model::{
        CreateAuthorizationRequest, CreateDeveloperAppRequest, DecideAuthorizationRequest,
        DeveloperInvokeRequest,
    },
    open_commerce_model::InvokeCapabilityRequest,
    open_commerce_service::{self, OpenCommerceActor},
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps",
            get(list_developer_apps).post(create_developer_app),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/rotate-token",
            post(rotate_developer_app_token),
        )
        .route(
            "/api/projects/:project_id/open-commerce/authorization-requests",
            get(list_authorization_requests),
        )
        .route(
            "/api/projects/:project_id/open-commerce/authorization-requests/:request_id/approve",
            post(approve_authorization_request),
        )
        .route(
            "/api/projects/:project_id/open-commerce/authorization-requests/:request_id/reject",
            post(reject_authorization_request),
        )
        .route(
            "/api/open-commerce/sandbox/discover",
            post(discover_for_consumer),
        )
        .route(
            "/api/open-commerce/authorization-requests",
            post(create_authorization_request),
        )
        .route(
            "/api/open-commerce/developer/invoke",
            post(developer_invoke),
        )
}

async fn list_developer_apps(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(
        state
            .store
            .list_project_open_commerce_developer_apps(&project_id)
            .map(|apps| json!({"schema":"open_commerce.developer_apps.v1","apps":apps})),
    )
}

async fn create_developer_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateDeveloperAppRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以注册开发者应用");
    }
    let credential =
        match state
            .store
            .create_open_commerce_developer_app(&project_id, &user_id, request)
        {
            Ok(credential) => credential,
            Err(error) => return service_error(error),
        };
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "developer_app.created",
        "developer_app",
        &credential.app.id,
        &json!({
            "app_id": credential.app.app_id,
            "environment": credential.app.environment
        }),
    ) {
        return service_error(error);
    }
    Json(credential).into_response()
}

async fn rotate_developer_app_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以轮换测试凭据");
    }
    let credential = match state
        .store
        .rotate_open_commerce_developer_app_token(&project_id, &app_record_id)
    {
        Ok(credential) => credential,
        Err(error) => return service_error(error),
    };
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "developer_app.token_rotated",
        "developer_app",
        &credential.app.id,
        &json!({"app_id": credential.app.app_id}),
    ) {
        return service_error(error);
    }
    Json(credential).into_response()
}

async fn discover_for_consumer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ConsumerDiscoveryRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    service_response(open_commerce_consumer::discover(
        &state.store,
        &user.id,
        request,
    ))
}

async fn create_authorization_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateAuthorizationRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    let authorization =
        match open_commerce_consumer::create_authorization_request(&state.store, &user.id, request)
        {
            Ok(authorization) => authorization,
            Err(error) => return service_error(error),
        };
    if let Err(error) = state.store.record_open_commerce_audit(
        &authorization.merchant_project_id,
        &user.id,
        Some(&authorization.requester_app_id),
        "authorization.requested",
        "authorization_request",
        &authorization.id,
        &json!({
            "merchant_id": authorization.merchant_id,
            "requester_app_id": authorization.requester_app_id,
            "scopes": authorization.scopes
        }),
    ) {
        return service_error(error);
    }
    Json(authorization).into_response()
}

async fn list_authorization_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(
        state
            .store
            .list_project_open_commerce_authorization_requests(&project_id, 100)
            .map(|requests| {
                json!({"schema":"open_commerce.authorization_requests.v1","requests":requests})
            }),
    )
}

async fn approve_authorization_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, request_id)): Path<(String, String)>,
    Json(decision): Json<DecideAuthorizationRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有商户项目编辑者可以批准授权");
    }
    let request = match state.store.open_commerce_authorization_request(&request_id) {
        Ok(request) => request,
        Err(error) => return service_error(error),
    };
    if request.merchant_project_id != project_id {
        return json_error(StatusCode::FORBIDDEN, "授权请求不属于当前商户项目");
    }
    if request.status != "pending" {
        return Json(request).into_response();
    }
    if let Err(error) = state
        .store
        .ensure_open_commerce_developer_app_owned_by_user(
            &request.requester_app_id,
            &request.requester_user_id,
        )
    {
        return service_error(error);
    }
    let actor = OpenCommerceActor {
        user_id: &user_id,
        app_id: "pc-web",
        project_role: Some(&role),
    };
    let grant = match open_commerce_service::create_grant(
        &state.store,
        &project_id,
        &actor,
        grant_request_for_authorization(&request, &decision),
    ) {
        Ok(grant) => grant,
        Err(error) => return service_error(error),
    };
    let authorization = match state.store.decide_open_commerce_authorization_request(
        &project_id,
        &request_id,
        &user_id,
        "approved",
        &decision.reason,
        Some(&grant.id),
    ) {
        Ok(authorization) => authorization,
        Err(error) => return service_error(error),
    };
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "authorization.approved",
        "authorization_request",
        &authorization.id,
        &json!({
            "requester_app_id": authorization.requester_app_id,
            "grant_id": grant.id,
            "expires_at": grant.expires_at,
            "max_invocations": grant.max_invocations,
            "max_amount_micros": grant.max_amount_micros,
            "budget_currency": grant.budget_currency
        }),
    ) {
        return service_error(error);
    }
    Json(authorization).into_response()
}

async fn reject_authorization_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, request_id)): Path<(String, String)>,
    Json(decision): Json<DecideAuthorizationRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有商户项目编辑者可以拒绝授权");
    }
    let authorization = match state.store.decide_open_commerce_authorization_request(
        &project_id,
        &request_id,
        &user_id,
        "rejected",
        &decision.reason,
        None,
    ) {
        Ok(authorization) => authorization,
        Err(error) => return service_error(error),
    };
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "authorization.rejected",
        "authorization_request",
        &authorization.id,
        &json!({"requester_app_id": authorization.requester_app_id}),
    ) {
        return service_error(error);
    }
    Json(authorization).into_response()
}

async fn developer_invoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<DeveloperInvokeRequest>,
) -> Response {
    let token = match bearer_token(&headers) {
        Ok(token) => token,
        Err(response) => return response,
    };
    let app = match state.store.authenticate_open_commerce_developer_app(&token) {
        Ok(app) => app,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    let merchant = match state.store.open_commerce_merchant(&request.merchant_id) {
        Ok(merchant) => merchant,
        Err(error) => return service_error(error),
    };
    let role = project_access(&state, &app.owner_user_id, &merchant.project_id)
        .ok()
        .map(|access| access.role);
    let actor = OpenCommerceActor {
        user_id: &app.owner_user_id,
        app_id: &app.app_id,
        project_role: role.as_deref(),
    };
    let action_confirmation_id = request.action_confirmation_id.clone();
    service_response(
        open_commerce_service::invoke_with_action_confirmation(
            &state.store,
            &actor,
            InvokeCapabilityRequest {
                merchant_id: request.merchant_id,
                capability_key: request.capability_key,
                requester_app_id: app.app_id.clone(),
                grant_id: request.grant_id,
                idempotency_key: request.idempotency_key,
                input: request.input,
            },
            action_confirmation_id.as_deref(),
        )
        .await,
    )
}

fn project_caller(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(String, String), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    Ok((user.id, access.role))
}

fn bearer_token(headers: &HeaderMap) -> Result<String, Response> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "缺少开发者测试凭据"))
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
    let schema_violation =
        error.is::<crate::open_commerce_capability_schema::CapabilitySchemaViolation>();
    let grant_budget_exceeded =
        error.is::<crate::open_commerce_grant_budget_model::OpenCommerceGrantBudgetExceeded>();
    let app_blocked = error.is::<crate::open_commerce_app_block_model::OpenCommerceAppBlocked>();
    let message = format!("{error:#}");
    let status = if schema_violation {
        StatusCode::UNPROCESSABLE_ENTITY
    } else if rate_limited {
        StatusCode::TOO_MANY_REQUESTS
    } else if app_blocked || grant_budget_exceeded {
        StatusCode::FORBIDDEN
    } else if message.contains("权限") || message.contains("不能代表") {
        StatusCode::FORBIDDEN
    } else if message.contains("不存在") {
        StatusCode::NOT_FOUND
    } else if message.contains("已有") || message.contains("冲突") {
        StatusCode::CONFLICT
    } else {
        StatusCode::BAD_REQUEST
    };
    json_error(status, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_violation_is_unprocessable_for_developer_clients() {
        let response = service_error(anyhow::Error::new(
            crate::open_commerce_capability_schema::CapabilitySchemaViolation {
                code: "required",
                path: "$.items".to_string(),
                side: "输入 schema",
            },
        ));
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
