//! OpenAI-compatible tool call extraction for API agents.
//!
//! Some providers return strict `tool_calls`; older or loosely compatible
//! providers may return a legacy `function_call` or omit the expected
//! `finish_reason`. Keep parsing tolerant here so the agent loop stays small.

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolCallRequest {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) args: Value,
    pub(crate) legacy_function_call: bool,
}

pub(crate) fn extract_tool_calls(message: &Value) -> Vec<ToolCallRequest> {
    let mut calls = extract_modern_tool_calls(message);
    if calls.is_empty() {
        calls.extend(extract_legacy_function_call(message));
    }
    calls
}

fn extract_modern_tool_calls(message: &Value) -> Vec<ToolCallRequest> {
    message["tool_calls"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    let name = item["function"]["name"].as_str()?.trim();
                    if name.is_empty() {
                        return None;
                    }
                    let id = item["id"]
                        .as_str()
                        .filter(|value| !value.trim().is_empty())
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| format!("tool_call_{index}"));
                    let args = parse_tool_args(item["function"]["arguments"].as_str());
                    Some(ToolCallRequest {
                        id,
                        name: name.to_string(),
                        args,
                        legacy_function_call: false,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_legacy_function_call(message: &Value) -> Option<ToolCallRequest> {
    let function_call = &message["function_call"];
    let name = function_call["name"].as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    Some(ToolCallRequest {
        id: "legacy_function_call".to_string(),
        name: name.to_string(),
        args: parse_tool_args(function_call["arguments"].as_str()),
        legacy_function_call: true,
    })
}

fn parse_tool_args(raw: Option<&str>) -> Value {
    raw.and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_else(|| json!({}))
}

#[cfg(test)]
#[path = "agent_tool_calls_tests.rs"]
mod tests;
