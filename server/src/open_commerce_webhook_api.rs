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
    open_commerce_webhook_model::CreateDeveloperWebhookRequest,
    open_commerce_webhook_service,
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks",
            get(list_webhooks).post(create_webhook),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks/:subscription_id/disable",
            post(disable_webhook),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks/:subscription_id/enable",
            post(enable_webhook),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks/:subscription_id/verify",
            post(verify_webhook),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks/:subscription_id/rotate-secret",
            post(rotate_webhook_secret),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks/:subscription_id/deliveries",
            get(list_deliveries),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/webhooks/:subscription_id/deliveries/:delivery_id/retry",
            post(retry_delivery),
        )
}

async fn list_webhooks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
) -> Response {
    let app = match caller_app(&state, &headers, &project_id, &app_record_id, false) {
        Ok(app) => app,
        Err(response) => return response,
    };
    service_response(
        open_commerce_webhook_service::list_webhooks(&state.store, &app).map(|webhooks| {
            json!({
                "schema":"open_commerce.developer_webhook_subscriptions.v1",
                "webhooks":webhooks
            })
        }),
    )
}

async fn create_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
    Json(request): Json<CreateDeveloperWebhookRequest>,
) -> Response {
    let (user_id, app) =
        match editable_caller_app(&state, &headers, &project_id, &app_record_id, true) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let credential =
        match open_commerce_webhook_service::create_webhook(&state.store, &app, request) {
            Ok(value) => value,
            Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
        };
    let callback_host = reqwest::Url::parse(&credential.subscription.callback_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| "invalid".to_string());
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "developer_webhook.created",
        "developer_webhook",
        &credential.subscription.id,
        &json!({
            "app_id":credential.subscription.app_id,
            "callback_host":callback_host,
            "signing_key_id":credential.subscription.signing_key_id
        }),
    ) {
        let _ = state.store.set_open_commerce_developer_webhook_enabled(
            &project_id,
            &app_record_id,
            &credential.subscription.id,
            false,
        );
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error);
    }
    Json(credential).into_response()
}

async fn disable_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id, subscription_id)): Path<(String, String, String)>,
) -> Response {
    mutate_webhook(
        &state,
        &headers,
        &project_id,
        &app_record_id,
        &subscription_id,
        false,
    )
}

async fn enable_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id, subscription_id)): Path<(String, String, String)>,
) -> Response {
    mutate_webhook(
        &state,
        &headers,
        &project_id,
        &app_record_id,
        &subscription_id,
        true,
    )
}

async fn verify_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id, subscription_id)): Path<(String, String, String)>,
) -> Response {
    let (user_id, app) =
        match editable_caller_app(&state, &headers, &project_id, &app_record_id, true) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let subscription = match state.store.open_commerce_developer_webhook_for_app(
        &project_id,
        &app_record_id,
        &subscription_id,
    ) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::NOT_FOUND, error),
    };
    let verified = match crate::open_commerce_webhook_verification::verify_endpoint(
        &state.store,
        &app,
        &subscription,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "developer_webhook.verified",
        "developer_webhook",
        &verified.id,
        &json!({"app_id":verified.app_id,"signing_key_id":verified.signing_key_id}),
    ) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error);
    }
    Json(verified).into_response()
}

async fn rotate_webhook_secret(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id, subscription_id)): Path<(String, String, String)>,
) -> Response {
    let (user_id, app) =
        match editable_caller_app(&state, &headers, &project_id, &app_record_id, true) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let credential = match open_commerce_webhook_service::rotate_webhook_secret(
        &state.store,
        &app,
        &subscription_id,
    ) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "developer_webhook.secret_rotated",
        "developer_webhook",
        &credential.subscription.id,
        &json!({
            "app_id":credential.subscription.app_id,
            "signing_key_id":credential.subscription.signing_key_id,
            "signing_secret_version":credential.subscription.signing_secret_version
        }),
    ) {
        tracing::warn!(
            webhook_id = %credential.subscription.id,
            error = %error,
            "Webhook 密钥已轮换，但审计记录写入失败"
        );
    }
    Json(credential).into_response()
}

async fn list_deliveries(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id, subscription_id)): Path<(String, String, String)>,
) -> Response {
    let app = match caller_app(&state, &headers, &project_id, &app_record_id, false) {
        Ok(app) => app,
        Err(response) => return response,
    };
    service_response(
        open_commerce_webhook_service::list_deliveries(&state.store, &app, &subscription_id).map(
            |deliveries| {
                json!({
                    "schema":"open_commerce.developer_webhook_deliveries.v1",
                    "deliveries":deliveries
                })
            },
        ),
    )
}

async fn retry_delivery(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id, subscription_id, delivery_id)): Path<(
        String,
        String,
        String,
        String,
    )>,
) -> Response {
    let (user_id, app) =
        match editable_caller_app(&state, &headers, &project_id, &app_record_id, true) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let delivery = match open_commerce_webhook_service::retry_delivery(
        &state.store,
        &app,
        &subscription_id,
        &delivery_id,
    ) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    if let Err(error) = state.store.record_open_commerce_audit(
        &project_id,
        &user_id,
        Some("pc-web"),
        "developer_webhook.delivery_retried",
        "developer_webhook_delivery",
        &delivery.id,
        &json!({
            "app_id":app.app_id,
            "subscription_id":delivery.subscription_id,
            "invocation_id":delivery.invocation_id,
            "manual_retry_count":delivery.manual_retry_count
        }),
    ) {
        tracing::warn!(
            delivery_id = %delivery.id,
            error = %error,
            "Webhook 死信已重新排队，但审计记录写入失败"
        );
    }
    Json(delivery).into_response()
}

fn mutate_webhook(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    app_record_id: &str,
    subscription_id: &str,
    enabled: bool,
) -> Response {
    let (user_id, app) =
        match editable_caller_app(state, headers, project_id, app_record_id, enabled) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let subscription = match open_commerce_webhook_service::set_webhook_enabled(
        &state.store,
        &app,
        subscription_id,
        enabled,
    ) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    if let Err(error) = state.store.record_open_commerce_audit(
        project_id,
        &user_id,
        Some("pc-web"),
        if enabled {
            "developer_webhook.enabled"
        } else {
            "developer_webhook.disabled"
        },
        "developer_webhook",
        &subscription.id,
        &json!({"app_id":subscription.app_id}),
    ) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error);
    }
    Json(subscription).into_response()
}

fn editable_caller_app(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    app_record_id: &str,
    require_active: bool,
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
            "只有项目编辑者可以管理 Webhook",
        ));
    }
    let app = open_commerce_webhook_service::ensure_owned_app(
        &state.store,
        project_id,
        app_record_id,
        &user.id,
        require_active,
    )
    .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    Ok((user.id, app))
}

fn caller_app(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    app_record_id: &str,
    require_active: bool,
) -> Result<crate::open_commerce_developer_model::OpenCommerceDeveloperApp, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    open_commerce_webhook_service::ensure_owned_app(
        &state.store,
        project_id,
        app_record_id,
        &user.id,
        require_active,
    )
    .map_err(|error| json_error(StatusCode::FORBIDDEN, error))
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
