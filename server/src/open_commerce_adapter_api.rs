use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::{
    open_commerce_adapter_model::{
        AdapterBusinessHandoffReceiptRequest, ConfirmedAdapterCredentialChangeRequest,
        RotateAdapterCredentialRequest,
    },
    open_commerce_adapter_service, open_commerce_business_handoff_service,
    open_commerce_model::normalize_app_id,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

struct ProjectCaller {
    user_id: String,
    role: String,
    app_id: String,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/adapter-credentials",
            get(list_credentials),
        )
        .route(
            "/api/projects/:project_id/open-commerce/integrations/:integration_id/adapter-credential/rotate",
            post(rotate_credential),
        )
        .route(
            "/api/projects/:project_id/open-commerce/adapter-credentials/:credential_id/revoke",
            post(revoke_credential),
        )
        .route(
            "/api/open-commerce/adapter/business-handoff-receipts",
            post(record_adapter_handoff),
        )
}

async fn list_credentials(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = authorize_project(&state, &headers, &project_id) {
        return response;
    }
    service_response(open_commerce_adapter_service::list_credentials(
        &state.store,
        &project_id,
    ))
}

async fn rotate_credential(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, integration_id)): Path<(String, String)>,
    Json(request): Json<RotateAdapterCredentialRequest>,
) -> Response {
    let caller = match authorize_project(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !request.confirmed_by_user {
        return json_error(
            StatusCode::BAD_REQUEST,
            "签发或轮换适配器凭据前必须取得用户明确确认",
        );
    }
    service_response(open_commerce_adapter_service::rotate_credential(
        &state.store,
        &project_id,
        &integration_id,
        request.expires_in_days,
        request.allow_task_claims,
        &OpenCommerceActor {
            user_id: &caller.user_id,
            app_id: &caller.app_id,
            project_role: Some(&caller.role),
        },
    ))
}

async fn revoke_credential(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, credential_id)): Path<(String, String)>,
    Json(request): Json<ConfirmedAdapterCredentialChangeRequest>,
) -> Response {
    let caller = match authorize_project(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !request.confirmed_by_user {
        return json_error(
            StatusCode::BAD_REQUEST,
            "撤销适配器凭据前必须取得用户明确确认",
        );
    }
    service_response(open_commerce_adapter_service::revoke_credential(
        &state.store,
        &project_id,
        &credential_id,
        &OpenCommerceActor {
            user_id: &caller.user_id,
            app_id: &caller.app_id,
            project_role: Some(&caller.role),
        },
    ))
}

async fn record_adapter_handoff(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<AdapterBusinessHandoffReceiptRequest>,
) -> Response {
    let token = match bearer_token(&headers) {
        Some(value) => value,
        None => return json_error(StatusCode::UNAUTHORIZED, "缺少适配器 Bearer 凭据"),
    };
    let credential = match state
        .store
        .authenticate_open_commerce_adapter_credential(token)
    {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, format!("{error:#}")),
    };
    service_response(
        open_commerce_business_handoff_service::record_adapter_receipt(
            &state.store,
            &credential,
            request,
        ),
    )
}

fn authorize_project(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<ProjectCaller, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    let app_id = headers
        .get("x-elon-app-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("pc-web");
    let app_id =
        normalize_app_id(app_id).map_err(|error| json_error(StatusCode::BAD_REQUEST, error))?;
    Ok(ProjectCaller {
        user_id: user.id,
        role: access.role,
        app_id,
    })
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if message.contains("只有项目编辑者") {
                StatusCode::FORBIDDEN
            } else if message.contains("不能用于不同结果") || message.contains("并发冲突")
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}
