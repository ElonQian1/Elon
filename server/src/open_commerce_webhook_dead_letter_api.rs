use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_webhook_model::AcknowledgeDeveloperWebhookDeadLetterRequest,
    open_commerce_webhook_service,
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks/:subscription_id/deliveries/:delivery_id/acknowledge",
        post(acknowledge_dead_letter),
    )
}

async fn acknowledge_dead_letter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id, subscription_id, delivery_id)): Path<(
        String,
        String,
        String,
        String,
    )>,
    Json(request): Json<AcknowledgeDeveloperWebhookDeadLetterRequest>,
) -> Response {
    let (user_id, app) = match editable_caller_app(&state, &headers, &project_id, &app_record_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let delivery = match open_commerce_webhook_service::acknowledge_dead_letter(
        &state.store,
        &app,
        &subscription_id,
        &delivery_id,
        &user_id,
        &request.reason,
    ) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "developer_webhook.dead_letter_acknowledged",
        "developer_webhook_delivery",
        &delivery.id,
        &json!({
            "app_id": app.app_id.clone(),
            "subscription_id": delivery.subscription_id.clone(),
            "invocation_id": delivery.invocation_id.clone(),
            "reason": delivery.dead_letter_acknowledgement_reason.clone()
        }),
    ) {
        tracing::warn!(
            delivery_id = %delivery.id,
            error = %error,
            "Webhook 死信已确认，但审计记录写入失败"
        );
    }
    Json(delivery).into_response()
}

fn editable_caller_app(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    app_record_id: &str,
) -> Result<
    (
        String,
        crate::open_commerce_developer_model::OpenCommerceDeveloperApp,
    ),
    Response,
> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    if !can_edit(&access.role) {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有项目编辑者可以确认 Webhook 死信",
        ));
    }
    let app = open_commerce_webhook_service::ensure_owned_app(
        &state.store,
        project_id,
        app_record_id,
        &user.id,
        false,
    )
    .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    Ok((user.id, app))
}
