// server/src/node_agent_api_runtime_tools.rs

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Map, Value};

pub(crate) fn add_tools_to_payload(payload: &mut Value) {
    payload["tools"] = tool_definitions();
    payload["tool_choice"] = json!("auto");
}

pub(crate) fn agent_response_from_tool_calls(response: &Value) -> Result<Option<String>> {
    let Some(message) = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
    else {
        return Ok(None);
    };
    let actions = tool_call_actions(message)?;
    if actions.is_empty() {
        return Ok(None);
    }

    let message = message
        .get("content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("模型请求执行 {} 个工具", actions.len()));
    serde_json::to_string(&json!({
        "message": message,
        "done": false,
        "actions": actions,
    }))
    .map(Some)
    .context("无法序列化 Route B tool_calls")
}

pub(crate) fn should_retry_without_tools(status: reqwest::StatusCode, body: &str) -> bool {
    if !matches!(
        status,
        reqwest::StatusCode::BAD_REQUEST
            | reqwest::StatusCode::UNPROCESSABLE_ENTITY
            | reqwest::StatusCode::NOT_IMPLEMENTED
    ) {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    lower.contains("tools")
        || lower.contains("tool_choice")
        || lower.contains("tool_calls")
        || lower.contains("function calling")
        || lower.contains("functions")
        || lower.contains("unsupported parameter")
        || lower.contains("unknown parameter")
        || lower.contains("unrecognized request argument")
}

fn action_from_tool_call(tool_call: &Value) -> Result<Value> {
    let function = tool_call
        .get("function")
        .ok_or_else(|| anyhow!("missing function"))?;
    let name = function
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("missing function.name"))?;
    let mut args = function_arguments(function)?;
    args.insert("tool".to_string(), json!(name));
    if let Some(tool_call_id) = tool_call
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.insert("tool_call_id".to_string(), json!(tool_call_id));
    }
    Ok(Value::Object(args))
}

fn tool_call_actions(message: &Value) -> Result<Vec<Value>> {
    if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
        let mut actions = Vec::with_capacity(tool_calls.len());
        for (index, tool_call) in tool_calls.iter().enumerate() {
            actions.push(action_from_tool_call(tool_call).with_context(|| {
                format!("本机 API runtime tool_calls[{index}] 不是有效工具调用")
            })?);
        }
        if !actions.is_empty() {
            return Ok(actions);
        }
    }

    if let Some(function_call) = message
        .get("function_call")
        .filter(|value| value.is_object())
    {
        return action_from_legacy_function_call(function_call)
            .map(|action| vec![action])
            .context("本机 API runtime function_call 不是有效工具调用");
    }

    Ok(Vec::new())
}

fn action_from_legacy_function_call(function_call: &Value) -> Result<Value> {
    let name = function_call
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("missing function_call.name"))?;
    let mut args = function_arguments(function_call)?;
    args.insert("tool".to_string(), json!(name));
    args.insert("tool_call_id".to_string(), json!("legacy_function_call"));
    Ok(Value::Object(args))
}

fn function_arguments(function: &Value) -> Result<Map<String, Value>> {
    match function.get("arguments") {
        Some(Value::String(raw)) if raw.trim().is_empty() => Ok(Map::new()),
        Some(Value::String(raw)) => {
            let value: Value =
                serde_json::from_str(raw).context("function.arguments is not JSON")?;
            value
                .as_object()
                .cloned()
                .ok_or_else(|| anyhow!("function.arguments must be a JSON object"))
        }
        Some(Value::Object(object)) => Ok(object.clone()),
        Some(_) => bail!("function.arguments must be a JSON object or string"),
        None => Ok(Map::new()),
    }
}

fn tool_definitions() -> Value {
    json!([
        function_tool(
            "list_dir",
            "List files in a project-relative directory.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project-relative directory path. Use . for the workspace root." }
                },
                "additionalProperties": false
            })
        ),
        function_tool(
            "search_files",
            "Search project-relative file names and text contents without modifying files.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Text to search for in file paths and text contents." },
                    "path": { "type": "string", "description": "Optional project-relative directory to search. Defaults to the workspace root." },
                    "max_results": { "type": "integer", "minimum": 1, "maximum": 200 }
                },
                "required": ["query"],
                "additionalProperties": false
            })
        ),
        function_tool(
            "file_info",
            "Inspect one project-relative file or directory before deciding whether to read it.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project-relative file or directory path." }
                },
                "required": ["path"],
                "additionalProperties": false
            })
        ),
        function_tool(
            "read_file",
            "Read a small project-relative text file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project-relative file path." }
                },
                "required": ["path"],
                "additionalProperties": false
            })
        ),
        function_tool(
            "read_file_range",
            "Read a numbered line range from a project-relative text file.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project-relative file path." },
                    "start_line": { "type": "integer", "minimum": 1 },
                    "line_count": { "type": "integer", "minimum": 1, "maximum": 400 }
                },
                "required": ["path", "start_line", "line_count"],
                "additionalProperties": false
            })
        ),
        function_tool(
            "git_status",
            "Inspect git status for the current project without modifying files.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            })
        ),
        function_tool(
            "git_diff",
            "Inspect the current project git diff without modifying files.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Optional project-relative file or directory to limit the diff. Defaults to the workspace root." },
                    "cached": { "type": "boolean", "description": "Show staged changes instead of unstaged changes." },
                    "stat": { "type": "boolean", "description": "Show only diff stat instead of the full diff." }
                },
                "additionalProperties": false
            })
        ),
        function_tool(
            "git_log",
            "Inspect recent project git history without modifying files.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Optional project-relative file or directory to limit history. Defaults to the workspace root." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100, "description": "Maximum number of commits to return. Defaults to 20." }
                },
                "additionalProperties": false
            })
        ),
        function_tool(
            "write_file",
            "Create or replace a project-relative text file. Requires user approval when writes are enabled.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project-relative file path." },
                    "content": { "type": "string", "description": "Full file content." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            })
        ),
        function_tool(
            "apply_patch",
            "Apply a unified diff patch inside the project workspace. Requires user approval when writes are enabled.",
            json!({
                "type": "object",
                "properties": {
                    "patch": { "type": "string", "description": "Unified diff patch." },
                    "check_only": { "type": "boolean", "description": "Validate without applying." },
                    "reason": { "type": "string", "description": "Short reason for the patch." }
                },
                "required": ["patch"],
                "additionalProperties": false
            })
        ),
        function_tool(
            "run_command",
            "Run an allowed project Git/build/test command. Requires user approval when command execution is enabled.",
            json!({
                "type": "object",
                "properties": {
                    "program": { "type": "string", "description": "Executable name, such as git, cargo, npm, pnpm, bun, python, go, dotnet, or gradle." },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Program arguments, without shell metacharacters."
                    },
                    "reason": { "type": "string", "description": "Short reason for running the command." }
                },
                "required": ["program", "args", "reason"],
                "additionalProperties": false
            })
        )
    ])
}

fn function_tool(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{add_tools_to_payload, agent_response_from_tool_calls, should_retry_without_tools};
    use serde_json::json;

    #[test]
    fn payload_adds_route_b_tool_definitions() {
        let mut payload = json!({
            "model": "gpt-test",
            "messages": []
        });

        add_tools_to_payload(&mut payload);

        assert_eq!(payload["tool_choice"], "auto");
        let tools = payload["tools"].as_array().unwrap();
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "search_files"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "file_info"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "read_file_range"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "git_status"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "git_diff"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "git_log"));
        assert!(tools
            .iter()
            .any(|tool| tool["function"]["name"] == "apply_patch"));
    }

    #[test]
    fn tool_calls_are_converted_to_existing_action_schema() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "read_file_range",
                            "arguments": "{\"path\":\"src/main.rs\",\"start_line\":10,\"line_count\":20}"
                        }
                    }]
                }
            }]
        });

        let content = agent_response_from_tool_calls(&response)
            .unwrap()
            .expect("tool calls should be converted");
        let agent: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(agent["done"], false);
        assert_eq!(agent["actions"][0]["tool"], "read_file_range");
        assert_eq!(agent["actions"][0]["tool_call_id"], "call_1");
        assert_eq!(agent["actions"][0]["path"], "src/main.rs");
        assert_eq!(agent["actions"][0]["start_line"], 10);
    }

    #[test]
    fn legacy_function_call_is_converted_to_existing_action_schema() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": null,
                    "function_call": {
                        "name": "read_file",
                        "arguments": "{\"path\":\"README.md\"}"
                    }
                }
            }]
        });

        let content = agent_response_from_tool_calls(&response)
            .unwrap()
            .expect("legacy function call should be converted");
        let agent: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(agent["done"], false);
        assert_eq!(agent["actions"][0]["tool"], "read_file");
        assert_eq!(agent["actions"][0]["tool_call_id"], "legacy_function_call");
        assert_eq!(agent["actions"][0]["path"], "README.md");
    }

    #[test]
    fn modern_tool_calls_take_precedence_over_legacy_function_call() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "list_dir",
                            "arguments": {"path":"src"}
                        }
                    }],
                    "function_call": {
                        "name": "read_file",
                        "arguments": "{\"path\":\"README.md\"}"
                    }
                }
            }]
        });

        let content = agent_response_from_tool_calls(&response)
            .unwrap()
            .expect("tool calls should be converted");
        let agent: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(agent["actions"].as_array().unwrap().len(), 1);
        assert_eq!(agent["actions"][0]["tool"], "list_dir");
        assert_eq!(agent["actions"][0]["tool_call_id"], "call_1");
        assert_eq!(agent["actions"][0]["path"], "src");
    }

    #[test]
    fn invalid_tool_call_arguments_fail_fast() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "function": {
                            "name": "read_file",
                            "arguments": "not json"
                        }
                    }]
                }
            }]
        });

        let error = agent_response_from_tool_calls(&response).unwrap_err();
        assert!(format!("{error:#}").contains("function.arguments is not JSON"));
    }

    #[test]
    fn invalid_legacy_function_call_arguments_fail_fast() {
        let response = json!({
            "choices": [{
                "message": {
                    "function_call": {
                        "name": "read_file",
                        "arguments": "not json"
                    }
                }
            }]
        });

        let error = agent_response_from_tool_calls(&response).unwrap_err();
        assert!(format!("{error:#}").contains("function.arguments is not JSON"));
    }

    #[test]
    fn tool_retry_is_limited_to_compatibility_errors() {
        assert!(should_retry_without_tools(
            reqwest::StatusCode::BAD_REQUEST,
            "Unrecognized request argument supplied: tools"
        ));
        assert!(should_retry_without_tools(
            reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            "function calling is not supported by this model"
        ));
        assert!(!should_retry_without_tools(
            reqwest::StatusCode::UNAUTHORIZED,
            "invalid api key"
        ));
    }
}
