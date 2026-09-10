use super::{authority, issue, model::*, observe, policy, protocol::Challenge, revoke};
use crate::types::AppState;
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmptyQuery {}
fn empty(query: Result<Query<EmptyQuery>, QueryRejection>) -> Result<()> {
    query.map(|_| ()).map_err(|_| Error::InvalidInput.into())
}
fn field<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str> {
    if headers.get_all(name).iter().count() != 1 {
        return Err(Error::Unauthorized.into());
    }
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Error::Unauthorized.into())
}
fn service(headers: &HeaderMap) -> Result<&str> {
    field(headers, "x-esk-game-service")
}
fn token(headers: &HeaderMap) -> Result<&str> {
    field(headers, "authorization")?
        .strip_prefix("Bearer ")
        .ok_or_else(|| Error::Unauthorized.into())
}
fn respond<T: serde::Serialize>(result: Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => failure(error),
    }
}
fn failure(error: anyhow::Error) -> Response {
    let (status, code) = match error.downcast_ref::<Error>() {
        Some(Error::Disabled) => (StatusCode::SERVICE_UNAVAILABLE, "game_access_disabled"),
        Some(Error::InvalidInput) => (StatusCode::BAD_REQUEST, "game_access_invalid_input"),
        Some(Error::Unauthorized | Error::InvalidGrant) => {
            (StatusCode::UNAUTHORIZED, "game_access_unauthorized")
        }
        Some(Error::ScopeMissing) => (StatusCode::FORBIDDEN, "game_access_scope_missing"),
        Some(Error::RevisionConflict | Error::Replay) => {
            (StatusCode::CONFLICT, "game_access_conflict")
        }
        Some(Error::Capacity) => (StatusCode::TOO_MANY_REQUESTS, "game_access_capacity"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "game_access_unavailable"),
    };
    (status, Json(json!({"error":code}))).into_response()
}

pub(super) async fn authorize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<AuthorizeRequest>, JsonRejection>,
) -> Response {
    let (user, parent) = match super::super::api::real_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    // Consent is a browser action on the main origin, including for bearer-authenticated UI.
    if headers.get_all(header::ORIGIN).iter().count() != 1 {
        return failure(Error::Unauthorized.into());
    }
    respond((|| {
        empty(query)?;
        let Json(request) = body.map_err(|_| Error::InvalidInput)?;
        let policy = policy::load()?;
        let mut conn = state.store.conn()?;
        issue::authorize(
            &mut conn,
            &policy,
            &user.id,
            parent,
            &request,
            &authority::now_ms,
        )
    })())
}
pub(super) async fn exchange(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<ExchangeRequest>, JsonRejection>,
) -> Response {
    respond((|| {
        empty(query)?;
        let policy = policy::load()?;
        let service = service(&headers)?;
        policy.check_service(service)?;
        let Json(request) = body.map_err(|_| Error::InvalidInput)?;
        let mut conn = state.store.conn()?;
        issue::exchange(&mut conn, &policy, service, &request, &authority::now_ms)
    })())
}
pub(super) async fn observe(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<Challenge>, JsonRejection>,
) -> Response {
    respond((|| {
        empty(query)?;
        let policy = policy::load()?;
        let service = service(&headers)?;
        policy.check_service(service)?;
        let token = token(&headers)?;
        let Json(challenge) = body.map_err(|_| Error::InvalidInput)?;
        let mut conn = state.store.conn()?;
        observe::observe(
            &mut conn,
            &policy,
            service,
            token,
            &challenge,
            &authority::now_ms,
        )
    })())
}
pub(super) async fn revoke_owner(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<RevokeRequest>, JsonRejection>,
) -> Response {
    let (user, parent) = match super::super::api::real_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    respond((|| {
        empty(query)?;
        let Json(request) = body.map_err(|_| Error::InvalidInput)?;
        let mut conn = state.store.conn()?;
        revoke::revoke_owner(
            &mut conn,
            &user.id,
            parent,
            &id,
            &request,
            &authority::now_ms,
        )?;
        Ok(json!({"schema":"esk.game.access.revoked.v1","revoked":true}))
    })())
}
pub(super) async fn revoke_self(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<RevokeRequest>, JsonRejection>,
) -> Response {
    respond((|| {
        empty(query)?;
        let policy = policy::load()?;
        let service = service(&headers)?;
        policy.check_service(service)?;
        let token = token(&headers)?;
        let Json(request) = body.map_err(|_| Error::InvalidInput)?;
        let mut conn = state.store.conn()?;
        revoke::revoke_self(
            &mut conn,
            &policy,
            service,
            token,
            &request,
            &authority::now_ms,
        )?;
        Ok(json!({"schema":"esk.game.access.revoked.v1","revoked":true}))
    })())
}
