use super::{funding, ledger, model::*, policy};
use crate::types::AppState;
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Empty {}
pub(super) fn respond<T: Serialize>(result: Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let (status, code) = match error.downcast_ref::<Error>() {
                Some(Error::Disabled) => (StatusCode::SERVICE_UNAVAILABLE, "game_rewards_disabled"),
                Some(Error::Invalid) => (StatusCode::BAD_REQUEST, "game_rewards_invalid"),
                Some(Error::Unauthorized) => {
                    (StatusCode::UNAUTHORIZED, "game_rewards_unauthorized")
                }
                Some(Error::Conflict) => (StatusCode::CONFLICT, "game_rewards_conflict"),
                Some(Error::Insufficient) => {
                    (StatusCode::CONFLICT, "game_rewards_insufficient_profit")
                }
                Some(Error::NotFound) => (StatusCode::NOT_FOUND, "game_rewards_not_found"),
                _ => (StatusCode::SERVICE_UNAVAILABLE, "game_rewards_unavailable"),
            };
            (status, Json(json!({"error":code}))).into_response()
        }
    }
}
fn clock() -> Result<i64> {
    Ok(chrono::Utc::now().timestamp_millis())
}
fn input<T>(
    query: std::result::Result<Query<Empty>, QueryRejection>,
    body: std::result::Result<Json<T>, JsonRejection>,
) -> Result<T> {
    query.map_err(|_| Error::Invalid)?;
    Ok(body.map_err(|_| Error::Invalid)?.0)
}
pub(super) async fn settlement(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: std::result::Result<Query<Empty>, QueryRejection>,
    body: std::result::Result<Json<Signed<Settlement>>, JsonRejection>,
) -> Response {
    let (user, token) = match super::super::api::real_user(&state, &headers) {
        Ok(v) => v,
        Err(e) => return e,
    };
    respond((|| {
        let proof = input(query, body)?;
        let policy = policy::load()?;
        let digest = ledger::record_settlement(
            &mut *state.store.conn()?,
            &policy,
            &user.id,
            token,
            &proof,
            &clock,
        )?;
        Ok(
            json!({"schema":"esk.game.rewards.recorded.v1","report_digest":digest,"offchain_payment_authorized":false}),
        )
    })())
}
pub(super) async fn prepare(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: std::result::Result<Query<Empty>, QueryRejection>,
    body: std::result::Result<Json<BudgetIntent>, JsonRejection>,
) -> Response {
    let (user, token) = match super::super::api::real_user(&state, &headers) {
        Ok(v) => v,
        Err(e) => return e,
    };
    respond((|| {
        let intent = input(query, body)?;
        ledger::prepare(
            &mut *state.store.conn()?,
            &policy::load()?,
            &user.id,
            token,
            &intent,
            &clock,
        )
    })())
}
pub(super) async fn confirm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: std::result::Result<Query<Empty>, QueryRejection>,
    body: std::result::Result<Json<Signed<FundingEvidence>>, JsonRejection>,
) -> Response {
    let (user, token) = match super::super::api::real_user(&state, &headers) {
        Ok(v) => v,
        Err(e) => return e,
    };
    respond((|| {
        let proof = input(query, body)?;
        funding::confirm(
            &mut *state.store.conn()?,
            &policy::load()?,
            &user.id,
            token,
            &proof,
            &clock,
        )
    })())
}
pub(super) async fn account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: std::result::Result<Query<Empty>, QueryRejection>,
) -> Response {
    let (user, token) = match super::super::api::real_user(&state, &headers) {
        Ok(v) => v,
        Err(e) => return e,
    };
    respond((|| {
        query.map_err(|_| Error::Invalid)?;
        funding::account(
            &mut *state.store.conn()?,
            &policy::load()?,
            &user.id,
            token,
            &clock,
        )
    })())
}
