//! server/src/external_app_context_query_intent.rs
//! Public query-intent contract for fb2 Context Pack requests.

use serde_json::{json, Value};

pub(crate) fn public_context_query_intent_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        // 主项目只描述“这次问题需要什么数据”，fb2 仍负责真实检索、权限裁剪和 Context Pack 投影。
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.context_query_intent.v1",
            "complete": true,
            "purpose": "main_project_to_fb2_context_pack_request_shape",
            "source_of_truth": "fb2_backend_live_business_data",
            "stores_fb2_business_data_in_main_project": false,
            "first_phase_delivery": "rest_context_pack_plus_tool_manifest_plus_tools_execute",
            "request_shape": {
                "required_fields": [
                    "query_intent_id",
                    "entrypoint",
                    "scenario_id",
                    "group_id",
                    "topic_hint",
                    "intent_lanes",
                    "requested_indexes",
                    "permission_scope",
                    "source_request",
                    "output_limits"
                ],
                "source_request_fields": [
                    "external_user_id",
                    "selected_message_id",
                    "include_platform_orders",
                    "context_audit_id",
                    "match_ids",
                    "lottery_type"
                ],
                "required_headers_by_scope": {
                    "current_user_only": ["X-FB2-AI-CONTEXT-USER-ID"],
                    "anonymous_aggregate_only": ["X-FB2-AI-CONTEXT-SCOPE=platform_order_summary"],
                    "single_group_context": ["group_id"]
                },
                "output_limits_fields": [
                    "max_context_chars",
                    "max_sources_per_lane",
                    "max_group_messages",
                    "freshness_window_hours"
                ]
            },
            "entrypoints": [
                {
                    "id": "group_mention_at_el",
                    "topic_hint_source": "last_effective_user_question",
                    "requires": ["group_id", "topic_hint"]
                },
                {
                    "id": "selected_message_ai_reply",
                    "topic_hint_source": "selected_message_summary_plus_user_instruction",
                    "requires": ["group_id", "selected_message_id", "topic_hint"]
                },
                {
                    "id": "group_summary_post",
                    "topic_hint_source": "summary_title_topic_or_instruction",
                    "requires": ["group_id", "topic_hint"]
                },
                {
                    "id": "chat_bootstrap_ai_reply",
                    "topic_hint_source": "current_chat_text",
                    "requires": ["topic_hint"]
                }
            ],
            "scenario_intents": [
                {
                    "scenario_id": "today_matches_analysis",
                    "intent_lanes": ["match_facts_and_odds"],
                    "requested_indexes": ["match_index", "odds_snapshot_index"],
                    "permission_scope": "group_context",
                    "source_request_required": ["group_id", "topic_hint"],
                    "must_not_request": ["other_user_orders", "raw_group_message_body"]
                },
                {
                    "scenario_id": "my_ticket_analysis",
                    "intent_lanes": ["current_user_tickets", "match_facts_and_odds"],
                    "requested_indexes": ["current_user_ticket_index", "match_index", "odds_snapshot_index"],
                    "permission_scope": "current_user_only",
                    "source_request_required": ["external_user_id", "topic_hint"],
                    "required_headers": ["X-FB2-AI-CONTEXT-USER-ID"],
                    "must_not_request": ["other_user_orders", "raw_user_identity"]
                },
                {
                    "scenario_id": "platform_order_risk",
                    "intent_lanes": ["platform_order_summary", "match_facts_and_odds"],
                    "requested_indexes": ["platform_order_risk_index", "match_index"],
                    "permission_scope": "anonymous_aggregate_only",
                    "source_request_required": ["include_platform_orders", "topic_hint"],
                    "required_headers": ["X-FB2-AI-CONTEXT-SCOPE=platform_order_summary"],
                    "must_not_request": ["single_user_order_detail", "raw_user_identity"]
                },
                {
                    "scenario_id": "group_opinion_summary",
                    "intent_lanes": ["group_opinions", "opinion_learning_loop", "match_facts_and_odds"],
                    "requested_indexes": ["group_opinion_index", "opinion_memory_index", "match_index"],
                    "permission_scope": "single_group_context",
                    "source_request_required": ["group_id", "topic_hint"],
                    "must_not_request": ["private_message", "raw_group_message_body"]
                },
                {
                    "scenario_id": "selected_message_review",
                    "intent_lanes": ["group_opinions", "match_facts_and_odds", "quality_feedback_audit"],
                    "requested_indexes": ["group_opinion_index", "match_index", "odds_snapshot_index", "context_audit_index"],
                    "permission_scope": "single_group_context",
                    "source_request_required": ["group_id", "selected_message_id", "topic_hint"],
                    "must_not_request": ["unsupported_claim_verdict_without_sources"]
                },
                {
                    "scenario_id": "group_discussion_summary_post",
                    "intent_lanes": ["group_opinions", "opinion_learning_loop", "quality_feedback_audit"],
                    "requested_indexes": ["group_opinion_index", "opinion_memory_index", "feedback_quality_index"],
                    "permission_scope": "single_group_context",
                    "source_request_required": ["group_id", "topic_hint"],
                    "must_not_request": ["raw_group_message_body", "fabricated_group_view"]
                },
                {
                    "scenario_id": "source_reference_audit",
                    "intent_lanes": ["quality_feedback_audit"],
                    "requested_indexes": ["context_audit_index", "feedback_quality_index"],
                    "permission_scope": "same_as_original_request",
                    "source_request_required": ["context_audit_id"],
                    "must_not_request": ["invented_source_id", "raw_context_pack_body"]
                }
            ],
            "scenario_count": 7,
            "routing_rules": [
                "topic_hint is a compact hint, not a raw transcript dump.",
                "For my_ticket_analysis, external_user_id and X-FB2-AI-CONTEXT-USER-ID must describe the same fb2 user.",
                "For platform_order_risk, include_platform_orders requires X-FB2-AI-CONTEXT-SCOPE=platform_order_summary and returns anonymous aggregates only.",
                "For selected_message_review, selected_message_id identifies the reviewed message; raw message body is not copied into audit artifacts.",
                "fb2 may use internal BM25/vector/cache/index retrieval, but model-visible output must be Context Pack plus citation_sources and retrieval_evidence."
            ],
            "fallback_rules": [
                "If topic_hint is empty, use the latest effective user question in the same group when available.",
                "If the requested lane is unavailable, set preflight_readiness.status=partial or degraded and add retrieval_evidence.missing_context.",
                "If permission headers are missing or mismatched, return a permission error and record permission_denied_count; never broaden scope silently."
            ],
            "privacy_rules": [
                "Do not copy fb2 raw databases, embeddings, full order rows, raw group message bodies, real tokens, or passwords into the main project.",
                "Audit artifacts may keep ids, source ids, text_len, text_sha256, counts, freshness and permission scope.",
                "User-order intent is current_user_only; platform intent is anonymous_aggregate_only; group opinion intent is single_group_context."
            ],
            "acceptance_signals": [
                "context_pack_request_contains_topic_hint",
                "scenario_id_maps_to_lane_and_indexes",
                "permission_scope_matches_headers",
                "retrieval_evidence_items_reference_query_intent",
                "context_audit_records_query_intent_id"
            ]
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::public_context_query_intent_guidance;
    use serde_json::{json, Value};

    fn values(value: &Value, field: &str) -> Vec<String> {
        value[field]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect()
    }

    #[test]
    fn exposes_fb2_context_query_intent_contract() {
        let contract = public_context_query_intent_guidance("fb2").unwrap();

        assert_eq!(contract["schema"], "fb2.context_query_intent.v1");
        assert_eq!(contract["complete"], true);
        assert_eq!(contract["stores_fb2_business_data_in_main_project"], false);
        assert_eq!(contract["scenario_count"], 7);

        let required_fields = values(&contract["request_shape"], "required_fields");
        for field in [
            "query_intent_id",
            "entrypoint",
            "scenario_id",
            "group_id",
            "topic_hint",
            "intent_lanes",
            "requested_indexes",
            "permission_scope",
            "source_request",
            "output_limits",
        ] {
            assert!(required_fields.contains(&field.to_string()));
        }

        let scenarios = contract["scenario_intents"].as_array().unwrap();
        for scenario_id in [
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
                .any(|scenario| scenario["scenario_id"] == json!(scenario_id)));
        }

        let ticket = scenarios
            .iter()
            .find(|scenario| scenario["scenario_id"] == "my_ticket_analysis")
            .unwrap();
        assert_eq!(ticket["permission_scope"], "current_user_only");
        assert!(ticket["requested_indexes"]
            .as_array()
            .unwrap()
            .contains(&json!("current_user_ticket_index")));
        assert!(ticket["required_headers"]
            .as_array()
            .unwrap()
            .contains(&json!("X-FB2-AI-CONTEXT-USER-ID")));

        let privacy_rules = values(&contract, "privacy_rules").join("\n");
        assert!(privacy_rules.contains("raw group message bodies"));
        assert!(public_context_query_intent_guidance("unknown").is_none());
    }
}
