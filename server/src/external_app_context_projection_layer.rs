//! server/src/external_app_context_projection_layer.rs
//! Public contract for fb2's long-term AI context projection layer.

use serde_json::{json, Value};

pub(crate) fn public_context_projection_layer_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.main_project.context_projection_layer.v1",
            "complete": true,
            "source_document": "docs/fb2-ai-center/context-projection-layer.md",
            "context_format": "xml_wrapped_markdown_context_pack_with_compact_json_metadata",
            "first_phase_delivery": "rest_context_pack_plus_tool_manifest_plus_tools_execute",
            "mcp_status": "future_wrapper_not_first_phase_fact_source",
            "source_of_truth": "fb2_backend_live_business_data",
            "stores_fb2_business_data_in_main_project": false,
            "ai_facing_payload": {
                "body_format": "XML-wrapped Markdown",
                "wrapper": "fb2_context_pack",
                "metadata_format": "compact_json_metadata",
                "not_allowed": [
                    "raw_html_prompt",
                    "giant_json_prompt",
                    "full_database_dump",
                    "raw_embedding_dump"
                ],
                "rule": "fb2 indexes and tools retrieve business data; the main project receives projected evidence, source references, metrics, and safe tool results."
            },
            "domain_lanes": [
                {
                    "id": "match_facts_and_odds",
                    "context_sections": ["match_facts", "retrieval_evidence"],
                    "primary_tools": ["match_analysis_brief", "search_matches", "get_match_detail"],
                    "permission": "group_context"
                },
                {
                    "id": "current_user_tickets",
                    "context_sections": ["user_order_slice", "match_facts", "retrieval_evidence"],
                    "primary_tools": ["match_analysis_brief", "search_user_orders", "get_order_detail"],
                    "permission": "current_user_only"
                },
                {
                    "id": "platform_order_summary",
                    "context_sections": ["platform_order_summary", "retrieval_evidence"],
                    "primary_tools": ["platform_orders"],
                    "permission": "privileged_anonymous_summary"
                },
                {
                    "id": "group_opinions",
                    "context_sections": ["group_opinion_slice", "match_facts", "retrieval_evidence"],
                    "primary_tools": ["group_opinion_summary", "search_group_opinions", "opinion_memories"],
                    "permission": "single_group_context"
                },
                {
                    "id": "opinion_learning_loop",
                    "context_sections": ["quality_feedback", "group_opinion_slice"],
                    "primary_tools": ["list_opinion_adoptions", "opinion_adoption_summary", "opinion_result_reviews"],
                    "permission": "single_group_quality_history"
                },
                {
                    "id": "quality_feedback_audit",
                    "context_sections": ["quality_feedback", "retrieval_evidence"],
                    "primary_tools": ["get_context_audit", "context_audit_summary", "list_context_feedbacks"],
                    "permission": "audit_metadata_only"
                }
            ],
            "domain_lane_count": 6,
            "domain_indexes": [
                {"id": "match_index", "owner": "fb2", "main_project_receives": "projected_evidence_only"},
                {"id": "odds_snapshot_index", "owner": "fb2", "main_project_receives": "projected_evidence_only"},
                {"id": "current_user_ticket_index", "owner": "fb2", "main_project_receives": "projected_evidence_only"},
                {"id": "platform_order_risk_index", "owner": "fb2", "main_project_receives": "projected_evidence_only"},
                {"id": "group_opinion_index", "owner": "fb2", "main_project_receives": "projected_evidence_only"},
                {"id": "opinion_memory_index", "owner": "fb2", "main_project_receives": "projected_evidence_only"},
                {"id": "context_audit_index", "owner": "fb2", "main_project_receives": "projected_evidence_only"},
                {"id": "feedback_quality_index", "owner": "fb2", "main_project_receives": "projected_evidence_only"}
            ],
            "domain_index_count": 8,
            "retrieval_evidence_contract": {
                "schema": "fb2.retrieval_evidence_item.v1",
                "model_visible_section": "retrieval_evidence",
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
                "rule": "Every model-visible recalled business fact must have a retrieval evidence item that explains source selection, permission scope, freshness, and the citation source id used for feedback."
            },
            "user_scenarios": [
                {
                    "id": "today_matches_analysis",
                    "required_source_kinds": ["match", "odds", "context_audit"],
                    "required_answer_layers": ["match_facts", "odds_facts", "ai_inference", "risk_boundary"]
                },
                {
                    "id": "my_ticket_analysis",
                    "required_source_kinds": ["user_order", "ticket", "context_audit"],
                    "required_answer_layers": ["current_user_orders", "match_facts", "ai_inference", "risk_boundary"]
                },
                {
                    "id": "platform_order_risk",
                    "required_source_kinds": ["platform_order_summary", "context_audit"],
                    "required_answer_layers": ["platform_aggregate", "ai_inference", "risk_boundary"]
                },
                {
                    "id": "group_opinion_summary",
                    "required_source_kinds": ["group_message", "opinion_memory", "context_audit"],
                    "required_answer_layers": ["group_opinion", "match_facts", "ai_inference", "risk_boundary"]
                },
                {
                    "id": "selected_message_review",
                    "required_source_kinds": ["group_message", "match", "odds", "context_audit"],
                    "required_answer_layers": ["reviewed_claim", "facts", "ai_inference", "risk_boundary"]
                },
                {
                    "id": "group_discussion_summary_post",
                    "required_source_kinds": ["group_message", "opinion_memory", "context_audit"],
                    "required_answer_layers": ["discussion_summary", "source_references", "risk_boundary"]
                },
                {
                    "id": "source_reference_audit",
                    "required_source_kinds": ["context_audit", "feedback"],
                    "required_answer_layers": ["source_registry", "data_fact_boundary", "quality_feedback"]
                }
            ],
            "user_scenario_count": 7,
            "forbidden_outputs": [
                "fabricated_odds",
                "guaranteed_win",
                "other_user_order_detail",
                "single_user_order_detail",
                "user_identity_leak",
                "fabricated_group_view",
                "group_opinion_as_fact",
                "uncited_source",
                "raw_embedding_dump",
                "full_database_dump"
            ],
            "group_chat_evidence": {
                "method": "direct_api_read",
                "screenshots_accepted": false,
                "required_fields": [
                    "message_id",
                    "type",
                    "sender_id",
                    "created_at",
                    "text_len",
                    "text_sha256"
                ],
                "rule": "Screenshots and recordings help UI debugging only; acceptance requires API-read message ids, text lengths, hashes, reply ids, and feedback evidence."
            }
        })),
        _ => None,
    }
}


#[cfg(test)]
#[path = "external_app_context_projection_layer_tests.rs"]
mod tests;
