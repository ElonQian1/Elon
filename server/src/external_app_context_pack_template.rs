//! server/src/external_app_context_pack_template.rs
//! Public Context Pack template contract for external app AI inputs.

use serde_json::{json, Value};

pub(crate) fn public_context_pack_template_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        // 第一阶段固定为 REST Context Pack：Markdown 负责可读正文，XML wrapper 负责边界，JSON 只承载机器契约。
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.context_pack_template.v1",
            "complete": true,
            "first_phase_delivery": "rest_context_pack_plus_tool_manifest_plus_tools_execute",
            "mcp_status": "future_wrapper_not_first_phase_fact_source",
            "body": {
                "format": "xml_wrapped_markdown",
                "wrapper": "fb2_context_pack",
                "max_role": "model_readable_business_projection",
                "not_allowed": [
                    "raw_html",
                    "full_database_dump",
                    "raw_embedding_dump",
                    "uncited_odds",
                    "uncited_order",
                    "platform_order_detail_leak"
                ]
            },
            "required_metadata": [
                "context_pack_version",
                "generated_at",
                "context_audit_id",
                "citation_sources",
                "metrics",
                "tool_contract",
                "usage_policy",
                "answer_policy",
                "preflight_readiness"
            ],
            "required_section_order": [
                "usage_boundary",
                "match_facts",
                "user_order_slice",
                "platform_order_summary",
                "group_opinion_slice",
                "retrieval_evidence",
                "quality_feedback"
            ],
            "sections": [
                {
                    "id": "usage_boundary",
                    "heading": "## usage_boundary 使用边界",
                    "purpose": "说明 AI 只能做比赛讨论、票据剖析和风险提示，不能承诺投注命中。",
                    "required_when": "always",
                    "required_source_kinds": ["context_audit"],
                    "empty_rule": "仍然输出本节，说明没有足够业务数据时必须保守回答。"
                },
                {
                    "id": "match_facts",
                    "heading": "## match_facts 比赛事实和赔率",
                    "purpose": "提供今日/近期比赛、赔率、更新时间和数据新鲜度。",
                    "required_when": "today_matches_or_ticket_or_group_opinion",
                    "required_source_kinds": ["match", "odds", "context_audit"],
                    "empty_rule": "写明没有匹配比赛或赔率，不能补写虚构赔率。"
                },
                {
                    "id": "user_order_slice",
                    "heading": "## user_order_slice 当前用户票据",
                    "purpose": "只提供当前用户自己的订单/票据摘要和组合风险。",
                    "required_when": "external_user_id_present_or_user_asks_my_ticket",
                    "required_source_kinds": ["user_order", "ticket", "context_audit"],
                    "permission_scope": "current_user_only",
                    "empty_rule": "写明当前用户没有可分析订单或权限不足，不能泄露其他用户订单。"
                },
                {
                    "id": "platform_order_summary",
                    "heading": "## platform_order_summary 平台匿名汇总",
                    "purpose": "提供平台/店铺维度匿名订单聚合、热度和风险偏斜。",
                    "required_when": "include_platform_orders=true_and_scope_header_present",
                    "required_source_kinds": ["platform_order_summary", "context_audit"],
                    "permission_scope": "anonymous_aggregate_only",
                    "empty_rule": "写明未授权或未返回平台聚合，不能输出单个用户明细。"
                },
                {
                    "id": "group_opinion_slice",
                    "heading": "## group_opinion_slice 群友观点",
                    "purpose": "提供群消息观点、分歧、观点记忆和可采纳线索。",
                    "required_when": "group_context_or_group_opinion_question",
                    "required_source_kinds": ["group_message", "opinion_memory", "context_audit"],
                    "permission_scope": "single_group_context",
                    "empty_rule": "写明没有可引用群观点，不能把 AI 推断伪装成群友观点。"
                },
                {
                    "id": "retrieval_evidence",
                    "heading": "## retrieval_evidence 召回理由和数据缺口",
                    "purpose": "记录 topic_hint、命中理由、权限范围、新鲜度、截断和缺口。",
                    "required_when": "always",
                    "required_source_kinds": ["context_audit"],
                    "recommended_fields": ["topic_hint", "query_terms", "match_reason", "freshness", "permission_scope", "truncated", "missing_context"],
                    "empty_rule": "仍然输出本节，明确本次缺少哪些比赛、赔率、订单或群观点。"
                },
                {
                    "id": "quality_feedback",
                    "heading": "## quality_feedback 反馈和质量闭环",
                    "purpose": "说明回答后如何写回 cited_sources、wrong_context、opinion_adoption 和质量摘要。",
                    "required_when": "always",
                    "required_source_kinds": ["context_audit"],
                    "quality_history_kinds": ["feedback", "opinion_adoption"],
                    "empty_rule": "仍然输出本节，给出 context_audit_id 和 feedback route。"
                }
            ],
            "citation_source_shape": {
                "required_fields": ["kind", "id", "label"],
                "recommended_fields": ["updated_at", "scope", "freshness", "source_hash"],
                "business_source_kinds": [
                    "context_audit",
                    "match",
                    "odds",
                    "user_order",
                    "ticket",
                    "group_message",
                    "opinion_memory",
                    "platform_order_summary"
                ],
                "quality_history_kinds": [
                    {
                        "kind": "feedback",
                        "scope": "quality_history",
                        "default_chat_fact": false
                    },
                    {
                        "kind": "opinion_adoption",
                        "scope": "quality_history",
                        "default_chat_fact": false
                    }
                ],
                "rule": "AI 回复中出现的比赛、赔率、订单、票据、群消息、观点记忆和平台摘要事实，必须能匹配 citation_sources 或 grounded tool source_ids。"
            },
            "retrieval_evidence_item_shape": {
                "schema": "fb2.retrieval_evidence_item.v1",
                "required_fields": [
                    "evidence_id",
                    "source_id",
                    "source_kind",
                    "section_id",
                    "lane_id",
                    "index_id",
                    "reason",
                    "freshness",
                    "permission_scope",
                    "citation_source_id"
                ],
                "recommended_fields": [
                    "query_terms",
                    "score",
                    "updated_at",
                    "truncated",
                    "missing_context",
                    "context_audit_id"
                ],
                "source_id_rule": "source_id 必须能匹配 citation_sources[].id、context_audit_id 或已执行 grounded/weak tool result 的 source_ids。",
                "reason_rule": "reason 必须说明该比赛、赔率、订单、平台摘要或群观点为什么与本次 topic_hint / selected_message / user order 请求相关。",
                "permission_rule": "permission_scope 必须与数据 lane 一致：current_user_only 不能混入其它用户订单，anonymous_aggregate_only 不能暴露单个用户明细，single_group_context 不能混入私聊。",
                "empty_rule": "没有可召回业务数据时仍输出至少一条 context_audit 证据，missing_context 写明比赛、赔率、订单或群观点缺口。"
            },
            "answer_boundaries": [
                "数据事实、用户订单、平台汇总、群友观点、AI推断、风险边界必须分层。",
                "没有 source id 的赔率、订单、伤停、群友观点只能说信息不足。",
                "不得承诺投注命中，不得建议重注或梭哈。",
                "平台订单只能是匿名聚合，不能泄露单个用户。"
            ],
            "minimal_markdown_template": [
                "<fb2_context_pack>",
                "## usage_boundary 使用边界",
                "## match_facts 比赛事实和赔率",
                "## user_order_slice 当前用户票据",
                "## platform_order_summary 平台匿名汇总",
                "## group_opinion_slice 群友观点",
                "## retrieval_evidence 召回理由和数据缺口",
                "## quality_feedback 反馈和质量闭环",
                "</fb2_context_pack>"
            ]
        })),
        _ => None,
    }
}

#[cfg(test)]
#[path = "external_app_context_pack_template_tests.rs"]
mod tests;
