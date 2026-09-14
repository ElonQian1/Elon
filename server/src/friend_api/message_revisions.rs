use crate::store::groups::revisions::RevisionError;
use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/groups/:group_id/messages/:message_id", patch(edit))
        .route(
            "/api/me/groups/:group_id/messages/:message_id/revisions",
            get(history),
        )
        .route(
            "/assets/group_message_revisions.js",
            get(|| async {
                (
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "text/javascript; charset=utf-8",
                    )],
                    include_str!("../assets/group_message_revisions.js"),
                )
            }),
        )
        .route(
            "/assets/group_message_revisions.css",
            get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../assets/group_message_revisions.css"),
                )
            }),
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditRequest {
    content: String,
    expected_revision: i64,
}

#[derive(Deserialize)]
struct HistoryQuery {
    before_revision: Option<i64>,
    limit: Option<i64>,
}

async fn edit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, message)): Path<(String, String)>,
    Json(req): Json<EditRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match state.store.edit_group_message(
        &user.id,
        &group,
        &message,
        req.expected_revision,
        &req.content,
    ) {
        Ok(edit) => {
            if edit.changed {
                if let Ok(members) = state.store.friend_group_member_ids(&user.id, &group) {
                    crate::friend_events::publish_group_message_edit(&edit, &user.id, members);
                }
            }
            // Editing, including adding @EL, never triggers an AI reply or a new-message sound.
            Json(serde_json::json!({ "message": edit })).into_response()
        }
        Err(error) => revision_error(error),
    }
}

async fn history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, message)): Path<(String, String)>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match state.store.group_message_history(
        &user.id,
        &group,
        &message,
        query.before_revision,
        query.limit.unwrap_or(50),
    ) {
        Ok(history) => Json(history).into_response(),
        Err(error) => revision_error(error),
    }
}

fn revision_error(error: anyhow::Error) -> Response {
    let status = match error.downcast_ref::<RevisionError>() {
        Some(RevisionError::Forbidden) => StatusCode::FORBIDDEN,
        Some(RevisionError::NotFound) => StatusCode::NOT_FOUND,
        Some(RevisionError::Conflict) => StatusCode::CONFLICT,
        Some(RevisionError::Recalled) => StatusCode::GONE,
        Some(RevisionError::Invalid) => StatusCode::BAD_REQUEST,
        Some(RevisionError::RateLimited) => StatusCode::TOO_MANY_REQUESTS,
        None => {
            tracing::error!(error = %error, "Group message revision operation failed");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "消息修改记录暂时不可用，请稍后重试",
            );
        }
    };
    json_error(status, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_require_version_and_reject_author_or_attachment_overrides() {
        for body in [
            r#"{"content":"new"}"#,
            r#"{"content":"new","expected_revision":1,"sender_user_id":"someone"}"#,
            r#"{"content":"new","expected_revision":1,"attachments":[]}"#,
        ] {
            assert!(serde_json::from_str::<EditRequest>(body).is_err());
        }
        assert!(
            serde_json::from_str::<EditRequest>(r#"{"content":"更正","expected_revision":1}"#)
                .is_ok()
        );
    }
    #[test]
    fn clients_can_distinguish_conflicts_and_hidden_history() {
        assert_eq!(
            revision_error(RevisionError::Conflict.into()).status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            revision_error(RevisionError::Recalled.into()).status(),
            StatusCode::GONE
        );
        assert_eq!(
            revision_error(RevisionError::Forbidden.into()).status(),
            StatusCode::FORBIDDEN
        );
    }
}
