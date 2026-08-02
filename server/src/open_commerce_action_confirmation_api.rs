use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};

use crate::{
    open_commerce_action_confirmation_model::ConfirmActionConfirmationRequest,
    open_commerce_action_confirmation_service,
    open_commerce_developer_model::DeveloperInvokeRequest,
    open_commerce_model::{normalize_app_id, InvokeCapabilityRequest},
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

const DEFAULT_HTTP_APP_ID: &str = "pc-web";

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/open-commerce/action-confirmations",
            post(prepare_authenticated),
        )
        .route(
            "/api/open-commerce/action-confirmations/:confirmation_id/confirm",
            post(confirm_authenticated),
        )
        .route(
            "/api/open-commerce/developer/action-confirmations",
            post(prepare_developer),
        )
        .route(
            "/api/open-commerce/developer/action-confirmations/:confirmation_id/confirm",
            post(confirm_developer),
        )
}

async fn prepare_authenticated(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<InvokeCapabilityRequest>,
) -> Response {
    let (user_id, app_id) = match authenticated_actor_identity(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let merchant = match state.store.open_commerce_merchant(&request.merchant_id) {
        Ok(value) => value,
        Err(error) => return service_error(error),
    };
    let role = project_access(&state, &user_id, &merchant.project_id)
        .ok()
        .map(|access| access.role);
    service_response(open_commerce_action_confirmation_service::prepare(
        &state.store,
        &OpenCommerceActor {
            user_id: &user_id,
            app_id: &app_id,
            project_role: role.as_deref(),
        },
        request,
    ))
}

async fn confirm_authenticated(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(confirmation_id): Path<String>,
    Json(request): Json<ConfirmActionConfirmationRequest>,
) -> Response {
    let (user_id, app_id) = match authenticated_actor_identity(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_action_confirmation_service::confirm(
        &state.store,
        &OpenCommerceActor {
            user_id: &user_id,
            app_id: &app_id,
            project_role: None,
        },
        &confirmation_id,
        &request.confirmation_phrase,
    ))
}

async fn prepare_developer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<DeveloperInvokeRequest>,
) -> Response {
    let app = match developer_app(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let merchant = match state.store.open_commerce_merchant(&request.merchant_id) {
        Ok(value) => value,
        Err(error) => return service_error(error),
    };
    let role = project_access(&state, &app.owner_user_id, &merchant.project_id)
        .ok()
        .map(|access| access.role);
    service_response(open_commerce_action_confirmation_service::prepare(
        &state.store,
        &OpenCommerceActor {
            user_id: &app.owner_user_id,
            app_id: &app.app_id,
            project_role: role.as_deref(),
        },
        InvokeCapabilityRequest {
            merchant_id: request.merchant_id,
            capability_key: request.capability_key,
            requester_app_id: app.app_id.clone(),
            grant_id: request.grant_id,
            idempotency_key: request.idempotency_key,
            input: request.input,
        },
    ))
}

async fn confirm_developer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(confirmation_id): Path<String>,
    Json(request): Json<ConfirmActionConfirmationRequest>,
) -> Response {
    let app = match developer_app(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_action_confirmation_service::confirm(
        &state.store,
        &OpenCommerceActor {
            user_id: &app.owner_user_id,
            app_id: &app.app_id,
            project_role: None,
        },
        &confirmation_id,
        &request.confirmation_phrase,
    ))
}

fn authenticated_actor_identity(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, String), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let raw_app_id = headers
        .get("x-elon-app-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(DEFAULT_HTTP_APP_ID);
    let app_id =
        normalize_app_id(raw_app_id).map_err(|error| json_error(StatusCode::BAD_REQUEST, error))?;
    Ok((user.id, app_id))
}

fn developer_app(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::open_commerce_developer_model::OpenCommerceDeveloperApp, Response> {
    let token = bearer_token(headers)?;
    state
        .store
        .authenticate_open_commerce_developer_app(&token)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn bearer_token(headers: &HeaderMap) -> Result<String, Response> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "缺少测试应用凭据"))?;
    Ok(value.to_string())
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => service_error(error),
    }
}

fn service_error(error: anyhow::Error) -> Response {
    let schema_violation =
        error.is::<crate::open_commerce_capability_schema::CapabilitySchemaViolation>();
    let app_blocked = error.is::<crate::open_commerce_app_block_model::OpenCommerceAppBlocked>();
    let message = format!("{error:#}");
    let status = if schema_violation {
        StatusCode::UNPROCESSABLE_ENTITY
    } else if message.contains("相同") || message.contains("已经") || message.contains("冲突")
    {
        StatusCode::CONFLICT
    } else if app_blocked
        || message.contains("权限")
        || message.contains("授权")
        || message.contains("不属于")
    {
        StatusCode::FORBIDDEN
    } else if message.contains("不存在") || message.contains("未发布") {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::BAD_REQUEST
    };
    json_error(status, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_errors_keep_contract_and_conflict_semantics() {
        assert_eq!(
            service_error(anyhow::anyhow!(
                crate::open_commerce_capability_schema::CapabilitySchemaViolation {
                    code: "required",
                    path: "$.quantity".to_string(),
                    side: "输入 schema",
                }
            ))
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            service_error(anyhow::anyhow!("相同幂等键不能用于不同输入或授权")).status(),
            StatusCode::CONFLICT
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
