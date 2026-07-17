//! Loopback API for actionable issue governance and document Git history.

use axum::{response::IntoResponse, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

use crate::{
    project_document_index::ProjectDocumentIndex,
    project_document_issue_workflow::{health_trend, update_issue, IssueWorkflowUpdate},
    project_document_maintenance::list_governed_issues,
    project_document_versioning::{
        document_version_diff, list_document_versions, restore_document_version,
    },
    NodeRuntime,
};

#[derive(Debug, Default, Deserialize)]
struct GovernanceRequest {
    project_root: String,
    #[serde(default)]
    fingerprint: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    due_at: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    snoozed_until: String,
    #[serde(default)]
    issue_types: Vec<String>,
    #[serde(default)]
    statuses: Vec<String>,
    #[serde(default)]
    severities: Vec<String>,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
    #[serde(default)]
    commit: String,
    #[serde(default)]
    path: String,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/project-docs/governance/issues", post(issues_handler))
        .route(
            "/api/project-docs/governance/issues/update",
            post(update_issue_handler),
        )
        .route("/api/project-docs/governance/trend", post(trend_handler))
        .route(
            "/api/project-docs/governance/history",
            post(history_handler),
        )
        .route("/api/project-docs/governance/diff", post(diff_handler))
        .route(
            "/api/project-docs/governance/restore",
            post(restore_handler),
        )
}

async fn issues_handler(Json(request): Json<GovernanceRequest>) -> axum::response::Response {
    let limit = if request.limit == 0 {
        100
    } else {
        request.limit
    };
    response(
        list_governed_issues(
            workspace(&request),
            &request.issue_types,
            &request.statuses,
            &request.severities,
            &request.owner,
            request.offset,
            limit,
        )
        .map(|issues| {
            let returned = issues.len();
            json!({"issues": issues, "offset": request.offset, "returned": returned})
        }),
    )
}

async fn update_issue_handler(Json(request): Json<GovernanceRequest>) -> axum::response::Response {
    let result = ProjectDocumentIndex::open(workspace(&request)).and_then(|index| {
        update_issue(
            &index,
            IssueWorkflowUpdate {
                fingerprint: request.fingerprint,
                status: request.status,
                owner: request.owner,
                due_at: request.due_at,
                reason: request.reason,
                snoozed_until: request.snoozed_until,
            },
        )
    });
    response(result.map(|workflow| json!({"workflow": workflow})))
}

async fn trend_handler(Json(request): Json<GovernanceRequest>) -> axum::response::Response {
    let limit = if request.limit == 0 {
        30
    } else {
        request.limit
    };
    response(
        ProjectDocumentIndex::open(workspace(&request))
            .and_then(|index| health_trend(&index, limit))
            .map(|trend| json!({"trend": trend})),
    )
}

async fn history_handler(Json(request): Json<GovernanceRequest>) -> axum::response::Response {
    let limit = if request.limit == 0 {
        20
    } else {
        request.limit
    };
    response(
        list_document_versions(workspace(&request), limit)
            .map(|versions| json!({"versions": versions})),
    )
}

async fn diff_handler(Json(request): Json<GovernanceRequest>) -> axum::response::Response {
    response(document_version_diff(
        workspace(&request),
        &request.commit,
        (!request.path.trim().is_empty()).then_some(request.path.as_str()),
    ))
}

async fn restore_handler(Json(request): Json<GovernanceRequest>) -> axum::response::Response {
    response(restore_document_version(
        workspace(&request),
        &request.commit,
    ))
}

fn workspace(request: &GovernanceRequest) -> &Path {
    Path::new(request.project_root.trim())
}

fn response(result: anyhow::Result<Value>) -> axum::response::Response {
    match result {
        Ok(value) => Json(json!({"ok": true, "result": value})).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": format!("{error:#}")})),
        )
            .into_response(),
    }
}
