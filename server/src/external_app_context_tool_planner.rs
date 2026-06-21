//! Tool planning for fb2 external context queries.

use serde_json::{json, Value};

use crate::external_app_context_config::{infer_lottery_type, platform_order_summary_enabled};

const PLANNER_SCHEMA: &str = "external_app.tool_plan.v1";
const PLANNER_STRATEGY: &str = "deterministic_fb2_chat_v1";

#[derive(Clone)]
pub(crate) struct PlannedTool {
    pub(crate) name: &'static str,
    pub(crate) reason: &'static str,
    pub(crate) arguments: Value,
    pub(crate) requires_external_user: bool,
    trigger: &'static str,
    confidence: u8,
    evidence: Vec<String>,
}

pub(crate) struct Fb2ToolPlan {
    topic_hint: String,
    pub(crate) tools: Vec<PlannedTool>,
    skipped_reasons: Vec<&'static str>,
}

impl Fb2ToolPlan {
    pub(crate) fn tool_names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|tool| tool.name).collect()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub(crate) fn into_tools(self) -> Vec<PlannedTool> {
        self.tools
    }

    pub(crate) fn to_metadata(&self) -> Value {
        json!({
            "schema": PLANNER_SCHEMA,
            "strategy": PLANNER_STRATEGY,
            "topic_hint": self.topic_hint,
            "planned_count": self.tools.len(),
            "planned_tools": self.tools.iter().map(PlannedTool::to_metadata).collect::<Vec<_>>(),
            "skipped_reasons": self.skipped_reasons
        })
    }
}

impl PlannedTool {
    fn to_metadata(&self) -> Value {
        json!({
            "name": self.name,
            "reason": self.reason,
            "trigger": self.trigger,
            "confidence": self.confidence,
            "requires_external_user": self.requires_external_user,
            "evidence": self.evidence
        })
    }
}

pub(crate) fn plan_fb2_tools(context: &Value, topic_hint: Option<&str>) -> Fb2ToolPlan {
    plan_fb2_tools_with_platform_scope(context, topic_hint, platform_order_summary_enabled())
}

fn plan_fb2_tools_with_platform_scope(
    context: &Value,
    topic_hint: Option<&str>,
    allow_platform_orders: bool,
) -> Fb2ToolPlan {
    let query = topic_hint.unwrap_or("").trim();
    let mut plans = Vec::new();
    let mut skipped_reasons = Vec::new();

    if let Some(evidence) = match_evidence(context, query) {
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
            trigger: "match_context_needed",
            confidence: confidence_for(&evidence),
            evidence,
        });
    }

    if let Some(evidence) = keyword_evidence(
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
            trigger: "current_user_order_needed",
            confidence: confidence_for(&evidence),
            evidence,
        });
    }

    if let Some(evidence) = keyword_evidence(
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
            trigger: "group_opinion_needed",
            confidence: confidence_for(&evidence),
            evidence,
        });
    }

    if let Some(evidence) = keyword_evidence(
        query,
        &[
            "平台",
            "全平台",
            "店铺",
            "订单风险",
            "投注集中",
            "集中",
            "派奖",
            "毛利",
            "销量",
            "成交",
            "赔付",
            "风险",
        ],
    ) {
        if allow_platform_orders {
            plans.push(PlannedTool {
                name: "platform_orders",
                reason: "用户要求平台或店铺维度订单风险概览，只能查询匿名聚合摘要。",
                arguments: json!({
                    "scope": "platform_order_summary",
                    "redaction": "anonymous_aggregate_only"
                }),
                requires_external_user: false,
                trigger: "platform_order_summary_needed",
                confidence: confidence_for(&evidence),
                evidence,
            });
        } else {
            skipped_reasons.push("platform_order_summary_disabled");
        }
    }

    if let Some(evidence) = audit_evidence(context) {
        if let Some(audit_id) = context.get("context_audit_id").and_then(Value::as_str) {
            plans.push(PlannedTool {
                name: "get_context_audit",
                reason: "当前 fb2 context pack 质量或预算状态有风险，需要回查来源和裁剪指标。",
                arguments: json!({
                    "context_audit_id": audit_id
                }),
                requires_external_user: false,
                trigger: "context_quality_audit_needed",
                confidence: confidence_for(&evidence),
                evidence,
            });
        } else {
            skipped_reasons.push("context_quality_warning_without_context_audit_id");
        }
    }

    plans.truncate(4);
    if plans.is_empty() {
        skipped_reasons.push("no_fb2_tool_trigger_matched");
    }
    Fb2ToolPlan {
        topic_hint: query.to_string(),
        tools: plans,
        skipped_reasons,
    }
}

fn match_evidence(context: &Value, query: &str) -> Option<Vec<String>> {
    let mut evidence = Vec::new();
    if let Some(keywords) = keyword_evidence(
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
    ) {
        evidence.extend(keywords);
    }
    if context_has_warning(context, "empty_matches") {
        evidence.push("context_quality.warning.empty_matches".to_string());
    }
    if context_has_warning(context, "fb2_budget_too_large") {
        evidence.push("context_quality.warning.fb2_budget_too_large".to_string());
    }

    if evidence.is_empty() {
        None
    } else {
        Some(evidence)
    }
}

fn audit_evidence(context: &Value) -> Option<Vec<String>> {
    let mut evidence = Vec::new();
    for warning in [
        "fb2_budget_empty",
        "fb2_budget_too_large",
        "missing_context_pack",
    ] {
        if context_has_warning(context, warning) {
            evidence.push(format!("context_quality.warning.{warning}"));
        }
    }
    if evidence.is_empty() {
        None
    } else {
        Some(evidence)
    }
}

fn context_has_warning(context: &Value, warning: &str) -> bool {
    context
        .get("context_quality")
        .and_then(|quality| quality.get("warnings"))
        .and_then(Value::as_array)
        .map(|warnings| warnings.iter().any(|value| value.as_str() == Some(warning)))
        .unwrap_or(false)
}

fn keyword_evidence(text: &str, needles: &[&str]) -> Option<Vec<String>> {
    if text.is_empty() {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    let evidence = needles
        .iter()
        .filter(|needle| {
            let needle_lower = needle.to_ascii_lowercase();
            lower.contains(&needle_lower) || text.contains(**needle)
        })
        .map(|needle| format!("query.keyword.{needle}"))
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        None
    } else {
        Some(evidence)
    }
}

fn confidence_for(evidence: &[String]) -> u8 {
    (60 + evidence.len().saturating_sub(1) as u8 * 10).min(90)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_match_order_and_opinion_tools_from_user_request() {
        let plan = plan_fb2_tools(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("今天比赛怎么预测？顺便看看我的票和群友观点"),
        );

        let names = plan.tool_names();
        assert_eq!(
            names,
            vec![
                "search_matches",
                "search_user_orders",
                "search_group_opinions"
            ]
        );
        assert!(plan.tools[1].requires_external_user);
        assert_eq!(
            plan.to_metadata()["planned_tools"][0]["trigger"].as_str(),
            Some("match_context_needed")
        );
    }

    #[test]
    fn plans_audit_when_context_pack_quality_is_blocking() {
        let plan = plan_fb2_tools(
            &json!({
                "context_audit_id": "audit-1",
                "context_quality": {"warnings": ["missing_context_pack"]}
            }),
            Some("帮我看看"),
        );

        assert!(plan
            .tools
            .iter()
            .any(|tool| tool.name == "get_context_audit"));
        assert!(plan.to_metadata()["planned_tools"][0]["evidence"]
            .as_array()
            .unwrap()
            .contains(&json!("context_quality.warning.missing_context_pack")));
    }

    #[test]
    fn plans_platform_orders_only_when_privileged_scope_is_enabled() {
        let disabled = plan_fb2_tools_with_platform_scope(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("平台今天订单风险集中在哪些方向？"),
            false,
        );
        assert!(!disabled.tool_names().contains(&"platform_orders"));
        assert!(disabled.to_metadata()["skipped_reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("platform_order_summary_disabled")));

        let enabled = plan_fb2_tools_with_platform_scope(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("平台今天订单风险集中在哪些方向？"),
            true,
        );
        assert!(enabled.tool_names().contains(&"platform_orders"));
        let metadata = enabled.to_metadata();
        let platform_tool = metadata["planned_tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"].as_str() == Some("platform_orders"))
            .unwrap();
        assert_eq!(
            platform_tool["trigger"].as_str(),
            Some("platform_order_summary_needed")
        );
    }

    #[test]
    fn records_skipped_reason_when_no_tool_matches() {
        let plan = plan_fb2_tools(&json!({"context_quality": {"warnings": []}}), Some("你好"));

        assert!(plan.is_empty());
        assert!(plan.to_metadata()["skipped_reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("no_fb2_tool_trigger_matched")));
    }
}
