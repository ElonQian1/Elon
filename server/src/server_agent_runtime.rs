// server/src/server_agent_runtime.rs

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    agent_llm_call::call_chat_llm_with_json_response_mode,
    project_auth::{auth_from_headers, json_error},
    server_agent_runtime_budget::{
        server_runtime_budget_status_for_user, try_record_route_c_call, ServerRuntimeBudgetError,
    },
    server_agent_runtime_guard::{
        admission_availability, admission_snapshot, audit_summary, operational_error_summary,
        protection_status, try_acquire_runtime_admission_for_request, ServerRuntimeAdmissionError,
    },
    server_agent_runtime_limits::ServerAgentRuntimeLimits,
    server_agent_runtime_output::validate_server_runtime_output,
    server_agent_runtime_policy::{ServerAgentRuntimeAgentPolicy, ServerAgentRuntimePolicy},
    server_agent_runtime_status::{
        route_c_blocking_reasons, route_c_status_code, ServerAgentRuntimeAgentStatus,
        ServerAgentRuntimeStatus,
    },
    types::{AgentConfig, AgentsConfig, AppState},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerAgentRuntimeRequest {
    pub messages: Vec<Value>,
    pub agent: Option<String>,
}

pub async fn status_handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };

    let limits = ServerAgentRuntimeLimits::current();
    let policy = ServerAgentRuntimePolicy::current();
    let agent_policy = ServerAgentRuntimeAgentPolicy::current();
    let resolved_agent = resolve_server_runtime_agent(&state, None, &agent_policy).await;
    let agent_status = match &resolved_agent {
        Ok(_) => "ready",
        Err(ServerRuntimeAgentResolveError::NotAllowed) => "agent_not_allowed",
        Err(ServerRuntimeAgentResolveError::UnsupportedUsageMode) => "unsupported_agent_usage_mode",
        Err(ServerRuntimeAgentResolveError::Unavailable) => "unavailable",
    };
    let agent = resolved_agent.as_ref().ok().map(|agent| {
        let usage_mode = agent.usage_mode().to_string();
        ServerAgentRuntimeAgentStatus {
            name: agent.name.clone(),
            model: agent.model.clone(),
            usage_mode,
        }
    });
    let admission = admission_snapshot(&user.id, limits);
    let admission_availability = admission_availability(&admission);
    let budget = server_runtime_budget_status_for_user(&state.store, &user.id);
    let ready = policy.enabled && agent.is_some() && admission_availability.ready && budget.ready();
    let blocking_reasons = route_c_blocking_reasons(
        &policy,
        agent.is_some(),
        agent_status,
        &budget,
        &admission_availability,
    );
    Json(ServerAgentRuntimeStatus {
        ready,
        status: route_c_status_code(
            &policy,
            agent.is_some(),
            agent_status,
            &budget,
            &admission_availability,
        ),
        agent,
        limits,
        protection: protection_status(),
        policy,
        agent_policy,
        budget,
        admission,
        admission_availability,
        blocking_reasons,
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

    let agent_policy = ServerAgentRuntimeAgentPolicy::current();
    let agent =
        match resolve_server_runtime_agent(&state, req.agent.as_deref(), &agent_policy).await {
            Ok(agent) => agent,
            Err(ServerRuntimeAgentResolveError::NotAllowed) => {
                tracing::warn!(
                    target: "server_agent_runtime",
                    user_id = %user.id,
                    requested_agent = req.agent.as_deref().unwrap_or(""),
                    policy_mode = agent_policy.mode,
                    policy_source = agent_policy.source,
                    request_fingerprint = %audit.request_fingerprint,
                    "pc_server_runtime request rejected by agent selection policy"
                );
                return json_error(StatusCode::FORBIDDEN, agent_policy.public_denied_message());
            }
            Err(ServerRuntimeAgentResolveError::Unavailable) => {
                return json_error(StatusCode::SERVICE_UNAVAILABLE, "服务器未配置可用 AI 代理");
            }
            Err(ServerRuntimeAgentResolveError::UnsupportedUsageMode) => {
                tracing::warn!(
                    target: "server_agent_runtime",
                    user_id = %user.id,
                    requested_agent = req.agent.as_deref().unwrap_or(""),
                    request_fingerprint = %audit.request_fingerprint,
                    "pc_server_runtime request rejected by unsupported agent usage mode"
                );
                return json_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    unsupported_agent_usage_mode_message(),
                );
            }
        };
    let _admission = match try_acquire_runtime_admission_for_request(
        &user.id,
        limits,
        Some(&audit.request_fingerprint),
    ) {
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
    let budget = match try_record_route_c_call(&state.store, &user.id, &audit.request_fingerprint) {
        Ok(record) => record,
        Err(error) => {
            tracing::warn!(
                target: "server_agent_runtime",
                user_id = %user.id,
                request_fingerprint = %audit.request_fingerprint,
                budget_status = error.status().status,
                daily_call_limit = error.status().daily_call_limit.unwrap_or_default(),
                used_calls_today = error.status().used_calls_today,
                "pc_server_runtime request rejected by daily budget fuse"
            );
            return budget_error_response(error);
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
        budget_status = budget.status.status,
        budget_used_calls_today = budget.status.used_calls_today,
        budget_daily_call_limit = budget.status.daily_call_limit.unwrap_or_default(),
        budget_event_id = %budget.event_id,
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
        Ok(response) => match validate_server_runtime_output(&response, limits) {
            Ok(()) => {
                record_route_c_completion(
                    &state,
                    &budget.event_id,
                    "success",
                    response_model(&response).or_else(|| Some(agent.model.clone())),
                    response_total_tokens(&response),
                    None,
                );
                Json(response).into_response()
            }
            Err(error) => {
                record_route_c_completion(
                    &state,
                    &budget.event_id,
                    "output_rejected",
                    response_model(&response).or_else(|| Some(agent.model.clone())),
                    response_total_tokens(&response),
                    Some(error.kind().to_string()),
                );
                tracing::warn!(
                    target: "server_agent_runtime",
                    user_id = %user.id,
                    request_fingerprint = %audit.request_fingerprint,
                    output_error = error.kind(),
                    "pc_server_runtime provider response rejected by output guard"
                );
                json_error(StatusCode::BAD_GATEWAY, error.public_message())
            }
        },
        Err(error) => {
            let summary = operational_error_summary(&error.to_string());
            record_route_c_completion(
                &state,
                &budget.event_id,
                "provider_error",
                Some(agent.model.clone()),
                None,
                Some(summary.clone()),
            );
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
    agent_policy: &ServerAgentRuntimeAgentPolicy,
) -> Result<AgentConfig, ServerRuntimeAgentResolveError> {
    let agent_name = requested_agent
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let agents = state.agents_config.read().await;
    if !agent_policy.allows_requested_agent(agent_name, &agents.default_agent) {
        return Err(ServerRuntimeAgentResolveError::NotAllowed);
    }
    let agent = if let Some(agent_name) = agent_name {
        named_agent_config(&agents, agent_name)
            .cloned()
            .ok_or(ServerRuntimeAgentResolveError::Unavailable)?
    } else {
        agents
            .get_agent(None)
            .cloned()
            .ok_or(ServerRuntimeAgentResolveError::Unavailable)?
    };
    if !server_runtime_agent_usage_mode_allowed(&agent) {
        return Err(ServerRuntimeAgentResolveError::UnsupportedUsageMode);
    }
    Ok(agent)
}

fn named_agent_config<'a>(agents: &'a AgentsConfig, agent_name: &str) -> Option<&'a AgentConfig> {
    agents.agents.get(agent_name).or_else(|| {
        agents
            .agents
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(agent_name))
            .map(|(_, agent)| agent)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerRuntimeAgentResolveError {
    NotAllowed,
    UnsupportedUsageMode,
    Unavailable,
}

fn server_runtime_agent_usage_mode_allowed(agent: &AgentConfig) -> bool {
    agent.usage_mode() == "server_api_key"
}

fn unsupported_agent_usage_mode_message() -> &'static str {
    "平台AI只允许使用平台 API key 模型通道；当前模型通道不是平台 API key 模式，请由运营调整默认通道或允许列表。"
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

fn budget_error_response(error: ServerRuntimeBudgetError) -> Response {
    let retry_after = error.retry_after_secs().to_string();
    let mut response = json_error(StatusCode::TOO_MANY_REQUESTS, error.public_message());
    if let Ok(value) = HeaderValue::from_str(&retry_after) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

fn record_route_c_completion(
    state: &Arc<AppState>,
    event_id: &str,
    outcome: &str,
    model: Option<String>,
    total_tokens: Option<i64>,
    error_summary: Option<String>,
) {
    if event_id.trim().is_empty() {
        return;
    }
    if let Err(error) = state.store.route_c_budget_mark_completed(
        event_id,
        crate::store::route_c_budget::RouteCBudgetCompletion {
            outcome: outcome.to_string(),
            model,
            total_tokens,
            error_summary,
        },
    ) {
        tracing::warn!(
            target: "server_agent_runtime",
            budget_event_id = %event_id,
            error = %error,
            "Route C completion audit update failed"
        );
    }
}

fn response_model(response: &Value) -> Option<String> {
    response
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn response_total_tokens(response: &Value) -> Option<i64> {
    response
        .get("usage")
        .and_then(|usage| usage.get("total_tokens"))
        .and_then(Value::as_i64)
        .filter(|value| *value >= 0)
}

#[cfg(test)]
mod tests {
    use super::{
        admission_error_response, audit_summary, budget_error_response, named_agent_config,
        operational_error_summary, protection_status, provider_error_message, response_model,
        response_total_tokens, server_runtime_agent_usage_mode_allowed,
        unsupported_agent_usage_mode_message, validate_runtime_messages,
    };
    use crate::server_agent_runtime_budget::{ServerRuntimeBudgetError, ServerRuntimeBudgetStatus};
    use crate::server_agent_runtime_guard::ServerRuntimeAdmissionError;
    use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;
    use crate::types::{AgentConfig, AgentsConfig};
    use axum::http::header;
    use serde_json::json;
    use std::collections::HashMap;

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
        assert!(protection.output_validation.contains("actions"));
        assert!(protection.input_validation.contains("message_chars"));
        assert!(protection.agent_selection.contains("default server agent"));
        assert!(protection.agent_selection.contains("server_api_key"));
        assert!(protection.admission_control.contains("global"));
        assert!(protection
            .duplicate_request_debounce
            .contains("DUPLICATE_WINDOW_SECS"));
        assert!(protection.budget_gate.contains("DAILY_CALL_LIMIT"));
        assert!(protection.budget_gate.contains("PER_USER_DAILY_CALL_LIMIT"));
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
    fn route_c_agent_usage_mode_must_be_server_api_key() {
        let server_key_agent = AgentConfig {
            name: "main".to_string(),
            api_base: "https://api.example.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "route-c-model".to_string(),
            embedding_model: None,
            usage_mode: Some("server_api_key".to_string()),
        };
        let legacy_server_key_agent = AgentConfig {
            usage_mode: None,
            ..server_key_agent.clone()
        };
        let user_proxy_agent = AgentConfig {
            usage_mode: Some("user_api_key_proxy".to_string()),
            ..server_key_agent.clone()
        };
        let copilot_agent = AgentConfig {
            name: "copilot:gpt-4o".to_string(),
            api_base: "https://api.githubcopilot.com".to_string(),
            usage_mode: Some("server_codex_cli".to_string()),
            ..server_key_agent
        };

        assert!(server_runtime_agent_usage_mode_allowed(
            &legacy_server_key_agent
        ));
        assert!(server_runtime_agent_usage_mode_allowed(&AgentConfig {
            usage_mode: Some("server_api_key".to_string()),
            ..legacy_server_key_agent
        }));
        assert!(!server_runtime_agent_usage_mode_allowed(&user_proxy_agent));
        assert!(!server_runtime_agent_usage_mode_allowed(&copilot_agent));
        assert!(unsupported_agent_usage_mode_message().contains("server_api_key"));
    }

    #[test]
    fn completion_audit_extracts_only_model_and_token_summary() {
        let response = json!({
            "model": "route-c-model",
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            },
            "choices": [{
                "message": {"content": "secret generated content"}
            }]
        });

        assert_eq!(response_model(&response).as_deref(), Some("route-c-model"));
        assert_eq!(response_total_tokens(&response), Some(15));
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

    #[test]
    fn duplicate_admission_error_response_sets_retry_after_header() {
        let response = admission_error_response(ServerRuntimeAdmissionError::DuplicateRecent {
            retry_after_secs: 5,
        });

        assert_eq!(response.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(header::RETRY_AFTER).unwrap(), "5");
    }

    #[test]
    fn budget_error_response_sets_retry_after_header() {
        let response = budget_error_response(ServerRuntimeBudgetError::DailyCallLimitReached(
            ServerRuntimeBudgetStatus {
                enabled: true,
                status: "exhausted",
                source: "test",
                used_calls_today: 5,
                daily_call_limit: Some(5),
                remaining_calls_today: Some(0),
                per_user_enabled: false,
                per_user_source: "default",
                used_calls_today_for_user: None,
                per_user_daily_call_limit: None,
                remaining_calls_today_for_user: None,
                reset_after_secs: 3600,
            },
        ));

        assert_eq!(response.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(header::RETRY_AFTER).unwrap(), "3600");
    }

    #[test]
    fn user_budget_error_response_sets_retry_after_header() {
        let response = budget_error_response(ServerRuntimeBudgetError::UserDailyCallLimitReached(
            ServerRuntimeBudgetStatus {
                enabled: true,
                status: "user_exhausted",
                source: "default",
                used_calls_today: 5,
                daily_call_limit: None,
                remaining_calls_today: None,
                per_user_enabled: true,
                per_user_source: "test",
                used_calls_today_for_user: Some(2),
                per_user_daily_call_limit: Some(2),
                remaining_calls_today_for_user: Some(0),
                reset_after_secs: 1800,
            },
        ));

        assert_eq!(response.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(header::RETRY_AFTER).unwrap(), "1800");
    }

    #[test]
    fn named_agent_lookup_accepts_exact_or_case_insensitive_match() {
        let agents = AgentsConfig {
            default_agent: "main".to_string(),
            agents: HashMap::from([(
                "route-c-fast".to_string(),
                AgentConfig {
                    name: "route-c-fast".to_string(),
                    api_base: "https://api.example.com/v1".to_string(),
                    api_key: "sk-test".to_string(),
                    model: "fast-model".to_string(),
                    embedding_model: None,
                    usage_mode: Some("server_api_key".to_string()),
                },
            )]),
        };

        assert_eq!(
            named_agent_config(&agents, "route-c-fast").map(|agent| agent.model.as_str()),
            Some("fast-model")
        );
        assert_eq!(
            named_agent_config(&agents, "ROUTE-C-FAST").map(|agent| agent.model.as_str()),
            Some("fast-model")
        );
        assert!(named_agent_config(&agents, "expensive").is_none());
    }
}
