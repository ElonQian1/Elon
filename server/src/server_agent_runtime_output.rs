// server/src/server_agent_runtime_output.rs

use serde_json::Value;

use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerRuntimeOutputError {
    ResponseNotObject,
    ActionsNotArray,
    TooManyActions {
        count: usize,
        max: usize,
    },
    ActionNotObject {
        index: usize,
    },
    MissingTool {
        index: usize,
    },
    ActionTooLarge {
        index: usize,
        chars: usize,
        max: usize,
    },
    ActionsTooLarge {
        chars: usize,
        max: usize,
    },
}

pub(crate) fn validate_server_runtime_output(
    response: &Value,
    limits: ServerAgentRuntimeLimits,
) -> Result<(), ServerRuntimeOutputError> {
    let Some(object) = response.as_object() else {
        return Err(ServerRuntimeOutputError::ResponseNotObject);
    };
    let Some(actions) = object.get("actions") else {
        return Ok(());
    };
    if actions.is_null() {
        return Ok(());
    }
    let Some(actions) = actions.as_array() else {
        return Err(ServerRuntimeOutputError::ActionsNotArray);
    };
    if actions.len() > limits.max_actions {
        return Err(ServerRuntimeOutputError::TooManyActions {
            count: actions.len(),
            max: limits.max_actions,
        });
    }

    let mut total_chars = 0usize;
    for (index, action) in actions.iter().enumerate() {
        let Some(action_object) = action.as_object() else {
            return Err(ServerRuntimeOutputError::ActionNotObject { index });
        };
        let tool = action_object
            .get("tool")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if tool.is_empty() {
            return Err(ServerRuntimeOutputError::MissingTool { index });
        }
        let action_chars = serialized_chars(action);
        if action_chars > limits.max_action_chars {
            return Err(ServerRuntimeOutputError::ActionTooLarge {
                index,
                chars: action_chars,
                max: limits.max_action_chars,
            });
        }
        total_chars += action_chars;
        if total_chars > limits.max_actions_total_chars {
            return Err(ServerRuntimeOutputError::ActionsTooLarge {
                chars: total_chars,
                max: limits.max_actions_total_chars,
            });
        }
    }

    Ok(())
}

impl ServerRuntimeOutputError {
    pub(crate) fn public_message(self) -> String {
        format!(
            "AI runtime provider returned an unsafe Route C action payload: {}",
            self.kind()
        )
    }

    pub(crate) fn kind(self) -> &'static str {
        match self {
            Self::ResponseNotObject => "response_not_object",
            Self::ActionsNotArray => "actions_not_array",
            Self::TooManyActions { .. } => "too_many_actions",
            Self::ActionNotObject { .. } => "action_not_object",
            Self::MissingTool { .. } => "missing_tool",
            Self::ActionTooLarge { .. } => "action_too_large",
            Self::ActionsTooLarge { .. } => "actions_too_large",
        }
    }
}

fn serialized_chars(value: &Value) -> usize {
    serde_json::to_string(value)
        .map(|text| text.chars().count())
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::{validate_server_runtime_output, ServerRuntimeOutputError};
    use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;
    use serde_json::json;

    #[test]
    fn accepts_message_only_or_small_actions() {
        let limits = test_limits();

        validate_server_runtime_output(&json!({"message": "ok"}), limits).unwrap();
        validate_server_runtime_output(
            &json!({
                "message": "ok",
                "actions": [
                    {"tool": "read_file", "path": "README.md"},
                    {"tool": "run_command", "program": "git", "args": ["status", "--short"]}
                ]
            }),
            limits,
        )
        .unwrap();
    }

    #[test]
    fn rejects_non_object_or_malformed_actions() {
        let limits = test_limits();

        assert_eq!(
            validate_server_runtime_output(&json!("oops"), limits).unwrap_err(),
            ServerRuntimeOutputError::ResponseNotObject
        );
        assert_eq!(
            validate_server_runtime_output(&json!({"actions": "oops"}), limits).unwrap_err(),
            ServerRuntimeOutputError::ActionsNotArray
        );
        assert_eq!(
            validate_server_runtime_output(&json!({"actions": ["oops"]}), limits).unwrap_err(),
            ServerRuntimeOutputError::ActionNotObject { index: 0 }
        );
        assert_eq!(
            validate_server_runtime_output(&json!({"actions": [{"path": "README.md"}]}), limits)
                .unwrap_err(),
            ServerRuntimeOutputError::MissingTool { index: 0 }
        );
    }

    #[test]
    fn rejects_action_budget_overflow_without_logging_payload() {
        let limits = test_limits();
        let too_many = json!({
            "actions": [
                {"tool": "read_file", "path": "a"},
                {"tool": "read_file", "path": "b"},
                {"tool": "read_file", "path": "c"}
            ]
        });

        assert_eq!(
            validate_server_runtime_output(&too_many, limits).unwrap_err(),
            ServerRuntimeOutputError::TooManyActions { count: 3, max: 2 }
        );

        let too_large = json!({"actions": [{"tool": "write_file", "content": "x".repeat(80)}]});
        let error = validate_server_runtime_output(&too_large, limits).unwrap_err();
        assert!(matches!(
            error,
            ServerRuntimeOutputError::ActionTooLarge { index: 0, .. }
        ));
        assert!(!error.public_message().contains('x'));
    }

    fn test_limits() -> ServerAgentRuntimeLimits {
        ServerAgentRuntimeLimits {
            max_messages: 8,
            max_message_chars: 1_000,
            max_total_chars: 2_000,
            max_output_tokens: 512,
            max_actions: 2,
            max_action_chars: 72,
            max_actions_total_chars: 120,
            max_requests_per_minute: 4,
            max_concurrent_per_user: 1,
            max_concurrent_global: 8,
            duplicate_request_window_secs: 5,
            temperature: 0.2,
        }
    }
}
