use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    agent_llm_call::call_chat_llm_with_options,
    project_auth::{auth_from_headers, json_error},
    types::{AgentConfig, AppState},
};

const MAX_MESSAGES: usize = 24;
const MAX_TOTAL_CHARS: usize = 80_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerAgentRuntimeRequest {
    pub messages: Vec<Value>,
    pub agent: Option<String>,
}

pub async fn chat_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ServerAgentRuntimeRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };

    if let Err(message) = validate_runtime_messages(&req.messages) {
        return json_error(StatusCode::BAD_REQUEST, message);
    }

    let agent = match resolve_server_runtime_agent(&state, req.agent.as_deref()).await {
        Some(agent) => agent,
        None => return json_error(StatusCode::SERVICE_UNAVAILABLE, "服务器未配置可用 AI 代理"),
    };

    match call_chat_llm_with_options(
        &state,
        &agent,
        &req.messages,
        &user.id,
        "pc_server_runtime",
        0.2,
        3000,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => json_error(StatusCode::BAD_GATEWAY, error),
    }
}

async fn resolve_server_runtime_agent(
    state: &Arc<AppState>,
    requested_agent: Option<&str>,
) -> Option<AgentConfig> {
    let agent_name = requested_agent
        .map(str::trim)
        .filter(|value| !value.is_empty());
    state
        .agents_config
        .read()
        .await
        .get_agent(agent_name)
        .cloned()
}

fn validate_runtime_messages(messages: &[Value]) -> Result<(), &'static str> {
    if messages.is_empty() {
        return Err("messages 不能为空");
    }
    if messages.len() > MAX_MESSAGES {
        return Err("messages 过多");
    }

    let mut total_chars = 0usize;
    for message in messages {
        let Some(object) = message.as_object() else {
            return Err("message 必须是对象");
        };
        let role = object
            .get("role")
            .and_then(Value::as_str)
            .ok_or("message.role 不能为空")?;
        if !matches!(role, "system" | "user" | "assistant") {
            return Err("message.role 只允许 system/user/assistant");
        }
        let content = object
            .get("content")
            .and_then(Value::as_str)
            .ok_or("message.content 不能为空")?;
        if content.trim().is_empty() {
            return Err("message.content 不能为空");
        }
        total_chars += content.chars().count();
        if total_chars > MAX_TOTAL_CHARS {
            return Err("messages 内容过长");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_runtime_messages;
    use serde_json::json;

    #[test]
    fn accepts_normal_runtime_messages() {
        let messages = vec![
            json!({"role": "system", "content": "Return JSON"}),
            json!({"role": "user", "content": "Read README"}),
        ];

        validate_runtime_messages(&messages).unwrap();
    }

    #[test]
    fn rejects_tool_role_messages() {
        let messages = vec![json!({"role": "tool", "content": "result"})];

        assert!(validate_runtime_messages(&messages).is_err());
    }

    #[test]
    fn rejects_empty_messages() {
        assert!(validate_runtime_messages(&[]).is_err());
    }
}
