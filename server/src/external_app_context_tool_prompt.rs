//! Prompt projection for executed external app tools.

use serde_json::Value;

const MAX_PROMPT_TOOL_JSON_CHARS: usize = 6_000;

pub(crate) fn prompt_executed_tools_block(execution: Option<&Value>) -> String {
    let Some(execution) = execution else {
        return String::new();
    };
    let status = execution
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let app_id = execution
        .get("app_id")
        .and_then(Value::as_str)
        .unwrap_or("external");
    let executed_at = execution
        .get("executed_at")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut body = serde_json::to_string_pretty(execution).unwrap_or_else(|_| "{}".to_string());
    if body.chars().count() > MAX_PROMPT_TOOL_JSON_CHARS {
        body = body
            .chars()
            .take(MAX_PROMPT_TOOL_JSON_CHARS)
            .collect::<String>();
        body.push_str("\n... [external app tool results truncated]");
    }

    format!(
        "按需执行的外部项目工具：\n\
         <executed_external_app_tools app_id=\"{app_id}\" status=\"{status}\" executed_at=\"{executed_at}\">\n\
         {body}\n\
         <tool_result_rules>\n\
         - 只有 status=ready 且 success=true 的单项结果可以当作已查询事实引用。\n\
         - plan.planned_tools 只说明为什么选择工具；不能把计划本身当作已经查询到的比赛、订单或观点事实。\n\
         - plan 中的 trigger、confidence、evidence 可用于解释本次为什么查询或为什么没有查询。\n\
         - skipped、failed、unavailable 结果只能作为数据缺口说明，不能编造成比赛、赔率、订单或群友观点事实。\n\
         - 引用工具结果时优先带 match_id、order_id、ticket_id、message_id 或 context_audit_id。\n\
         - 如果使用工具事实给出分析，回答中要自然写出关键来源 ID；没有 source_ids 时要说明 fb2 工具结果缺少可追溯 ID。\n\
         - 当前用户订单只允许基于 current_user_only 结果分析，不能推断或暴露其他用户明细。\n\
         </tool_result_rules>\n\
         </executed_external_app_tools>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rendered_tool_block_marks_success_as_required_for_facts() {
        let block = prompt_executed_tools_block(Some(&json!({
            "schema": "external_app.executed_tools.v1",
            "app_id": "fb2",
            "status": "ready",
            "executed_at": "2026-06-21T00:00:00Z",
            "results": [{"tool_name": "search_matches", "status": "ready", "success": true}]
        })));

        assert!(block.contains("<executed_external_app_tools"));
        assert!(block.contains("success=true"));
        assert!(block.contains("不能编造"));
        assert!(block.contains("来源 ID"));
    }
}
