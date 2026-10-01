use crate::{
    project_auth::{auth_from_headers, json_error},
    store::groups::{membership::MembershipCommand, roster::RosterQuery},
    types::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/groups/:group_id/roster", get(roster))
        .route("/api/me/groups/:group_id/membership", post(command))
        .route("/api/me/groups/:group_id/invitations", get(invitations))
        .route("/assets/group_roster.js", get(script))
        .route("/assets/group_roster.css", get(styles))
}

fn failure(error: anyhow::Error) -> Response {
    let message = error.to_string();
    let status = if message.starts_with("GROUP_ACCESS_DENIED")
        || message.starts_with("GROUP_PERMISSION_DENIED")
    {
        StatusCode::FORBIDDEN
    } else if message.starts_with("ROSTER_CHANGED") || message.starts_with("REQUEST_CONFLICT") {
        StatusCode::CONFLICT
    } else {
        StatusCode::BAD_REQUEST
    };
    json_error(status, message)
}

async fn roster(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    Query(query): Query<RosterQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match state.store.group_roster(&user.id, &group, &query) {
        Ok(roster) => ([("cache-control", "no-store")], Json(roster)).into_response(),
        Err(error) => failure(error),
    }
}

async fn command(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    Json(command): Json<MembershipCommand>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    let mut recipients = state
        .store
        .friend_group_member_ids(&user.id, &group)
        .unwrap_or_default();
    match state
        .store
        .group_membership_command(&user.id, &group, &command)
    {
        Ok(receipt) => {
            recipients.extend(
                state
                    .store
                    .friend_group_member_ids(&user.id, &group)
                    .unwrap_or_default(),
            );
            recipients.sort();
            recipients.dedup();
            crate::friend_events::publish_group_members_changed(&group, &user.id, recipients);
            Json(receipt).into_response()
        }
        Err(error) => failure(error),
    }
}

#[derive(serde::Deserialize)]
struct InvitationQuery {
    #[serde(default)]
    offset: usize,
}

async fn invitations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    Query(query): Query<InvitationQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match state
        .store
        .group_pending_invitations(&user.id, &group, query.offset)
    {
        Ok(value) => ([("cache-control", "no-store")], Json(value)).into_response(),
        Err(error) => failure(error),
    }
}

async fn script() -> impl IntoResponse {
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../assets/group_roster.js"),
    )
}
async fn styles() -> impl IntoResponse {
    (
        [
            ("content-type", "text/css; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../assets/group_roster.css"),
    )
}
