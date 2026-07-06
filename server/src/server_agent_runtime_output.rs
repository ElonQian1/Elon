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
#[path = "server_agent_runtime_output_tests.rs"]
mod tests;
