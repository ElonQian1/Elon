use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    group_ai::{
        artifacts::assignment_artifact,
        automation::{
            schedule_matter_assignments, schedule_review_assignment, AutomationRunResult,
        },
        bot_selector::bots_for_project,
        live::matter_events_delta,
        permissions::{
            authenticate_project_member, ensure_can_decide_matter, ensure_can_operate_assignment,
        },
    },
    project_auth::json_error,
    types::AppState,
};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct AutomationActionRequest {
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct EventsQuery {
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

pub(crate) async fn run_matter_assignments(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<AutomationActionRequest>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match state.store.get_project_ai_matter(&project_id, &matter_id) {
        Ok(Some(matter)) => matter,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Matter 不存在"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    if let Err(response) = ensure_can_decide_matter(&access, &user.id, &matter) {
        return response;
    }

    match schedule_matter_assignments(
        state,
        access,
        &project_id,
        &matter_id,
        &user.id,
        req.comment,
    ) {
        Ok(result) => automation_response(result),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn run_review_assignment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<AutomationActionRequest>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match state.store.get_project_ai_matter(&project_id, &matter_id) {
        Ok(Some(matter)) => matter,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Matter 不存在"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    if let Err(response) = ensure_can_decide_matter(&access, &user.id, &matter) {
        return response;
    }
    let bots = match bots_for_project(&state, &project_id).await {
        Ok(bots) => bots,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };

    match schedule_review_assignment(
        state,
        access,
        &project_id,
        &matter_id,
        &user.id,
        req.comment,
        &bots,
    ) {
        Ok(result) => automation_response(result),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn list_matter_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Query(query): Query<EventsQuery>,
) -> Response {
    if let Err(response) = authenticate_project_member(&state, &headers, &project_id) {
        return response;
    }
    match matter_events_delta(
        &state,
        &project_id,
        &matter_id,
        query.after.as_deref(),
        query.limit.unwrap_or(100),
    ) {
        Ok(delta) => Json(json!({
            "ok": true,
            "events": delta.events,
            "latest_event_id": delta.latest_event_id,
            "latest_event_created_at": delta.latest_event_created_at,
            "has_more": delta.has_more,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn get_assignment_artifact(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, assignment_id)): Path<(String, String, String)>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match state.store.get_project_ai_matter(&project_id, &matter_id) {
        Ok(Some(matter)) => matter,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Matter 不存在"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    let assignment = match state.store.get_project_ai_matter_assignment(&assignment_id) {
        Ok(Some(assignment)) if assignment.matter_id == matter_id => assignment,
        Ok(Some(_)) => return json_error(StatusCode::BAD_REQUEST, "Assignment 不属于当前 Matter"),
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Assignment 不存在"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    if let Err(response) = ensure_can_operate_assignment(&access, &user.id, &matter, &assignment) {
        return response;
    }
    match assignment_artifact(&state, &project_id, &matter_id, &assignment_id) {
        Ok(artifact) => Json(json!({
            "ok": true,
            "artifact": artifact,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn automation_response(result: AutomationRunResult) -> Response {
    Json(json!({
        "ok": true,
        "matter": result.detail.matter,
        "assignments": result.detail.assignments,
        "events": result.detail.events,
        "scheduled_count": result.scheduled_count,
        "skipped_count": result.skipped_count,
        "errors": result.errors,
    }))
    .into_response()
}
