/// 管理后台模块：提供 Web UI 和 REST API 用于运行时配置 AI 代理参数
///
/// 路由：
///   GET  /admin                       → 管理页面 HTML
///   GET  /api/admin/agents            → 列出所有代理（key 脱敏）
///   POST /api/admin/agents            → 新增或更新代理
///   DELETE /api/admin/agents/:name    → 删除代理
///   POST /api/admin/default/:name     → 设置默认代理
///   GET  /api/admin/agents/:name/key  → 查看某代理的完整 API key
///
/// 鉴权：所有 API 需要请求头 `Authorization: Bearer <ADMIN_TOKEN>`
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::types::{AgentConfig, AppState};

// ─────────────────────────────────────────────
// 鉴权工具函数
// ─────────────────────────────────────────────

pub(crate) fn check_auth(headers: &HeaderMap, token: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim_start_matches("Bearer ").trim() == token)
        .unwrap_or(false)
}

fn auth_error() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "无效的管理员令牌，请在页面顶部输入正确的 ADMIN_TOKEN"})),
    )
        .into_response()
}

/// 将 API key 脱敏：仅显示前4个字符和后4个字符（按字符而非字节，兼容中文）
fn mask_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "••••••••".into();
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{}••••{}", head, tail)
}

// ─────────────────────────────────────────────
// 路由处理函数
// ─────────────────────────────────────────────

/// 返回管理后台 HTML 页面
pub async fn admin_page() -> Html<&'static str> {
    Html(crate::admin_html::ADMIN_HTML)
}

/// 列出所有 AI 代理配置（API key 脱敏）
pub async fn list_agents(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let config = state.agents_config.read().await;
    let mut agents: Vec<serde_json::Value> = config
        .agents
        .values()
        .map(|a| {
            serde_json::json!({
                "name": a.name,
                "api_base": a.api_base,
                "api_key_masked": mask_key(&a.api_key),
                "model": a.model,
                "is_default": a.name == config.default_agent,
            })
        })
        .collect();

    // 按名称排序，让 UI 稳定
    agents.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    Json(serde_json::json!({
        "agents": agents,
        "default_agent": config.default_agent,
    }))
    .into_response()
}

/// 新增或更新 AI 代理配置
#[derive(Deserialize)]
pub struct UpsertAgentReq {
    pub name: String,
    pub api_base: String,
    /// 传空字符串表示不修改现有密钥
    pub api_key: String,
    pub model: String,
    pub set_as_default: bool,
}

pub async fn upsert_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpsertAgentReq>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let name = req.name.to_lowercase().trim().to_string();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "代理名称不能为空"})),
        )
            .into_response();
    }
    // 只允许字母、数字、连字符
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "代理名称只能包含字母、数字、连字符(-_)"})),
        )
            .into_response();
    }

    let mut config = state.agents_config.write().await;

    // API key 为空时保留原有密钥
    let api_key = if req.api_key.trim().is_empty() {
        config
            .agents
            .get(&name)
            .map(|a| a.api_key.clone())
            .unwrap_or_default()
    } else {
        req.api_key.trim().to_string()
    };

    let is_new = !config.agents.contains_key(&name);
    config.agents.insert(
        name.clone(),
        AgentConfig {
            name: name.clone(),
            api_base: req.api_base.trim().to_string(),
            api_key,
            model: req.model.trim().to_string(),
        },
    );

    if req.set_as_default || (is_new && config.agents.len() == 1) {
        config.default_agent = name.clone();
    }

    if let Err(e) = config.save_to_file(&state.config_path) {
        tracing::error!("保存代理配置到文件失败: {}", e);
    }

    tracing::info!(
        "管理后台：{} 代理 '{}'",
        if is_new { "新增" } else { "更新" },
        name
    );

    Json(serde_json::json!({"ok": true, "name": name})).into_response()
}

/// 删除 AI 代理
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let mut config = state.agents_config.write().await;

    if config.agents.len() <= 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "至少需要保留一个 AI 代理，无法删除"})),
        )
            .into_response();
    }

    if config.agents.remove(&name).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "代理不存在"})),
        )
            .into_response();
    }

    // 如果删掉的是默认代理，自动切换到第一个
    if config.default_agent == name {
        config.default_agent = config.agents.keys().next().unwrap().clone();
        tracing::info!("默认代理已切换为 '{}'", config.default_agent);
    }

    if let Err(e) = config.save_to_file(&state.config_path) {
        tracing::error!("保存代理配置到文件失败: {}", e);
    }

    tracing::info!("管理后台：删除代理 '{}'", name);
    Json(serde_json::json!({"ok": true})).into_response()
}

/// 设置默认代理
pub async fn set_default_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let mut config = state.agents_config.write().await;

    if !config.agents.contains_key(&name) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "代理不存在"})),
        )
            .into_response();
    }

    config.default_agent = name.clone();

    if let Err(e) = config.save_to_file(&state.config_path) {
        tracing::error!("保存代理配置到文件失败: {}", e);
    }

    tracing::info!("管理后台：默认代理设为 '{}'", name);
    Json(serde_json::json!({"ok": true})).into_response()
}

/// 查看指定代理的完整 API key（需要 Bearer token）
pub async fn get_agent_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    let config = state.agents_config.read().await;
    match config.agents.get(&name) {
        Some(a) => Json(serde_json::json!({"name": a.name, "api_key": a.api_key})).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "代理不存在"})),
        )
            .into_response(),
    }
}

/// 列出所有用户及其 AI 代理配置（仅管理员）
pub async fn list_users(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    match state.store.list_users() {
        Ok(users) => {
            Json(serde_json::json!({ "users": users, "total": users.len() })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateUserReq {
    pub account: String,
    pub password: String,
    pub nickname: Option<String>,
    pub role: Option<String>,
}

/// 管理员创建可登录用户
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateUserReq>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    match state.store.create_user(
        &req.account,
        &req.password,
        req.nickname.as_deref(),
        req.role.as_deref(),
    ) {
        Ok(user) => Json(serde_json::json!({ "ok": true, "user": user })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// 列出所有项目及其创建者、工作区路径、最新 APK 信息（仅管理员）
pub async fn list_projects(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }

    match state.store.list_all_projects_admin() {
        Ok(projects) => {
            let enriched: Vec<serde_json::Value> = projects
                .into_iter()
                .map(|p| {
                    let workspace_dir = state
                        .resolve_project_workspace(&p.workspace_key, p.workspace_path.as_deref())
                        .to_string_lossy()
                        .to_string();
                    serde_json::json!({
                        "id": p.id,
                        "name": p.name,
                        "workspace_key": p.workspace_key,
                        "workspace_dir": workspace_dir,
                        "node_id": p.node_id,
                        "source_type": p.source_type,
                        "template": p.template,
                        "status": p.status,
                        "created_by_id": p.created_by_id,
                        "created_by_account": p.created_by_account,
                        "last_task_status": p.last_task_status,
                        "last_apk_url": p.last_apk_url,
                        "last_device_name": p.last_device_name,
                        "last_apk_version": p.last_apk_version,
                        "updated_at": p.updated_at,
                    })
                })
                .collect();
            Json(serde_json::json!({ "projects": enriched, "total": enriched.len() }))
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// 列出所有活跃 Session（未过期的登录记录，即在线设备），仅管理员可见
pub async fn list_sessions(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }
    match state.store.list_active_sessions_admin() {
        Ok(sessions) => Json(serde_json::json!({
            "sessions": sessions,
            "total": sessions.len(),
            "require_login": state.require_login,
            "min_apk_version_code": state.min_apk_version_code,
            "platform_apk_url": format!("{}/app/ElonSpeed-latest.apk", state.public_url),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// 列出某项目下所有会话（对话 ID、用户、消息数等），仅管理员可见
pub async fn list_project_conversations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return auth_error();
    }
    match state
        .store
        .list_conversations_for_project_admin(&project_id)
    {
        Ok(convs) => Json(serde_json::json!({
            "conversations": convs,
            "total": convs.len(),
            "project_id": project_id,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
