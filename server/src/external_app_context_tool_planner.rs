//! Tool planning for fb2 external context queries.

use serde_json::{json, Value};

use crate::{
    external_app_context_config::{
        infer_lottery_type, platform_order_summary_enabled, platform_order_summary_requested,
    },
    external_app_context_scenario_prompt::fb2_domain_scenario_selection,
};

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
    domain_scenario_selection: Value,
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
            "domain_scenario_selection": self.domain_scenario_selection,
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
    let mut opinion_memories_planned = false;

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
        if context_has_current_user_orders(context) {
            skipped_reasons.push("current_user_orders_already_in_context_pack");
        } else {
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
            name: "opinion_memories",
            reason:
                "用户要求参考群友观点、建议或采纳结论，需要优先读取 fb2 本群最近的持久观点记忆。",
            arguments: opinion_memory_arguments(),
            requires_external_user: false,
            trigger: "group_opinion_memory_needed",
            confidence: confidence_for(&evidence),
            evidence: evidence.clone(),
        });
        opinion_memories_planned = true;

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
        if !opinion_memories_planned {
            plans.push(PlannedTool {
                name: "opinion_memories",
                reason: "用户要求参考群友历史观点、建议或长期记忆，需要检索 fb2 群观点记忆索引。",
                arguments: opinion_memory_arguments(),
                requires_external_user: false,
                trigger: "group_opinion_memory_needed",
                confidence: confidence_for(&evidence),
                evidence,
            });
        }
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

    if platform_order_summary_requested(Some(query)) {
        let evidence = keyword_evidence(
            query,
            &[
                "平台",
                "全平台",
                "全站",
                "店铺",
                "经营",
                "汇总",
                "整体",
                "所有用户",
                "全体用户",
                "匿名汇总",
                "大盘",
                "订单",
                "订单风险",
                "投注集中",
                "投注",
                "集中",
                "派奖",
                "毛利",
                "销量",
                "成交",
                "赔付",
                "风险",
            ],
        )
        .unwrap_or_else(|| vec!["query.intent.platform_order_summary".to_string()]);
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
    let tool_names = plans.iter().map(|tool| tool.name).collect::<Vec<_>>();
    let domain_scenario_selection =
        fb2_domain_scenario_selection(Some(context), Some(query), &tool_names);

    Fb2ToolPlan {
        topic_hint: query.to_string(),
        tools: plans,
        skipped_reasons,
        domain_scenario_selection,
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

fn opinion_memory_arguments() -> Value {
    json!({
        "include_expired": false,
        "limit": 12
    })
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

fn context_has_current_user_orders(context: &Value) -> bool {
    if context
        .get("user_orders")
        .and_then(Value::as_array)
        .map(|orders| !orders.is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    context
        .get("metrics")
        .and_then(|metrics| metrics.get("source_counts"))
        .and_then(Value::as_array)
        .map(|counts| {
            counts.iter().any(|entry| {
                matches!(
                    entry.get("source_type").and_then(Value::as_str),
                    Some("user_order" | "user_orders")
                ) && entry
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or_default()
                    > 0
            })
        })
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
#[path = "external_app_context_tool_planner_tests.rs"]
mod tests;
