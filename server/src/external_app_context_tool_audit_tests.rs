    use super::*;

    #[test]
    fn audit_counts_statuses_and_sources() {
        let results = vec![
            json!({
                "tool_name": "search_matches",
                "status": "ready",
                "source_ids": ["match-1", "match-2"],
                "visibility": "group_context",
                "grounding": {"status": "grounded", "warnings": []}
            }),
            json!({
                "tool_name": "search_user_orders",
                "status": "skipped",
                "grounding": {"status": "unavailable", "warnings": []}
            }),
        ];
        let audit = execution_audit(
            "exec-1",
            &["search_matches", "search_user_orders"],
            &results,
            42,
        );

        assert_eq!(audit["ready_count"], 1);
        assert_eq!(audit["skipped_count"], 1);
        assert_eq!(audit["source_id_count"], 2);
        assert_eq!(audit["grounded_result_count"], 1);
        assert_eq!(
            audit["answer_grounding"]["facts_allowed_from_grounded_results"],
            true
        );
        assert_eq!(execution_status(&results), "partial");
    }

    #[test]
    fn audit_surfaces_grounding_warnings() {
        let results = vec![json!({
            "tool_name": "search_user_orders",
            "status": "ready",
            "source_ids": [],
            "visibility": "current_user_only",
            "grounding": {"status": "weak", "warnings": ["missing_source_ids"]}
        })];
        let audit = execution_audit("exec-1", &["search_user_orders"], &results, 42);

        assert_eq!(audit["weak_result_count"], 1);
        assert_eq!(
            audit["answer_grounding"]["weak_results_require_caveat"],
            true
        );
        assert!(audit["grounding_warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("missing_source_ids")));
    }

    #[test]
    fn audit_counts_match_brief_user_orders_as_current_user_data() {
        let results = vec![json!({
            "tool_name": "match_analysis_brief",
            "status": "ready",
            "source_ids": ["match-1", "order-1"],
            "visibility": "match_focused_brief",
            "data": {
                "user_orders": [{"order_id": "order-1"}]
            },
            "grounding": {"status": "grounded", "warnings": []}
        })];
        let audit = execution_audit("exec-1", &["match_analysis_brief"], &results, 42);

        assert_eq!(audit["has_current_user_only_result"], true);
        assert_eq!(
            audit["answer_grounding"]["current_user_only_data_present"],
            true
        );
    }
