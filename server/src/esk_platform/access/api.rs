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

use super::super::api::real_user;
use super::*;
use crate::{
    esk_asset::platform::{
        sellback::{self, SellbackError},
        PlatformError,
    },
    types::AppState,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmptyQuery {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AssetQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    cursor: Option<String>,
    #[serde(default)]
    include_progress: bool,
}
fn default_limit() -> usize {
    20
}

fn credential(headers: &HeaderMap) -> Result<(&str, &str), Response> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1
        || headers.get_all(CLIENT_HEADER).iter().count() != 1
    {
        return Err(error(AccessError::Unauthorized.into()));
    }
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| valid_secret(v, "aat_"))
        .ok_or_else(|| error(AccessError::Unauthorized.into()))?;
    let client = headers
        .get(CLIENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|v| valid_client(v))
        .ok_or_else(|| error(AccessError::Unauthorized.into()))?;
    Ok((token, client))
}

fn empty(query: Result<Query<EmptyQuery>, QueryRejection>) -> Result<(), Response> {
    query
        .map(|_| ())
        .map_err(|_| error(AccessError::InvalidInput.into()))
}

pub(super) async fn authorize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<AuthorizeBody>, JsonRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    if let Err(response) = empty(query) {
        return response;
    }
    let body = match body {
        Ok(Json(v)) => v,
        Err(_) => return error(AccessError::InvalidInput.into()),
    };
    match state
        .store
        .authorize_asset_access(&user.id, token, &body, &state.public_url)
    {
        Ok(value) => Json(value).into_response(),
        Err(cause) => error(cause),
    }
}

pub(super) async fn exchange(
    State(state): State<Arc<AppState>>,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<TokenBody>, JsonRejection>,
) -> Response {
    if let Err(response) = empty(query) {
        return response;
    }
    let body = match body {
        Ok(Json(v)) => v,
        Err(_) => return error(AccessError::InvalidGrant.into()),
    };
    match state
        .store
        .exchange_asset_access_code(&body, &state.public_url)
    {
        Ok(value) => Json(value).into_response(),
        Err(cause) => error(cause),
    }
}

pub(super) async fn list_grants(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
) -> Response {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return error(AccessError::Unauthorized.into());
    }
    let token = match headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        Some(v) if state.owner_token.as_deref() != Some(v) && state.admin_token != v => v,
        _ => return error(AccessError::Unauthorized.into()),
    };
    let user = match state.store.asset_access_owner_id(token) {
        Ok(v) => v,
        Err(cause) => return error(cause),
    };
    if let Err(response) = empty(query) {
        return response;
    }
    match state.store.list_asset_access_grants(&user, token) {
        Ok(grants) => {
            Json(json!({"schema":"yilong.asset_access.grants.v1","grants":grants,"limit":100}))
                .into_response()
        }
        Err(cause) => error(cause),
    }
}

pub(super) async fn revoke_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<RevokeBody>, JsonRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    if let Err(response) = empty(query) {
        return response;
    }
    let body = match body {
        Ok(Json(v)) => v,
        Err(_) => return error(AccessError::InvalidInput.into()),
    };
    if let Err(cause) = validate_revoke(&body) {
        return error(cause);
    }
    match state.store.revoke_asset_access_grant(&user.id, token, &id) {
        Ok(()) => revoked(),
        Err(cause) => error(cause),
    }
}

pub(super) async fn revoke_self(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
    body: Result<Json<RevokeBody>, JsonRejection>,
) -> Response {
    let (token, client) = match credential(&headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    if let Err(response) = empty(query) {
        return response;
    }
    let body = match body {
        Ok(Json(v)) => v,
        Err(_) => return error(AccessError::InvalidInput.into()),
    };
    if let Err(cause) = validate_revoke(&body) {
        return error(cause);
    }
    match state.store.revoke_asset_access_token(token, client) {
        Ok(()) => revoked(),
        Err(cause) => error(cause),
    }
}

fn revoked() -> Response {
    Json(json!({"schema":"yilong.asset_access.revoked.v1","revoked":true,"funds_moved":false}))
        .into_response()
}

pub(super) async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<EmptyQuery>, QueryRejection>,
) -> Response {
    let (token, client) = match credential(&headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    if let Err(response) = empty(query) {
        return response;
    }
    match state.store.asset_access_me(token, client) {
        Ok(value) => Json(value).into_response(),
        Err(cause) => error(cause),
    }
}

pub(super) async fn esk(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<AssetQuery>, QueryRejection>,
) -> Response {
    let (token, client) = match credential(&headers) {
        Ok(v) => v,
        Err(v) => return v,
    };
    let query = match query {
        Ok(Query(v)) => v,
        Err(_) => return error(AccessError::InvalidInput.into()),
    };
    if !(1..=20).contains(&query.limit)
        || query.cursor.as_ref().is_some_and(|v| v.len() > 160)
        || (!query.include_progress && query.cursor.is_some())
    {
        return error(AccessError::InvalidInput.into());
    }
    match state.store.asset_access_esk(
        token,
        client,
        query.limit,
        query.cursor.as_deref(),
        query.include_progress,
        &sellback::load_configuration(),
    ) {
        Ok(value) => Json(value).into_response(),
        Err(cause) => error(cause),
    }
}

fn error(cause: anyhow::Error) -> Response {
    let (status, code) = if let Some(kind) = cause.downcast_ref::<AccessError>() {
        let status = match kind {
            AccessError::Unauthorized => StatusCode::UNAUTHORIZED,
            AccessError::InvalidGrant | AccessError::InvalidInput => StatusCode::BAD_REQUEST,
            AccessError::InsufficientScope => StatusCode::FORBIDDEN,
            AccessError::NotFound => StatusCode::NOT_FOUND,
            AccessError::Capacity => StatusCode::CONFLICT,
            AccessError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            AccessError::Corrupt => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, kind.code())
    } else if matches!(
        cause.downcast_ref::<SellbackError>(),
        Some(SellbackError::SnapshotChanged)
    ) || matches!(
        cause.downcast_ref::<PlatformError>(),
        Some(PlatformError::HistoryChanged)
    ) {
        (StatusCode::CONFLICT, "asset_access_snapshot_changed")
    } else if matches!(
        cause.downcast_ref::<SellbackError>(),
        Some(SellbackError::InvalidInput)
    ) {
        (StatusCode::BAD_REQUEST, "asset_access_invalid_input")
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "asset_access_storage_error",
        )
    };
    // Never expose SQL text, tokens, session identifiers or request content.
    (status, Json(json!({"code":code,"message":code}))).into_response()
}
