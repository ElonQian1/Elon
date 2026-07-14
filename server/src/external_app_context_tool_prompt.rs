//! server/src/external_app_context_tool_prompt.rs
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
    let fact_summary = prompt_tool_fact_summary(execution);
    let gap_summary = prompt_tool_gap_summary(execution);

    format!(
        "按需执行的外部项目工具：\n\
         <executed_external_app_tools app_id=\"{app_id}\" status=\"{status}\" executed_at=\"{executed_at}\">\n\
         {fact_summary}\
         {gap_summary}\
         {body}\n\
         <tool_result_rules>\n\
         - 只有 status=ready、success=true 且 grounding.status=grounded 的单项结果可以作为强事实引用。\n\
         - grounding.status=weak 的结果只能谨慎使用，并必须说明缺少 source_ids、visibility 或其他追溯信息。\n\
         - grounding.status=unsafe 的结果不能用于比赛、赔率、订单或群友观点事实。\n\
         - plan.planned_tools 只说明为什么选择工具；不能把计划本身当作已经查询到的比赛、订单或观点事实。\n\
         - plan 中的 trigger、confidence、evidence 可用于解释本次为什么查询或为什么没有查询。\n\
         - skipped、failed、unavailable 结果只能作为数据缺口说明，不能编造成比赛、赔率、订单或群友观点事实。\n\
         - 引用工具结果时优先带 match_id、order_id、ticket_id、message_id 或 context_audit_id。\n\
         - 如果使用工具事实给出分析，回答中要自然写出关键来源 ID；没有 source_ids 时要说明 fb2 工具结果缺少可追溯 ID。\n\
         - 当前用户订单只允许基于 current_user_only 结果，或已按 external_user_id + X-FB2-AI-CONTEXT-USER-ID 裁剪的 Context Pack user_orders / match_focused_brief.data.user_orders；不能推断或暴露其他用户明细。\n\
         - 如果 Context Pack 或 match_focused_brief 已经包含 user_orders，后续 search_user_orders unavailable 只表示补充查询失败，不能否定已有本人订单事实。\n\
         - single_group_lightweight_memory 只代表本群轻量观点摘要；必须标注为群友观点，不得当作比赛事实。\n\
         - match_focused_brief 是比赛候选、群观点和可选本人订单的组合简报；data.user_orders 非空时可作为当前用户订单来源，回答必须拆开说明比赛事实、群友观点、本人订单和 AI 推断。\n\
         - single_group_persistent_opinion_index 只代表群友历史观点记忆；可用于说明“群友过去怎么看”，不能当作比赛事实、赔率事实或命中承诺。\n\
         - answer_opinion_adoption_* 只代表主项目 AI 曾经采纳或引用过哪些群观点；它不是比赛事实，也不能证明观点正确。\n\
         - single_group_opinion_result_review_* 只代表历史赛后复盘或统计；可用于说明历史表现和样本限制，不能承诺未来投注命中。\n\
         </tool_result_rules>\n\
         </executed_external_app_tools>"
    )
}

fn prompt_tool_gap_summary(execution: &Value) -> String {
    let lines = execution
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|result| {
            matches!(
                result.get("status").and_then(Value::as_str),
                Some("skipped" | "failed" | "unavailable")
            )
        })
        .filter_map(tool_gap_summary_for_result)
        .take(8)
        .collect::<Vec<_>>();

    if lines.is_empty() {
        String::new()
    } else {
        format!(
            "<tool_gap_summary>\n{}\n</tool_gap_summary>\n",
            lines.join("\n")
        )
    }
}

fn tool_gap_summary_for_result(result: &Value) -> Option<String> {
    let tool_name = result
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("unknown_tool");
    let status = result
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let reason = first_value_as_string(
        result,
        &[
            "reason",
            "error_code",
            "error",
            "message",
            "skipped_reason",
            "fallback_reason",
        ],
    )
    .unwrap_or_else(|| "unspecified".to_string());
    Some(format!(
        "- {tool_name}: status={status}, reason={reason}. 这只是数据缺口，不是比赛、赔率、订单或群友观点事实。"
    ))
}

fn prompt_tool_fact_summary(execution: &Value) -> String {
    let lines = execution
        .get("results")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|result| result.get("status").and_then(Value::as_str) == Some("ready"))
        .filter_map(order_fact_summary_for_result)
        .collect::<Vec<_>>();

    if lines.is_empty() {
        String::new()
    } else {
        format!(
            "<tool_fact_summary>\n{}\n</tool_fact_summary>\n",
            lines.join("\n")
        )
    }
}

fn order_fact_summary_for_result(result: &Value) -> Option<String> {
    let tool_name = result
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("unknown_tool");
    let visibility = result
        .get("visibility")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let orders = result
        .get("data")
        .and_then(|data| data.get("user_orders").or_else(|| data.get("orders")))
        .and_then(Value::as_array)?;
    if orders.is_empty() {
        return None;
    }

    let samples = orders
        .iter()
        .filter_map(compact_order_sample)
        .take(4)
        .collect::<Vec<_>>();
    if samples.is_empty() {
        return None;
    }

    Some(format!(
        "- {tool_name}: current_user_order_count={}, visibility={visibility}; samples=[{}]. 这些订单已由 fb2 按当前用户权限裁剪，可用于“我的票”分析。",
        orders.len(),
        samples.join(" | ")
    ))
}

fn compact_order_sample(order: &Value) -> Option<String> {
    let id = first_value_as_string(order, &["order_id", "id", "ticket_id"])?;
    let status = first_value_as_string(order, &["status", "order_status"])
        .unwrap_or_else(|| "unknown".to_string());
    let amount = first_value_as_string(order, &["total_amount", "amount", "stake"])
        .unwrap_or_else(|| "unknown".to_string());
    let slip_count = order
        .get("bet_slips")
        .or_else(|| order.get("slips"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let first_slip = order
        .get("bet_slips")
        .or_else(|| order.get("slips"))
        .and_then(Value::as_array)
        .and_then(|slips| slips.first())
        .and_then(compact_slip_sample);

    let mut parts = vec![
        format!("order_id={id}"),
        format!("status={status}"),
        format!("amount={amount}"),
        format!("slips={slip_count}"),
    ];
    if let Some(slip) = first_slip {
        parts.push(format!("first_slip={slip}"));
    }
    Some(parts.join(", "))
}

fn compact_slip_sample(slip: &Value) -> Option<String> {
    let home = first_value_as_string(slip, &["home_team"])?;
    let away = first_value_as_string(slip, &["away_team"])?;
    let selection = first_value_as_string(slip, &["selection", "pick", "bet_selection"])
        .unwrap_or_else(|| "unknown".to_string());
    let odds = first_value_as_string(slip, &["odds", "actual_odds", "original_odds"])
        .unwrap_or_else(|| "unknown".to_string());
    Some(format!("{home} vs {away} {selection} odds={odds}"))
}

fn first_value_as_string(value: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| {
        let raw = value.get(*field)?;
        match raw {
            Value::String(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(flag) => Some(flag.to_string()),
            _ => None,
        }
    })
}

#[cfg(test)]
#[path = "external_app_context_tool_prompt_tests.rs"]
mod tests;
