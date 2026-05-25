use crate::types::AppState;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

/// 健康检查
pub async fn health() -> &'static str {
    "OK"
}

#[derive(serde::Serialize)]
pub struct ReadyResponse {
    pub service: &'static str,
    pub status: &'static str,
    pub backend: &'static str,
    pub local_cli_enabled: bool,
    pub codex_cli_only: bool,
    pub cli_options: Vec<ReadyCliOption>,
    pub api_agents: usize,
    pub image_generation: bool,
    pub codex_network: crate::codex_health::CodexNetworkHealthSnapshot,
}

#[derive(serde::Serialize)]
pub struct ReadyCliOption {
    pub id: String,
    pub label: String,
    pub provider: String,
    pub model: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct ImageGenerateRequest {
    pub prompt: String,
}

#[derive(serde::Serialize)]
pub struct ImageGenerateResponse {
    pub job_id: String,
    pub url: String,
    pub revised_prompt: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerVersionResponse {
    pub service: &'static str,
    pub status: &'static str,
    pub version_name: &'static str,
    pub git_sha: &'static str,
}

pub async fn server_version() -> Json<ServerVersionResponse> {
    // 版本号优先取构建脚本注入的 ELON_BUILD_VERSION（来自 release_claim 服务器分配），
    // 否则回落到 CARGO_PKG_VERSION（本地 cargo run 时 Cargo.toml 的占位值）。
    Json(ServerVersionResponse {
        service: "elon-server",
        status: "ok",
        version_name: option_env!("ELON_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
        git_sha: option_env!("ELON_SERVER_GIT_SHA").unwrap_or("dev"),
    })
}

#[derive(serde::Deserialize)]
pub struct ServerTraceQuery {
    pub limit: Option<usize>,
}

pub async fn server_trace(
    State(state): State<Arc<AppState>>,
    AxumPath(trace_id): AxumPath<String>,
    Query(query): Query<ServerTraceQuery>,
) -> Json<serde_json::Value> {
    Json(
        state
            .server_traces
            .trace_json(&trace_id, query.limit.unwrap_or(120)),
    )
}

pub async fn codex_health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "codexNetwork": state.codex_network.snapshot().await,
    }))
}

pub async fn readyz(State(state): State<Arc<AppState>>) -> Json<ReadyResponse> {
    let api_agents = state.agents_config.read().await.agents.len();
    let cli_options = state
        .ai_cli
        .options
        .iter()
        .map(|opt| ReadyCliOption {
            id: opt.id.clone(),
            label: opt.label.clone(),
            provider: opt.provider.clone(),
            model: opt.model.clone(),
        })
        .collect();

    Json(ReadyResponse {
        service: "elon-server",
        status: "ok",
        backend: state.default_backend.as_str(),
        local_cli_enabled: state.ai_cli.enabled,
        codex_cli_only: state.ai_cli.codex_cli_only,
        cli_options,
        api_agents,
        image_generation: state.image_model.is_some(),
        codex_network: state.codex_network.snapshot().await,
    })
}

pub async fn generate_image(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImageGenerateRequest>,
) -> Result<Json<ImageGenerateResponse>, (StatusCode, Json<serde_json::Value>)> {
    let prompt = req.prompt.trim();
    if prompt.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "prompt 不能为空"})),
        ));
    }
    if state.image_model.is_none() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "文生图模型未配置，请设置 IMAGE_API_KEY"})),
        ));
    }

    match crate::image_generation::generate_text_to_image(&state, prompt).await {
        Ok(image) => Ok(Json(ImageGenerateResponse {
            job_id: image.job_id,
            url: image.url,
            revised_prompt: image.revised_prompt,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}
