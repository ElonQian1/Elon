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
        assert!(source_kinds.contains(&json!("context_audit")));
        assert!(source_kinds.contains(&json!("match")));
        assert!(source_kinds.contains(&json!("odds")));
        assert!(source_kinds.contains(&json!("user_order")));
        assert!(source_kinds.contains(&json!("group_message")));
        assert!(source_kinds.contains(&json!("platform_order_summary")));
        assert!(!source_kinds.contains(&json!("feedback")));
        assert!(!source_kinds.contains(&json!("opinion_adoption")));

        let quality_kinds = contract["source_registry"]["quality_history_kinds"]
            .as_array()
            .unwrap();
        assert!(quality_kinds
            .iter()
            .any(|kind| { kind["kind"] == "feedback" && kind["scope"] == "quality_history" }));
        assert!(quality_kinds.iter().any(|kind| {
            kind["kind"] == "opinion_adoption" && kind["default_chat_fact"] == false
        }));

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
        assert_eq!(
            contract["retrieval_projection"]["item_shape"]["schema"],
            "fb2.retrieval_evidence_item.v1"
        );
        let evidence_fields = &contract["retrieval_projection"]["item_shape"]["required_fields"];
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
            assert!(array_contains(evidence_fields, field));
        }
        let linking_rules = &contract["retrieval_projection"]["item_shape"]["linking_rules"];
        assert!(linking_rules
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule.as_str().unwrap().contains("citation_sources[].id")));

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

    fn scenario<'a>(matrix: &'a [Value], id: &str) -> &'a Value {
        matrix
            .iter()
            .find(|scenario| scenario["id"] == id)
            .unwrap_or_else(|| panic!("missing domain scenario {id}"))
    }

    #[test]
    fn fb2_domain_projection_declares_scenario_matrix() {
        let contract = public_context_projection_guidance("fb2").unwrap();
        let matrix = contract["domain_scenario_matrix"].as_array().unwrap();
        assert_eq!(matrix.len(), 7);

        let today = scenario(matrix, "today_matches_analysis");
        assert!(array_contains(
            &today["context_pack_sections"],
            "match_facts"
        ));
        assert!(array_contains(
            &today["primary_tools"],
            "match_analysis_brief"
        ));
        assert!(array_contains(&today["required_source_kinds"], "odds"));
        assert!(array_contains(
            &today["forbidden_outputs"],
            "fabricated_odds"
        ));

        let ticket = scenario(matrix, "my_ticket_analysis");
        assert_eq!(ticket["permission_scope"], "current_user_only");
        assert!(array_contains(
            &ticket["context_pack_sections"],
            "user_order_slice"
        ));
        assert!(array_contains(
            &ticket["required_request"],
            "X-FB2-AI-CONTEXT-USER-ID"
        ));
        assert!(array_contains(&ticket["required_citations"], "ticket_id"));
        assert!(array_contains(
            &ticket["acceptance_signals"],
            "only_current_user_orders"
        ));

        let platform = scenario(matrix, "platform_order_risk");
        assert_eq!(platform["permission_scope"], "anonymous_aggregate_only");
        assert!(array_contains(
            &platform["required_request"],
            "X-FB2-AI-CONTEXT-SCOPE=platform_order_summary"
        ));
        assert!(array_contains(
            &platform["forbidden_outputs"],
            "single_user_order_detail"
        ));

        let opinions = scenario(matrix, "group_opinion_summary");
        assert!(array_contains(
            &opinions["context_pack_sections"],
            "group_opinion_slice"
        ));
        assert!(array_contains(
            &opinions["feedback_routes"],
            "/api/main-project/context/opinion-adoption-summary"
        ));
        assert!(array_contains(
            &opinions["acceptance_signals"],
            "memory_refs_present"
        ));

        let selected = scenario(matrix, "selected_message_review");
        assert!(array_contains(
            &selected["trigger_source_ids"],
            "selected_message_id"
        ));
        assert!(array_contains(
            &selected["acceptance_signals"],
            "reply_rejects_guarantee_claims"
        ));

        let summary_post = scenario(matrix, "group_discussion_summary_post");
        assert!(array_contains(
            &summary_post["entrypoints"],
            "group_summary_post"
        ));
        assert!(array_contains(
            &summary_post["required_source_kinds"],
            "opinion_memory"
        ));
        assert!(array_contains(
            &summary_post["acceptance_signals"],
            "summary_post_feedback_recorded"
        ));
        assert!(array_contains(
            &summary_post["forbidden_outputs"],
            "fabricated_group_view"
        ));

        let audit = scenario(matrix, "source_reference_audit");
        assert!(array_contains(
            &audit["context_pack_sections"],
            "quality_feedback"
        ));
        assert!(array_contains(
            &audit["feedback_routes"],
            "/api/main-project/context/feedbacks"
        ));
    }

    #[test]
    fn exposes_fb2_domain_data_blueprint_contract() {
        let blueprint = public_domain_data_blueprint_guidance("fb2").unwrap();
        assert_eq!(
            blueprint["schema"],
            "fb2.main_project.domain_data_blueprint.v1"
        );
        assert_eq!(
            blueprint["context_format"],
            "xml_wrapped_markdown_context_pack_with_json_metadata"
        );
        assert_eq!(
            blueprint["first_phase_delivery"],
            "rest_context_pack_plus_tool_manifest_plus_tools_execute"
        );
        assert_eq!(
            blueprint["mcp_status"],
            "future_wrapper_not_first_phase_fact_source"
        );
        assert_eq!(
            blueprint["stores_fb2_business_data_in_main_project"],
            json!(false)
        );
        assert_eq!(blueprint["lane_count"], json!(6));

        let lanes = blueprint["lanes"].as_array().unwrap();
        for id in [
            "match_facts_and_odds",
            "current_user_tickets",
            "platform_order_summary",
            "group_opinions",
            "opinion_learning_loop",
            "quality_feedback_audit",
        ] {
            assert!(lanes.iter().any(|lane| lane["id"] == id));
        }

        let ticket_lane = lanes
            .iter()
            .find(|lane| lane["id"] == "current_user_tickets")
            .unwrap();
        assert_eq!(ticket_lane["permission_scope"], "current_user_only");
        assert!(array_contains(&ticket_lane["source_kinds"], "user_order"));
        assert!(array_contains(
            &ticket_lane["forbidden_outputs"],
            "other_user_order_detail"
        ));

        let opinion_lane = lanes
            .iter()
            .find(|lane| lane["id"] == "opinion_learning_loop")
            .unwrap();
        assert!(array_contains(
            &opinion_lane["source_kinds"],
            "opinion_adoption"
        ));
        assert!(array_contains(
            &opinion_lane["forbidden_outputs"],
            "quality_history_as_match_fact"
        ));

        assert!(array_contains(
            &blueprint["required_context_pack_sections"],
            "group_opinion_slice"
        ));
        assert!(array_contains(
            &blueprint["required_metadata"],
            "citation_sources"
        ));
        assert!(array_contains(
            &blueprint["anti_patterns"],
            "full_database_dump"
        ));
        assert!(public_domain_data_blueprint_guidance("unknown").is_none());
    }
