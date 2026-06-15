use std::{path::PathBuf, sync::Arc};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{admin, types::AppState};

use super::{
    symbol_index_patch_apply::{apply_reviewed_symbol_patch, rollback_symbol_patch},
    symbol_index_patch_apply_types::{PatchApplyMode, PatchApplyOptions},
    symbol_index_patch_check::check_symbol_patch_diff,
    symbol_index_patch_dry_run::dry_run_symbol_patch as run_symbol_patch_dry_run,
    symbol_index_patch_repair_attempt::build_symbol_patch_repair_attempt_response,
    symbol_index_patch_repair_generate::generate_symbol_patch_repair as run_symbol_patch_repair_generation,
    symbol_index_patch_review::build_symbol_patch_review,
    symbol_index_patch_verification_repair::{
        PatchVerificationCommandResultInput, build_symbol_patch_verification_repair_response,
    },
    symbol_index_patch_verification_run::run_symbol_patch_verification as run_symbol_patch_verification_flow,
    symbol_index_task_pack::{SymbolTaskPackQuery, build_latest_symbol_task_pack},
};

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolPatchBody {
    pub(crate) q: Option<String>,
    pub(crate) query: Option<String>,
    #[serde(alias = "userId")]
    pub(crate) user_id: Option<String>,
    #[serde(alias = "traceId")]
    pub(crate) trace_id: Option<String>,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
    #[serde(alias = "edgeKind")]
    pub(crate) edge_kind: Option<String>,
    pub(crate) depth: Option<usize>,
    #[serde(alias = "searchLimit")]
    pub(crate) search_limit: Option<usize>,
    #[serde(alias = "chunkLimit")]
    pub(crate) chunk_limit: Option<usize>,
    #[serde(alias = "vectorModel")]
    pub(crate) vector_model: Option<String>,
    #[serde(alias = "vectorLimit")]
    pub(crate) vector_limit: Option<usize>,
    #[serde(alias = "impactLimit")]
    pub(crate) impact_limit: Option<usize>,
    #[serde(alias = "maxChars")]
    pub(crate) max_chars: Option<usize>,
    pub(crate) workspace: Option<String>,
    #[serde(
        alias = "workspacePath",
        alias = "workspaceRoot",
        alias = "workspace_root"
    )]
    pub(crate) workspace_path: Option<String>,
    pub(crate) diff: Option<String>,
    #[serde(alias = "generatedDiff")]
    pub(crate) generated_diff: Option<String>,
    pub(crate) patch: Option<String>,
    #[serde(alias = "repairPatch", alias = "repairDiff")]
    pub(crate) repair_patch: Option<String>,
    pub(crate) attempt: Option<usize>,
    #[serde(
        alias = "maxAttempts",
        alias = "maxRepairAttempts",
        alias = "max_repairs"
    )]
    pub(crate) max_attempts: Option<usize>,
    #[serde(default, alias = "verificationResults")]
    pub(crate) verification_results: Vec<PatchVerificationCommandResultInput>,
    #[serde(alias = "applyMode", alias = "mode")]
    pub(crate) apply_mode: Option<String>,
    pub(crate) confirm: Option<bool>,
    pub(crate) commit: Option<bool>,
    #[serde(alias = "keepWorktree")]
    pub(crate) keep_worktree: Option<bool>,
    #[serde(alias = "branchName", alias = "branch")]
    pub(crate) branch_name: Option<String>,
    #[serde(alias = "commitMessage")]
    pub(crate) commit_message: Option<String>,
    #[serde(alias = "commitSha")]
    pub(crate) commit_sha: Option<String>,
    #[serde(alias = "requireReviewApproval")]
    pub(crate) require_review_approval: Option<bool>,
}

pub(crate) async fn check_symbol_patch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_parts(false) {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match build_latest_symbol_task_pack(&state.data_dir, &parts.query) {
        Ok(response) => Json(check_symbol_patch_diff(
            &response.patch_generation,
            &parts.diff,
        ))
        .into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn dry_run_symbol_patch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_parts(true) {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    let Some(workspace) = parts.workspace else {
        return json_error(StatusCode::BAD_REQUEST, "workspace 不能为空");
    };
    match build_latest_symbol_task_pack(&state.data_dir, &parts.query) {
        Ok(response) => {
            let generation = response.patch_generation.clone();
            let diff = parts.diff;
            match tokio::task::spawn_blocking(move || {
                run_symbol_patch_dry_run(&generation, &diff, &workspace)
            })
            .await
            {
                Ok(result) => Json(result).into_response(),
                Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
            }
        }
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn verify_symbol_patch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_parts(true) {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    let Some(workspace) = parts.workspace else {
        return json_error(StatusCode::BAD_REQUEST, "workspace 不能为空");
    };
    match build_latest_symbol_task_pack(&state.data_dir, &parts.query) {
        Ok(response) => {
            let generation = response.patch_generation.clone();
            let diff = parts.diff;
            let verification_results = parts.verification_results;
            match tokio::task::spawn_blocking(move || {
                build_symbol_patch_verification_repair_response(
                    &generation,
                    &diff,
                    &workspace,
                    &verification_results,
                )
            })
            .await
            {
                Ok(result) => Json(result).into_response(),
                Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
            }
        }
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn run_symbol_patch_verification(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_parts(true) {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    let Some(workspace) = parts.workspace else {
        return json_error(StatusCode::BAD_REQUEST, "workspace 不能为空");
    };
    match build_latest_symbol_task_pack(&state.data_dir, &parts.query) {
        Ok(response) => {
            let generation = response.patch_generation.clone();
            let diff = parts.diff;
            match tokio::task::spawn_blocking(move || {
                run_symbol_patch_verification_flow(&generation, &diff, &workspace)
            })
            .await
            {
                Ok(result) => Json(result).into_response(),
                Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
            }
        }
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn run_symbol_patch_repair_attempt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_parts(true) {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    let Some(workspace) = parts.workspace else {
        return json_error(StatusCode::BAD_REQUEST, "workspace 不能为空");
    };
    let Some(repair_patch) = parts.repair_patch else {
        return json_error(StatusCode::BAD_REQUEST, "repairPatch 不能为空");
    };
    match build_latest_symbol_task_pack(&state.data_dir, &parts.query) {
        Ok(response) => {
            let generation = response.patch_generation.clone();
            let original_patch = parts.diff;
            let attempt = parts.attempt;
            let max_attempts = parts.max_attempts;
            match tokio::task::spawn_blocking(move || {
                build_symbol_patch_repair_attempt_response(
                    &generation,
                    &original_patch,
                    &repair_patch,
                    &workspace,
                    attempt,
                    max_attempts,
                )
            })
            .await
            {
                Ok(result) => Json(result).into_response(),
                Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
            }
        }
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn generate_symbol_patch_repair(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_parts(true) {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    let Some(workspace) = parts.workspace else {
        return json_error(StatusCode::BAD_REQUEST, "workspace 不能为空");
    };
    match build_latest_symbol_task_pack(&state.data_dir, &parts.query) {
        Ok(response) => {
            let user_id = parts.user_id.unwrap_or_else(|| "admin".to_string());
            match run_symbol_patch_repair_generation(
                &state,
                response.patch_generation.clone(),
                parts.diff,
                workspace,
                user_id,
                parts.attempt,
                parts.max_attempts,
            )
            .await
            {
                Ok(result) => Json(result).into_response(),
                Err(error) => json_error(StatusCode::BAD_GATEWAY, &error),
            }
        }
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn review_symbol_patch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_parts(true) {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    let Some(workspace) = parts.workspace else {
        return json_error(StatusCode::BAD_REQUEST, "workspace 不能为空");
    };
    match build_latest_symbol_task_pack(&state.data_dir, &parts.query) {
        Ok(response) => {
            let generation = response.patch_generation.clone();
            let plan = response.patch_plan.clone();
            let diff = parts.diff;
            match tokio::task::spawn_blocking(move || {
                let verification =
                    run_symbol_patch_verification_flow(&generation, &diff, &workspace);
                build_symbol_patch_review(&plan, &generation, &diff, Some(&verification))
            })
            .await
            {
                Ok(result) => Json(result).into_response(),
                Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
            }
        }
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn apply_symbol_patch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_parts(true) {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    let Some(workspace) = parts.workspace else {
        return json_error(StatusCode::BAD_REQUEST, "workspace 不能为空");
    };
    let options = match PatchApplyMode::parse(parts.apply_mode.as_deref()) {
        Ok(mode) => PatchApplyOptions {
            mode,
            confirm: parts.confirm,
            commit: parts.commit,
            keep_worktree: parts.keep_worktree,
            branch_name: parts.branch_name,
            commit_message: parts.commit_message,
            require_review_approval: parts.require_review_approval,
        },
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };

    match build_latest_symbol_task_pack(&state.data_dir, &parts.query) {
        Ok(response) => {
            let plan = response.patch_plan.clone();
            let generation = response.patch_generation.clone();
            let diff = parts.diff;
            match tokio::task::spawn_blocking(move || {
                apply_reviewed_symbol_patch(&plan, &generation, &diff, &workspace, options)
            })
            .await
            {
                Ok(result) => Json(result).into_response(),
                Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
            }
        }
        Err(error) => json_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

pub(crate) async fn rollback_symbol_patch_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SymbolPatchBody>,
) -> Response {
    if !admin::check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let parts = match body.into_rollback_parts() {
        Ok(parts) => parts,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, &message),
    };
    match tokio::task::spawn_blocking(move || {
        rollback_symbol_patch(
            &parts.workspace,
            parts.diff.as_deref(),
            parts.commit_sha.as_deref(),
            parts.confirm,
        )
    })
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
    }
}

impl SymbolPatchBody {
    fn into_parts(self, require_workspace: bool) -> Result<PatchBodyParts, String> {
        let text = clean(self.q).or_else(|| clean(self.query));
        if text.is_none() {
            return Err("q 不能为空".to_string());
        }
        let workspace = clean(self.workspace)
            .or_else(|| clean(self.workspace_path))
            .map(PathBuf::from);
        if require_workspace && workspace.is_none() {
            return Err("workspace 不能为空".to_string());
        }
        let diff = non_empty_patch(self.diff)
            .or_else(|| non_empty_patch(self.generated_diff))
            .or_else(|| non_empty_patch(self.patch))
            .ok_or_else(|| "diff 不能为空".to_string())?;

        Ok(PatchBodyParts {
            query: SymbolTaskPackQuery {
                trace_id: clean(self.trace_id),
                text,
                kind: clean(self.kind),
                path: clean(self.path),
                edge_kind: clean(self.edge_kind),
                depth: self.depth.unwrap_or_default(),
                search_limit: self.search_limit.unwrap_or_default(),
                chunk_limit: self.chunk_limit.unwrap_or_default(),
                vector_model: clean(self.vector_model),
                vector_limit: self.vector_limit.unwrap_or_default(),
                impact_limit: self.impact_limit.unwrap_or_default(),
                max_chars: self.max_chars.unwrap_or_default(),
            },
            workspace,
            diff,
            repair_patch: non_empty_patch(self.repair_patch),
            user_id: clean(self.user_id),
            attempt: self.attempt,
            max_attempts: self.max_attempts,
            verification_results: self.verification_results,
            apply_mode: clean(self.apply_mode),
            confirm: self.confirm.unwrap_or(false),
            commit: self.commit.unwrap_or(false),
            keep_worktree: self.keep_worktree.unwrap_or(true),
            branch_name: clean(self.branch_name),
            commit_message: clean(self.commit_message),
            require_review_approval: self.require_review_approval.unwrap_or(true),
        })
    }

    fn into_rollback_parts(self) -> Result<PatchRollbackParts, String> {
        let workspace = clean(self.workspace)
            .or_else(|| clean(self.workspace_path))
            .map(PathBuf::from)
            .ok_or_else(|| "workspace 不能为空".to_string())?;
        let diff = non_empty_patch(self.diff)
            .or_else(|| non_empty_patch(self.generated_diff))
            .or_else(|| non_empty_patch(self.patch));
        let commit_sha = clean(self.commit_sha);
        if diff.is_none() && commit_sha.is_none() {
            return Err("rollback 需要 diff 或 commitSha".to_string());
        }
        Ok(PatchRollbackParts {
            workspace,
            diff,
            commit_sha,
            confirm: self.confirm.unwrap_or(false),
        })
    }
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn non_empty_patch(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message,
        })),
    )
        .into_response()
}

struct PatchBodyParts {
    query: SymbolTaskPackQuery,
    workspace: Option<PathBuf>,
    diff: String,
    repair_patch: Option<String>,
    user_id: Option<String>,
    attempt: Option<usize>,
    max_attempts: Option<usize>,
    verification_results: Vec<PatchVerificationCommandResultInput>,
    apply_mode: Option<String>,
    confirm: bool,
    commit: bool,
    keep_worktree: bool,
    branch_name: Option<String>,
    commit_message: Option<String>,
    require_review_approval: bool,
}

struct PatchRollbackParts {
    workspace: PathBuf,
    diff: Option<String>,
    commit_sha: Option<String>,
    confirm: bool,
}
