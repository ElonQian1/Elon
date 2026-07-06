    use super::*;

    #[test]
    fn grounded_when_visibility_and_source_ids_match() {
        let result = normalize_parsed_tool_result(
            "search_matches",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "source_ids": ["match-1"],
                "visibility": "group_context"
            }),
        );

        assert_eq!(result["grounding"]["status"], "grounded");
        assert_eq!(result["schema"], "external_app.normalized_tool_result.v1");
        assert_eq!(result["grounding"]["source_id_count"], 1);
        assert_eq!(result["grounding"]["facts_allowed"], true);
    }

    #[test]
    fn weak_when_source_ids_are_missing() {
        let result = normalize_parsed_tool_result(
            "search_user_orders",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "visibility": "current_user_only"
            }),
        );

        assert_eq!(result["grounding"]["status"], "weak");
        assert_eq!(result["grounding"]["requires_caveat"], true);
        assert!(result["grounding"]["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("missing_source_ids")));
    }

    #[test]
    fn unsafe_when_visibility_is_wrong() {
        let result = normalize_parsed_tool_result(
            "search_user_orders",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "source_ids": ["order-1"],
                "visibility": "group_context"
            }),
        );

        assert_eq!(result["grounding"]["status"], "unsafe");
        assert_eq!(result["grounding"]["facts_allowed"], false);
    }

    #[test]
    fn platform_orders_require_privileged_visibility_and_source_ids() {
        let result = normalize_parsed_tool_result(
            "platform_orders",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "source_ids": ["platform_order_summary:2026-06-21:all"],
                "visibility": "privileged_summary"
            }),
        );

        assert_eq!(result["grounding"]["status"], "grounded");
        assert_eq!(
            result["grounding"]["expected_visibility"].as_str(),
            Some("privileged_summary")
        );
    }

    #[test]
    fn opinion_memories_require_persistent_index_visibility_and_source_ids() {
        let result = normalize_parsed_tool_result(
            "opinion_memories",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "source_ids": ["opinion-memory-1"],
                "visibility": "single_group_persistent_opinion_index"
            }),
        );

        assert_eq!(result["grounding"]["status"], "grounded");
        assert_eq!(
            result["grounding"]["expected_visibility"].as_str(),
            Some("single_group_persistent_opinion_index")
        );
    }

    #[test]
    fn aggregate_opinion_and_match_brief_have_dedicated_visibility() {
        let opinion_summary = normalize_parsed_tool_result(
            "group_opinion_summary",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "source_ids": ["message-1"],
                "visibility": "single_group_lightweight_memory"
            }),
        );
        assert_eq!(opinion_summary["grounding"]["status"], "grounded");
        assert_eq!(
            opinion_summary["grounding"]["expected_visibility"].as_str(),
            Some("single_group_lightweight_memory")
        );

        let match_brief = normalize_parsed_tool_result(
            "match_analysis_brief",
            "reason",
            "req-2",
            &json!({
                "success": true,
                "source_ids": ["match-1", "message-1"],
                "visibility": "match_focused_brief"
            }),
        );
        assert_eq!(match_brief["grounding"]["status"], "grounded");
        assert_eq!(
            match_brief["grounding"]["expected_visibility"].as_str(),
            Some("match_focused_brief")
        );
    }

    #[test]
    fn opinion_result_review_summary_accepts_metrics_visibility_without_source_ids() {
        let result = normalize_parsed_tool_result(
            "opinion_result_review_summary",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "visibility": "single_group_opinion_result_review_metrics"
            }),
        );

        assert_eq!(result["grounding"]["status"], "grounded");
        assert_eq!(
            result["grounding"]["expected_visibility"].as_str(),
            Some("single_group_opinion_result_review_metrics")
        );
        assert_eq!(result["grounding"]["source_ids_required"], false);
    }

    #[test]
    fn opinion_result_review_samples_require_source_ids() {
        let result = normalize_parsed_tool_result(
            "opinion_result_reviews",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "visibility": "single_group_opinion_result_review_samples"
            }),
        );

        assert_eq!(result["grounding"]["status"], "weak");
        assert!(result["grounding"]["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("missing_source_ids")));
    }
