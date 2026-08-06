//! Sanitized provider-auth diagnostics. No codes, URLs, tokens, or file contents.

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{node_agent_provider_auth_attempt::ProviderLoginAttempt, NodeRuntime};

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new().route(
        "/api/ai-provider-accounts/diagnostics",
        get(diagnostics_handler),
    )
}

async fn diagnostics_handler(
    axum::extract::State(runtime): axum::extract::State<Arc<NodeRuntime>>,
) -> Json<Value> {
    runtime.ensure_cli_probe_background(false).await;
    let probe = runtime.cached_cli_probe().await;
    let mut attempts = Vec::new();
    for provider_id in ["codex_cli", "gemini_cli", "claude_cli", "copilot_cli"] {
        if let Some(attempt) = runtime.provider_auth.latest(provider_id).await {
            attempts.push(attempt_diagnostic(&attempt));
        }
    }
    Json(json!({
        "ok": true,
        "schema": "elon.ai_provider_diagnostics.v1",
        "state_machine": state_machine_contract(),
        "journal": {
            "sanitized": true,
            "retention_hours": 24,
            "max_attempts": 64,
            "secrets_persisted": false,
            "restart_recovery": "active_to_failed_node_restarted"
        },
        "probe": {
            "tools": probe.tools.iter().map(|tool| json!({
                "name": tool.name,
                "available": tool.available,
                "runnable": tool.runnable,
                "logged_in": tool.logged_in,
                "status": tool.status,
                "reason": tool.reason,
            })).collect::<Vec<_>>()
        },
        "latest_attempts": attempts,
        "redaction": {
            "verification_url": "omitted",
            "auth_url": "omitted",
            "user_code": "omitted",
            "provider_tokens": "never_collected"
        }
    }))
}

fn attempt_diagnostic(attempt: &ProviderLoginAttempt) -> Value {
    json!({
        "login_id": attempt.login_id,
        "provider_id": attempt.provider_id,
        "flow": attempt.flow,
        "state": attempt.state,
        "request_id_present": attempt.request_id.is_some(),
        "recovered": attempt.recovered,
        "retryable": attempt.retryable(),
        "next_action": attempt.next_action(),
        "error_code": attempt.error_code,
        "started_at_ms": attempt.started_at_ms,
        "updated_at_ms": attempt.updated_at_ms,
    })
}

pub(crate) fn state_machine_contract() -> Value {
    json!({
        "initial": ["starting", "waiting_for_user"],
        "active": ["starting", "waiting_for_user"],
        "terminal": ["completed", "failed", "canceled", "expired"],
        "retryable_terminal": ["failed", "canceled", "expired"],
        "terminal_immutable": true,
        "restart_transition": "starting_or_waiting_to_failed_node_restarted"
    })
}
