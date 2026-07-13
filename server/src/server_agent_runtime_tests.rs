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
    assert!(unsupported_agent_usage_mode_message().contains("平台 API key"));
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
