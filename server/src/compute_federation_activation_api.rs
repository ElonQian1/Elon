use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    compute_federation_activation_application_service::{self, ApplyComputeActivationPlanBody},
    compute_federation_activation_lifecycle_service::{
        self, SupersedeComputeActivationEvidenceRequestBody,
    },
    compute_federation_activation_plan_review_service::{self, ReviewComputeActivationPlanBody},
    compute_federation_activation_plan_service::{self, PrepareComputeActivationPlanBody},
    compute_federation_activation_quarantine_service::{
        self, QuarantineComputeActivationApplicationBody,
    },
    compute_federation_activation_service::{
        self, CancelMyComputeActivationEvidenceRequest, ReviewComputeActivationEvidenceRequestBody,
        SubmitMyComputeActivationEvidenceRequest,
    },
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests",
            get(list_my_requests).post(submit_my_request),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id",
            get(get_my_request),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id/cancel",
            post(cancel_my_request),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id/preflight",
            get(preflight_my_request),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests",
            get(list_reviewable_requests),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/review",
            post(review_request),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/preflight",
            get(preflight_review_request),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/supersede",
            post(supersede_request),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/activation-plan",
            get(get_activation_plan).post(prepare_activation_plan),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/preflight",
            get(preflight_activation_plan),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/review",
            get(get_activation_plan_review).post(review_activation_plan),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/application",
            get(get_activation_application).post(apply_activation_plan),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/activation-plan/application/quarantine",
            get(get_activation_quarantine).post(quarantine_activation_application),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct ReviewQueueQuery {
    #[serde(default = "default_review_status")]
    status: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn submit_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Json(request): Json<SubmitMyComputeActivationEvidenceRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::submit_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        request,
    ))
}

async fn list_my_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(
        compute_federation_activation_service::list_for_user(
            &state.store,
            &user_id,
            &provider_id,
            &pool_id,
            query.limit,
        )
        .map(|items| json!({"activation_evidence_requests":items})),
    )
}

async fn get_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, request_id)): Path<(String, String, String)>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::get_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &request_id,
    ))
}

async fn cancel_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, request_id)): Path<(String, String, String)>,
    Json(request): Json<CancelMyComputeActivationEvidenceRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::cancel_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &request_id,
        request,
    ))
}

async fn preflight_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, request_id)): Path<(String, String, String)>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::preflight_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &request_id,
    ))
}

async fn list_reviewable_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ReviewQueueQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(
        compute_federation_activation_service::list_for_review(
            &state.store,
            &query.status,
            query.limit,
        )
        .map(|items| json!({"activation_evidence_requests":items})),
    )
}

async fn review_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(request): Json<ReviewComputeActivationEvidenceRequestBody>,
) -> Response {
    let reviewer_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::review(
        &state.store,
        &reviewer_user_id,
        &request_id,
        request,
    ))
}

async fn preflight_review_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(compute_federation_activation_service::preflight_for_review(
        &state.store,
        &request_id,
    ))
}

async fn supersede_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(request): Json<SupersedeComputeActivationEvidenceRequestBody>,
) -> Response {
    let actor_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(
        compute_federation_activation_lifecycle_service::supersede_for_review(
            &state.store,
            &actor_user_id,
            &request_id,
            request,
        ),
    )
}

async fn prepare_activation_plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(request): Json<PrepareComputeActivationPlanBody>,
) -> Response {
    let actor_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(
        compute_federation_activation_plan_service::prepare_for_review(
            &state.store,
            &actor_user_id,
            &request_id,
            request,
        ),
    )
}

async fn get_activation_plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(
        compute_federation_activation_plan_service::get_for_review(&state.store, &request_id)
            .map(|plan| json!({"activation_plan":plan,"activation_effect":"none"})),
    )
}

async fn preflight_activation_plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(
        compute_federation_activation_plan_service::preflight_for_review(&state.store, &request_id),
    )
}

async fn review_activation_plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(request): Json<ReviewComputeActivationPlanBody>,
) -> Response {
    let reviewer_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(
        compute_federation_activation_plan_review_service::review_for_admin(
            &state.store,
            &reviewer_user_id,
            &request_id,
            request,
        ),
    )
}

async fn get_activation_plan_review(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(
        compute_federation_activation_plan_review_service::get_for_admin(&state.store, &request_id)
            .map(|review| json!({"activation_plan_review":review,"activation_effect":"none"})),
    )
}

async fn apply_activation_plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(request): Json<ApplyComputeActivationPlanBody>,
) -> Response {
    let actor_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(
        compute_federation_activation_application_service::apply_for_review(
            &state.store,
            &actor_user_id,
            &request_id,
            request,
        ),
    )
}

async fn get_activation_application(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(
        compute_federation_activation_application_service::get_for_review(
            &state.store,
            &request_id,
        )
        .map(|application| json!({"activation_application":application})),
    )
}

async fn quarantine_activation_application(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(request): Json<QuarantineComputeActivationApplicationBody>,
) -> Response {
    let actor_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(
        compute_federation_activation_quarantine_service::quarantine_for_review(
            &state.store,
            &actor_user_id,
            &request_id,
            request,
        ),
    )
}

async fn get_activation_quarantine(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(
        compute_federation_activation_quarantine_service::get_for_review(&state.store, &request_id)
            .map(|quarantine| json!({"activation_quarantine":quarantine})),
    )
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以审核算力激活证据申请",
        ));
    }
    Ok(user.id)
}

fn activation_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

fn default_limit() -> usize {
    20
}

fn default_review_status() -> String {
    "submitted".to_string()
}
