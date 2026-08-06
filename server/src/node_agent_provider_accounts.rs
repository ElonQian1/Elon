//! Unified local account control plane for official AI CLI integrations.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

use crate::{
    node_agent_cli_probe::{LocalCliProbeSnapshot, LocalCliToolStatus},
    node_agent_provider_auth_runtime::ProviderLoginAttempt,
    NodeRuntime,
};

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/ai-provider-accounts", get(list_handler))
        .route(
            "/api/ai-provider-accounts/:provider_id/login",
            post(start_login_handler),
        )
        .route(
            "/api/ai-provider-accounts/:provider_id/logins/:login_id",
            get(login_status_handler),
        )
        .route(
            "/api/ai-provider-accounts/:provider_id/logins/:login_id/cancel",
            post(cancel_login_handler),
        )
        .route(
            "/api/ai-provider-accounts/:provider_id/logout",
            post(logout_handler),
        )
}

/// The provider account surface is callable either by the trusted local PC UI
/// or through `/api/pc-relay/:agent_id/...` with the exact owner user token.
/// The relay never receives or forwards the local-admin token.
pub(crate) async fn require_provider_account_access(
    State(runtime): State<Arc<NodeRuntime>>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    let local = crate::node_agent_local_admin::verify_local_admin_request(
        &headers,
        runtime.local_admin_token(),
        &runtime.cloud_http_url(),
    )
    .is_ok();
    let owner = bearer_token(&headers)
        .zip(runtime.user_token().await)
        .is_some_and(|(actual, expected)| actual == expected);
    if local || owner {
        return next.run(request).await;
    }
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "ok": false,
            "error": "仅允许本机管理页或当前节点所有者管理 AI 厂商账号。"
        })),
    )
        .into_response()
}

async fn list_handler(State(runtime): State<Arc<NodeRuntime>>) -> Json<Value> {
    runtime.ensure_cli_probe_background(false).await;
    let probe = runtime.cached_cli_probe().await;
    let codex_attempt = runtime.provider_auth.latest("codex_cli").await;
    let gemini_attempt = runtime.provider_auth.latest("gemini_cli").await;
    let claude_attempt = runtime.provider_auth.latest("claude_cli").await;
    let copilot_attempt = runtime.provider_auth.latest("copilot_cli").await;
    Json(accounts_payload(
        &probe,
        codex_attempt,
        gemini_attempt,
        claude_attempt,
        copilot_attempt,
    ))
}

#[derive(Deserialize, Default)]
struct StartLoginRequest {
    flow: Option<String>,
}

async fn start_login_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(raw_provider_id): Path<String>,
    Json(request): Json<StartLoginRequest>,
) -> (StatusCode, Json<Value>) {
    let Some(provider_id) = normalize_provider_id(&raw_provider_id) else {
        return provider_error(
            StatusCode::CONFLICT,
            reserved_provider_message(&raw_provider_id),
        );
    };
    let probe = runtime.refresh_cli_probe_now().await;
    let Some(program) = runnable_program(&probe, provider_id) else {
        return provider_error(
            StatusCode::BAD_REQUEST,
            &format!("{} CLI 尚未安装或无法运行。", provider_label(provider_id)),
        );
    };
    let result = match provider_id {
        "codex_cli" => {
            let flow = request
                .flow
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("device_code");
            runtime
                .provider_auth
                .start_codex_login(&program, flow)
                .await
        }
        "gemini_cli" => {
            if request
                .flow
                .as_deref()
                .is_some_and(|flow| !matches!(flow.trim(), "" | "agent"))
            {
                return provider_error(
                    StatusCode::BAD_REQUEST,
                    "Gemini ACP 公开登录目前只支持由 Gemini CLI 在 Win 端接管浏览器流程。",
                );
            }
            runtime.provider_auth.start_gemini_login(&program).await
        }
        "claude_cli" => runtime.provider_auth.start_claude_login(&program).await,
        "copilot_cli" => runtime.provider_auth.start_copilot_login(&program).await,
        _ => unreachable!(),
    };
    match result {
        Ok(attempt) => (
            StatusCode::ACCEPTED,
            Json(json!({
                "ok": true,
                "schema": "elon.ai_provider_login.v1",
                "attempt": attempt,
            })),
        ),
        Err(error) => provider_error(StatusCode::BAD_GATEWAY, &error.to_string()),
    }
}

async fn login_status_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((raw_provider_id, login_id)): Path<(String, String)>,
) -> (StatusCode, Json<Value>) {
    let Some(provider_id) = normalize_provider_id(&raw_provider_id) else {
        return provider_error(StatusCode::NOT_FOUND, "未知 AI 厂商账号接口。 ");
    };
    let Some(attempt) = runtime.provider_auth.get(provider_id, &login_id).await else {
        return provider_error(StatusCode::NOT_FOUND, "找不到该登录任务。 ");
    };
    if attempt.is_terminal() {
        runtime.refresh_cli_probe_now().await;
    }
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "schema": "elon.ai_provider_login.v1",
            "attempt": attempt,
        })),
    )
}

async fn cancel_login_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((raw_provider_id, login_id)): Path<(String, String)>,
) -> (StatusCode, Json<Value>) {
    let Some(provider_id) = normalize_provider_id(&raw_provider_id) else {
        return provider_error(StatusCode::NOT_FOUND, "未知 AI 厂商账号接口。 ");
    };
    match runtime.provider_auth.cancel(provider_id, &login_id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({"ok":true,"message":"已请求取消登录。"})),
        ),
        Err(error) => provider_error(StatusCode::NOT_FOUND, &error.to_string()),
    }
}

async fn logout_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(raw_provider_id): Path<String>,
) -> (StatusCode, Json<Value>) {
    let Some(provider_id) = normalize_provider_id(&raw_provider_id) else {
        return provider_error(
            StatusCode::CONFLICT,
            reserved_provider_message(&raw_provider_id),
        );
    };
    let probe = runtime.refresh_cli_probe_now().await;
    let Some(program) = runnable_program(&probe, provider_id) else {
        return provider_error(StatusCode::BAD_REQUEST, "官方 CLI 尚未安装或无法运行。 ");
    };
    let result = match provider_id {
        "codex_cli" => runtime.provider_auth.logout_codex(&program).await,
        "gemini_cli" => runtime.provider_auth.logout_gemini(&program).await,
        "claude_cli" => runtime.provider_auth.logout_claude(&program).await,
        "copilot_cli" => Err(anyhow::anyhow!(
            "GitHub Copilot CLI 目前只公开了交互式 /logout；请在 Copilot CLI 中执行 /logout。"
        )),
        _ => unreachable!(),
    };
    match result {
        Ok(()) => {
            let probe = runtime.refresh_cli_probe_now().await;
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": format!("已通过官方协议退出 {}。", provider_label(provider_id)),
                    "provider": cli_tool(&probe, provider_id),
                })),
            )
        }
        Err(error) => provider_error(StatusCode::CONFLICT, &error.to_string()),
    }
}

pub(crate) fn accounts_payload(
    probe: &LocalCliProbeSnapshot,
    codex_attempt: Option<ProviderLoginAttempt>,
    gemini_attempt: Option<ProviderLoginAttempt>,
    claude_attempt: Option<ProviderLoginAttempt>,
    copilot_attempt: Option<ProviderLoginAttempt>,
) -> Value {
    json!({
        "ok": true,
        "schema": "elon.ai_provider_accounts.v1",
        "transport": {
            "local": "/api/ai-provider-accounts",
            "ownerRelay": "/api/pc-relay/{agent_id}/api/ai-provider-accounts"
        },
        "providers": [
            {
                "id": "codex_cli",
                "vendor": "openai",
                "label": "Codex CLI",
                "surface": "cli_agent",
                "protocol": "codex_app_server_jsonrpc",
                "implementation_state": "available",
                "official_login": true,
                "login_flows": ["device_code", "browser"],
                "remote_login_supported": true,
                "logout_supported": true,
                "credential_owner": "codex_cli",
                "credential_storage": "official_cli_store_with_explicit_codex_vault_backup",
                "cli": cli_tool(probe, "codex_cli"),
                "active_login": codex_attempt,
            },
            {
                "id": "gemini_cli",
                "vendor": "google",
                "label": "Gemini CLI",
                "surface": "cli_agent",
                "protocol": "acp_v1_stdio",
                "implementation_state": "available",
                "official_login": true,
                "login_flows": ["agent"],
                "remote_login_supported": false,
                "logout_supported": true,
                "credential_owner": "gemini_cli",
                "credential_storage": "official_cli_store_only",
                "cli": cli_tool(probe, "gemini_cli"),
                "active_login": gemini_attempt,
            },
            {
                "id": "claude_cli",
                "vendor": "anthropic",
                "label": "Claude Code",
                "surface": "cli_agent",
                "protocol": "claude_auth_cli_v1",
                "implementation_state": "available",
                "official_login": true,
                "login_flows": ["agent"],
                "remote_login_supported": false,
                "logout_supported": true,
                "credential_owner": "claude_code",
                "credential_storage": "official_cli_store_only",
                "cli": cli_tool(probe, "claude_cli"),
                "active_login": claude_attempt,
            },
            {
                "id": "copilot_cli",
                "vendor": "github",
                "label": "GitHub Copilot CLI",
                "surface": "cli_agent",
                "protocol": "copilot_oauth_web_flow_v1",
                "implementation_state": "available",
                "official_login": true,
                "login_flows": ["agent"],
                "remote_login_supported": false,
                "logout_supported": false,
                "credential_owner": "copilot_cli",
                "credential_storage": "official_cli_system_credential_store",
                "cli": cli_tool_with_attempt(probe, "copilot_cli", copilot_attempt.as_ref()),
                "active_login": copilot_attempt,
            },
            reserved_provider(
                "chatgpt_web",
                "openai",
                "ChatGPT 网页聊天",
                "等待官方网页聊天嵌入或获批接口；Codex 登录不能冒充 ChatGPT 网页会话。"
            ),
            reserved_provider(
                "gemini_web",
                "google",
                "Gemini 网页聊天",
                "等待官方网页聊天嵌入或获批接口；Gemini CLI 登录不能冒充 Gemini 网页会话。"
            )
        ]
    })
}

fn reserved_provider(id: &str, vendor: &str, label: &str, reason: &str) -> Value {
    json!({
        "id": id,
        "vendor": vendor,
        "label": label,
        "surface": "web_chat",
        "protocol": "reserved_provider_adapter_v1",
        "implementation_state": "reserved",
        "official_login": false,
        "login_flows": [],
        "remote_login_supported": false,
        "logout_supported": false,
        "credential_owner": "vendor",
        "credential_storage": "not_available",
        "reason": reason,
    })
}

fn cli_tool(probe: &LocalCliProbeSnapshot, provider_id: &str) -> Option<LocalCliToolStatus> {
    let name = match provider_id {
        "codex_cli" => "codex",
        "gemini_cli" => "gemini",
        "claude_cli" => "claude",
        "copilot_cli" => "copilot",
        _ => return None,
    };
    probe.tools.iter().find(|tool| tool.name == name).cloned()
}

fn cli_tool_with_attempt(
    probe: &LocalCliProbeSnapshot,
    provider_id: &str,
    attempt: Option<&ProviderLoginAttempt>,
) -> Option<LocalCliToolStatus> {
    let mut tool = cli_tool(probe, provider_id)?;
    if attempt.is_some_and(|value| value.state == "completed") {
        tool.logged_in = Some(true);
        tool.available = true;
        tool.status = "ready".to_string();
        tool.detail = Some("官方登录流程已完成；Copilot 凭据由系统凭据存储保管。".to_string());
    }
    Some(tool)
}

fn runnable_program(probe: &LocalCliProbeSnapshot, provider_id: &str) -> Option<PathBuf> {
    cli_tool(probe, provider_id)
        .filter(|tool| tool.runnable)
        .and_then(|tool| tool.path)
        .map(PathBuf::from)
}

pub(crate) fn normalize_provider_id(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "codex" | "codex_cli" => Some("codex_cli"),
        "gemini" | "gemini_cli" => Some("gemini_cli"),
        "claude" | "claude_cli" | "claude_code" => Some("claude_cli"),
        "copilot" | "copilot_cli" | "github_copilot" => Some("copilot_cli"),
        _ => None,
    }
}

fn provider_label(provider_id: &str) -> &'static str {
    match provider_id {
        "codex_cli" => "Codex CLI",
        "gemini_cli" => "Gemini CLI",
        "claude_cli" => "Claude Code",
        "copilot_cli" => "GitHub Copilot CLI",
        _ => "AI CLI",
    }
}

fn reserved_provider_message(provider_id: &str) -> &'static str {
    match provider_id.trim().to_ascii_lowercase().as_str() {
        "chatgpt_web" => "ChatGPT 网页聊天接口已保留，但没有公开可用的网页会话接入协议。",
        "gemini_web" => "Gemini 网页聊天接口已保留，但没有公开可用的网页会话接入协议。",
        _ => "未知 AI 厂商账号接口。",
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?.trim();
    let token = value.strip_prefix("Bearer ")?.trim();
    (!token.is_empty()).then(|| token.to_string())
}

fn provider_error(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "ok": false,
            "error": crate::node_agent_cli_redaction::redact_text(message)
                .chars()
                .take(500)
                .collect::<String>()
        })),
    )
}
