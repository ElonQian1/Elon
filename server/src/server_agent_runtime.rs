// server/src/server_agent_runtime.rs

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::{
    agent_llm_call::call_chat_llm_with_options,
    project_auth::{auth_from_headers, json_error},
    server_agent_runtime_limits::ServerAgentRuntimeLimits,
    types::{AgentConfig, AppState},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerAgentRuntimeRequest {
    pub messages: Vec<Value>,
    pub agent: Option<String>,
}

#[derive(Debug, Serialize)]
struct ServerAgentRuntimeStatus {
    ready: bool,
    status: &'static str,
    agent: Option<ServerAgentRuntimeAgentStatus>,
    limits: ServerAgentRuntimeLimits,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerAgentRuntimeAgentStatus {
    name: String,
    model: String,
    usage_mode: String,
}

pub async fn status_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if auth_from_headers(&state, &headers).is_err() {
        return json_error(StatusCode::UNAUTHORIZED, "未登录");
    }

    let agent = resolve_server_runtime_agent(&state, None)
        .await
        .map(|agent| {
            let usage_mode = agent.usage_mode().to_string();
            ServerAgentRuntimeAgentStatus {
                name: agent.name,
                model: agent.model,
                usage_mode,
            }
        });
    let ready = agent.is_some();
    Json(ServerAgentRuntimeStatus {
        ready,
        status: if ready { "ready" } else { "unavailable" },
        agent,
        limits: ServerAgentRuntimeLimits::current(),
    })
    .into_response()
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

    let limits = ServerAgentRuntimeLimits::current();
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
        limits.temperature,
        limits.max_output_tokens,
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
    ServerAgentRuntimeLimits::current().validate_messages(messages)
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
