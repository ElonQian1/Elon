//! Secret-free interpretation of official CLI login terminal messages.

use serde_json::Value;

#[derive(Clone, Copy)]
pub(crate) enum MonitorKind {
    Codex,
    Gemini,
}

pub(crate) fn login_terminal_state(
    kind: MonitorKind,
    message: &Value,
    upstream_login_id: Option<&str>,
) -> Option<(&'static str, Option<String>)> {
    match kind {
        MonitorKind::Codex => {
            if message.get("method")?.as_str()? != "account/login/completed" {
                return None;
            }
            let params = message.get("params")?;
            if let Some(expected) = upstream_login_id {
                if params.get("loginId").and_then(Value::as_str) != Some(expected) {
                    return None;
                }
            }
            if params.get("success").and_then(Value::as_bool) == Some(true) {
                Some(("completed", None))
            } else {
                Some((
                    "failed",
                    Some(safe_error(
                        params
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("Codex 登录失败"),
                    )),
                ))
            }
        }
        MonitorKind::Gemini => {
            if message.get("id").and_then(Value::as_i64) != Some(2) {
                return None;
            }
            if let Some(error) = message.get("error") {
                Some(("failed", Some(rpc_error_message(error))))
            } else {
                Some(("completed", None))
            }
        }
    }
}

fn rpc_error_message(error: &Value) -> String {
    safe_error(
        error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("官方 CLI 返回登录错误"),
    )
}

fn safe_error(message: &str) -> String {
    crate::node_agent_cli_redaction::redact_text(message)
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fake_codex_notifications_require_the_expected_upstream_login() {
        let completed = json!({
            "method":"account/login/completed",
            "params":{"loginId":"official-login-1","success":true}
        });
        assert_eq!(
            login_terminal_state(MonitorKind::Codex, &completed, Some("official-login-1")),
            Some(("completed", None))
        );
        assert_eq!(
            login_terminal_state(MonitorKind::Codex, &completed, Some("other-login")),
            None
        );
    }

    #[test]
    fn fake_gemini_acp_response_is_redacted_and_classified() {
        let failed = json!({
            "id":2,
            "error":{"message":"token=super-secret provider failed"}
        });
        let (state, error) = login_terminal_state(MonitorKind::Gemini, &failed, None).unwrap();
        assert_eq!(state, "failed");
        assert!(!error.unwrap().contains("super-secret"));
        assert_eq!(
            login_terminal_state(MonitorKind::Gemini, &json!({"id":3,"result":{}}), None),
            None
        );
    }
}
