//! Loopback API adapter for the same feature registry used by MCP agents.

use axum::{response::IntoResponse, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

use crate::{
    project_feature_registry::ProjectFeatureStatus,
    project_feature_registry_service::{
        check_drift, claim_feature, feature_history, list_features, plan_feature, record_evidence,
        register_feature, release_claim, transition_feature, RegisterFeatureRequest,
    },
    project_feature_registry_store::FeatureEvidenceInput,
    project_feature_registry_update::{
        rebind_requirement, update_feature, RebindRequirementRequest, UpdateFeatureRequest,
    },
    NodeRuntime,
};

#[derive(Debug, Deserialize)]
struct FeatureApiRequest {
    project_root: String,
    #[serde(default)]
    feature_id: String,
    #[serde(default)]
    statuses: Vec<ProjectFeatureStatus>,
    #[serde(default)]
    query: String,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
    #[serde(default)]
    agent_id: String,
    #[serde(default)]
    actor: String,
    #[serde(default)]
    claim_id: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    lease_minutes: u64,
    #[serde(default)]
    to_status: Option<ProjectFeatureStatus>,
    #[serde(default)]
    evidence: Vec<FeatureEvidenceInput>,
    #[serde(default)]
    expected_registry_revision: Option<String>,
    #[serde(default)]
    feature: Option<RegisterFeatureRequest>,
}

#[derive(Debug, Deserialize)]
struct FeatureUpdateApiRequest {
    project_root: String,
    #[serde(flatten)]
    update: UpdateFeatureRequest,
}

#[derive(Debug, Deserialize)]
struct FeatureRebindApiRequest {
    project_root: String,
    #[serde(flatten)]
    rebind: RebindRequirementRequest,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/project-docs/features/list", post(list_handler))
        .route("/api/project-docs/features/plan", post(plan_handler))
        .route(
            "/api/project-docs/features/register",
            post(register_handler),
        )
        .route("/api/project-docs/features/update", post(update_handler))
        .route(
            "/api/project-docs/features/rebind-requirement",
            post(rebind_handler),
        )
        .route("/api/project-docs/features/claim", post(claim_handler))
        .route(
            "/api/project-docs/features/release-claim",
            post(release_claim_handler),
        )
        .route(
            "/api/project-docs/features/transition",
            post(transition_handler),
        )
        .route(
            "/api/project-docs/features/evidence",
            post(evidence_handler),
        )
        .route("/api/project-docs/features/drift", post(drift_handler))
        .route("/api/project-docs/features/history", post(history_handler))
}

async fn list_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root).and_then(|workspace| {
            list_features(
                &workspace,
                &request.statuses,
                &request.query,
                request.offset,
                if request.limit == 0 {
                    50
                } else {
                    request.limit
                },
            )
        }),
    )
}

async fn plan_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root)
            .and_then(|workspace| plan_feature(&workspace, &request.feature_id)),
    )
}

async fn register_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root).and_then(|workspace| {
            request
                .feature
                .ok_or_else(|| anyhow::anyhow!("feature 不能为空"))
                .and_then(|feature| register_feature(&workspace, feature))
        }),
    )
}

async fn update_handler(Json(request): Json<FeatureUpdateApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root)
            .and_then(|workspace| update_feature(&workspace, request.update)),
    )
}

async fn rebind_handler(Json(request): Json<FeatureRebindApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root)
            .and_then(|workspace| rebind_requirement(&workspace, request.rebind)),
    )
}

async fn claim_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root).and_then(|workspace| {
            claim_feature(
                &workspace,
                &request.feature_id,
                &request.agent_id,
                if request.lease_minutes == 0 {
                    120
                } else {
                    request.lease_minutes
                },
                request.expected_registry_revision.as_deref(),
            )
        }),
    )
}

async fn release_claim_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root).and_then(|workspace| {
            release_claim(
                &workspace,
                &request.feature_id,
                &request.claim_id,
                &request.reason,
                request.expected_registry_revision.as_deref(),
            )
        }),
    )
}

async fn transition_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root).and_then(|workspace| {
            request
                .to_status
                .ok_or_else(|| anyhow::anyhow!("to_status 不能为空"))
                .and_then(|status| {
                    transition_feature(
                        &workspace,
                        &request.feature_id,
                        status,
                        &request.actor,
                        &request.reason,
                        &request.claim_id,
                        request.expected_registry_revision.as_deref(),
                    )
                })
        }),
    )
}

async fn evidence_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root).and_then(|workspace| {
            record_evidence(
                &workspace,
                &request.feature_id,
                &request.claim_id,
                &request.actor,
                request.evidence,
                request.expected_registry_revision.as_deref(),
            )
        }),
    )
}

async fn drift_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root).and_then(|workspace| {
            check_drift(
                &workspace,
                (!request.feature_id.trim().is_empty()).then_some(request.feature_id.as_str()),
            )
        }),
    )
}

async fn history_handler(Json(request): Json<FeatureApiRequest>) -> axum::response::Response {
    response(
        validated_workspace(&request.project_root).and_then(|workspace| {
            feature_history(
                &workspace,
                (!request.feature_id.trim().is_empty()).then_some(request.feature_id.as_str()),
                request.offset,
                if request.limit == 0 {
                    50
                } else {
                    request.limit
                },
            )
        }),
    )
}

fn validated_workspace(project_root: &str) -> anyhow::Result<PathBuf> {
    crate::node_agent_project_docs_mcp::validate_project_root(project_root)
}

fn response(result: anyhow::Result<Value>) -> axum::response::Response {
    match result {
        Ok(value) => Json(json!({"ok":true,"result":value})).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"ok":false,"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}
