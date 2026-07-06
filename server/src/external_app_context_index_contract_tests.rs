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

        assert_eq!(
            contract["retrieval_evidence_output_shape"]["schema"],
            "fb2.retrieval_evidence_item.v1"
        );
        let evidence_fields = contract["retrieval_evidence_output_shape"]["required_fields"]
            .as_array()
            .unwrap();
        for field in [
            "source_id",
            "source_kind",
            "lane_id",
            "index_id",
            "reason",
            "freshness",
            "permission_scope",
            "citation_source_id",
        ] {
            assert!(evidence_fields.contains(&json!(field)));
        }
    }

    #[test]
    fn ignores_unknown_apps() {
        assert!(public_context_index_guidance("unknown").is_none());
    }
