/// 用户端 AI 代理配置 API
///
/// 每个用户可以通过 APK 的「设置」页面配置自己专属的 AI 代理。
/// 当前默认 `AI_CODEX_CLI_ONLY=true`，用户侧会被锁定到 Codex CLI；
/// 预设 API 代理仍需显式关闭该模式后才可选择；用户自带 API Key
/// 可通过 `AI_USER_BYOK_API_ENABLED=true` 作为显式例外。
///
/// 路由（无需管理员权限，user_id 即身份标识）：
///   GET /api/user/:user_id/agent   → 获取当前配置 + 可用全局代理列表
///   PUT /api/user/:user_id/agent   → 保存配置
///   POST /api/user/:user_id/agent/test → 测试自定义 API 模型连通性
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::project_auth::{auth_from_headers, json_error};
use crate::types::{AiBackend, AiCliOption, AppState, UserAgentConfig};
use crate::user_agent_probe::{
    normalize_api_base, probe_development_agent_capability, probe_openai_compatible_api,
    resolve_probe_config, UserAgentProbeConfig, UserAgentProbeRequest,
};
use crate::user_agent_readiness::build_user_agent_rag_readiness;
use crate::user_agent_secrets::user_byok_api_enabled;

#[derive(Deserialize)]
pub struct UpdateMyPresenceRequest {
    pub status: Option<String>,
    pub custom_status: Option<String>,
    pub activity: Option<String>,
}

/// GET /api/me/presence — 当前用户展示在线状态
pub async fn get_my_presence(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(err) => return json_error(StatusCode::UNAUTHORIZED, err.to_string()),
    };
    match state.store.user_presence_settings(&user.id) {
        Ok(presence) => Json(presence).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

/// PATCH /api/me/presence — 设置在线/离开/勿扰/隐身与自定义展示文案
pub async fn update_my_presence(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpdateMyPresenceRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(err) => return json_error(StatusCode::UNAUTHORIZED, err.to_string()),
    };
    let current = match state.store.user_presence_settings(&user.id) {
        Ok(presence) => presence,
        Err(err) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    };
    let status = req.status.as_deref().unwrap_or(&current.status);
    match state.store.set_user_presence_settings(
        &user.id,
        status,
        req.custom_status.as_deref(),
        req.activity.as_deref(),
    ) {
        Ok(presence) => Json(presence).into_response(),
        Err(err) => json_error(StatusCode::BAD_REQUEST, err.to_string()),
    }
}

/// 获取用户的 AI 代理配置（同时返回可选的全局代理列表）
pub async fn get_user_agent(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Response {
    let workspace = state.get_user_workspace(&user_id);
    let config = UserAgentConfig::load(&workspace).unwrap_or_default();
    let byok_api_enabled = user_byok_api_enabled();
    let mut response_config = config.clone();
    response_config.api_key = None;
    response_config.api_key_encrypted = None;
    if state.ai_cli.codex_cli_only {
        response_config.use_agent = response_config
            .use_agent
            .as_deref()
            .filter(|name| is_cli_selection(&state, name))
            .map(ToOwned::to_owned);
        if !byok_api_enabled {
            response_config.api_base = None;
            response_config.model = None;
            response_config.embedding_model = None;
        }
    }

    // 列出管理员配置的全局代理名称（供 APK 下拉选择）
    let global = state.agents_config.read().await;
    let mut available_agents: Vec<serde_json::Value> = if state.ai_cli.codex_cli_only {
        Vec::new()
    } else {
        global
            .agents
            .values()
            .map(|a| {
                let (provider, label) = agent_display_meta(&a.name, &a.model);
                serde_json::json!({
                    "name": a.name,
                    "model": a.model,
                    "embedding_model": a.embedding_model.as_deref(),
                    "api_base": a.api_base,
                    "backend": "api",
                    "provider": provider,
                    "label": label,
                })
            })
            .collect()
    };
    if state.ai_cli.enabled {
        available_agents.extend(state.ai_cli.options.iter().map(|opt| {
            serde_json::json!({
                "name": opt.id,
                "model": opt.model.as_deref().unwrap_or("default"),
                "display_model": opt.display_label(),
                "reasoning_effort": opt.reasoning_effort.as_deref(),
                "reasoning_summary": opt.reasoning_summary.as_deref(),
                "verbosity": opt.verbosity.as_deref(),
                "api_base": "local",
                "backend": "cli",
                "provider": opt.provider,
                "label": cli_option_display_label(opt),
            })
        }));
    }
    available_agents = dedupe_available_agents(available_agents);
    available_agents.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    let default_agent = if state.default_backend == AiBackend::LocalCli && state.ai_cli.enabled {
        state
            .ai_cli
            .default_option
            .as_deref()
            .unwrap_or("local_cli")
    } else {
        global.default_agent.as_str()
    };

    Json(serde_json::json!({
        "user_id": user_id,
        "config": response_config,
        "available_agents": available_agents,
        "default_agent": default_agent,
        "codex_cli_only": state.ai_cli.codex_cli_only,
        "user_byok_api_enabled": byok_api_enabled,
        "api_key_set": config.has_api_key_reference(),
        "rag_readiness": build_user_agent_rag_readiness(
            &config,
            state.ai_cli.codex_cli_only,
            byok_api_enabled
        ),
    }))
    .into_response()
}

/// 保存用户的 AI 代理配置请求体
#[derive(Deserialize)]
pub struct SetUserAgentReq {
    /// 选择的全局代理名（None 或 "" = 使用服务器默认）
    pub use_agent: Option<String>,
    /// 自定义 API 地址（空字符串 = 不覆盖）
    pub api_base: Option<String>,
    /// 自定义 API 密钥（空字符串 = 不覆盖）
    pub api_key: Option<String>,
    /// 自定义模型名（空字符串 = 不覆盖）
    pub model: Option<String>,
    /// 自定义 embedding 模型，例如 openai:text-embedding-3-small（空字符串 = 不覆盖）
    pub embedding_model: Option<String>,
    /// 昵称（可选）
    pub nickname: Option<String>,
}

/// 保存用户的 AI 代理配置
pub async fn set_user_agent(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(req): Json<SetUserAgentReq>,
) -> Response {
    let workspace = state.get_user_workspace(&user_id);
    let existing_config = UserAgentConfig::load(&workspace).unwrap_or_default();
    let byok_api_enabled = user_byok_api_enabled();

    // 校验：如果指定了全局代理名，必须存在
    let use_agent = req.use_agent.and_then(|s| {
        let s = s.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    });
    let api_base = match req.api_base.as_deref().map(str::trim) {
        Some(value) if value.is_empty() => None,
        Some(value) => match normalize_api_base(value) {
            Some(normalized) => Some(normalized),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "API 地址必须以 http:// 或 https:// 开头" })),
                )
                    .into_response();
            }
        },
        None => None,
    };
    let mut api_key = req.api_key.and_then(|s| {
        let s = s.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    });
    let model = req.model.and_then(|s| {
        let s = s.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    });
    let embedding_model = req.embedding_model.and_then(|s| {
        let s = s.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    });
    let custom_api_requested =
        api_base.is_some() || api_key.is_some() || model.is_some() || embedding_model.is_some();
    let mut api_key_encrypted = None;
    if api_key.is_none() && (api_base.is_some() || model.is_some() || embedding_model.is_some()) {
        api_key = existing_config.api_key.clone();
        api_key_encrypted = existing_config.api_key_encrypted.clone();
    }
    if (api_base.is_some() || model.is_some() || embedding_model.is_some())
        && api_key.is_none()
        && api_key_encrypted.is_some()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "已保存的 API 密钥当前无法解密，请重新填写 API Key 或检查服务器密钥配置" })),
        )
            .into_response();
    }
    if custom_api_requested && (api_base.is_none() || model.is_none()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "自定义 API 模型需要同时填写 API 地址和模型名称" })),
        )
            .into_response();
    }
    if (api_base.is_some() || model.is_some()) && api_key.is_none() && api_key_encrypted.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "自定义 API 模型需要填写 API 密钥；留空仅用于保留已保存的密钥" })),
        )
            .into_response();
    }

    if let Some(ref name) = use_agent {
        let global = state.agents_config.read().await;
        if is_cli_selection(&state, name) && !state.ai_cli.enabled {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "本地 AI CLI 未启用" })),
            )
                .into_response();
        }
        if !is_cli_selection(&state, name) && !global.agents.contains_key(name.as_str()) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("代理 '{}' 不存在，可用代理: {:?}",
                        name,
                        global.agents.keys().collect::<Vec<_>>()
                    )
                })),
            )
                .into_response();
        }
    }
    if state.ai_cli.codex_cli_only
        && (use_agent
            .as_deref()
            .map(|name| !is_cli_selection(&state, name))
            .unwrap_or(false)
            || (custom_api_requested && !byok_api_enabled))
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "当前已锁定使用 Codex CLI，暂不允许切换到其他 AI 代理；如需用户自带 API Key，请开启 AI_USER_BYOK_API_ENABLED"
            })),
        )
            .into_response();
    }

    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    let mut cfg = UserAgentConfig {
        use_agent,
        api_base,
        api_key,
        api_key_encrypted,
        model,
        embedding_model,
        nickname: req.nickname.and_then(|s| {
            let s = s.trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }),
        updated_at: Some(now),
        ..Default::default()
    };

    let capability_result = if custom_api_requested {
        let probe_cfg = {
            let global = state.agents_config.read().await;
            match cfg.resolve(&global) {
                Some(agent) => UserAgentProbeConfig {
                    api_base: agent.api_base,
                    api_key: agent.api_key,
                    model: agent.model,
                },
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "自定义 API 模型无法解析为可用代理，请检查 API 地址、密钥和模型名称"
                        })),
                    )
                        .into_response();
                }
            }
        };

        match probe_development_agent_capability(&state.http_client, &probe_cfg).await {
            Ok(result) => {
                cfg.remember_capability_probe(&result, cfg.updated_at.clone().unwrap_or_default());
                Some(result)
            }
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };
    if !custom_api_requested {
        cfg.clear_capability_probe();
    }

    if let Err(e) = cfg.save(&workspace) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("保存失败: {}", e) })),
        )
            .into_response();
    }

    tracing::info!("用户 '{}' 更新了 AI 代理配置: {:?}", user_id, cfg.use_agent);
    Json(serde_json::json!({
        "ok": true,
        "tool_call_ok": capability_result.as_ref().map(|result| result.tool_call_ok),
        "capability": capability_result.as_ref().map(|result| result.capability.clone()),
        "warning": capability_result.and_then(|result| result.warning),
        "rag_readiness": build_user_agent_rag_readiness(
            &cfg,
            state.ai_cli.codex_cli_only,
            byok_api_enabled
        ),
    }))
    .into_response()
}

/// 测试用户自定义 API 模型连通性，不保存 API Key。
pub async fn test_user_agent(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(req): Json<UserAgentProbeRequest>,
) -> Response {
    if state.ai_cli.codex_cli_only && !user_byok_api_enabled() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "当前已锁定使用 Codex CLI，暂不允许测试自定义 API 模型"
            })),
        )
            .into_response();
    }

    let workspace = state.get_user_workspace(&user_id);
    let existing_config = UserAgentConfig::load(&workspace).unwrap_or_default();
    let cfg = match resolve_probe_config(req, &existing_config) {
        Ok(cfg) => cfg,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    match probe_openai_compatible_api(&state.http_client, &cfg).await {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "api_base": result.api_base,
            "model": result.model,
            "latency_ms": result.latency_ms,
            "sample": result.sample,
            "tool_call_ok": result.tool_call_ok,
            "tool_call_name": result.tool_call_name,
            "capability": result.capability,
            "warning": result.warning,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

fn is_cli_selection(state: &AppState, name: &str) -> bool {
    is_cli_alias(name) || state.ai_cli.has_option(name)
}

fn is_cli_alias(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "codex" | "codex_cli" | "copilot" | "copilot_cli" | "cli" | "local" | "local_cli"
    )
}

/// 将代理名转换为 (provider, 用户可见模型名)。
/// - `copilot:gpt-4o`  → ("copilot", "GPT-4o")
/// - `openai`          → ("openai", "GPT-4o")
fn agent_display_meta(name: &str, model: &str) -> (String, String) {
    if let Some(model_id) = name.strip_prefix("copilot:") {
        (
            "copilot".to_string(),
            copilot_model_friendly_name(model_id).to_string(),
        )
    } else {
        (name.to_string(), direct_model_label(model, name))
    }
}

fn cli_option_display_label(option: &AiCliOption) -> String {
    option.display_label()
}

fn dedupe_available_agents(agents: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let mut deduped: Vec<serde_json::Value> = Vec::new();
    for agent in agents {
        let key = available_agent_key(&agent);
        if key.is_empty() {
            deduped.push(agent);
            continue;
        }

        if let Some(existing) = deduped
            .iter_mut()
            .find(|existing| available_agent_key(existing) == key)
        {
            if available_agent_priority(&agent) > available_agent_priority(existing) {
                *existing = agent;
            }
        } else {
            deduped.push(agent);
        }
    }
    deduped
}

fn available_agent_key(agent: &serde_json::Value) -> String {
    agent["name"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn available_agent_priority(agent: &serde_json::Value) -> u8 {
    match agent["backend"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "cli" => 2,
        "api" => 1,
        _ => 0,
    }
}

fn direct_model_label(model: &str, fallback: &str) -> String {
    let model = model.trim();
    if model.is_empty() || model.eq_ignore_ascii_case("default") {
        strip_provider_prefix(fallback)
    } else {
        copilot_model_friendly_name(model).to_string()
    }
}

fn strip_provider_prefix(label: &str) -> String {
    let label = label.trim();
    if let Some((_, model)) = label.rsplit_once('/') {
        let model = model.trim();
        if !model.is_empty() {
            return model.to_string();
        }
    }
    if let Some(start) = label.rfind('[') {
        if label.ends_with(']') && start + 1 < label.len() - 1 {
            let model = label[start + 1..label.len() - 1].trim();
            if !model.is_empty() {
                return model.to_string();
            }
        }
    }
    label.to_string()
}

/// 将 Copilot / GitHub Models 的模型 ID 映射为用户可读名称。
fn copilot_model_friendly_name(model: &str) -> &str {
    match model {
        // GPT 系列
        "gpt-4o" => "GPT-4o",
        "gpt-4o-mini" => "GPT-4o mini",
        "gpt-4.1" => "GPT-4.1",
        "gpt-4.1-mini" => "GPT-4.1 mini",
        "gpt-4.1-nano" => "GPT-4.1 nano",
        "gpt-4.5-preview" => "GPT-4.5 Preview",
        "gpt-5" | "gpt-5.0" => "GPT-5",
        "gpt-5-mini" => "GPT-5 mini",
        "gpt-5.3-codex" => "GPT-5.3 Codex",
        "gpt-5.4" => "GPT-5.4",
        "gpt-5.4-mini" => "GPT-5.4 mini",
        "gpt-5.5" => "GPT-5.5",
        // Claude 系列（Copilot CLI 实际使用的 model ID 格式：claude-{role}-{major}.{minor}）
        "claude-haiku-4.5" => "Claude Haiku 4.5",
        "claude-sonnet-4" | "claude-sonnet-4.0" => "Claude Sonnet 4",
        "claude-sonnet-4.5" | "claude-3-sonnet-4-5" => "Claude Sonnet 4.5",
        "claude-sonnet-4.6" => "Claude Sonnet 4.6",
        "claude-opus-4" | "claude-opus-4.0" => "Claude Opus 4",
        "claude-opus-4.7" => "Claude Opus 4.7",
        "claude-opus-4.8" => "Claude Opus 4.8",
        // 旧版 Claude（向后兼容）
        "claude-3.5-sonnet" | "claude-3-5-sonnet-20241022" => "Claude 3.5 Sonnet",
        "claude-3.7-sonnet" | "claude-3-7-sonnet-20250219" => "Claude 3.7 Sonnet",
        // 推理模型
        "o1" => "o1",
        "o1-mini" => "o1 mini",
        "o1-preview" => "o1 preview",
        "o3" => "o3",
        "o3-mini" => "o3 mini",
        "o4-mini" => "o4 mini",
        // Gemini 系列
        "gemini-2.0-flash" | "gemini-2.0-flash-001" => "Gemini 2.0 Flash",
        "gemini-2.5-pro" | "gemini-2.5-pro-preview" => "Gemini 2.5 Pro",
        "gemini-2.5-flash" | "gemini-2.5-flash-preview" => "Gemini 2.5 Flash",
        "gemini-3.1-pro-preview" => "Gemini 3.1 Pro",
        "gemini-3.5-flash" => "Gemini 3.5 Flash",
        // 混元
        "hunyuan-turbo" => "混元 Turbo",
        "hunyuan-2.0-instruct-20251111" => "混元 2.0 Instruct",
        "hy-image-v3.0" => "混元生图 3.0",
        other => other,
    }
}

/// GET /api/users/:user_id/avatar
/// 公开接口——返回用户头像（PNG/JPEG 原始字节，无需认证）
pub async fn get_user_avatar(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Response {
    let data_url = match state.store.get_user_avatar(&user_id) {
        Ok(Some(v)) => v,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "用户没有设置头像"),
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    // 解析 "data:<mime>;base64,<data>"
    let (mime, b64) = match data_url.split_once(',') {
        Some((prefix, data)) => {
            let mime = prefix
                .strip_prefix("data:")
                .and_then(|s| s.split(';').next())
                .unwrap_or("image/png")
                .to_string();
            (mime, data)
        }
        None => ("image/png".to_string(), data_url.as_str()),
    };

    use base64::{engine::general_purpose, Engine};
    let bytes = match general_purpose::STANDARD.decode(b64.trim()) {
        Ok(b) => b,
        Err(_) => return json_error(StatusCode::BAD_REQUEST, "头像数据无效"),
    };

    (StatusCode::OK, [(header::CONTENT_TYPE, mime)], bytes).into_response()
}

#[derive(Deserialize)]
pub struct PutAvatarRequest {
    pub avatar_data_url: String,
}

/// PUT /api/me/avatar
/// 登录用户上传自己的头像（存为 data URL，限制 500KB）
pub async fn put_my_avatar(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<PutAvatarRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if req.avatar_data_url.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "头像不能为空");
    }
    const MAX_AVATAR_LEN: usize = 700_000; // ~500 KB base64
    if req.avatar_data_url.len() > MAX_AVATAR_LEN {
        return json_error(StatusCode::BAD_REQUEST, "头像数据太大（最大约 500 KB）");
    }
    match state.store.save_user_avatar(&user.id, &req.avatar_data_url) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CliPromptMode;

    #[test]
    fn copilot_agent_label_shows_provider_and_model() {
        let (provider, label) = agent_display_meta("copilot:gpt-4o", "gpt-4o");
        assert_eq!(provider, "copilot");
        assert_eq!(label, "GPT-4o");
    }

    #[test]
    fn generic_agent_label_prefers_model_over_provider() {
        let (provider, label) = agent_display_meta("openai", "gpt-4o-mini");
        assert_eq!(provider, "openai");
        assert_eq!(label, "GPT-4o mini");
    }

    #[test]
    fn cli_label_keeps_provider_identity() {
        let option = AiCliOption {
            id: "codex:gpt-5".into(),
            label: "Codex CLI / gpt-5".into(),
            provider: "codex".into(),
            model: Some("gpt-5".into()),
            reasoning_effort: None,
            reasoning_summary: None,
            verbosity: None,
            bin: "codex".into(),
            args: Vec::new(),
            prompt_mode: CliPromptMode::Arg,
            timeout_secs: 1800,
        };
        assert_eq!(cli_option_display_label(&option), "GPT-5");
    }

    #[test]
    fn available_agents_dedupe_prefers_cli_for_same_name() {
        let agents = vec![
            serde_json::json!({
                "name": "copilot:gpt-4o",
                "backend": "api",
                "provider": "copilot",
                "model": "gpt-4o",
                "label": "GPT-4o"
            }),
            serde_json::json!({
                "name": "copilot:gpt-4o",
                "backend": "cli",
                "provider": "copilot",
                "model": "gpt-4o",
                "label": "GPT-4o"
            }),
            serde_json::json!({
                "name": "openai",
                "backend": "api",
                "provider": "openai",
                "model": "gpt-4o",
                "label": "GPT-4o"
            }),
        ];

        let deduped = dedupe_available_agents(agents);

        assert_eq!(deduped.len(), 2);
        let copilot = deduped
            .iter()
            .find(|agent| agent["name"].as_str() == Some("copilot:gpt-4o"))
            .expect("copilot option should remain");
        assert_eq!(copilot["backend"].as_str(), Some("cli"));
        assert!(deduped
            .iter()
            .any(|agent| agent["name"].as_str() == Some("openai")));
    }
}
