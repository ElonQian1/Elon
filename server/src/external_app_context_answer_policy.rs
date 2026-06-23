//! Answer policy for AI replies grounded in external app context.

use serde_json::{json, Value};

const FB2_ANSWER_RULES: &[&str] = &[
    "使用 fb2 外部上下文时，回答中必须区分并显式使用「数据事实：」「用户订单：」「平台汇总：」「群友观点：」「AI推断：」「风险边界：」等短标签；没有对应材料的标签可以省略，但「数据事实」「AI推断」「风险边界」不能省。",
    "涉及比赛、赔率、票据、推荐、预测或今日比赛讨论时，必须写明「风险边界：赛果不确定，不保证命中，不建议重注或梭哈」。",
    "引用比赛时尽量带 match id；引用订单/票据时尽量带 order id 或 ticket id；引用群友观点时必须带 message id；引用平台匿名订单汇总时必须带 platform_order_summary source id。",
    "如果上下文缺少用户订单、赔率更新时间或消息来源，必须说明信息不足，不能编造。",
    "如果 context_quality.warnings 非空，回答中必须显式提示相关数据缺口或新鲜度风险。",
    "如果需要更多比赛、订单或群友观点明细，只能提出需要调用的外部工具，不能把未调用工具的结果当事实。",
    "如果 context_quality.tool_readiness.status 不是 ready，说明外部项目按需检索能力还不完整，回答要更保守。",
];

pub(crate) fn public_answer_policy_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.answer_policy.v1",
            "grounding_sections": [
                {
                    "name": "data_facts",
                    "description": "fb2 提供的比赛、赔率、赛程、数据源和更新时间。",
                    "required_source_ids": ["match_id", "odds_updated_at", "source"]
                },
                {
                    "name": "user_orders",
                    "description": "当前用户自己的订单、票据和组合风险。",
                    "required_source_ids": ["order_id", "ticket_id"],
                    "visibility": "current_user_only"
                },
                {
                    "name": "group_opinions",
                    "description": "群友围绕比赛、赔率或订单的观点摘要。",
                    "required_source_ids": ["message_id"],
                    "visibility": "group_visible"
                },
                {
                    "name": "platform_order_summary",
                    "description": "平台/店铺匿名聚合订单摘要，不包含单个用户订单明细。",
                    "required_source_ids": ["platform_order_summary"],
                    "visibility": "anonymous_aggregate_only"
                },
                {
                    "name": "ai_inference",
                    "description": "AI 基于事实、订单和观点做出的分析或预测。",
                    "must_disclose_uncertainty": true
                }
            ],
            "forbidden_behaviors": [
                "不能把 AI 推断写成 fb2 数据事实。",
                "不能编造未提供的赔率、伤停、订单或群友观点。",
                "不能承诺命中、诱导投注或代替用户决策。",
                "不能暴露其他用户的订单明细。"
            ],
            "prompt_answer_rules": FB2_ANSWER_RULES,
            "preferred_answer_shape": [
                "先给结论和风险等级。",
                "再列数据依据：比赛、赔率、订单或群观点 source id。",
                "再区分群友观点和 AI 推断。",
                "最后给下一步建议：需要补查哪场比赛、哪个订单或哪些群观点。"
            ],
            "canonical_eval_questions": [
                "总结今天有哪些比赛值得讨论？",
                "分析 match_id=m-001 这场，赔率变化说明什么？",
                "帮我看看我今天的票风险在哪里？",
                "总结群里大家对这场比赛的不同观点。",
                "生成一篇今天群聊讨论总结帖。",
                "平台今天订单集中在哪些方向？只说匿名聚合。",
                "你刚才依据了哪些比赛、订单和群消息？"
            ],
            "eval_scenarios": [
                {
                    "id": "today_matches_analysis",
                    "question": "今天比赛怎么看？",
                    "entrypoints": ["group_mention_at_el", "summary_post", "chat_bootstrap_ai_reply"],
                    "preferred_context": ["context_pack", "match_analysis_brief"],
                    "required_source_kinds": ["match", "odds"],
                    "required_answer_sections": ["data_facts", "ai_inference"],
                    "required_citations": ["match_id", "context_audit_id"],
                    "forbidden_outputs": ["guaranteed_win", "fabricated_odds", "betting_inducement"]
                },
                {
                    "id": "my_ticket_analysis",
                    "question": "帮我分析我的票。",
                    "entrypoints": ["group_mention_at_el", "chat_bootstrap_ai_reply"],
                    "preferred_context": ["context_pack", "match_analysis_brief"],
                    "required_headers": ["X-FB2-AI-CONTEXT-USER-ID"],
                    "required_query_fields": ["external_user_id"],
                    "required_source_kinds": ["user_order", "match"],
                    "required_answer_sections": ["data_facts", "user_orders", "ai_inference"],
                    "required_citations": ["order_id", "match_id", "context_audit_id"],
                    "permission_boundary": "current_user_only",
                    "forbidden_outputs": ["other_user_order_detail", "guaranteed_win"]
                },
                {
                    "id": "platform_order_risk",
                    "question": "平台今天订单风险怎么样？",
                    "entrypoints": ["privileged_group_mention_at_el", "operations_summary"],
                    "preferred_context": ["context_pack", "platform_orders"],
                    "required_headers": ["X-FB2-AI-CONTEXT-SCOPE=platform_order_summary"],
                    "required_query_fields": ["include_platform_orders=true"],
                    "required_source_kinds": ["platform_order_summary"],
                    "required_answer_sections": ["data_facts", "platform_order_summary", "ai_inference"],
                    "required_citations": ["platform_order_summary", "context_audit_id"],
                    "permission_boundary": "anonymous_aggregate_only",
                    "forbidden_outputs": ["single_user_order_detail", "user_identity_leak"]
                },
                {
                    "id": "group_opinion_summary",
                    "question": "群里大家怎么看这场？",
                    "entrypoints": ["group_mention_at_el", "summary_post"],
                    "preferred_context": ["context_pack", "group_opinion_summary"],
                    "required_query_fields": ["group_id", "topic_hint"],
                    "required_source_kinds": ["group_message", "opinion_memory"],
                    "required_answer_sections": ["group_opinions", "ai_inference"],
                    "required_citations": ["message_id", "context_audit_id"],
                    "forbidden_outputs": ["group_opinion_as_fact", "fabricated_group_view"]
                },
                {
                    "id": "selected_message_review",
                    "question": "这条消息说得对吗？",
                    "entrypoints": ["selected_message_ai_reply"],
                    "preferred_context": ["context_pack", "match_analysis_brief", "opinion_result_review_summary"],
                    "required_query_fields": ["group_id", "topic_hint", "selected_message_id"],
                    "required_source_kinds": ["selected_message", "match"],
                    "required_answer_sections": ["data_facts", "group_opinions", "ai_inference"],
                    "required_citations": ["selected_message_id", "match_id", "context_audit_id"],
                    "forbidden_outputs": ["unsupported_claim_verdict", "guaranteed_win"]
                },
                {
                    "id": "group_discussion_summary_post",
                    "question": "总结今天群聊讨论。",
                    "entrypoints": ["summary_post", "group_summary_post"],
                    "preferred_context": ["context_pack", "group_opinion_summary", "context_feedback_summary"],
                    "required_query_fields": ["group_id", "topic_hint"],
                    "required_source_kinds": ["group_message", "opinion_memory", "context_audit"],
                    "required_answer_sections": ["group_opinions", "source_references", "risk_boundary"],
                    "required_citations": ["message_id", "opinion_memory_id", "context_audit_id"],
                    "forbidden_outputs": ["fabricated_group_view", "group_opinion_as_fact", "guaranteed_win"]
                },
                {
                    "id": "source_reference_audit",
                    "question": "你刚才依据了哪些比赛、订单和群消息？",
                    "entrypoints": ["group_followup", "chat_bootstrap_ai_reply"],
                    "preferred_context": ["previous_answer_feedback", "context_pack"],
                    "required_source_kinds": ["citation_sources"],
                    "required_answer_sections": ["data_facts", "user_orders", "group_opinions"],
                    "required_citations": ["context_audit_id"],
                    "forbidden_outputs": ["uncited_claim", "invented_source_id"]
                }
            ],
            "default_answer_policy": default_answer_policy()
        })),
        _ => None,
    }
}

pub(crate) fn default_answer_policy() -> Value {
    json!({
        "schema": "fb2.answer_policy.v1",
        "must_distinguish": ["data_facts", "user_orders", "group_opinions", "ai_inference"],
        "required_citations": ["match_id", "order_id", "ticket_id", "message_id", "platform_order_summary", "context_audit_id"],
        "risk_rules": {
            "no_guaranteed_win": true,
            "no_betting_inducement": true,
            "explain_uncertainty": true
        },
        "permission_rules": {
            "user_orders": "current_user_only",
            "platform_orders": "anonymous_aggregate_only_by_default"
        },
        "prompt_answer_rules": FB2_ANSWER_RULES
    })
}

pub(crate) fn prompt_answer_rules_block(_context: &Value) -> String {
    let rules = FB2_ANSWER_RULES
        .iter()
        .map(|rule| format!("- {rule}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("<answer_rules>\n{rules}\n</answer_rules>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_public_fb2_answer_policy() {
        let guidance = public_answer_policy_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.answer_policy.v1");
        assert!(guidance["grounding_sections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|section| section["name"] == "group_opinions"));
        assert!(guidance["grounding_sections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|section| section["name"] == "platform_order_summary"
                && section["visibility"] == "anonymous_aggregate_only"));
        assert!(guidance["forbidden_behaviors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule.as_str().unwrap_or("").contains("编造")));
        assert!(guidance["canonical_eval_questions"]
            .as_array()
            .unwrap()
            .contains(&json!("帮我看看我今天的票风险在哪里？")));
        let scenarios = guidance["eval_scenarios"].as_array().unwrap();
        for expected_id in [
            "today_matches_analysis",
            "my_ticket_analysis",
            "platform_order_risk",
            "group_opinion_summary",
            "selected_message_review",
            "group_discussion_summary_post",
            "source_reference_audit",
        ] {
            assert!(scenarios
                .iter()
                .any(|scenario| scenario["id"] == expected_id));
        }
        assert!(scenarios
            .iter()
            .any(|scenario| scenario["id"] == "my_ticket_analysis"
                && scenario["permission_boundary"] == "current_user_only"
                && scenario["required_headers"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("X-FB2-AI-CONTEXT-USER-ID"))));
        assert!(scenarios
            .iter()
            .any(|scenario| scenario["id"] == "platform_order_risk"
                && scenario["permission_boundary"] == "anonymous_aggregate_only"
                && scenario["forbidden_outputs"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("single_user_order_detail"))));
        assert_eq!(
            guidance["default_answer_policy"]["risk_rules"]["no_guaranteed_win"],
            true
        );
        assert!(public_answer_policy_guidance("unknown").is_none());
    }

    #[test]
    fn prompt_rules_keep_source_boundaries() {
        let block = prompt_answer_rules_block(&json!({}));
        assert!(block.contains("<answer_rules>"));
        assert!(block.contains("必须区分"));
        assert!(block.contains("数据事实："));
        assert!(block.contains("风险边界："));
        assert!(block.contains("不保证命中"));
        assert!(block.contains("message id"));
        assert!(block.contains("platform_order_summary source id"));
        assert!(block.contains("不能编造"));
    }
}
