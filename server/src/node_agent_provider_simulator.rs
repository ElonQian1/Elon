//! Secret-free fake-provider matrix for deterministic auth-flow tests.

use serde_json::{json, Value};

pub(crate) fn simulation_contract() -> Value {
    json!({
        "schema": "elon.provider_auth_simulation_matrix.v1",
        "uses_real_vendor_account": false,
        "uses_real_browser": false,
        "scenarios": [
            {"id":"codex_completed", "expected":"completed", "fixture":"json_rpc_notification"},
            {"id":"codex_rejected", "expected":"failed_redacted", "fixture":"json_rpc_notification"},
            {"id":"codex_wrong_login_id", "expected":"ignored", "fixture":"json_rpc_notification"},
            {"id":"codex_malformed_or_noise", "expected":"ignored", "fixture":"mixed_stdout"},
            {"id":"gemini_completed", "expected":"completed", "fixture":"acp_response"},
            {"id":"gemini_rejected", "expected":"failed_redacted", "fixture":"acp_error"},
            {"id":"process_exits_before_terminal", "expected":"failed", "fixture":"fake_cli_exit"},
            {"id":"user_cancel", "expected":"canceled", "fixture":"control_plane"},
            {"id":"attempt_timeout", "expected":"expired", "fixture":"virtual_clock"},
            {"id":"node_restart", "expected":"failed_node_restarted", "fixture":"journal_recovery"}
        ],
        "cleanup": {
            "active_expiry_minutes": 15,
            "terminal_retention_hours": 24,
            "max_attempts": 64,
            "terminal_state_immutable": true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_provider_auth_monitor::{login_terminal_state, MonitorKind};

    #[test]
    fn phase2_contract_fake_codex_matrix_requires_login_id_and_redacts_errors() {
        let completed = json!({
            "method":"account/login/completed",
            "params":{"loginId":"login-a","success":true}
        });
        assert_eq!(
            login_terminal_state(MonitorKind::Codex, &completed, Some("login-a")),
            Some(("completed", None))
        );
        assert_eq!(
            login_terminal_state(MonitorKind::Codex, &completed, Some("login-b")),
            None
        );
        let rejected = json!({
            "method":"account/login/completed",
            "params":{"loginId":"login-a","success":false,"error":"token=secret denied"}
        });
        let (state, error) =
            login_terminal_state(MonitorKind::Codex, &rejected, Some("login-a")).unwrap();
        assert_eq!(state, "failed");
        assert!(!error.unwrap().contains("secret"));
        assert_eq!(
            login_terminal_state(MonitorKind::Codex, &json!({"noise":true}), None),
            None
        );
    }

    #[test]
    fn phase2_contract_fake_gemini_matrix_accepts_only_terminal_acp_response() {
        assert_eq!(
            login_terminal_state(MonitorKind::Gemini, &json!({"id":2,"result":{}}), None),
            Some(("completed", None))
        );
        let (state, error) = login_terminal_state(
            MonitorKind::Gemini,
            &json!({"id":2,"error":{"message":"authorization denied"}}),
            None,
        )
        .unwrap();
        assert_eq!(state, "failed");
        assert!(error.unwrap().contains("denied"));
        assert_eq!(
            login_terminal_state(MonitorKind::Gemini, &json!({"id":99,"result":{}}), None),
            None
        );
    }
}
