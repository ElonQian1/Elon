/// 用户端 AI 代理配置 API
///
/// 每个用户可以通过 APK 的「设置」页面配置自己专属的 AI 代理。
/// 当前默认 `AI_CODEX_CLI_ONLY=true`，用户侧会被锁定到 Codex CLI；
/// 显式关闭该模式后，才恢复预设 API 代理或自定义 API 模型选择。
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

use crate::types::{AiBackend, AppState, UserAgentConfig};

/// 获取用户的 AI 代理配置（同时返回可选的全局代理列表）
pub async fn get_user_agent(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Response {
    let workspace = state.get_user_workspace(&user_id);
    let config = UserAgentConfig::load(&workspace).unwrap_or_default();
    let mut response_config = config.clone();
    if state.ai_cli.codex_cli_only {
        response_config.use_agent = None;
        response_config.api_base = None;
        response_config.api_key = None;
        response_config.model = None;
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
                let (provider, label) = copilot_agent_meta(&a.name, &a.model);
                serde_json::json!({
                    "name": a.name,
                    "model": a.model,
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
                "api_base": "local",
                "backend": "cli",
                "provider": opt.provider,
                "label": opt.label,
            })
        }));
    }
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
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    });
    let api_base = req.api_base.and_then(|s| {
        let s = s.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    });
    let api_key = req.api_key.and_then(|s| {
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
            || api_base.is_some()
            || api_key.is_some()
            || model.is_some())
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "当前已锁定使用 Codex CLI，暂不允许切换到其他 AI 代理或自定义 API 模型"
            })),
        )
            .into_response();
    }

    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    let cfg = UserAgentConfig {
        use_agent,
        api_base,
        api_key,
        model,
        nickname: req.nickname.and_then(|s| {
            let s = s.trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }),
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

fn is_cli_selection(state: &AppState, name: &str) -> bool {
    is_cli_alias(name) || state.ai_cli.has_option(name)
}

fn is_cli_alias(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "codex" | "codex_cli" | "cli" | "local" | "local_cli"
    )
}

/// 将 Copilot 代理名 / 普通代理名 转换为 (provider, 友好 label)。
/// - `copilot:gpt-4o`  → ("copilot", "Copilot / GPT-4o")
/// - `openai`          → ("openai", "openai / gpt-4o")
fn copilot_agent_meta(name: &str, model: &str) -> (String, String) {
    if let Some(model_id) = name.strip_prefix("copilot:") {
        (
            "copilot".to_string(),
            format!("Copilot / {}", copilot_model_friendly_name(model_id)),
        )
    } else {
        (name.to_string(), format!("{} / {}", name, model))
    }
}

/// 将 Copilot / GitHub Models 的模型 ID 映射为用户可读名称。
fn copilot_model_friendly_name(model: &str) -> &str {
    match model {
        "gpt-4o" => "GPT-4o",
        "gpt-4o-mini" => "GPT-4o mini",
        "gpt-4.1" => "GPT-4.1",
        "gpt-4.5-preview" => "GPT-4.5 Preview",
        "claude-3.5-sonnet" | "claude-3-5-sonnet-20241022" => "Claude 3.5 Sonnet",
        "claude-3.7-sonnet" | "claude-3-7-sonnet-20250219" => "Claude 3.7 Sonnet",
        "claude-sonnet-4" | "claude-sonnet-4-5" => "Claude Sonnet 4",
        "o1" => "o1",
        "o1-mini" => "o1 mini",
        "o1-preview" => "o1 preview",
        "o3" => "o3",
        "o3-mini" => "o3 mini",
        "gemini-2.0-flash" | "gemini-2.0-flash-001" => "Gemini 2.0 Flash",
        "gemini-2.5-pro" | "gemini-2.5-pro-preview" => "Gemini 2.5 Pro",
        other => other,
    }
}
