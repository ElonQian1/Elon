use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::{
    open_commerce_consumer_vault_model::{
        CreateConsumerDataVaultItemRequest, DeleteConsumerDataVaultItemRequest,
        UpdateConsumerDataVaultItemRequest,
    },
    open_commerce_consumer_vault_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/consumer-data-vault-items",
            get(list_items).post(create_item),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-data-vault-items/:item_id",
            get(get_item),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-data-vault-items/:item_id/update",
            post(update_item),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-data-vault-items/:item_id/delete",
            post(delete_item),
        )
}

async fn create_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateConsumerDataVaultItemRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_consumer_vault_service::create_item(
            &state.store,
            &project_id,
            &actor,
            request,
        )
    })
}

async fn update_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, item_id)): Path<(String, String)>,
    Json(request): Json<UpdateConsumerDataVaultItemRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_consumer_vault_service::update_item(
            &state.store,
            &project_id,
            &item_id,
            &actor,
            request,
        )
    })
}

async fn get_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, item_id)): Path<(String, String)>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_consumer_vault_service::get_item(&state.store, &project_id, &item_id, &actor)
    })
}

async fn list_items(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_consumer_vault_service::list_items(&state.store, &project_id, &actor, 100)
            .map(|items| {
                json!({
                    "schema": "open_commerce.consumer_data_vault_items.v1",
                    "items": items,
                })
            })
    })
}

async fn delete_item(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, item_id)): Path<(String, String)>,
    Json(request): Json<DeleteConsumerDataVaultItemRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_consumer_vault_service::delete_item(
            &state.store,
            &project_id,
            &item_id,
            &actor,
            request,
        )
    })
}

fn with_caller<T: serde::Serialize>(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    operation: impl FnOnce(OpenCommerceActor<'_>) -> anyhow::Result<T>,
) -> Response {
    let user = match auth_from_headers(state, headers) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    let access = match project_access(state, &user.id, project_id) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::FORBIDDEN, error),
    };
    match operation(OpenCommerceActor {
        user_id: &user.id,
        app_id: "pc-web",
        project_role: Some(&access.role),
    }) {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}
