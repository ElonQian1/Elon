use crate::{client_protocol, types::AppState};
use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

/// 健康检查
pub async fn health() -> &'static str {
    "OK"
}

/// REST 接口（APK 也可以用这个，不需要 WebSocket）
#[derive(serde::Deserialize)]
pub struct ChatRequest {
    /// 用户 ID（决定工作区目录，不同用户的项目相互隔离）
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub message: String,
    /// 可选，指定 AI 代理名称（openai / deepseek / claude）
    pub agent: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub apk_url: Option<String>,
    pub image_url: Option<String>,
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
    Json(ServerVersionResponse {
        service: "elon-server",
        status: "ok",
        version_name: env!("CARGO_PKG_VERSION"),
        git_sha: option_env!("ELON_SERVER_GIT_SHA").unwrap_or("dev"),
    })
}

pub async fn chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Json<ChatResponse> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let state_clone = state.clone();
    let user_id = req.user_id.clone().unwrap_or_else(|| "default".into());
    let workspace_user_id = client_protocol::workspace_user_id(&user_id, req.project_id.as_deref());
    let msg = req.message.clone();
    let agent_name = req.agent.clone();

    tokio::spawn(async move {
        crate::agent::run(
            &user_id,
            &workspace_user_id,
            &msg,
            agent_name.as_deref(),
            &state_clone,
            tx,
        )
        .await;
    });

    let mut final_reply = String::new();
    let mut apk_url = None;
    let mut image_url = None;

    while let Some(raw) = rx.recv().await {
        if let Ok(ws_msg) = serde_json::from_str::<serde_json::Value>(&raw) {
            match ws_msg.get("type").and_then(|t| t.as_str()) {
                Some("done") => {
                    final_reply = ws_msg["message"].as_str().unwrap_or("完成").to_string();
                    apk_url = ws_msg["apk_url"].as_str().map(|s| s.to_string());
                    image_url = ws_msg["image_url"].as_str().map(|s| s.to_string());
                    break;
                }
                Some("error") => {
                    final_reply = ws_msg["message"].as_str().unwrap_or("发生错误").to_string();
                    break;
                }
                _ => {}
            }
        }
    }

    Json(ChatResponse {
        reply: final_reply,
        apk_url,
        image_url,
    })
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
