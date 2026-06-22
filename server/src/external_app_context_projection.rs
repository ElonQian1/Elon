//! server/src/external_app_context_projection.rs
//! Domain-specific Context Pack projection guidance for external apps.

use serde_json::{json, Value};

pub(crate) fn public_context_projection_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.domain_context_projection.v1",
            "format": {
                "body": "XML-wrapped Markdown",
                "wrapper": "fb2_context_pack",
                "structured_metadata": [
                    "context_pack_version",
                    "generated_at",
                    "context_audit_id",
                    "matches",
                    "user_orders",
                    "group_messages",
                    "platform_order_summary",
                    "citation_sources",
                    "metrics",
                    "tool_contract",
                    "usage_policy",
                    "answer_policy",
                    "preflight_readiness"
                ],
                "principle": "正文给模型读，metadata 给主项目和评测系统验；不要把 fb2 原始数据库、HTML、embedding 或完整订单明细直接塞进 prompt。"
            },
            "rcp_mapping": {
                "project_brief": "使用边界 + fb2 数据域说明",
                "repo_map": "Context Pack 小节顺序和 source registry",
                "symbol_graph": "match_id/order_id/message_id/opinion_memory_id/context_audit_id 的引用关系",
                "retrieval_evidence": "每条比赛、订单、群观点为什么被召回",
                "tests": "quality-summary、feedback-summary、permission-summary 和 data-only acceptance"
            },
            "required_sections": required_sections(),
            "source_registry": {
                "required_field": "citation_sources",
                "required_kinds": [
                    "match",
                    "odds",
                    "user_order",
                    "ticket",
                    "group_message",
                    "opinion_memory",
                    "platform_order_summary",
                    "context_audit",
                    "feedback",
                    "opinion_adoption"
                ],
                "minimum_shape": {
                    "kind": "match | odds | user_order | ticket | group_message | opinion_memory | platform_order_summary | context_audit | feedback | opinion_adoption",
                    "id": "stable source id",
                    "label": "short human-readable label",
                    "updated_at": "optional ISO-8601 freshness timestamp"
                },
                "rule": "AI 回答中出现的关键判断必须能追到 citation_sources 或 executed tool source_ids；没有来源时只能说明缺口。"
            },
            "retrieval_projection": {
                "recommended_fields": [
                    "topic_hint",
                    "query_terms",
                    "match_reason",
                    "source_score",
                    "freshness",
                    "permission_scope",
                    "truncated"
                ],
                "rule": "fb2 不只返回数据，还要说明为什么这些比赛、订单或观点与用户问题相关，方便主项目后续做质量评测。"
            },
            "permission_projection": [
                {
                    "data": "user_orders",
                    "scope": "current_user_only",
                    "required_request": ["external_user_id", "X-FB2-AI-CONTEXT-USER-ID"],
                    "forbidden": ["other_user_order_detail", "raw_user_identity"]
                },
                {
                    "data": "platform_order_summary",
                    "scope": "anonymous_aggregate_only",
                    "required_request": ["include_platform_orders=true", "X-FB2-AI-CONTEXT-SCOPE=platform_order_summary"],
                    "forbidden": ["single_user_order_detail", "raw_user_identity"]
                },
                {
                    "data": "group_opinions",
                    "scope": "group_visible",
                    "required_request": ["group_id"],
                    "forbidden": ["private_message", "opinion_without_message_id"]
                }
            ],
            "quality_closure": {
                "required_feedback_routes": [
                    "/api/main-project/context/feedback",
                    "/api/main-project/context/feedback-summary",
                    "/api/main-project/context/opinion-adoption-summary",
                    "/api/main-project/context/quality-summary"
                ],
                "minimum_non_synthetic_ready": {
                    "feedback_count": 1,
                    "opinion_adoption_count": 1,
                    "opinion_memory_ref_count": "present"
                },
                "rule": "真实群聊可见回答必须把引用来源、是否采纳群观点、缺失上下文和错误上下文写回 fb2，不能只靠一次性 prompt。"
            },
            "anti_patterns": [
                "raw_html_prompt",
                "giant_json_prompt",
                "full_database_dump",
                "raw_embedding_dump",
                "uncited_odds",
                "uncited_order",
                "uncited_group_opinion",
                "platform_order_detail_leak",
                "guaranteed_betting_outcome"
            ],
            "answer_grounding_rule": "回答时必须区分 数据事实、用户订单、平台汇总、群友观点、AI推断、风险边界；比赛事实、赔率、订单、群观点不得互相冒充。"
        })),
        _ => None,
    }
}

fn required_sections() -> Value {
    json!([
        {
            "id": "usage_boundary",
            "heading": "使用边界",
            "must_include": [
                "只用于比赛讨论和订单剖析参考",
                "不承诺命中",
                "不诱导重注或梭哈"
            ]
        },
        {
            "id": "match_facts",
            "heading": "今日/近期比赛与赔率",
            "source_kinds": ["match", "odds"],
            "required_ids": ["match_id", "odds_updated_at"],
            "recommended_line_shape": "- match_id=<id> league=<league> home=<team> away=<team> match_time=<iso> odds_updated_at=<iso> source=<source>"
        },
        {
            "id": "user_order_slice",
            "heading": "当前用户订单/票据",
            "source_kinds": ["user_order", "ticket"],
            "required_ids": ["order_id", "ticket_id"],
            "permission": "current_user_only",
            "recommended_line_shape": "- order_id=<id> ticket_id=<id> match_ids=[...] visibility=current_user_only risk_summary=<short>"
        },
        {
            "id": "platform_order_summary",
            "heading": "平台/店铺订单摘要",
            "source_kinds": ["platform_order_summary"],
            "required_ids": ["platform_order_summary"],
            "permission": "anonymous_aggregate_only",
            "recommended_line_shape": "- platform_order_summary=<id> scope=anonymous_aggregate metric=<name> value=<value>"
        },
        {
            "id": "group_opinion_slice",
            "heading": "群讨论观点",
            "source_kinds": ["group_message", "opinion_memory"],
            "required_ids": ["message_id"],
            "permission": "group_visible",
            "recommended_line_shape": "- message_id=<id> match_id=<id?> stance=<support|oppose|neutral> opinion=<summary>"
        },
        {
            "id": "retrieval_evidence",
            "heading": "召回理由和数据缺口",
            "source_kinds": ["context_audit"],
            "required_ids": ["context_audit_id"],
            "recommended_line_shape": "- source_id=<id> reason=<why_selected> freshness=<fresh|stale|unknown> missing=<gap?>"
        },
        {
            "id": "quality_feedback",
            "heading": "质量回填口径",
            "source_kinds": ["feedback", "opinion_adoption"],
            "required_ids": ["main_request_id", "context_audit_id"],
            "recommended_line_shape": "- feedback_trigger=<visible_mention|selected_message|summary_post> cited_sources=[...]"
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn array_contains(value: &Value, expected: &str) -> bool {
        value.as_array().unwrap().contains(&json!(expected))
    }

    fn permission_for<'a>(permissions: &'a [Value], data: &str) -> &'a Value {
        permissions
            .iter()
            .find(|permission| permission["data"] == data)
            .unwrap_or_else(|| panic!("missing permission projection for {data}"))
    }

    #[test]
    fn exposes_fb2_domain_context_projection_contract() {
        let contract = public_context_projection_guidance("fb2").unwrap();
        assert_eq!(contract["schema"], "fb2.domain_context_projection.v1");
        assert_eq!(contract["format"]["wrapper"], "fb2_context_pack");

        let sections = contract["required_sections"].as_array().unwrap();
        assert!(sections
            .iter()
            .any(|section| section["id"] == "match_facts"));
        assert!(sections
            .iter()
            .any(|section| section["id"] == "user_order_slice"));
        assert!(sections
            .iter()
            .any(|section| section["id"] == "group_opinion_slice"));
        assert!(sections
            .iter()
            .any(|section| section["id"] == "retrieval_evidence"));

        let source_kinds = contract["source_registry"]["required_kinds"]
            .as_array()
            .unwrap();
        assert!(source_kinds.contains(&json!("match")));
        assert!(source_kinds.contains(&json!("odds")));
        assert!(source_kinds.contains(&json!("user_order")));
        assert!(source_kinds.contains(&json!("group_message")));
        assert!(source_kinds.contains(&json!("platform_order_summary")));
        assert!(source_kinds.contains(&json!("feedback")));
        assert!(source_kinds.contains(&json!("opinion_adoption")));

        assert!(contract["anti_patterns"]
            .as_array()
            .unwrap()
            .contains(&json!("raw_embedding_dump")));
        assert!(public_context_projection_guidance("unknown").is_none());
    }

    #[test]
    fn fb2_domain_projection_declares_permissions_quality_and_grounding() {
        let contract = public_context_projection_guidance("fb2").unwrap();

        let retrieval_fields = &contract["retrieval_projection"]["recommended_fields"];
        for field in [
            "topic_hint",
            "match_reason",
            "permission_scope",
            "truncated",
        ] {
            assert!(array_contains(retrieval_fields, field));
        }

        let permissions = contract["permission_projection"].as_array().unwrap();
        let user_orders = permission_for(permissions, "user_orders");
        assert_eq!(user_orders["scope"], "current_user_only");
        assert!(array_contains(
            &user_orders["required_request"],
            "external_user_id"
        ));
        assert!(array_contains(
            &user_orders["required_request"],
            "X-FB2-AI-CONTEXT-USER-ID"
        ));
        assert!(array_contains(
            &user_orders["forbidden"],
            "other_user_order_detail"
        ));

        let platform_summary = permission_for(permissions, "platform_order_summary");
        assert_eq!(platform_summary["scope"], "anonymous_aggregate_only");
        assert!(array_contains(
            &platform_summary["required_request"],
            "include_platform_orders=true"
        ));
        assert!(array_contains(
            &platform_summary["required_request"],
            "X-FB2-AI-CONTEXT-SCOPE=platform_order_summary"
        ));
        assert!(array_contains(
            &platform_summary["forbidden"],
            "single_user_order_detail"
        ));

        let group_opinions = permission_for(permissions, "group_opinions");
        assert_eq!(group_opinions["scope"], "group_visible");
        assert!(array_contains(
            &group_opinions["required_request"],
            "group_id"
        ));
        assert!(array_contains(
            &group_opinions["forbidden"],
            "private_message"
        ));
        assert!(array_contains(
            &group_opinions["forbidden"],
            "opinion_without_message_id"
        ));

        let quality_routes = &contract["quality_closure"]["required_feedback_routes"];
        for route in [
            "/api/main-project/context/feedback",
            "/api/main-project/context/feedback-summary",
            "/api/main-project/context/opinion-adoption-summary",
            "/api/main-project/context/quality-summary",
        ] {
            assert!(array_contains(quality_routes, route));
        }
        let readiness = &contract["quality_closure"]["minimum_non_synthetic_ready"];
        assert_eq!(readiness["feedback_count"], json!(1));
        assert_eq!(readiness["opinion_adoption_count"], json!(1));
        assert_eq!(readiness["opinion_memory_ref_count"], "present");

        let grounding_rule = contract["answer_grounding_rule"].as_str().unwrap();
        for phrase in [
            "数据事实",
            "用户订单",
            "平台汇总",
            "群友观点",
            "AI推断",
            "风险边界",
        ] {
            assert!(grounding_rule.contains(phrase));
        }
    }
}
