//! Codex CLI `--json` 流式事件解析。
//!
//! 单一职责：把 codex CLI 输出的一行 JSON 翻译成 0~N 条 WebSocket 消息
//! （`tool_call` / `tool_result` / `usage` / `progress`）。
//!
//! 从 `ai_cli.rs` 抽离，便于：
//! - 单元测试聚焦
//! - 后续新增更多 codex item.* 事件类型（例如 web_search、mcp_tool_call）
//!   时仅修改本模块
//! - 让 `ai_cli.rs` 主体回到“流程编排”职责，不再混入解析细节

use serde_json::Value;

use crate::ai_cli::truncate_chars;
use crate::types::WsMessage;

/// 将一行 codex --json 事件翻译成 0~N 条 WS 消息（已序列化的 JSON 字符串）。
///
/// `model_used`：当前轮次使用的模型名，附加到 AssistantMessage 让用户感知。
pub(crate) fn stream_event_to_ws_messages(line: &str, model_used: Option<&str>) -> Vec<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return Vec::new();
    };
    let Some(event_type) = value.get("type").and_then(Value::as_str) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    let model_used_owned = model_used.map(|s| s.to_string());
    let push_progress = |out: &mut Vec<String>, message: String| {
        out.push(WsMessage::progress(message).to_json());
    };

    match event_type {
        "item.started" => handle_item_started(&value, &mut out, push_progress),
        "item.completed" => {
            handle_item_completed(&value, &mut out, push_progress, model_used_owned.as_deref())
        }
        // codex --json 在每次 turn 完成时会发出 token_count 事件，
        // 或在 turn.completed 里携带 usage 字段。两种格式都尝试解析。
        "token_count" | "turn.completed" => handle_usage_event(&value, &mut out),
        _ => {}
    }
    out
}

fn handle_item_started<F>(value: &Value, out: &mut Vec<String>, push_progress: F)
where
    F: Fn(&mut Vec<String>, String),
{
    let Some(item) = value.get("item") else {
        return;
    };
    let Some(item_type) = item.get("type").and_then(Value::as_str) else {
        return;
    };
    match item_type {
        "agent_reasoning" => push_progress(out, "AI 正在思考……".to_string()),
        "command_execution" => {
            let cmd = item
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let mut args = serde_json::Map::new();
            if !cmd.is_empty() {
                args.insert("command".into(), Value::String(cmd.to_string()));
            }
            if let Some(id) = item.get("id").and_then(Value::as_str) {
                args.insert("id".into(), Value::String(id.to_string()));
            }
            if let Some(cwd) = item.get("cwd").and_then(Value::as_str) {
                args.insert("cwd".into(), Value::String(cwd.to_string()));
            }
            out.push(
                WsMessage::ToolCall {
                    tool: "shell".to_string(),
                    args: Value::Object(args),
                }
                .to_json(),
            );
            let snippet = truncate_chars(cmd, 80);
            if snippet.is_empty() {
                push_progress(out, "AI 正在执行命令……".to_string());
            } else {
                push_progress(out, format!("AI 执行命令：{}", snippet));
            }
        }
        "file_change" => {
            let mut args = serde_json::Map::new();
            if let Some(id) = item.get("id").and_then(Value::as_str) {
                args.insert("id".into(), Value::String(id.to_string()));
            }
            if let Some(changes) = item.get("changes") {
                args.insert("changes".into(), changes.clone());
            } else if let Some(path) = item.get("path").and_then(Value::as_str) {
                args.insert("path".into(), Value::String(path.to_string()));
            }
            out.push(
                WsMessage::ToolCall {
                    tool: "file_change".to_string(),
                    args: Value::Object(args),
                }
                .to_json(),
            );
            push_progress(out, "AI 正在修改文件……".to_string());
        }
        _ => {}
    }
}

fn handle_item_completed<F>(
    value: &Value,
    out: &mut Vec<String>,
    push_progress: F,
    model_used: Option<&str>,
) where
    F: Fn(&mut Vec<String>, String),
{
    let Some(item) = value.get("item") else {
        return;
    };
    let Some(item_type) = item.get("type").and_then(Value::as_str) else {
        return;
    };
    match item_type {
        "agent_message" => {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                let trimmed = text.trim();
                let display = trimmed.strip_prefix("用户可见：").unwrap_or(trimmed).trim();
                if !display.is_empty() {
                    out.push(
                        WsMessage::AssistantMessage {
                            text: display.to_string(),
                            model_used: model_used.map(|s| s.to_string()),
                            stream_id: None,
                            node_id: None,
                        }
                        .to_json(),
                    );
                }
            }
        }
        "command_execution" => {
            let exit_code = item.get("exit_code").and_then(Value::as_i64);
            let status = item
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let aggregated_output = item
                .get("aggregated_output")
                .and_then(Value::as_str)
                .or_else(|| item.get("output").and_then(Value::as_str))
                .unwrap_or_default();
            let result_snippet = truncate_chars(aggregated_output, 500);
            let result_text = if !result_snippet.is_empty() {
                if let Some(code) = exit_code {
                    format!("exit={} {}", code, result_snippet)
                } else {
                    result_snippet.clone()
                }
            } else if let Some(code) = exit_code {
                format!("exit={}", code)
            } else if !status.is_empty() {
                status.to_string()
            } else {
                "完成".to_string()
            };
            out.push(
                WsMessage::ToolResult {
                    tool: "shell".to_string(),
                    result: result_text,
                }
                .to_json(),
            );
            push_progress(out, "命令执行完毕".to_string());
        }
        "file_change" => {
            let status = item
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("completed");
            let summary = item
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let result = if summary.is_empty() {
                status.to_string()
            } else {
                format!("{} {}", status, truncate_chars(summary, 200))
            };
            out.push(
                WsMessage::ToolResult {
                    tool: "file_change".to_string(),
                    result,
                }
                .to_json(),
            );
            push_progress(out, "文件修改完毕".to_string());
        }
        _ => {}
    }
}

fn handle_usage_event(value: &Value, out: &mut Vec<String>) {
    let usage = value
        .get("usage")
        .or_else(|| value.get("token_count"))
        .or_else(|| value.get("info"))
        .unwrap_or(value);
    let pick_u64 = |obj: &Value, keys: &[&str]| -> Option<u64> {
        for k in keys {
            if let Some(n) = obj.get(*k).and_then(Value::as_u64) {
                return Some(n);
            }
        }
        None
    };
    let input = pick_u64(usage, &["input_tokens", "prompt_tokens", "input"]);
    let output = pick_u64(usage, &["output_tokens", "completion_tokens", "output"]);
    let total = pick_u64(usage, &["total_tokens", "total"]);
    let cached = pick_u64(
        usage,
        &["cached_input_tokens", "cache_read_input_tokens", "cached"],
    );
    let reasoning = pick_u64(
        usage,
        &["reasoning_output_tokens", "reasoning_tokens", "reasoning"],
    );
    let cost = usage
        .get("total_cost_usd")
        .and_then(Value::as_f64)
        .or_else(|| usage.get("cost_usd").and_then(Value::as_f64));
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .map(|s| s.to_string());
    if input.is_some()
        || output.is_some()
        || total.is_some()
        || cached.is_some()
        || reasoning.is_some()
        || cost.is_some()
    {
        out.push(
            WsMessage::Usage {
                input_tokens: input,
                output_tokens: output,
                total_tokens: total,
                cached_input_tokens: cached,
                reasoning_output_tokens: reasoning,
                total_cost_usd: cost,
                model,
            }
            .to_json(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn stream_event_emits_tool_call_and_progress_for_command_started() {
        let line = r#"{"type":"item.started","item":{"id":"call_1","type":"command_execution","command":"cargo check"}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 2);
        let tool: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(tool["type"], "tool_call");
        assert_eq!(tool["tool"], "shell");
        assert_eq!(tool["args"]["command"], "cargo check");
        assert_eq!(tool["args"]["id"], "call_1");
        let progress: Value = serde_json::from_str(&msgs[1]).unwrap();
        assert_eq!(progress["type"], "progress");
        assert!(progress["message"]
            .as_str()
            .unwrap()
            .contains("cargo check"));
    }

    #[test]
    fn stream_event_emits_tool_result_for_command_completed() {
        let line = r#"{"type":"item.completed","item":{"id":"call_1","type":"command_execution","command":"cargo check","exit_code":0,"aggregated_output":"Compiling foo\nFinished","status":"completed"}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 2);
        let tool: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(tool["type"], "tool_result");
        assert_eq!(tool["tool"], "shell");
        assert!(tool["result"].as_str().unwrap().contains("exit=0"));
        assert!(tool["result"].as_str().unwrap().contains("Compiling foo"));
    }

    #[test]
    fn stream_event_emits_tool_call_and_result_for_file_change() {
        let started = r#"{"type":"item.started","item":{"id":"fc_1","type":"file_change","changes":[{"path":"src/main.rs","kind":"modify"}]}}"#;
        let completed = r#"{"type":"item.completed","item":{"id":"fc_1","type":"file_change","status":"applied","summary":"1 file changed"}}"#;
        let s = stream_event_to_ws_messages(started, None);
        assert_eq!(s.len(), 2);
        let tool: Value = serde_json::from_str(&s[0]).unwrap();
        assert_eq!(tool["type"], "tool_call");
        assert_eq!(tool["tool"], "file_change");
        assert!(tool["args"]["changes"].is_array());

        let c = stream_event_to_ws_messages(completed, None);
        let result: Value = serde_json::from_str(&c[0]).unwrap();
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool"], "file_change");
        assert!(result["result"].as_str().unwrap().contains("applied"));
    }

    #[test]
    fn stream_event_emits_usage_event_for_token_count() {
        let line = r#"{"type":"token_count","model":"gpt-5-codex","usage":{"input_tokens":1200,"output_tokens":350,"total_tokens":1550,"cached_input_tokens":800,"total_cost_usd":0.0123}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 1);
        let usage: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(usage["type"], "usage");
        assert_eq!(usage["input_tokens"], 1200);
        assert_eq!(usage["output_tokens"], 350);
        assert_eq!(usage["total_tokens"], 1550);
        assert_eq!(usage["cached_input_tokens"], 800);
        assert_eq!(usage["total_cost_usd"], 0.0123);
        assert_eq!(usage["model"], "gpt-5-codex");
    }

    #[test]
    fn stream_event_emits_usage_event_for_turn_completed_with_usage() {
        let line = r#"{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":20}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 1);
        let usage: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(usage["type"], "usage");
        assert_eq!(usage["input_tokens"], 10);
        assert_eq!(usage["output_tokens"], 20);
    }

    #[test]
    fn stream_event_ignores_blank_and_unknown_events() {
        assert!(stream_event_to_ws_messages("", None).is_empty());
        assert!(stream_event_to_ws_messages("not json", None).is_empty());
        assert!(stream_event_to_ws_messages(r#"{"type":"unknown_event"}"#, None).is_empty());
    }

    #[test]
    fn stream_event_emits_assistant_message_for_agent_message_completed() {
        let line = r#"{"type":"item.completed","item":{"type":"agent_message","text":"  我已经读完了 main.rs，准备开始改造。  "}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 1);
        let value: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(value["type"], "assistant_message");
        assert_eq!(value["text"], "我已经读完了 main.rs，准备开始改造。");
    }

    #[test]
    fn stream_event_strips_yonghu_kejian_prefix() {
        let line = r#"{"type":"item.completed","item":{"type":"agent_message","text":"用户可见：正在读取 main.rs，马上修改。"}}"#;
        let msgs = stream_event_to_ws_messages(line, None);
        assert_eq!(msgs.len(), 1);
        let value: Value = serde_json::from_str(&msgs[0]).unwrap();
        assert_eq!(value["type"], "assistant_message");
        assert_eq!(value["text"], "正在读取 main.rs，马上修改。");
    }

    #[test]
    fn stream_event_skips_blank_after_prefix_strip() {
        let line =
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"用户可见：   "}}"#;
        assert!(stream_event_to_ws_messages(line, None).is_empty());
    }

    #[test]
    fn stream_event_skips_blank_agent_message() {
        let line = r#"{"type":"item.completed","item":{"type":"agent_message","text":"   "}}"#;
        assert!(stream_event_to_ws_messages(line, None).is_empty());
    }
}
