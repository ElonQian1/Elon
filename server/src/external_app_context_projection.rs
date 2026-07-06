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
            "domain_scenario_matrix": fb2_domain_scenario_matrix(),
            "required_sections": required_sections(),
            "source_registry": {
                "required_field": "citation_sources",
                "required_kinds": [
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
                "minimum_shape": {
                    "kind": "context_audit | match | odds | user_order | ticket | group_message | opinion_memory | platform_order_summary",
                    "id": "stable source id",
                    "label": "short human-readable label",
                    "updated_at": "optional ISO-8601 freshness timestamp"
                },
                "quality_history_shape": {
                    "kind": "feedback | opinion_adoption",
                    "scope": "quality_history",
                    "default_chat_fact": false,
                    "id": "feedback/adoption source id when explicitly used as quality history"
                },
                "rule": "AI 回答中出现的业务事实必须能追到 citation_sources 或 executed tool source_ids；feedback/opinion_adoption 默认只属于质量闭环，除非显式标注 scope=quality_history，否则不能当比赛、赔率、订单或群观点事实。"
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
                "item_shape": {
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
                    "linking_rules": [
                        "source_id must resolve to citation_sources[].id, context_audit_id, or grounded/weak tool_result.source_ids",
                        "citation_source_id must point at the exact source registry entry used by the answer",
                        "index_id must be one of the domain_context_index_contract indexes when the evidence comes from fb2 retrieval",
                        "permission_scope must match the lane and request headers before the evidence can be model-visible"
                    ],
                    "privacy_rules": [
                        "current_user_only evidence cannot contain other user order detail",
                        "anonymous_aggregate_only evidence cannot contain single-user order rows",
                        "single_group_context evidence cannot contain private messages"
                    ]
                },
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

pub(crate) fn public_domain_data_blueprint_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.main_project.domain_data_blueprint.v1",
            "complete": true,
            "context_format": "xml_wrapped_markdown_context_pack_with_json_metadata",
            "first_phase_delivery": "rest_context_pack_plus_tool_manifest_plus_tools_execute",
            "mcp_status": "future_wrapper_not_first_phase_fact_source",
            "source_of_truth": "fb2_backend_live_business_data",
            "stores_fb2_business_data_in_main_project": false,
            "required_context_pack_sections": [
                "usage_boundary",
                "match_facts",
                "user_order_slice",
                "platform_order_summary",
                "group_opinion_slice",
                "retrieval_evidence",
                "quality_feedback"
            ],
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
            // 这里把 fb2 长期业务数据拆成稳定 lane；fb2 可以优化内部索引，但对主项目暴露的语义边界不能漂移。
            "lanes": [
                {
                    "id": "match_facts_and_odds",
                    "user_need": "今天比赛怎么看 / 这场赔率怎么变",
                    "context_sections": ["match_facts", "retrieval_evidence"],
                    "source_kinds": ["match", "odds", "context_audit"],
                    "primary_tools": ["match_analysis_brief", "search_matches", "get_match_detail"],
                    "permission_scope": "group_context",
                    "answer_layers": ["match_facts", "odds_facts", "ai_inference", "risk_boundary"],
                    "forbidden_outputs": ["fabricated_odds", "guaranteed_win"],
                    "future_indexes": ["match_index", "odds_snapshot_index"]
                },
                {
                    "id": "current_user_tickets",
                    "user_need": "帮我分析我的票 / 我的订单风险",
                    "context_sections": ["user_order_slice", "match_facts", "retrieval_evidence"],
                    "source_kinds": ["user_order", "ticket", "match", "odds", "context_audit"],
                    "primary_tools": ["match_analysis_brief", "search_user_orders", "get_order_detail"],
                    "permission_scope": "current_user_only",
                    "answer_layers": ["current_user_orders", "match_facts", "ai_inference", "risk_boundary"],
                    "forbidden_outputs": ["other_user_order_detail", "guaranteed_win"],
                    "future_indexes": ["order_risk_index", "ticket_result_review_index"]
                },
                {
                    "id": "platform_order_summary",
                    "user_need": "平台今天订单风险怎么样",
                    "context_sections": ["platform_order_summary", "retrieval_evidence"],
                    "source_kinds": ["platform_order_summary", "context_audit"],
                    "primary_tools": ["platform_orders"],
                    "permission_scope": "privileged_anonymous_summary",
                    "answer_layers": ["platform_aggregate", "ai_inference", "risk_boundary"],
                    "forbidden_outputs": ["single_user_order_detail", "user_identity_leak"],
                    "future_indexes": ["platform_order_risk_index"]
                },
                {
                    "id": "group_opinions",
                    "user_need": "群里大家怎么看这场 / 总结群聊观点",
                    "context_sections": ["group_opinion_slice", "match_facts", "retrieval_evidence"],
                    "source_kinds": ["group_message", "opinion_memory", "match", "context_audit"],
                    "primary_tools": ["group_opinion_summary", "search_group_opinions", "opinion_memories"],
                    "permission_scope": "single_group_context",
                    "answer_layers": ["group_opinion", "match_facts", "ai_inference", "risk_boundary"],
                    "forbidden_outputs": ["group_opinion_as_fact", "fabricated_group_view"],
                    "future_indexes": ["group_opinion_index", "opinion_memory_index"]
                },
                {
                    "id": "opinion_learning_loop",
                    "user_need": "采纳用户观点并持续复盘，让群聊分析逐步进化",
                    "context_sections": ["quality_feedback", "group_opinion_slice"],
                    "source_kinds": ["opinion_memory", "feedback", "opinion_adoption"],
                    "primary_tools": ["list_opinion_adoptions", "opinion_adoption_summary", "opinion_result_reviews", "opinion_result_review_summary"],
                    "permission_scope": "single_group_quality_history",
                    "answer_layers": ["opinion_history", "quality_signal", "ai_inference", "risk_boundary"],
                    "forbidden_outputs": ["quality_history_as_match_fact", "uncited_opinion_memory"],
                    "future_indexes": ["opinion_adoption_index", "opinion_result_review_index"]
                },
                {
                    "id": "quality_feedback_audit",
                    "user_need": "回答有没有引用错来源 / 哪些失败样本需要改进",
                    "context_sections": ["quality_feedback", "retrieval_evidence"],
                    "source_kinds": ["context_audit", "feedback", "opinion_adoption"],
                    "primary_tools": ["get_context_audit", "context_audit_summary", "list_context_feedbacks"],
                    "permission_scope": "audit_metadata_only",
                    "answer_layers": ["source_registry", "data_fact_boundary", "quality_feedback"],
                    "forbidden_outputs": ["uncited_source", "fabricated_source"],
                    "future_indexes": ["context_audit_index", "feedback_quality_index"]
                }
            ],
            "lane_count": 6,
            "anti_patterns": [
                "raw_html_prompt",
                "giant_json_prompt",
                "full_database_dump",
                "raw_embedding_dump",
                "uncited_odds",
                "uncited_order",
                "platform_order_detail_leak"
            ],
            "next_evolution": [
                "keep REST Context Pack as the AI-facing payload",
                "add fb2-side domain indexes for faster retrieval",
                "wrap existing REST/tool contracts with MCP later only if it preserves permissions and audit"
            ]
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

pub(crate) fn fb2_domain_scenario_matrix() -> Value {
    json!([
        {
            "id": "today_matches_analysis",
            "user_question": "今天比赛怎么看？",
            "entrypoints": ["group_mention_at_el", "summary_post", "chat_bootstrap_ai_reply"],
            "context_pack_sections": ["match_facts", "retrieval_evidence", "quality_feedback"],
            "primary_tools": ["match_analysis_brief", "search_matches"],
            "required_source_kinds": ["match", "odds", "context_audit"],
            "required_citations": ["match_id", "context_audit_id"],
            "permission_scope": "group_visible",
            "required_request": ["group_id", "topic_hint"],
            "feedback_routes": ["/api/main-project/context/feedback", "/api/main-project/context/quality-summary"],
            "acceptance_signals": [
                "reply_has_data_facts",
                "reply_has_ai_inference",
                "reply_has_risk_boundary",
                "matched_cited_sources"
            ],
            "forbidden_outputs": ["guaranteed_win", "fabricated_odds", "betting_inducement"]
        },
        {
            "id": "my_ticket_analysis",
            "user_question": "帮我分析我的票。",
            "entrypoints": ["group_mention_at_el", "chat_bootstrap_ai_reply"],
            "context_pack_sections": ["match_facts", "user_order_slice", "retrieval_evidence", "quality_feedback"],
            "primary_tools": ["match_analysis_brief", "search_user_orders"],
            "required_source_kinds": ["user_order", "ticket", "match", "context_audit"],
            "required_citations": ["order_id", "ticket_id", "match_id", "context_audit_id"],
            "permission_scope": "current_user_only",
            "required_request": ["external_user_id", "X-FB2-AI-CONTEXT-USER-ID", "topic_hint"],
            "feedback_routes": ["/api/main-project/context/feedback", "/api/main-project/context/quality-summary"],
            "acceptance_signals": [
                "reply_has_user_orders",
                "only_current_user_orders",
                "matched_cited_sources",
                "permission_summary_records_wrong_user_blocks"
            ],
            "forbidden_outputs": ["other_user_order_detail", "guaranteed_win"]
        },
        {
            "id": "platform_order_risk",
            "user_question": "平台今天订单风险怎么样？",
            "entrypoints": ["privileged_group_mention_at_el", "operations_summary", "summary_post"],
            "context_pack_sections": ["platform_order_summary", "match_facts", "retrieval_evidence", "quality_feedback"],
            "primary_tools": ["platform_orders", "match_analysis_brief"],
            "required_source_kinds": ["platform_order_summary", "match", "context_audit"],
            "required_citations": ["platform_order_summary", "context_audit_id"],
            "permission_scope": "anonymous_aggregate_only",
            "required_request": ["include_platform_orders=true", "X-FB2-AI-CONTEXT-SCOPE=platform_order_summary"],
            "feedback_routes": ["/api/main-project/context/feedback", "/api/main-project/context/quality-summary"],
            "acceptance_signals": [
                "reply_has_platform_summary",
                "no_single_user_order_detail",
                "matched_cited_sources",
                "permission_summary_records_platform_scope_blocks"
            ],
            "forbidden_outputs": ["single_user_order_detail", "user_identity_leak"]
        },
        {
            "id": "group_opinion_summary",
            "user_question": "群里大家怎么看这场？",
            "entrypoints": ["group_mention_at_el", "summary_post"],
            "context_pack_sections": ["group_opinion_slice", "match_facts", "retrieval_evidence", "quality_feedback"],
            "primary_tools": ["group_opinion_summary", "opinion_memories"],
            "required_source_kinds": ["group_message", "opinion_memory", "match", "context_audit"],
            "required_citations": ["message_id", "opinion_memory_id", "context_audit_id"],
            "permission_scope": "group_visible",
            "required_request": ["group_id", "topic_hint"],
            "feedback_routes": [
                "/api/main-project/context/feedback",
                "/api/main-project/context/opinion-adoption-summary",
                "/api/main-project/context/quality-summary"
            ],
            "acceptance_signals": [
                "reply_has_group_opinions",
                "opinion_adoption_count_non_synthetic",
                "matched_cited_sources",
                "memory_refs_present"
            ],
            "forbidden_outputs": ["group_opinion_as_fact", "fabricated_group_view"]
        },
        {
            "id": "selected_message_review",
            "user_question": "这条消息说得对吗？",
            "entrypoints": ["selected_message_ai_reply"],
            "context_pack_sections": ["match_facts", "group_opinion_slice", "retrieval_evidence", "quality_feedback"],
            "primary_tools": ["match_analysis_brief", "opinion_result_review_summary"],
            "required_source_kinds": ["match", "group_message", "context_audit"],
            "trigger_source_ids": ["selected_message_id"],
            "required_citations": ["selected_message_id", "match_id", "context_audit_id"],
            "permission_scope": "group_visible",
            "required_request": ["group_id", "topic_hint", "selected_message_id"],
            "feedback_routes": ["/api/main-project/context/feedback", "/api/main-project/context/quality-summary"],
            "acceptance_signals": [
                "reply_references_selected_message",
                "reply_rejects_guarantee_claims",
                "reply_has_risk_boundary",
                "matched_cited_sources"
            ],
            "forbidden_outputs": ["unsupported_claim_verdict", "guaranteed_win"]
        },
        {
            "id": "group_discussion_summary_post",
            "user_question": "总结今天群聊讨论。",
            "entrypoints": ["summary_post", "group_summary_post"],
            "context_pack_sections": ["group_opinion_slice", "match_facts", "retrieval_evidence", "quality_feedback"],
            "primary_tools": ["group_opinion_summary", "opinion_memories", "context_feedback_summary"],
            "required_source_kinds": ["group_message", "opinion_memory", "context_audit"],
            "required_citations": ["message_id", "opinion_memory_id", "context_audit_id"],
            "permission_scope": "group_visible",
            "required_request": ["group_id", "topic_hint"],
            "feedback_routes": [
                "/api/main-project/context/feedback",
                "/api/main-project/context/opinion-adoption-summary",
                "/api/main-project/context/quality-summary"
            ],
            "acceptance_signals": [
                "summary_has_group_discussion",
                "summary_has_source_references",
                "matched_cited_sources",
                "summary_post_feedback_recorded"
            ],
            "forbidden_outputs": ["fabricated_group_view", "group_opinion_as_fact", "guaranteed_win"]
        },
        {
            "id": "source_reference_audit",
            "user_question": "你刚才依据了哪些比赛、订单和群消息？",
            "entrypoints": ["group_followup", "chat_bootstrap_ai_reply"],
            "context_pack_sections": ["retrieval_evidence", "quality_feedback", "match_facts", "user_order_slice", "group_opinion_slice"],
            "primary_tools": ["context_feedback_summary", "context_audit_summary"],
            "required_source_kinds": ["context_audit"],
            "required_citations": ["context_audit_id"],
            "permission_scope": "same_as_original_request",
            "required_request": ["context_audit_id_or_previous_main_request_id"],
            "feedback_routes": ["/api/main-project/context/feedbacks", "/api/main-project/context/quality-summary"],
            "acceptance_signals": [
                "reply_lists_sources",
                "does_not_invent_source_id",
                "matched_cited_sources"
            ],
            "forbidden_outputs": ["uncited_claim", "invented_source_id"]
        }
    ])
}


#[cfg(test)]
#[path = "external_app_context_projection_tests.rs"]
mod tests;
