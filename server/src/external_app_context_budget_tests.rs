    use super::*;

    #[test]
    fn prompt_prefers_context_pack_body() {
        let block = prompt_context_block(&json!({
            "source": "fb2:/api/main-project/context/pack",
            "status": "ready",
            "context_pack": "<fb2_context_pack>hello</fb2_context_pack>",
            "tool_contract": {"tools": [{"name": "get_match_detail"}]},
            "usage_policy": {"no_guaranteed_win": true},
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "context_audit_id": "audit-1",
            "metrics": {"budget_status": "ok"},
            "preflight_readiness": {
                "status": "degraded",
                "warnings": ["fb2_readiness_degraded"]
            },
            "matches": [{"id": "match-1"}],
            "citation_sources": [
                {"kind": "match", "id": "match-1", "label": "比赛 match-1"},
                {"kind": "platform_order_summary", "id": "platform_order_summary:2026-06-21:all", "label": "平台订单摘要"}
            ],
            "user_orders": [{
                "order_id": "order-1",
                "status": "pending",
                "total_amount": 54,
                "bet_slips": [{
                    "match_id": "match-1",
                    "home_team": "主队",
                    "away_team": "客队",
                    "selection": "主胜",
                    "odds": 1.96
                }]
            }],
            "group_messages": [{"message_id": "message-1"}]
        }));
        assert!(block.contains("<fb2_context_pack>hello</fb2_context_pack>"));
        assert!(block.contains("answer_policy="));
        assert!(block.contains("fb2.answer_policy.v1"));
        assert!(block.contains("context_quality="));
        assert!(block.contains("context_gap_summary="));
        assert!(block.contains("external_metrics="));
        assert!(block.contains("context_fact_summary="));
        assert!(block.contains("\"user_order_count\":1"));
        assert!(block.contains("\"preflight_readiness\""));
        assert!(block.contains("\"status\":\"degraded\""));
        assert!(block.contains("fb2_readiness_degraded"));
        assert!(block.contains("platform_order_summary:2026-06-21:all"));
        assert!(block.contains("\"kind\":\"platform_order_summary\""));
        assert!(block.contains("order-1"));
        assert!(block.contains("\"bet_slip_count\":1"));
        assert!(block.contains("\"selection\":\"主胜\""));
        assert!(block.contains("context_audit_id=audit-1"));
        assert!(block.contains("get_match_detail"));
        assert!(block.contains("tool_readiness.status"));
        assert!(block.contains("必须区分"));
    }

    #[test]
    fn prompt_gap_summary_surfaces_blocked_or_empty_context() {
        let context = json!({
            "source": "fb2:/api/main-project/context/pack",
            "status": "ready",
            "generated_at": "2026-06-22T12:00:00+08:00",
            "context_pack": "<fb2_context_pack>没有可用比赛</fb2_context_pack>",
            "metrics": {"budget_status": "empty"},
            "preflight_readiness": {
                "status": "blocked",
                "warnings": ["fb2_readiness_blocked"]
            },
            "context_quality": {
                "warnings": ["fb2_readiness_blocked", "fb2_budget_empty", "empty_matches"]
            },
            "matches": [],
            "user_orders": [],
            "group_messages": []
        });

        let block = prompt_context_block(&context);

        assert!(block.contains("context_gap_summary="));
        assert!(block.contains("\"readiness_status\":\"blocked\""));
        assert!(block.contains("\"budget_status\":\"empty\""));
        assert!(block.contains("\"matches\":false"));
        assert!(block.contains("\"user_orders\":false"));
        assert!(block.contains("\"fact_answer_allowed\":false"));
        assert!(block.contains("fb2_context_gap_or_unverified_data_present"));
    }

    #[test]
    fn trims_large_arrays() {
        let context = json!({
            "group_messages": (0..80).map(|index| json!({"id": index, "content": "x".repeat(200)})).collect::<Vec<_>>(),
            "context_pack": "y".repeat(60_000)
        });
        let budgeted = budgeted_context(context);
        assert!(budgeted["_context_budget"]["trimmed"].as_bool().unwrap());
        assert!(budgeted["group_messages"].as_array().unwrap().len() <= 24);
    }

    #[test]
    fn gap_summary_records_budget_truncation_fields() {
        let context = json!({
            "status": "ready",
            "_context_budget": {"trimmed": true},
            "group_messages_truncated": {"original_count": 80, "kept_count": 24},
            "matches_truncated": {"original_count": 60, "kept_count": 24},
            "context_quality": {"warnings": ["fb2_budget_too_large"]},
            "matches": [{"id": "match-1"}],
            "user_orders": [{"order_id": "order-1"}],
            "group_messages": [{"message_id": "message-1"}],
            "metrics": {"budget_status": "too_large"},
            "preflight_readiness": {"status": "degraded"}
        });

        let summary = context_gap_summary(&context);

        assert_eq!(summary["truncation"]["context_budget_trimmed"], true);
        assert!(summary["truncation"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("group_messages")));
        assert!(summary["truncation"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("matches")));
        assert_eq!(summary["business_data_available"]["user_orders"], true);
        assert_eq!(summary["fact_answer_allowed"], true);
        assert_eq!(summary["partial_answer_only"], true);
        assert_eq!(
            summary["required_user_notice"],
            "fb2_context_partial_or_truncated_context_present"
        );
    }
