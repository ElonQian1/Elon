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
#[path = "codex_stream_tests.rs"]
mod tests;
