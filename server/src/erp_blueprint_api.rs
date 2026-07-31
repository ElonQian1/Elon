//! HTTP adapter for ERP blueprint governance.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    erp_blueprint::{
        model::{
            CreateBlueprintRequest, CreateBlueprintVersionRequest, CreateErpInstanceRequest,
            DecideProposalRequest, DecideUpgradeRequest, PrepareUpgradeRequest,
            ResolveRequirementRequest, SubmitFeatureSignalRequest,
        },
        proposal, service,
    },
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

#[derive(Debug, Deserialize)]
struct CapabilityQuery {
    #[serde(default)]
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/projects/:project_id/erp/overview", get(overview))
        .route(
            "/api/projects/:project_id/erp/blueprints",
            post(create_blueprint),
        )
        .route(
            "/api/projects/:project_id/erp/blueprints/:blueprint_id/versions",
            post(publish_version),
        )
        .route(
            "/api/projects/:project_id/erp/blueprints/:blueprint_id/instances",
            post(create_instance),
        )
        .route(
            "/api/projects/:project_id/erp/capabilities",
            get(search_capabilities),
        )
        .route(
            "/api/projects/:project_id/erp/requirements/resolve",
            post(resolve_requirement),
        )
        .route(
            "/api/projects/:project_id/erp/instances/:instance_id/signals",
            post(submit_signal),
        )
        .route(
            "/api/projects/:project_id/erp/proposals/:proposal_id/decision",
            post(decide_proposal),
        )
        .route(
            "/api/projects/:project_id/erp/proposals/:proposal_id/matter",
            post(create_proposal_matter),
        )
        .route(
            "/api/projects/:project_id/erp/instances/:instance_id/upgrades",
            post(prepare_upgrade),
        )
        .route(
            "/api/projects/:project_id/erp/upgrades/:campaign_id/decision",
            post(decide_upgrade),
        )
}

async fn overview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = authenticate(&state, &headers, &project_id, false) {
        return response;
    }
    respond(service::overview(&state.store, &project_id))
}

async fn create_blueprint(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateBlueprintRequest>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(service::create_blueprint(
        &state.store,
        &project_id,
        &user.id,
        request,
    ))
}

async fn publish_version(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, blueprint_id)): Path<(String, String)>,
    Json(request): Json<CreateBlueprintVersionRequest>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(service::publish_version(
        &state.store,
        &project_id,
        &blueprint_id,
        &user.id,
        request,
    ))
}

async fn create_instance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, blueprint_id)): Path<(String, String)>,
    Json(request): Json<CreateErpInstanceRequest>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(service::create_instance(
        &state.store,
        &project_id,
        &blueprint_id,
        &user.id,
        request,
    ))
}

async fn search_capabilities(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<CapabilityQuery>,
) -> Response {
    if let Err(response) = authenticate(&state, &headers, &project_id, false) {
        return response;
    }
    let result = state
        .store
        .erp_blueprint_for_project(&project_id)
        .and_then(|blueprint| {
            let blueprint =
                blueprint.ok_or_else(|| anyhow::anyhow!("当前项目尚未关联 ERP 蓝图"))?;
            Ok(proposal::search_capabilities(
                &blueprint.definition,
                &query.query,
                query.limit,
            ))
        });
    respond(result.map(|capabilities| {
        json!({"schema":"yilong.erp.capability_catalog.v1","capabilities":capabilities})
    }))
}

async fn resolve_requirement(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<ResolveRequirementRequest>,
) -> Response {
    if let Err(response) = authenticate(&state, &headers, &project_id, false) {
        return response;
    }
    respond(service::resolve_requirement(
        &state.store,
        &project_id,
        request,
    ))
}

async fn submit_signal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, instance_id)): Path<(String, String)>,
    Json(request): Json<SubmitFeatureSignalRequest>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(service::submit_signal(
        &state.store,
        &project_id,
        &instance_id,
        &user.id,
        request,
    ))
}

async fn decide_proposal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, proposal_id)): Path<(String, String)>,
    Json(request): Json<DecideProposalRequest>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(
        service::decide_proposal(&state.store, &project_id, &proposal_id, &user.id, request)
            .map(|(proposal, matter_id)| json!({"proposal":proposal,"matter_id":matter_id})),
    )
}

async fn create_proposal_matter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, proposal_id)): Path<(String, String)>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(
        service::create_proposal_matter(&state.store, &project_id, &proposal_id, &user.id)
            .map(|(proposal, matter_id)| json!({"proposal":proposal,"matter_id":matter_id})),
    )
}

async fn prepare_upgrade(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, instance_id)): Path<(String, String)>,
    Json(request): Json<PrepareUpgradeRequest>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(service::prepare_upgrade(
        &state.store,
        &project_id,
        &instance_id,
        &user.id,
        request,
    ))
}

async fn decide_upgrade(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, campaign_id)): Path<(String, String)>,
    Json(request): Json<DecideUpgradeRequest>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(service::decide_upgrade(
        &state.store,
        &project_id,
        &campaign_id,
        &user.id,
        request,
    ))
}

fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    write: bool,
) -> Result<crate::store::PublicUser, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error.to_string()))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;
    if write && !can_edit(&access.role) {
        return Err(json_error(StatusCode::FORBIDDEN, "当前项目只有查看权限"));
    }
    Ok(user)
}

fn respond<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn default_limit() -> usize {
    20
}
