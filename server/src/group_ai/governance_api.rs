use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    group_ai::{
        artifacts::assignment_artifact,
        governance::matter_governance_summary,
        merge_gate::{
            apply_merge_request as apply_merge_request_action, check_merge_gate,
            ApplyMergeRequestBody,
        },
        permissions::{
            authenticate_project_member, ensure_can_decide_matter, ensure_can_operate_assignment,
        },
        policy::{budget_policy_payload, update_matter_budget_policy},
        types::{
            CreateMergeRequestInput, CreateMergeRequestRequest, ProjectAiMatter,
            ProjectAiMatterAssignment, RecordAssignmentArtifactInput,
            RecordAssignmentArtifactRequest, RecordReviewInput, RecordReviewRequest,
            UpdateMatterBudgetPolicyRequest, UpdateMergeRequestRequest,
        },
    },
    project_auth::json_error,
    store::{ProjectAccess, PublicUser},
    types::AppState,
};

pub(crate) async fn record_assignment_artifact(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, assignment_id)): Path<(String, String, String)>,
    Json(req): Json<RecordAssignmentArtifactRequest>,
) -> Response {
    let (user, access, matter, assignment) =
        match assignment_context(&state, &headers, &project_id, &matter_id, &assignment_id) {
            Ok(value) => value,
            Err(response) => return response,
        };
    if let Err(response) = ensure_can_operate_assignment(&access, &user.id, &matter, &assignment) {
        return response;
    }
    let artifact =
        match state
            .store
            .record_project_ai_assignment_artifact(RecordAssignmentArtifactInput {
                project_id: project_id.clone(),
                matter_id: matter_id.clone(),
                assignment_id: assignment_id.clone(),
                uploader_user_id: Some(user.id.clone()),
                artifact_kind: clean_or(req.artifact_kind, "manual_upload"),
                summary: clean_optional(req.summary),
                worktree_path: clean_optional(req.worktree_path)
                    .or(assignment.worktree_path.clone()),
                branch_name: clean_optional(req.branch_name).or(assignment.branch_name.clone()),
                files: req.files,
                diff_stat: req.diff_stat,
                test_results: req.test_results,
                metadata: req.metadata.unwrap_or(Value::Null),
            }) {
            Ok(artifact) => artifact,
            Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
        };
    insert_event(
        &state,
        &matter,
        &user.id,
        "assignment_artifact_recorded",
        json!({
            "assignment_id": assignment_id,
            "artifact_id": artifact.id,
            "artifact_kind": artifact.artifact_kind,
            "branch_name": artifact.branch_name,
            "worktree_path": artifact.worktree_path
        }),
    );
    match assignment_artifact(&state, &project_id, &matter_id, &assignment.id) {
        Ok(package) => {
            Json(json!({ "ok": true, "artifact": artifact, "package": package })).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn record_matter_review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<RecordReviewRequest>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match require_matter(&state, &project_id, &matter_id) {
        Ok(matter) => matter,
        Err(response) => return response,
    };
    if let Err(response) = ensure_can_decide_matter(&access, &user.id, &matter) {
        return response;
    }
    let review = match state.store.record_project_ai_review(RecordReviewInput {
        matter_id: matter_id.clone(),
        reviewer_bot_id: clean_optional(req.reviewer_bot_id),
        reviewer_user_id: Some(user.id.clone()),
        target_assignment_id: clean_optional(req.target_assignment_id),
        severity: clean_or(req.severity, "medium"),
        finding: req
            .finding
            .unwrap_or_else(|| json!({ "summary": "manual review" })),
        status: clean_or(req.status, "open"),
    }) {
        Ok(review) => review,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    insert_event(
        &state,
        &matter,
        &user.id,
        "review_result_recorded",
        json!({
            "review_id": review.id,
            "status": review.status,
            "severity": review.severity,
            "target_assignment_id": review.target_assignment_id
        }),
    );
    Json(json!({ "ok": true, "review": review })).into_response()
}

pub(crate) async fn get_matter_governance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = authenticate_project_member(&state, &headers, &project_id) {
        return response;
    }
    match matter_governance_summary(&state, &project_id, &matter_id) {
        Ok(summary) => Json(json!({ "ok": true, "governance": summary })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn update_matter_budget_policy_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<UpdateMatterBudgetPolicyRequest>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match require_matter(&state, &project_id, &matter_id) {
        Ok(matter) => matter,
        Err(response) => return response,
    };
    if let Err(response) = ensure_can_decide_matter(&access, &user.id, &matter) {
        return response;
    }
    match update_matter_budget_policy(&state, &project_id, &matter_id, req) {
        Ok(updated) => {
            insert_event(
                &state,
                &updated,
                &user.id,
                "budget_policy_updated",
                budget_policy_payload(&updated),
            );
            Json(json!({ "ok": true, "matter": updated })).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn create_merge_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id)): Path<(String, String)>,
    Json(req): Json<CreateMergeRequestRequest>,
) -> Response {
    let (user, access, matter, assignment) = match assignment_context(
        &state,
        &headers,
        &project_id,
        &matter_id,
        &req.assignment_id,
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) = ensure_can_operate_assignment(&access, &user.id, &matter, &assignment) {
        return response;
    }
    let merge_request = match state
        .store
        .create_project_ai_merge_request(CreateMergeRequestInput {
            project_id: project_id.clone(),
            matter_id: matter_id.clone(),
            assignment_id: assignment.id.clone(),
            requested_by_user_id: Some(user.id.clone()),
            worktree_path: clean_optional(req.worktree_path).or(assignment.worktree_path.clone()),
            branch_name: clean_optional(req.branch_name).or(assignment.branch_name.clone()),
            merge_strategy: clean_or(req.merge_strategy, "manual"),
            review_status: clean_or(req.review_status, "pending"),
            risk_level: clean_or(req.risk_level, "medium"),
            notes: clean_optional(req.notes),
        }) {
        Ok(merge_request) => merge_request,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    insert_event(
        &state,
        &matter,
        &user.id,
        "merge_request_created",
        json!({
            "merge_request_id": merge_request.id,
            "assignment_id": merge_request.assignment_id,
            "branch_name": merge_request.branch_name,
            "status": merge_request.status
        }),
    );
    Json(json!({ "ok": true, "merge_request": merge_request })).into_response()
}

pub(crate) async fn update_merge_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, merge_request_id)): Path<(String, String, String)>,
    Json(req): Json<UpdateMergeRequestRequest>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match require_matter(&state, &project_id, &matter_id) {
        Ok(matter) => matter,
        Err(response) => return response,
    };
    if let Err(response) = ensure_can_decide_matter(&access, &user.id, &matter) {
        return response;
    }
    let merge_request = match state.store.update_project_ai_merge_request(
        &project_id,
        &matter_id,
        &merge_request_id,
        req,
    ) {
        Ok(merge_request) => merge_request,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    insert_event(
        &state,
        &matter,
        &user.id,
        "merge_request_updated",
        json!({
            "merge_request_id": merge_request.id,
            "assignment_id": merge_request.assignment_id,
            "status": merge_request.status,
            "review_status": merge_request.review_status
        }),
    );
    Json(json!({ "ok": true, "merge_request": merge_request })).into_response()
}

pub(crate) async fn check_merge_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, merge_request_id)): Path<(String, String, String)>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match require_matter(&state, &project_id, &matter_id) {
        Ok(matter) => matter,
        Err(response) => return response,
    };
    if let Err(response) = ensure_can_decide_matter(&access, &user.id, &matter) {
        return response;
    }
    match check_merge_gate(&state, &access, &project_id, &matter_id, &merge_request_id) {
        Ok(report) => {
            insert_event(
                &state,
                &matter,
                &user.id,
                "merge_gate_checked",
                json!({
                    "merge_request_id": merge_request_id,
                    "can_apply": report.can_apply,
                    "review_gate": &report.review_gate.status,
                    "check_count": report.checks.len()
                }),
            );
            Json(json!({ "ok": true, "merge_gate": report })).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

pub(crate) async fn apply_merge_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, matter_id, merge_request_id)): Path<(String, String, String)>,
    Json(req): Json<ApplyMergeRequestBody>,
) -> Response {
    let (user, access) = match authenticate_project_member(&state, &headers, &project_id) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let matter = match require_matter(&state, &project_id, &matter_id) {
        Ok(matter) => matter,
        Err(response) => return response,
    };
    if let Err(response) = ensure_can_decide_matter(&access, &user.id, &matter) {
        return response;
    }
    match apply_merge_request_action(
        &state,
        &access,
        &project_id,
        &matter_id,
        &merge_request_id,
        &user.id,
        req,
    ) {
        Ok(report) => Json(json!({ "ok": true, "merge_apply": report })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn assignment_context(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    project_id: &str,
    matter_id: &str,
    assignment_id: &str,
) -> Result<
    (
        PublicUser,
        ProjectAccess,
        ProjectAiMatter,
        ProjectAiMatterAssignment,
    ),
    Response,
> {
    let (user, access) = authenticate_project_member(state, headers, project_id)?;
    let matter = require_matter(state, project_id, matter_id)?;
    let assignment = match state.store.get_project_ai_matter_assignment(assignment_id) {
        Ok(Some(assignment)) if assignment.matter_id == matter_id => assignment,
        Ok(Some(_)) => {
            return Err(json_error(
                StatusCode::BAD_REQUEST,
                "Assignment 不属于当前 Matter",
            ))
        }
        Ok(None) => return Err(json_error(StatusCode::NOT_FOUND, "Assignment 不存在")),
        Err(error) => return Err(json_error(StatusCode::BAD_REQUEST, error.to_string())),
    };
    Ok((user, access, matter, assignment))
}

fn require_matter(
    state: &Arc<AppState>,
    project_id: &str,
    matter_id: &str,
) -> Result<ProjectAiMatter, Response> {
    match state.store.get_project_ai_matter(project_id, matter_id) {
        Ok(Some(matter)) => Ok(matter),
        Ok(None) => Err(json_error(StatusCode::NOT_FOUND, "Matter 不存在")),
        Err(error) => Err(json_error(StatusCode::BAD_REQUEST, error.to_string())),
    }
}

fn insert_event(
    state: &AppState,
    matter: &ProjectAiMatter,
    actor_user_id: &str,
    event_type: &str,
    payload: Value,
) {
    if let Err(error) = state.store.insert_project_ai_event(
        &matter.project_id,
        &matter.id,
        Some(actor_user_id),
        event_type,
        payload,
    ) {
        tracing::warn!(
            matter_id = matter.id,
            event_type,
            "群体 AI 治理事件写入失败: {error:#}"
        );
    } else {
        crate::project_events::publish_group_ai_matter_event(
            state,
            &matter.project_id,
            &matter.id,
            Some(actor_user_id),
            event_type,
            "群体 AI 治理状态已更新。",
        );
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_or(value: Option<String>, fallback: &str) -> String {
    clean_optional(value).unwrap_or_else(|| fallback.to_string())
}
