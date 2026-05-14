/// 用户端 AI 代理配置 API
///
/// 每个用户可以通过 APK 的「设置」页面配置自己专属的 AI 代理：
///   - 选择服务器预设的代理（OpenAI / DeepSeek / Hunyuan ...）
///   - 或填写自己的 API Key + 地址 + 模型（完全自定义）
///
/// 路由（无需管理员权限，user_id 即身份标识）：
///   GET /api/user/:user_id/agent   → 获取当前配置 + 可用全局代理列表
///   PUT /api/user/:user_id/agent   → 保存配置

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::types::{AppState, UserAgentConfig};

/// 获取用户的 AI 代理配置（同时返回可选的全局代理列表）
pub async fn get_user_agent(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Response {
    let workspace = state.get_user_workspace(&user_id);
    let config = UserAgentConfig::load(&workspace).unwrap_or_default();

    // 列出管理员配置的全局代理名称（供 APK 下拉选择）
    let global = state.agents_config.read().await;
    let mut available_agents: Vec<serde_json::Value> = global
        .agents
        .values()
        .map(|a| {
            serde_json::json!({
                "name": a.name,
                "model": a.model,
                "api_base": a.api_base,
            })
        })
        .collect();
    available_agents.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    Json(serde_json::json!({
        "user_id": user_id,
        "config": config,
        "available_agents": available_agents,
        "default_agent": global.default_agent,
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

    // 校验：如果指定了全局代理名，必须存在
    let use_agent = req.use_agent.and_then(|s| {
        let s = s.trim().to_string();
        if s.is_empty() { None } else { Some(s) }
    });

    if let Some(ref name) = use_agent {
        let global = state.agents_config.read().await;
        if !global.agents.contains_key(name.as_str()) {
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

    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    let cfg = UserAgentConfig {
        use_agent,
        api_base:  req.api_base.and_then(|s| { let s = s.trim().to_string(); if s.is_empty() { None } else { Some(s) } }),
        api_key:   req.api_key.and_then(|s|  { let s = s.trim().to_string(); if s.is_empty() { None } else { Some(s) } }),
        model:     req.model.and_then(|s|    { let s = s.trim().to_string(); if s.is_empty() { None } else { Some(s) } }),
        nickname:  req.nickname.and_then(|s| { let s = s.trim().to_string(); if s.is_empty() { None } else { Some(s) } }),
        updated_at: Some(now),
    };

    if let Err(e) = cfg.save(&workspace) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("保存失败: {}", e) })),
        )
            .into_response();
    }

    tracing::info!("用户 '{}' 更新了 AI 代理配置: {:?}", user_id, cfg.use_agent);
    Json(serde_json::json!({ "ok": true })).into_response()
}
