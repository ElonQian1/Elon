use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_consumer_preference_model::{
        UpsertConsumerPreferenceDisclosureRequest, UpsertConsumerPreferenceProfileRequest,
    },
    open_commerce_consumer_preference_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/consumer-preference-profile",
            get(get_profile).put(upsert_profile).delete(delete_profile),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-preference-disclosures",
            get(list_consumer_disclosures),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-relationships/:relationship_id/preference-disclosure",
            get(get_disclosure)
                .put(upsert_disclosure)
                .delete(delete_disclosure),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/preference-disclosures",
            get(list_merchant_disclosures),
        )
}

async fn list_consumer_disclosures(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_consumer_preference_service::list_consumer_disclosures(
            &state.store,
            &project_id,
            &actor(&caller),
            100,
        )
        .map(|disclosures| {
            json!({
                "schema":"open_commerce.consumer_preference_disclosures.v1",
                "disclosures":disclosures
            })
        }),
    )
}

async fn get_profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_consumer_preference_service::get_profile(
            &state.store,
            &project_id,
            &actor(&caller),
        )
        .map(|profile| json!({"schema":"open_commerce.consumer_preference_profile.v1","profile":profile})),
    )
}

async fn upsert_profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<UpsertConsumerPreferenceProfileRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_consumer_preference_service::upsert_profile(
        &state.store,
        &project_id,
        &actor(&caller),
        request,
    ))
}

async fn delete_profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_consumer_preference_service::delete_profile(
        &state.store,
        &project_id,
        &actor(&caller),
    ))
}

async fn get_disclosure(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, relationship_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_consumer_preference_service::get_disclosure(
            &state.store,
            &project_id,
            &relationship_id,
            &actor(&caller),
        )
        .map(|disclosure| {
            json!({
                "schema":"open_commerce.consumer_preference_disclosure.v1",
                "disclosure":disclosure
            })
        }),
    )
}

async fn upsert_disclosure(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, relationship_id)): Path<(String, String)>,
    Json(request): Json<UpsertConsumerPreferenceDisclosureRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_consumer_preference_service::upsert_disclosure(
            &state.store,
            &project_id,
            &relationship_id,
            &actor(&caller),
            request,
        ),
    )
}

async fn delete_disclosure(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, relationship_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_consumer_preference_service::delete_disclosure(
            &state.store,
            &project_id,
            &relationship_id,
            &actor(&caller),
        ),
    )
}

async fn list_merchant_disclosures(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_consumer_preference_service::list_merchant_disclosures(
            &state.store,
            &project_id,
            &merchant_id,
            &actor(&caller),
            100,
        )
        .map(|disclosures| {
            json!({
                "schema":"open_commerce.merchant_preference_disclosures.v1",
                "disclosures":disclosures
            })
        }),
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
