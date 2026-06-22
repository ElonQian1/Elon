// server/src/server_agent_runtime.rs

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::{
    agent_llm_call::call_chat_llm_with_json_response_mode,
    project_auth::{auth_from_headers, json_error},
    server_agent_runtime_guard::{
        admission_availability, admission_snapshot, audit_summary, operational_error_summary,
        protection_status, try_acquire_runtime_admission, ServerRuntimeAdmissionAvailability,
        ServerRuntimeAdmissionError, ServerRuntimeAdmissionSnapshot, ServerRuntimeProtectionStatus,
    },
    server_agent_runtime_limits::ServerAgentRuntimeLimits,
    server_agent_runtime_policy::ServerAgentRuntimePolicy,
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
    protection: ServerRuntimeProtectionStatus,
    policy: ServerAgentRuntimePolicy,
    admission: ServerRuntimeAdmissionSnapshot,
    #[serde(rename = "admissionAvailability")]
    admission_availability: ServerRuntimeAdmissionAvailability,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerAgentRuntimeAgentStatus {
    name: String,
    model: String,
    usage_mode: String,
}

pub async fn status_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };

    let limits = ServerAgentRuntimeLimits::current();
    let policy = ServerAgentRuntimePolicy::current();
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
    let admission = admission_snapshot(&user.id, limits);
    let admission_availability = admission_availability(&admission);
    let ready = policy.enabled && agent.is_some() && admission_availability.ready;
    Json(ServerAgentRuntimeStatus {
        ready,
        status: if !policy.enabled {
            "disabled"
        } else if agent.is_none() {
            "unavailable"
        } else if !admission_availability.ready {
            "limited"
        } else {
            "ready"
        },
        agent,
        limits,
        protection: protection_status(),
        policy,
        admission,
        admission_availability,
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

    let policy = ServerAgentRuntimePolicy::current();
    if !policy.enabled {
        tracing::warn!(
            target: "server_agent_runtime",
            user_id = %user.id,
            source = policy.source,
            reason = policy.reason.as_deref().unwrap_or("operator_disabled"),
            "pc_server_runtime request rejected by runtime policy"
        );
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            policy.public_disabled_message(),
        );
    }

    let limits = ServerAgentRuntimeLimits::current();
    if let Err(message) = validate_runtime_messages(&req.messages) {
        return json_error(StatusCode::BAD_REQUEST, message);
    }
    let audit = audit_summary(&req.messages, limits);

    let agent = match resolve_server_runtime_agent(&state, req.agent.as_deref()).await {
        Some(agent) => agent,
        None => return json_error(StatusCode::SERVICE_UNAVAILABLE, "服务器未配置可用 AI 代理"),
    };
    let _admission = match try_acquire_runtime_admission(&user.id, limits) {
        Ok(guard) => guard,
        Err(error) => {
            tracing::warn!(
                target: "server_agent_runtime",
                user_id = %user.id,
                request_fingerprint = %audit.request_fingerprint,
                admission_error = %error,
                "pc_server_runtime request rejected by admission control"
            );
            return admission_error_response(error);
        }
    };
    tracing::info!(
        target: "server_agent_runtime",
        user_id = %user.id,
        agent = %agent.name,
        model = %agent.model,
        usage_mode = %agent.usage_mode(),
        request_fingerprint = %audit.request_fingerprint,
        message_count = audit.message_count,
        total_chars = audit.total_chars,
        max_message_chars = audit.max_message_chars,
        "pc_server_runtime request accepted"
    );

    match call_chat_llm_with_json_response_mode(
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
        Err(error) => {
            let summary = operational_error_summary(&error.to_string());
            tracing::warn!(
                target: "server_agent_runtime",
                user_id = %user.id,
                request_fingerprint = %audit.request_fingerprint,
                error = %summary,
                "pc_server_runtime request failed"
            );
            json_error(StatusCode::BAD_GATEWAY, provider_error_message(&summary))
        }
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

fn provider_error_message(summary: &str) -> String {
    format!("AI runtime provider failed: {summary}")
}

fn admission_error_response(error: ServerRuntimeAdmissionError) -> Response {
    let retry_after = error.retry_after_secs().to_string();
    let mut response = json_error(StatusCode::TOO_MANY_REQUESTS, error.public_message());
    if let Ok(value) = HeaderValue::from_str(&retry_after) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::{
        admission_error_response, audit_summary, operational_error_summary, protection_status,
        provider_error_message, validate_runtime_messages,
    };
    use crate::server_agent_runtime_guard::ServerRuntimeAdmissionError;
    use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;
    use axum::http::header;
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

    #[test]
    fn runtime_status_exposes_protection_contract() {
        let protection = protection_status();

        assert!(protection.input_validation.contains("total_chars"));
        assert!(protection.input_validation.contains("message_chars"));
        assert!(protection.admission_control.contains("global"));
        assert!(protection
            .operational_switch
            .contains("ELON_SERVER_AGENT_RUNTIME_ENABLED"));
        assert!(protection.billing_gate.contains("call_chat_llm"));
        assert!(protection.request_fingerprint.contains("sha256"));
    }

    #[test]
    fn audit_summary_keeps_prompt_text_out_of_operational_metadata() {
        let messages = vec![json!({"role": "user", "content": "very secret prompt"})];
        let audit = audit_summary(&messages, ServerAgentRuntimeLimits::current());
        let text = serde_json::to_string(&audit).unwrap();

        assert_eq!(audit.message_count, 1);
        assert_eq!(audit.roles, vec!["user"]);
        assert!(!text.contains("very secret prompt"));
    }

    #[test]
    fn provider_error_response_uses_summary_not_raw_body() {
        let raw = "429 rate limit: sk-secret and user prompt text";
        let message = provider_error_message(&operational_error_summary(raw));

        assert!(message.contains("rate_limit"));
        assert!(message.contains("fingerprint="));
        assert!(!message.contains("sk-secret"));
        assert!(!message.contains("user prompt text"));
    }

    #[test]
    fn admission_error_response_sets_retry_after_header() {
        let response = admission_error_response(ServerRuntimeAdmissionError::RateLimited {
            max_requests_per_minute: 1,
            retry_after_secs: 23,
        });

        assert_eq!(response.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(header::RETRY_AFTER).unwrap(), "23");
    }
}
