    use super::*;

    fn ids(value: &Value, field: &str) -> Vec<String> {
        value[field]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item.get("id").and_then(Value::as_str))
            .map(ToOwned::to_owned)
            .collect()
    }

    #[test]
    fn exposes_fb2_context_projection_layer_contract() {
        let contract = public_context_projection_layer_guidance("fb2").unwrap();
        assert_eq!(
            contract["schema"],
            "fb2.main_project.context_projection_layer.v1"
        );
        assert_eq!(contract["complete"], true);
        assert_eq!(contract["ai_facing_payload"]["wrapper"], "fb2_context_pack");
        assert_eq!(contract["stores_fb2_business_data_in_main_project"], false);
        assert_eq!(
            contract["first_phase_delivery"],
            "rest_context_pack_plus_tool_manifest_plus_tools_execute"
        );
        assert_eq!(
            contract["mcp_status"],
            "future_wrapper_not_first_phase_fact_source"
        );

        let lane_ids = ids(&contract, "domain_lanes");
        for lane in [
            "match_facts_and_odds",
            "current_user_tickets",
            "platform_order_summary",
            "group_opinions",
            "opinion_learning_loop",
            "quality_feedback_audit",
        ] {
            assert!(lane_ids.contains(&lane.to_string()));
        }
        assert_eq!(contract["domain_lane_count"], 6);

        let index_ids = ids(&contract, "domain_indexes");
        for index in [
            "match_index",
            "odds_snapshot_index",
            "current_user_ticket_index",
            "platform_order_risk_index",
            "group_opinion_index",
            "opinion_memory_index",
            "context_audit_index",
            "feedback_quality_index",
        ] {
            assert!(index_ids.contains(&index.to_string()));
        }
        assert_eq!(contract["domain_index_count"], 8);
        assert_eq!(
            contract["retrieval_evidence_contract"]["schema"],
            "fb2.retrieval_evidence_item.v1"
        );
        assert!(contract["retrieval_evidence_contract"]["required_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("citation_source_id")));

        let scenario_ids = ids(&contract, "user_scenarios");
        for scenario in [
            "today_matches_analysis",
            "my_ticket_analysis",
            "platform_order_risk",
            "group_opinion_summary",
            "selected_message_review",
            "group_discussion_summary_post",
            "source_reference_audit",
        ] {
            assert!(scenario_ids.contains(&scenario.to_string()));
        }
        assert_eq!(contract["user_scenario_count"], 7);

        let forbidden = contract["forbidden_outputs"].as_array().unwrap();
        assert!(forbidden.contains(&json!("fabricated_odds")));
        assert!(forbidden.contains(&json!("raw_embedding_dump")));
        assert!(forbidden.contains(&json!("full_database_dump")));

        assert_eq!(contract["group_chat_evidence"]["method"], "direct_api_read");
        assert_eq!(
            contract["group_chat_evidence"]["screenshots_accepted"],
            false
        );
        assert!(contract["group_chat_evidence"]["required_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("text_sha256")));
        assert!(public_context_projection_layer_guidance("unknown").is_none());
    }
