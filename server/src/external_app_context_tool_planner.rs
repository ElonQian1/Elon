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
    let mut match_analysis_brief_planned = false;
    let mut group_opinion_summary_planned = false;

    if let Some(evidence) = match_evidence(context, query) {
        let mut brief_arguments = json!({
            "topic_hint": query,
            "limit": 6,
            "order_limit": 8
        });
        if let Some(lottery_type) = infer_lottery_type(Some(query)) {
            brief_arguments["lottery_type"] = json!(lottery_type);
        }
        plans.push(PlannedTool {
            name: "match_analysis_brief",
            reason: "用户询问 fb2 比赛、预测、赔率或今日场次，优先读取比赛候选、群观点摘要和可选本人订单简报。",
            arguments: brief_arguments,
            requires_external_user: false,
            trigger: "match_analysis_brief_needed",
            confidence: confidence_for(&evidence),
            evidence: evidence.clone(),
        });
        match_analysis_brief_planned = true;

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
        if !match_analysis_brief_planned {
            plans.push(PlannedTool {
                name: "match_analysis_brief",
                reason: "用户要求分析自己的票，需要补充相关比赛候选和群观点简报；若已绑定 fb2 用户则只混入本人订单。",
                arguments: json!({
                    "topic_hint": query,
                    "limit": 6,
                    "order_limit": 8
                }),
                requires_external_user: false,
                trigger: "user_ticket_match_brief_needed",
                confidence: 70,
                evidence: vec!["query.intent.current_user_ticket_review".to_string()],
            });
        }
    }

    if let Some(evidence) = keyword_evidence(
        query,
        &[
            "群友", "大家", "观点", "讨论", "分歧", "群里", "建议", "采纳",
        ],
    ) {
        plans.push(PlannedTool {
            name: "group_opinion_summary",
            reason: "用户要求总结群友观点、讨论分歧或采纳建议，优先读取本群轻量观点摘要。",
            arguments: json!({
                "query": query,
                "limit": 12
            }),
            requires_external_user: false,
            trigger: "group_opinion_summary_needed",
            confidence: confidence_for(&evidence),
            evidence: evidence.clone(),
        });
        group_opinion_summary_planned = true;

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
            "群友", "大家", "观点", "建议", "采纳", "记住", "记忆", "长期", "以前", "之前", "历史",
            "复盘",
        ],
    ) {
        if !group_opinion_summary_planned {
            plans.push(PlannedTool {
                name: "group_opinion_summary",
                reason:
                    "用户要求参考群友历史观点或长期记忆，先读取本群轻量观点摘要作为当前讨论概览。",
                arguments: json!({
                    "query": query,
                    "limit": 12
                }),
                requires_external_user: false,
                trigger: "group_opinion_summary_needed",
                confidence: confidence_for(&evidence),
                evidence: evidence.clone(),
            });
        }
        plans.push(PlannedTool {
            name: "opinion_memories",
            reason: "用户要求参考群友历史观点、建议或长期记忆，需要检索 fb2 群观点记忆索引。",
            arguments: json!({
                "query": query,
                "include_expired": false
            }),
            requires_external_user: false,
            trigger: "group_opinion_memory_needed",
            confidence: confidence_for(&evidence),
            evidence,
        });
    }

    if let Some(evidence) = keyword_evidence(
        query,
        &[
            "采纳",
            "引用",
            "采用",
            "被采纳",
            "AI采纳",
            "AI 回复",
            "AI回答",
        ],
    ) {
        plans.push(PlannedTool {
            name: "opinion_adoption_summary",
            reason: "用户询问主项目 AI 回答采纳过哪些群观点或采纳质量，需要查询本群观点采纳汇总。",
            arguments: json!({
                "query": query
            }),
            requires_external_user: false,
            trigger: "opinion_adoption_summary_needed",
            confidence: confidence_for(&evidence),
            evidence: evidence.clone(),
        });
        if has_any_keyword(
            query,
            &["哪些", "明细", "样本", "列表", "谁", "哪条", "具体"],
        ) {
            plans.push(PlannedTool {
                name: "list_opinion_adoptions",
                reason: "用户要求查看被 AI 采纳的群观点样本，需要查询本群观点采纳记录列表。",
                arguments: json!({
                    "query": query,
                    "limit": 12
                }),
                requires_external_user: false,
                trigger: "opinion_adoption_samples_needed",
                confidence: confidence_for(&evidence),
                evidence,
            });
        }
    }

    if let Some(evidence) = keyword_evidence(
        query,
        &[
            "赛后复盘",
            "复盘",
            "命中",
            "准不准",
            "说对",
            "说错",
            "对吗",
            "对不对",
            "是否正确",
            "靠谱吗",
            "靠谱不",
            "合理吗",
            "有没有道理",
            "这条消息",
            "这句",
            "这段",
            "判断对不对",
            "说法对不对",
            "质量",
            "权重",
            "结果验证",
            "历史表现",
        ],
    ) {
        plans.push(PlannedTool {
            name: "opinion_result_review_summary",
            reason: "用户询问群友历史观点是否被赛果支持，需要查询本群观点赛后复盘汇总。",
            arguments: json!({
                "query": query
            }),
            requires_external_user: false,
            trigger: "opinion_result_review_summary_needed",
            confidence: confidence_for(&evidence),
            evidence: evidence.clone(),
        });
        if has_any_keyword(
            query,
            &[
                "哪些", "明细", "样本", "列表", "谁", "哪条", "这条", "这句", "这段", "消息",
                "具体",
            ],
        ) {
            plans.push(PlannedTool {
                name: "opinion_result_reviews",
                reason: "用户要求查看群友观点赛后复盘样本，需要查询本群观点复盘记录列表。",
                arguments: json!({
                    "query": query,
                    "limit": 12
                }),
                requires_external_user: false,
                trigger: "opinion_result_review_samples_needed",
                confidence: confidence_for(&evidence),
                evidence,
            });
        }
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

    plans.truncate(5);
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

fn has_any_keyword(text: &str, needles: &[&str]) -> bool {
    keyword_evidence(text, needles).is_some()
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
                "match_analysis_brief",
                "search_matches",
                "search_user_orders",
                "group_opinion_summary",
                "search_group_opinions",
            ]
        );
        assert!(plan.tools[2].requires_external_user);
        assert_eq!(
            plan.to_metadata()["planned_tools"][0]["trigger"].as_str(),
            Some("match_analysis_brief_needed")
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
    fn plans_opinion_memories_for_group_history_questions() {
        let plan = plan_fb2_tools(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("群里大家以前对这场有什么观点和建议？"),
        );

        assert!(plan.tool_names().contains(&"search_group_opinions"));
        assert!(plan.tool_names().contains(&"opinion_memories"));
        let metadata = plan.to_metadata();
        assert!(metadata["planned_tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| {
                tool["name"].as_str() == Some("opinion_memories")
                    && tool["trigger"].as_str() == Some("group_opinion_memory_needed")
            }));
    }

    #[test]
    fn plans_opinion_result_review_tools_for_quality_questions() {
        let plan = plan_fb2_tools(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("群里大家以前观点复盘准不准？具体哪些观点说对了？"),
        );

        let names = plan.tool_names();
        assert!(names.contains(&"opinion_memories"));
        assert!(names.contains(&"opinion_result_review_summary"));
        assert!(names.contains(&"opinion_result_reviews"));
        let metadata = plan.to_metadata();
        assert!(metadata["planned_tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| {
                tool["name"].as_str() == Some("opinion_result_review_summary")
                    && tool["trigger"].as_str() == Some("opinion_result_review_summary_needed")
            }));
    }

    #[test]
    fn plans_opinion_result_review_for_message_correctness_questions() {
        let plan = plan_fb2_tools(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("这条消息说得对吗？靠谱吗？"),
        );

        let names = plan.tool_names();
        assert!(names.contains(&"opinion_result_review_summary"));
        assert!(names.contains(&"opinion_result_reviews"));
        assert!(plan
            .tools
            .iter()
            .filter(|tool| {
                matches!(
                    tool.name,
                    "opinion_result_review_summary" | "opinion_result_reviews"
                )
            })
            .all(|tool| !tool.requires_external_user));
    }

    #[test]
    fn plans_opinion_adoption_tools_for_adoption_questions() {
        let plan = plan_fb2_tools(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("AI 之前采纳了哪些群友观点？列出具体样本"),
        );

        let names = plan.tool_names();
        assert!(names.contains(&"opinion_adoption_summary"));
        assert!(names.contains(&"list_opinion_adoptions"));
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
