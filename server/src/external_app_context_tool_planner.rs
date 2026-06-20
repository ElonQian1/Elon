//! Tool planning for fb2 external context queries.

use serde_json::{json, Value};

use crate::external_app_context_config::infer_lottery_type;

#[derive(Clone)]
pub(crate) struct PlannedTool {
    pub(crate) name: &'static str,
    pub(crate) reason: &'static str,
    pub(crate) arguments: Value,
    pub(crate) requires_external_user: bool,
}

pub(crate) fn plan_fb2_tools(context: &Value, topic_hint: Option<&str>) -> Vec<PlannedTool> {
    let query = topic_hint.unwrap_or("").trim();
    let mut plans = Vec::new();

    if should_search_matches(context, query) {
        let mut arguments = json!({
            "query": query,
            "date": "today",
            "include_odds": true
        });
        if let Some(lottery_type) = infer_lottery_type(Some(query)) {
            arguments["lottery_type"] = json!(lottery_type);
        }
        plans.push(PlannedTool {
            name: "search_matches",
            reason: "用户在 fb2 群聊中询问比赛、赔率、预测或今日场次，需要补充可引用比赛候选。",
            arguments,
            requires_external_user: false,
        });
    }

    if contains_any(
        query,
        &[
            "订单",
            "我的票",
            "我的单",
            "票据",
            "方案",
            "下单",
            "已买",
            "串关",
        ],
    ) {
        plans.push(PlannedTool {
            name: "search_user_orders",
            reason: "用户提到自己的订单、票据或方案，需要只查询当前用户可见数据。",
            arguments: json!({
                "query": query,
                "scope": "current_user"
            }),
            requires_external_user: true,
        });
    }

    if contains_any(
        query,
        &[
            "群友", "大家", "观点", "讨论", "分歧", "群里", "建议", "采纳",
        ],
    ) {
        plans.push(PlannedTool {
            name: "search_group_opinions",
            reason: "用户要求总结群友观点、讨论分歧或采纳建议，需要检索群聊观点来源。",
            arguments: json!({
                "query": query
            }),
            requires_external_user: false,
        });
    }

    if should_query_audit(context) {
        if let Some(audit_id) = context.get("context_audit_id").and_then(Value::as_str) {
            plans.push(PlannedTool {
                name: "get_context_audit",
                reason: "当前 fb2 context pack 质量或预算状态有风险，需要回查来源和裁剪指标。",
                arguments: json!({
                    "context_audit_id": audit_id
                }),
                requires_external_user: false,
            });
        }
    }

    plans.truncate(3);
    plans
}

fn should_search_matches(context: &Value, query: &str) -> bool {
    contains_any(
        query,
        &[
            "今天",
            "比赛",
            "赛事",
            "场次",
            "赔率",
            "预测",
            "推荐",
            "竞彩",
            "北单",
            "足球",
            "篮球",
            "NBA",
            "让球",
            "大小分",
            "胜负",
        ],
    ) || context_has_warning(context, "empty_matches")
        || context_has_warning(context, "fb2_budget_too_large")
}

fn should_query_audit(context: &Value) -> bool {
    context_has_warning(context, "fb2_budget_empty")
        || context_has_warning(context, "fb2_budget_too_large")
        || context_has_warning(context, "missing_context_pack")
}

fn context_has_warning(context: &Value, warning: &str) -> bool {
    context
        .get("context_quality")
        .and_then(|quality| quality.get("warnings"))
        .and_then(Value::as_array)
        .map(|warnings| warnings.iter().any(|value| value.as_str() == Some(warning)))
        .unwrap_or(false)
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    if text.is_empty() {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| {
        let needle_lower = needle.to_ascii_lowercase();
        lower.contains(&needle_lower) || text.contains(needle)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_match_order_and_opinion_tools_from_user_request() {
        let plans = plan_fb2_tools(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("今天比赛怎么预测？顺便看看我的票和群友观点"),
        );

        let names = plans.iter().map(|plan| plan.name).collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "search_matches",
                "search_user_orders",
                "search_group_opinions"
            ]
        );
        assert!(plans[1].requires_external_user);
    }

    #[test]
    fn plans_audit_when_context_pack_quality_is_blocking() {
        let plans = plan_fb2_tools(
            &json!({
                "context_audit_id": "audit-1",
                "context_quality": {"warnings": ["missing_context_pack"]}
            }),
            Some("帮我看看"),
        );

        assert!(plans.iter().any(|plan| plan.name == "get_context_audit"));
    }
}
