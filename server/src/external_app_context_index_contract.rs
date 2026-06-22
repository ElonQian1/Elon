//! server/src/external_app_context_index_contract.rs
//! Public fb2 domain index contract for long-term Context Pack retrieval.

use serde_json::{json, Value};

pub(crate) fn public_context_index_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        // fb2 的长期优化方向是“业务索引 -> Context Pack 投影 -> 工具补查”，不是把索引或向量原文直接塞给模型。
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.main_project.domain_context_index.v1",
            "complete": true,
            "purpose": "fb2_side_hybrid_retrieval_for_match_order_group_opinion_context_packs",
            "source_of_truth": "fb2_backend_live_business_data",
            "stores_fb2_business_data_in_main_project": false,
            "first_phase_delivery": "index_guides_rest_context_pack_and_tool_manifest",
            "mcp_status": "future_wrapper_not_first_phase_fact_source",
            "index_output_boundary": {
                "model_visible_output": "retrieval_evidence_section_plus_citation_sources",
                "machine_visible_output": "json_metadata_metrics_and_tool_contract",
                "not_allowed": [
                    "raw_embedding_dump",
                    "full_database_dump",
                    "uncited_index_hit",
                    "other_user_order_detail",
                    "platform_order_detail_leak"
                ]
            },
            "required_query_inputs": [
                "group_id",
                "topic_hint",
                "external_user_id_when_user_orders_are_requested",
                "selected_message_id_when_reviewing_message",
                "include_platform_orders_with_platform_scope_only"
            ],
            "retrieval_plan": [
                "classify_user_intent",
                "select_domain_indexes",
                "retrieve_minimal_source_ids",
                "project_to_xml_wrapped_markdown_context_pack",
                "attach_citation_sources_and_metrics",
                "record_context_audit_id",
                "feed_answer_feedback_back_to_fb2_quality_indexes"
            ],
            "indexes": [
                {
                    "id": "match_index",
                    "lane_id": "match_facts_and_odds",
                    "source_kinds": ["match"],
                    "lookup_keys": ["match_id", "league", "team_alias", "match_time", "lottery_type", "status"],
                    "required_fields": ["match_id", "league", "home_team", "away_team", "match_time", "status", "updated_at"],
                    "permission_scope": "group_context",
                    "context_pack_sections": ["match_facts", "retrieval_evidence"],
                    "primary_tools": ["search_matches", "match_analysis_brief", "get_match_detail"],
                    "freshness_rule": "prefer_today_and_next_48h_matches_for_today_questions",
                    "forbidden_outputs": ["fabricated_match", "uncited_match_fact"]
                },
                {
                    "id": "odds_snapshot_index",
                    "lane_id": "match_facts_and_odds",
                    "source_kinds": ["odds"],
                    "lookup_keys": ["match_id", "market", "updated_at", "odds_provider"],
                    "required_fields": ["match_id", "market", "odds_value", "updated_at", "source_id"],
                    "permission_scope": "group_context",
                    "context_pack_sections": ["match_facts", "retrieval_evidence"],
                    "primary_tools": ["match_analysis_brief", "get_match_detail"],
                    "freshness_rule": "include_latest_snapshot_and_change_summary_when_available",
                    "forbidden_outputs": ["fabricated_odds", "odds_without_updated_at"]
                },
                {
                    "id": "current_user_ticket_index",
                    "lane_id": "current_user_tickets",
                    "source_kinds": ["user_order", "ticket"],
                    "lookup_keys": ["external_user_id", "order_id", "ticket_id", "match_id", "created_at", "status"],
                    "required_fields": ["order_id", "ticket_id", "external_user_id_hash", "match_ids", "stake_summary", "status", "source_id"],
                    "permission_scope": "current_user_only",
                    "context_pack_sections": ["user_order_slice", "match_facts", "retrieval_evidence"],
                    "primary_tools": ["match_analysis_brief", "search_user_orders", "get_order_detail"],
                    "freshness_rule": "current_user_orders_only_never_cross_user",
                    "forbidden_outputs": ["other_user_order_detail", "raw_user_identity", "guaranteed_win"]
                },
                {
                    "id": "platform_order_risk_index",
                    "lane_id": "platform_order_summary",
                    "source_kinds": ["platform_order_summary"],
                    "lookup_keys": ["date", "shop_id", "match_id", "market", "risk_bucket"],
                    "required_fields": ["summary_id", "date", "scope", "aggregate_count", "risk_bucket", "source_id"],
                    "permission_scope": "privileged_anonymous_summary",
                    "context_pack_sections": ["platform_order_summary", "retrieval_evidence"],
                    "primary_tools": ["platform_orders"],
                    "freshness_rule": "aggregate_only_never_emit_single_user_rows",
                    "forbidden_outputs": ["single_user_order_detail", "user_identity_leak"]
                },
                {
                    "id": "group_opinion_index",
                    "lane_id": "group_opinions",
                    "source_kinds": ["group_message"],
                    "lookup_keys": ["group_id", "message_id", "match_id", "team_alias", "stance", "created_at"],
                    "required_fields": ["message_id", "group_id", "stance", "opinion_summary", "created_at", "text_hash"],
                    "permission_scope": "single_group_context",
                    "context_pack_sections": ["group_opinion_slice", "retrieval_evidence"],
                    "primary_tools": ["group_opinion_summary", "search_group_opinions"],
                    "freshness_rule": "prefer_recent_group_messages_and_keep_text_hash_not_full_body_in_audits",
                    "forbidden_outputs": ["fabricated_group_view", "group_opinion_as_match_fact"]
                },
                {
                    "id": "opinion_memory_index",
                    "lane_id": "opinion_learning_loop",
                    "source_kinds": ["opinion_memory", "opinion_adoption"],
                    "lookup_keys": ["group_id", "memory_id", "match_id", "intent", "adopted_at", "expires_at"],
                    "required_fields": ["memory_id", "group_id", "summary", "source_message_ids", "adoption_count", "source_id"],
                    "permission_scope": "single_group_quality_history",
                    "context_pack_sections": ["group_opinion_slice", "quality_feedback"],
                    "primary_tools": ["opinion_memories", "list_opinion_adoptions", "opinion_adoption_summary"],
                    "freshness_rule": "include_unexpired_memories_and_recent_adoptions_only",
                    "forbidden_outputs": ["uncited_opinion_memory", "quality_history_as_match_fact"]
                },
                {
                    "id": "context_audit_index",
                    "lane_id": "quality_feedback_audit",
                    "source_kinds": ["context_audit"],
                    "lookup_keys": ["context_audit_id", "main_request_id", "group_id", "scenario", "generated_at"],
                    "required_fields": ["context_audit_id", "source_counts", "context_pack_chars", "latency_ms", "budget_status", "missing_context"],
                    "permission_scope": "audit_metadata_only",
                    "context_pack_sections": ["retrieval_evidence", "quality_feedback"],
                    "primary_tools": ["get_context_audit", "context_audit_summary"],
                    "freshness_rule": "retain_enough_metadata_to_replay_source_selection_without_business_row_copy",
                    "forbidden_outputs": ["raw_order_detail", "raw_group_message_body"]
                },
                {
                    "id": "feedback_quality_index",
                    "lane_id": "quality_feedback_audit",
                    "source_kinds": ["feedback"],
                    "lookup_keys": ["main_request_id", "trigger", "context_audit_id", "wrong_context", "missing_context", "created_at"],
                    "required_fields": ["feedback_id", "trigger", "matched_cited_source_count", "unmatched_cited_source_count", "wrong_context", "missing_context"],
                    "permission_scope": "quality_metrics_only",
                    "context_pack_sections": ["quality_feedback"],
                    "primary_tools": ["list_context_feedbacks", "context_audit_summary"],
                    "freshness_rule": "feed_failed_samples_back_into_prompt_and_tool_selection_tuning",
                    "forbidden_outputs": ["feedback_as_match_fact", "private_user_detail"]
                }
            ],
            "index_count": 8,
            "context_pack_projection_rules": [
                "Every index hit used as a fact must become a citation_sources entry.",
                "Context Pack may include summaries and source ids, not raw full tables.",
                "retrieval_evidence must explain selected indexes, query keys, freshness, permission scope and missing_context.",
                "Quality feedback and opinion adoption are quality history, not match facts."
            ],
            "required_metrics": [
                "index_latency_ms",
                "retrieved_source_count",
                "source_counts",
                "stale_source_count",
                "permission_denied_count",
                "budget_status",
                "fallback_used"
            ],
            "acceptance_queries": [
                {
                    "id": "today_matches",
                    "question": "今天比赛怎么看",
                    "required_indexes": ["match_index", "odds_snapshot_index"],
                    "required_source_kinds": ["match", "odds", "context_audit"]
                },
                {
                    "id": "my_ticket",
                    "question": "帮我分析我的票",
                    "required_indexes": ["current_user_ticket_index", "match_index", "odds_snapshot_index"],
                    "required_source_kinds": ["user_order", "ticket", "match", "context_audit"],
                    "permission_scope": "current_user_only"
                },
                {
                    "id": "platform_order_risk",
                    "question": "平台今天订单风险怎么样",
                    "required_indexes": ["platform_order_risk_index"],
                    "required_source_kinds": ["platform_order_summary", "context_audit"],
                    "permission_scope": "privileged_anonymous_summary"
                },
                {
                    "id": "group_opinion",
                    "question": "群里大家怎么看这场",
                    "required_indexes": ["group_opinion_index", "opinion_memory_index"],
                    "required_source_kinds": ["group_message", "opinion_memory", "context_audit"],
                    "permission_scope": "single_group_context"
                }
            ]
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::public_context_index_guidance;
    use serde_json::json;

    #[test]
    fn exposes_fb2_domain_context_index_contract() {
        let contract = public_context_index_guidance("fb2").unwrap();

        assert_eq!(
            contract["schema"],
            "fb2.main_project.domain_context_index.v1"
        );
        assert_eq!(contract["complete"], true);
        assert_eq!(contract["stores_fb2_business_data_in_main_project"], false);
        assert_eq!(contract["index_count"], 8);

        let indexes = contract["indexes"].as_array().unwrap();
        for id in [
            "match_index",
            "odds_snapshot_index",
            "current_user_ticket_index",
            "platform_order_risk_index",
            "group_opinion_index",
            "opinion_memory_index",
            "context_audit_index",
            "feedback_quality_index",
        ] {
            assert!(indexes.iter().any(|entry| entry["id"] == json!(id)));
        }

        let required_inputs = contract["required_query_inputs"].as_array().unwrap();
        assert!(required_inputs.contains(&json!("topic_hint")));
        assert!(required_inputs.contains(&json!("external_user_id_when_user_orders_are_requested")));

        let not_allowed = contract["index_output_boundary"]["not_allowed"]
            .as_array()
            .unwrap();
        assert!(not_allowed.contains(&json!("raw_embedding_dump")));
        assert!(not_allowed.contains(&json!("full_database_dump")));
    }

    #[test]
    fn ignores_unknown_apps() {
        assert!(public_context_index_guidance("unknown").is_none());
    }
}
