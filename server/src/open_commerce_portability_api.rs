use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_portability_import_model::CreateConsumerPortabilityImportRequest,
    open_commerce_portability_import_service,
    open_commerce_portability_model::CreateConsumerPortabilityExportRequest,
    open_commerce_portability_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-exports",
            get(list_exports).post(create_export),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-exports/:export_id",
            get(get_export),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-imports",
            get(list_imports)
                .post(create_import)
                .layer(DefaultBodyLimit::max(6 * 1024 * 1024)),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-imports/:import_id",
            get(get_import).delete(delete_import),
        )
}

async fn list_exports(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_portability_service::list_exports(
            &state.store,
            &project_id,
            &actor(&caller),
            100,
        )
        .map(|exports| {
            json!({"schema":"open_commerce.consumer_portability_exports.v3","exports":exports})
        }),
    )
}

async fn create_export(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateConsumerPortabilityExportRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_portability_service::create_export(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn get_export(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, export_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_portability_service::get_export(
        &state.store,
        &project_id,
        &export_id,
        &actor(&caller),
    ))
}

async fn list_imports(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_portability_import_service::list_imports(
            &state.store,
            &project_id,
            &actor(&caller),
            100,
        )
        .map(|imports| {
            json!({"schema":"open_commerce.consumer_portability_imports.v1","imports":imports})
        }),
    )
}

async fn create_import(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateConsumerPortabilityImportRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_portability_import_service::create_import(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn get_import(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, import_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_portability_import_service::get_import(
        &state.store,
        &project_id,
        &import_id,
        &actor(&caller),
    ))
}

async fn delete_import(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, import_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_portability_import_service::delete_import(
        &state.store,
        &project_id,
        &import_id,
        &actor(&caller),
    ))
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

fn actor<'a>(caller: &'a (String, String)) -> OpenCommerceActor<'a> {
    OpenCommerceActor {
        user_id: &caller.0,
        app_id: "pc-web",
        project_role: Some(&caller.1),
    }
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}
