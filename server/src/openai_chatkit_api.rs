//! Official OpenAI ChatKit bridge authenticated by a Yilong account.
//!
//! This is an API chat surface, not a ChatGPT account login. It never accepts
//! or reuses chatgpt.com cookies, Codex CLI credentials, or ChatGPT plans.

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};

use crate::{
    account_security::authenticated_account, auth_request_guard::check_rate_limit, types::AppState,
};

const CHATKIT_SESSIONS_URL: &str = "https://api.openai.com/v1/chatkit/sessions";
const CHATKIT_DOCS_URL: &str = "https://developers.openai.com/api/docs/guides/chatkit";

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/openai-chatkit/capability", get(capability_handler))
        .route("/api/openai-chatkit/session", post(create_session_handler))
}

async fn capability_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err(response) = authenticated_account(&state, &headers) {
        return response;
    }
    no_store_json(
        StatusCode::OK,
        capability_payload(&ChatKitConfig::from_env()),
    )
}

async fn create_session_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let (user_id, _) = match authenticated_account(&state, &headers) {
        Ok(account) => account,
        Err(response) => return response,
    };
    if let Err(limit) = check_rate_limit(
        "openai_chatkit_session",
        &stable_chatkit_user(&user_id),
        20,
        Duration::from_secs(60),
    ) {
        let mut response = chatkit_error(
            StatusCode::TOO_MANY_REQUESTS,
            "chatkit_rate_limited",
            "ChatKit 会话创建过于频繁，请稍后重试。",
        );
        if let Ok(value) = HeaderValue::from_str(&limit.retry_after_seconds.to_string()) {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        return response;
    }

    let config = ChatKitConfig::from_env();
    let (Some(api_key), Some(workflow_id)) = (config.api_key, config.workflow_id) else {
        return chatkit_error(
            StatusCode::CONFLICT,
            "chatkit_not_configured",
            "管理员尚未配置 OpenAI ChatKit API 服务。",
        );
    };

    let upstream = state
        .http_client
        .post(CHATKIT_SESSIONS_URL)
        .timeout(Duration::from_secs(30))
        .bearer_auth(api_key)
        .header("OpenAI-Beta", "chatkit_beta=v1")
        .json(&json!({
            "workflow": { "id": workflow_id },
            "user": stable_chatkit_user(&user_id),
        }))
        .send()
        .await;

    let upstream = match upstream {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(error_kind = ?error.status(), "OpenAI ChatKit session request failed");
            return chatkit_error(
                StatusCode::BAD_GATEWAY,
                "chatkit_upstream_unavailable",
                "OpenAI ChatKit 服务暂时不可用，请稍后重试。",
            );
        }
    };
    let upstream_status = upstream.status();
    if !upstream_status.is_success() {
        tracing::warn!(status = %upstream_status, "OpenAI ChatKit rejected session creation");
        return chatkit_error(
            StatusCode::BAD_GATEWAY,
            "chatkit_session_rejected",
            "OpenAI 未能创建 ChatKit 会话，请管理员检查 API Key、工作流和项目权限。",
        );
    }

    let session = match upstream.json::<ChatKitSessionResponse>().await {
        Ok(session) if !session.client_secret.trim().is_empty() => session,
        _ => {
            return chatkit_error(
                StatusCode::BAD_GATEWAY,
                "chatkit_invalid_response",
                "OpenAI ChatKit 返回了无法使用的会话。",
            );
        }
    };

    no_store_json(
        StatusCode::OK,
        json!({
            "ok": true,
            "schema": "elon.openai_chatkit_session.v1",
            "client_secret": session.client_secret,
            "credential_scope": "short_lived_chatkit_session",
            "stored_by_elon": false,
        }),
    )
}

#[derive(Debug, Deserialize)]
struct ChatKitSessionResponse {
    client_secret: String,
}

#[derive(Debug, Default)]
struct ChatKitConfig {
    api_key: Option<String>,
    workflow_id: Option<String>,
}

impl ChatKitConfig {
    fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
        let api_key = first_clean(
            &mut lookup,
            &["ELON_CHATKIT_OPENAI_API_KEY", "OPENAI_API_KEY"],
        );
        let workflow_id = first_clean(
            &mut lookup,
            &["ELON_CHATKIT_WORKFLOW_ID", "OPENAI_CHATKIT_WORKFLOW_ID"],
        )
        .filter(|value| valid_workflow_id(value));
        Self {
            api_key,
            workflow_id,
        }
    }

    fn configured(&self) -> bool {
        self.api_key.is_some() && self.workflow_id.is_some()
    }
}

fn first_clean(lookup: &mut impl FnMut(&str) -> Option<String>, names: &[&str]) -> Option<String> {
    names
        .iter()
        .filter_map(|name| lookup(name))
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}

fn valid_workflow_id(value: &str) -> bool {
    value.starts_with("wf_")
        && value.len() <= 200
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn capability_payload(config: &ChatKitConfig) -> Value {
    json!({
        "ok": true,
        "schema": "elon.openai_chatkit_capability.v1",
        "provider_id": "openai_chatkit",
        "label": "OpenAI ChatKit（API 聊天）",
        "configured": config.configured(),
        "implementation_state": if config.configured() { "available" } else { "configuration_required" },
        "integration_mode": "hosted_workflow_transition",
        "login": {
            "required": true,
            "provider": "elon_account",
            "chatgpt_account_login": false,
        },
        "privacy": {
            "chatgpt_cookie_reusable": false,
            "chatgpt_history_reusable": false,
            "chatgpt_subscription_reusable": false,
            "openai_api_key_exposed_to_client": false,
            "client_receives_short_lived_secret_only": true,
        },
        "session_endpoint": "/api/openai-chatkit/session",
        "official_docs": CHATKIT_DOCS_URL,
        "transition": {
            "existing_agent_builder_workflow_required": true,
            "recommended_new_architecture": "custom_server_integration",
            "agent_builder_shutdown_date": "2026-11-30",
        },
        "message": if config.configured() {
            "已使用当前一龙账号接入官方 ChatKit API。"
        } else {
            "管理员配置 OpenAI API Key 和现有 ChatKit workflow 后即可使用。"
        },
    })
}

fn stable_chatkit_user(user_id: &str) -> String {
    format!(
        "elon_{}",
        hex::encode(Sha256::digest(
            format!("openai-chatkit:{user_id}").as_bytes()
        ))
    )
}

fn no_store_json(status: StatusCode, value: Value) -> Response {
    let mut response = (status, Json(value)).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn chatkit_error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    no_store_json(
        status,
        json!({
            "ok": false,
            "code": code,
            "error": message,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_never_claims_chatgpt_account_reuse() {
        let payload = capability_payload(&ChatKitConfig {
            api_key: Some("sk-test".to_string()),
            workflow_id: Some("wf_test".to_string()),
        });
        assert_eq!(payload["configured"], true);
        assert_eq!(payload["login"]["provider"], "elon_account");
        assert_eq!(payload["login"]["chatgpt_account_login"], false);
        assert_eq!(payload["privacy"]["chatgpt_cookie_reusable"], false);
        assert_eq!(payload["privacy"]["chatgpt_subscription_reusable"], false);
    }

    #[test]
    fn configuration_requires_an_official_workflow_id_and_api_key() {
        let configured = ChatKitConfig::from_lookup(|name| match name {
            "OPENAI_API_KEY" => Some("sk-test".to_string()),
            "OPENAI_CHATKIT_WORKFLOW_ID" => Some("wf_example".to_string()),
            _ => None,
        });
        assert!(configured.configured());

        let invalid = ChatKitConfig::from_lookup(|name| match name {
            "OPENAI_API_KEY" => Some("sk-test".to_string()),
            "OPENAI_CHATKIT_WORKFLOW_ID" => Some("not-a-workflow".to_string()),
            _ => None,
        });
        assert!(!invalid.configured());
    }

    #[test]
    fn stable_user_identifier_is_private_and_deterministic() {
        let first = stable_chatkit_user("user-15692409892");
        assert_eq!(first, stable_chatkit_user("user-15692409892"));
        assert_ne!(first, stable_chatkit_user("another-user"));
        assert!(!first.contains("15692409892"));
    }
}
