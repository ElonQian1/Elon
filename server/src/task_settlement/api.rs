use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use std::sync::Arc;

use crate::{
    group_ai::bot_selector::bots_for_project,
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

use super::{
    correction_service, dispute_service,
    model::{
        CreateSettlementCorrectionRequest, OpenSettlementDisputeRequest,
        PrepareSuiProjectionPackageRequest, ResolveSettlementDisputeRequest,
        UpdateTaskEconomyProjectSettingRequest, WithdrawSettlementDisputeRequest,
    },
    service, sui_projection_service,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/economy/overview",
            get(project_overview),
        )
        .route(
            "/api/projects/:project_id/economy/settings",
            patch(update_project_setting),
        )
        .route(
            "/api/projects/:project_id/economy/settlements/:receipt_id",
            get(settlement_detail),
        )
        .route(
            "/api/projects/:project_id/economy/settlements/:receipt_id/sui-envelope",
            get(sui_envelope),
        )
        .route(
            "/api/projects/:project_id/economy/settlements/:receipt_id/sui-projections",
            post(prepare_sui_projection),
        )
        .route(
            "/api/projects/:project_id/economy/sui-projections",
            get(list_sui_projections),
        )
        .route(
            "/api/projects/:project_id/economy/sui-projections/:projection_id",
            get(sui_projection_detail),
        )
        .route(
            "/api/projects/:project_id/economy/sui-projections/:projection_id/verify",
            post(verify_sui_projection),
        )
        .route(
            "/api/projects/:project_id/economy/settlements/:receipt_id/disputes",
            get(list_settlement_disputes).post(open_settlement_dispute),
        )
        .route(
            "/api/projects/:project_id/economy/disputes/:dispute_id/withdraw",
            post(withdraw_settlement_dispute),
        )
        .route(
            "/api/projects/:project_id/economy/disputes/:dispute_id/resolve",
            post(resolve_settlement_dispute),
        )
        .route(
            "/api/projects/:project_id/economy/settlements/:receipt_id/corrections",
            get(list_settlement_corrections),
        )
        .route(
            "/api/projects/:project_id/economy/disputes/:dispute_id/corrections",
            post(create_settlement_correction),
        )
        .route(
            "/api/projects/:project_id/economy/corrections/:correction_id/finalize",
            post(finalize_settlement_correction),
        )
        .merge(super::sui_correction_api::routes())
}

async fn project_overview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::overview(&state.store, &project_id))
}

async fn update_project_setting(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<UpdateTaskEconomyProjectSettingRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以修改影子经济设置");
    }
    service_response(state.store.set_task_economy_project_enabled(
        &project_id,
        &user_id,
        request.enabled,
    ))
}

async fn settlement_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, receipt_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::receipt_detail(
        &state.store,
        &project_id,
        &receipt_id,
    ))
}

async fn sui_envelope(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, receipt_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::sui_envelope(
        &state.store,
        &project_id,
        &receipt_id,
    ))
}

async fn prepare_sui_projection(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, receipt_id)): Path<(String, String)>,
    Json(request): Json<PrepareSuiProjectionPackageRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以准备 Sui 投影包");
    }
    service_response(sui_projection_service::prepare(
        &state.store,
        &project_id,
        &receipt_id,
        &user_id,
        &request.target_network,
    ))
}

async fn list_sui_projections(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(sui_projection_service::list(&state.store, &project_id))
}

async fn sui_projection_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, projection_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(sui_projection_service::detail(
        &state.store,
        &project_id,
        &projection_id,
    ))
}

async fn verify_sui_projection(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, projection_id)): Path<(String, String)>,
) -> Response {
    let (_, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以复核 Sui 投影包");
    }
    service_response(sui_projection_service::verify(
        &state.store,
        &project_id,
        &projection_id,
    ))
}

async fn list_settlement_disputes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, receipt_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(dispute_service::list(
        &state.store,
        &project_id,
        &receipt_id,
    ))
}

async fn open_settlement_dispute(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, receipt_id)): Path<(String, String)>,
    Json(request): Json<OpenSettlementDisputeRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以提出影子结算争议");
    }
    service_response(dispute_service::open(
        &state.store,
        &project_id,
        &receipt_id,
        &user_id,
        &request,
    ))
}

async fn withdraw_settlement_dispute(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, dispute_id)): Path<(String, String)>,
    Json(request): Json<WithdrawSettlementDisputeRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以撤回影子结算争议");
    }
    service_response(dispute_service::withdraw(
        &state.store,
        &project_id,
        &dispute_id,
        &user_id,
        &request,
    ))
}

async fn resolve_settlement_dispute(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, dispute_id)): Path<(String, String)>,
    Json(request): Json<ResolveSettlementDisputeRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以审核影子结算争议");
    }
    service_response(dispute_service::resolve(
        &state.store,
        &project_id,
        &dispute_id,
        &user_id,
        &request,
    ))
}

async fn list_settlement_corrections(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, receipt_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(correction_service::list(
        &state.store,
        &project_id,
        &receipt_id,
    ))
}

async fn create_settlement_correction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, dispute_id)): Path<(String, String)>,
    Json(request): Json<CreateSettlementCorrectionRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以创建纠正 Matter");
    }
    let bots = match bots_for_project(&state, &project_id).await {
        Ok(bots) => bots,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    service_response(correction_service::create(
        &state.store,
        &project_id,
        &dispute_id,
        &user_id,
        &request,
        &bots,
    ))
}

async fn finalize_settlement_correction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, correction_id)): Path<(String, String)>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以重试纠正过账");
    }
    service_response(correction_service::finalize(
        &state.store,
        &project_id,
        &correction_id,
        &user_id,
    ))
}

pub(super) fn project_caller(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(String, String), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    Ok((user.id, access.role))
}

pub(super) fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("权限") {
                StatusCode::FORBIDDEN
            } else if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if message.contains("冲突")
                || message.contains("幂等")
                || message.contains("不能")
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}
