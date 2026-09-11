//! Source queries inherit the reward router's TLS, Origin, no-store and size limits.
use super::{
    api::{self, Empty},
    funding_source::{self, InspectRequest, PendingRequest},
    model::*,
    policy,
};
use crate::types::AppState;
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Query, State,
    },
    http::HeaderMap,
    response::Response,
    Json,
};
use std::sync::Arc;

pub(super) async fn inspect(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: std::result::Result<Query<Empty>, QueryRejection>,
    body: std::result::Result<Json<InspectRequest>, JsonRejection>,
) -> Response {
    let (user, token) = match super::super::api::real_user(&state, &headers) {
        Ok(v) => v,
        Err(e) => return e,
    };
    api::respond((|| {
        query.map_err(|_| Error::Invalid)?;
        let request = body.map_err(|_| Error::Invalid)?.0;
        funding_source::inspect(
            &mut *state.store.conn()?,
            &policy::load()?,
            &user.id,
            token,
            &request,
            &|| Ok(chrono::Utc::now().timestamp_millis()),
        )
    })())
}
pub(super) async fn pending(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: std::result::Result<Query<Empty>, QueryRejection>,
    body: std::result::Result<Json<PendingRequest>, JsonRejection>,
) -> Response {
    let (user, token) = match super::super::api::real_user(&state, &headers) {
        Ok(v) => v,
        Err(e) => return e,
    };
    api::respond((|| {
        query.map_err(|_| Error::Invalid)?;
        let request = body.map_err(|_| Error::Invalid)?.0;
        funding_source::pending(
            &mut *state.store.conn()?,
            &policy::load()?,
            &user.id,
            token,
            &request,
            &|| Ok(chrono::Utc::now().timestamp_millis()),
        )
    })())
}
