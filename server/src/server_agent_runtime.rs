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
#[path = "server_agent_runtime_tests.rs"]
mod tests;
