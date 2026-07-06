use crate::{project_auth::auth_from_headers, types::AppState};
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, OnceLock},
};

static SERVER_RELEASE_CHANGELOG: OnceLock<Option<String>> = OnceLock::new();

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
    pub display_model: String,
    pub reasoning_effort: Option<String>,
    pub reasoning_summary: Option<String>,
    pub verbosity: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
}

pub async fn server_version() -> Json<ServerVersionResponse> {
    // 版本号优先取构建脚本注入的 ELON_BUILD_VERSION（来自 release_claim 服务器分配），
    // 否则回落到 CARGO_PKG_VERSION（本地 cargo run 时 Cargo.toml 的占位值）。
    // 该接口也用于 PC / APK 端实测当前运行版本是否已完成发布切换。
    Json(ServerVersionResponse {
        service: "elon-server",
        status: "ok",
        version_name: option_env!("ELON_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
        git_sha: option_env!("ELON_SERVER_GIT_SHA").unwrap_or("dev"),
        changelog: server_release_changelog(),
    })
}

fn server_release_changelog() -> Option<String> {
    SERVER_RELEASE_CHANGELOG
        .get_or_init(|| {
            option_env!("ELON_RELEASE_CHANGELOG")
                .and_then(clean_release_summary)
                .or_else(git_commit_summary)
        })
        .clone()
}

fn clean_release_summary(value: &str) -> Option<String> {
    let text = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return None;
    }
    if text.chars().count() > 240 {
        let mut short = text.chars().take(237).collect::<String>();
        short.push_str("...");
        return Some(short);
    }
    Some(text)
}

fn git_commit_summary() -> Option<String> {
    let sha = option_env!("ELON_SERVER_GIT_SHA")?.trim();
    if sha.is_empty() || sha == "dev" {
        return None;
    }

    for dir in git_lookup_dirs() {
        let output = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["log", "-1", "--format=%s", sha])
            .output()
            .ok()?;
        if !output.status.success() {
            continue;
        }
        let summary = String::from_utf8_lossy(&output.stdout);
        if let Some(cleaned) = clean_release_summary(&summary) {
            return Some(cleaned);
        }
    }
    None
}

fn git_lookup_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        dirs.push(current.clone());
        if let Some(parent) = current.parent() {
            dirs.push(parent.to_path_buf());
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
        }
    }
    dirs.push(PathBuf::from("/root/Elon"));
    dirs
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
            display_model: opt.display_label(),
            reasoning_effort: opt.reasoning_effort.clone(),
            reasoning_summary: opt.reasoning_summary.clone(),
            verbosity: opt.verbosity.clone(),
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
    headers: HeaderMap,
    Json(req): Json<ImageGenerateRequest>,
) -> Result<Json<ImageGenerateResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user = auth_from_headers(&state, &headers).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "未登录"})),
        )
    })?;
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
    let image_model = state
        .image_model
        .as_ref()
        .map(|cfg| cfg.model.clone())
        .unwrap_or_else(|| "image".to_string());
    let image_key = crate::billing_lifecycle::new_compute_call_id("image_generate_api");
    let mut image_billing_call = crate::compute_usage::reserve_image_generation(
        &state.store,
        &user.id,
        &image_key,
        "image_generate_api",
        &image_model,
        prompt,
    )
    .map_err(|msg| {
        (
            StatusCode::PAYMENT_REQUIRED,
            Json(serde_json::json!({"error": msg})),
        )
    })?;

    match crate::image_generation::generate_text_to_image(&state, prompt).await {
        Ok(image) => {
            crate::compute_usage::record_image_generation_with_key(
                &state.store,
                &user.id,
                "image_generate_api",
                &image_model,
                prompt,
                Some(image_billing_call.key()),
            );
            image_billing_call.mark_settled();
            Ok(Json(ImageGenerateResponse {
                job_id: image.job_id,
                url: image.url,
                revised_prompt: image.revised_prompt,
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}
