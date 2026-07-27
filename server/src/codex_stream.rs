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
        "elon.ui_design.route" => {
            let message = match value.get("status").and_then(Value::as_str) {
                Some("AMBIGUOUS") => {
                    "当前表达可能涉及 UI，正在进行一次语义确认；确认后才会读取源码"
                }
                Some("LEARNED") => "已命中本项目的 UI 路由经验，跳过二次判断并进入实时调优",
                Some("CLUSTER_LEARNED") => "已命中受控近义经验簇，零 Token 复用已验证的 UI 判断",
                Some("LOCAL_CONFIRMED") => "本地规则已高置信度识别为 UI 任务，进入实时调优工具链",
                Some("READY") => "已识别 UI 样式任务，正在使用实时调优工具链（先预览，后写回源码）",
                _ => "已识别 UI 样式任务，但本地 UI 工件准备失败，正在安全降级诊断",
            };
            push_progress(&mut out, message.to_string());
        }
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
        "mcp_tool_call" | "dynamic_tool_call" | "tool_call" => {
            let tool = codex_tool_name(item);
            let args = item
                .get("arguments")
                .or_else(|| item.get("args"))
                .or_else(|| item.get("input"))
                .cloned()
                .unwrap_or_else(|| Value::Object(Default::default()));
            out.push(
                WsMessage::ToolCall {
                    tool: tool.clone(),
                    args,
                }
                .to_json(),
            );
            push_progress(out, ui_tool_progress(&tool, false));
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
        "mcp_tool_call" | "dynamic_tool_call" | "tool_call" => {
            let tool = codex_tool_name(item);
            let result = item
                .get("result")
                .or_else(|| item.get("output"))
                .map(|value| truncate_chars(&value.to_string(), 500))
                .unwrap_or_else(|| {
                    item.get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("完成")
                        .to_string()
                });
            out.push(
                WsMessage::ToolResult {
                    tool: tool.clone(),
                    result,
                }
                .to_json(),
            );
            push_progress(out, ui_tool_progress(&tool, true));
        }
        _ => {}
    }
}

fn codex_tool_name(item: &Value) -> String {
    item.get("tool")
        .or_else(|| item.get("name"))
        .or_else(|| item.get("tool_name"))
        .and_then(Value::as_str)
        .unwrap_or("mcp_tool")
        .to_string()
}

fn ui_tool_progress(tool: &str, completed: bool) -> String {
    let phase = match tool {
        "ui_confirm_route" => "正在确认本轮是否属于 UI 设计任务",
        "ui_get_project_profile" | "ui_get_design_task" => "正在读取项目 UI 档案和设计任务",
        "ui_get_runtime_status" => "正在检查真实 Android Renderer 是否已连接",
        "ui_get_screen_summary" => "正在读取当前页面的实时组件摘要",
        "ui_capture_pwa_runtime" => "正在用 PC 节点无头浏览器保存真实 PWA PNG 证据",
        "ui_verify_with_fallback" => "正在按 PWA 优先、Android 模拟器回退策略验证界面",
        "ui_get_node" | "ui_get_subtree" => "正在精准定位需要修改的组件",
        "ui_get_source_bundle" => "实时能力不足，正在按需读取最小源码片段",
        "ui_create_compose_screen_scaffold" => "正在创建全新页面骨架和 Preview",
        "ui_prepare_debug_runtime" => "正在首次构建并切换到真实 Android Renderer",
        "ui_bind_target_design" => "正在绑定目标设计图",
        "ui_map_annotations_to_nodes" => "正在把图片标注映射到真实组件",
        "ui_propose_live_patch" => "正在生成类型安全的实时样式预览",
        "ui_apply_live_patch" => "正在让真实 Android 组件立即重绘（尚未改源码）",
        "ui_get_current_crop" | "ui_get_target_crop" => "正在读取目标区域的精确画面",
        "ui_get_visual_diff" => "正在本地计算设计图与真实画面的差异",
        "ui_start_fit_run" => "正在启动可恢复的自动拟合任务",
        "ui_run_visual_solver" => "正在本地试算样式参数（不消耗模型 Token）",
        "ui_control_fit_run" => "正在推进拟合、源码写回与验收",
        "ui_get_commit_plan" => "正在规划确定性写回，避免重复读取源码",
        "ui_commit_bound_styles" => "正在把确认样式确定性写回源码",
        "ui_build_and_verify" => "正在重新构建并进行无临时 Patch 验收",
        _ if tool.starts_with("ui_") => "正在使用 UI 设计工具",
        _ => "AI 正在调用开发工具",
    };
    if completed {
        format!("{phase}：本步骤已完成")
    } else {
        format!("{phase}…")
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
