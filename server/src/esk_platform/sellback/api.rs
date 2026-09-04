use super::super::api::real_user;
use super::{wire, *};
use crate::{project_auth::json_error, types::AppState};
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

fn current_configuration() -> SellbackConfiguration {
    #[cfg(test)]
    if let Some(value) = CONFIG_OVERRIDE.with(|value| value.borrow().clone()) {
        return value;
    }
    load_configuration()
}

#[cfg(test)]
thread_local! {
    static CONFIG_OVERRIDE: std::cell::RefCell<Option<SellbackConfiguration>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
pub(crate) struct ConfigurationGuard(Option<SellbackConfiguration>);
#[cfg(test)]
impl Drop for ConfigurationGuard {
    fn drop(&mut self) {
        CONFIG_OVERRIDE.with(|value| *value.borrow_mut() = self.0.take());
    }
}
#[cfg(test)]
pub(crate) fn override_configuration(value: SellbackConfiguration) -> ConfigurationGuard {
    ConfigurationGuard(CONFIG_OVERRIDE.with(|cell| cell.replace(Some(value))))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    cursor: Option<String>,
}
fn default_limit() -> usize {
    MAX_PAGE_SIZE
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmptyQuery {}

pub(super) async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let query = match query {
        Ok(Query(v)) => v,
        Err(_) => return invalid_input(),
    };
    if !(1..=MAX_PAGE_SIZE).contains(&query.limit)
        || query
            .cursor
            .as_deref()
            .is_some_and(|cursor| parse_cursor(cursor).is_err())
    {
        return invalid_input();
    }
    match state.store.esk_platform_sellback_page(
        &user.id,
        token,
        query.limit,
        query.cursor.as_deref(),
        &current_configuration(),
    ) {
        Ok(page) => wire::page_response(page),
        Err(error) => domain_error(error),
    }
}

pub(super) async fn get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<EmptyQuery>, QueryRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let id = match path {
        Ok(Path(v)) if valid_request_id(&v) => v,
        _ => return invalid_input(),
    };
    if query.is_err() {
        return invalid_input();
    }
    match state
        .store
        .esk_platform_sellback_request(&user.id, token, &id, &current_configuration())
    {
        Ok(result) => wire::result_response(result),
        Err(error) => domain_error(error),
    }
}

pub(super) async fn submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<SellbackSubmitBody>, JsonRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    if query.is_err() {
        return invalid_input();
    }
    let body = match body {
        Ok(Json(v)) => v,
        Err(_) => return invalid_input(),
    };
    let input = match validate_submit_body(body) {
        Ok(v) => v,
        Err(error) => return domain_error(error),
    };
    match state.store.submit_esk_platform_sellback(
        &user.id,
        token,
        &input,
        &current_configuration(),
    ) {
        Ok(result) => wire::result_response(result),
        Err(error) => domain_error(error),
    }
}

pub(super) async fn cancel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<SellbackCancelBody>, JsonRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let id = match path {
        Ok(Path(v)) if valid_request_id(&v) => v,
        _ => return invalid_input(),
    };
    if query.is_err() {
        return invalid_input();
    }
    let body = match body {
        Ok(Json(v)) => v,
        Err(_) => return invalid_input(),
    };
    if body.schema != CANCEL_SCHEMA || body.confirmation != CANCEL_CONFIRMATION {
        return invalid_input();
    }
    match state
        .store
        .cancel_esk_platform_sellback(&user.id, token, &id, &current_configuration())
    {
        Ok(result) => wire::result_response(result),
        Err(error) => domain_error(error),
    }
}

/// Read-only lookup keeps the private idempotency key out of URLs and access logs.
pub(super) async fn lookup(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<SellbackLookupBody>, JsonRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    if query.is_err() {
        return invalid_input();
    }
    let body = match body {
        Ok(Json(v)) => v,
        Err(_) => return invalid_input(),
    };
    if body.schema != LOOKUP_SCHEMA || !label(&body.idempotency_key, 96) {
        return invalid_input();
    }
    match state.store.lookup_esk_platform_sellback(
        &user.id,
        token,
        &body.idempotency_key,
        &current_configuration(),
    ) {
        Ok(result) => wire::result_response(result),
        Err(error) => domain_error(error),
    }
}

fn invalid_input() -> Response {
    domain_error(SellbackError::InvalidInput.into())
}

fn domain_error(error: anyhow::Error) -> Response {
    let Some(kind) = error.downcast_ref::<SellbackError>() else {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ESK_PLATFORM_SELLBACK_STORAGE_ERROR",
        );
    };
    let status = match kind {
        SellbackError::Unauthorized => StatusCode::UNAUTHORIZED,
        SellbackError::InvalidInput => StatusCode::BAD_REQUEST,
        SellbackError::Disabled => StatusCode::SERVICE_UNAVAILABLE,
        SellbackError::Ineligible => StatusCode::FORBIDDEN,
        SellbackError::NotFound => StatusCode::NOT_FOUND,
        SellbackError::Corrupt => StatusCode::INTERNAL_SERVER_ERROR,
        SellbackError::PolicyChanged
        | SellbackError::Conflict
        | SellbackError::SnapshotChanged
        | SellbackError::LimitExceeded
        | SellbackError::InsufficientAvailable => StatusCode::CONFLICT,
    };
    json_error(status, kind)
}
