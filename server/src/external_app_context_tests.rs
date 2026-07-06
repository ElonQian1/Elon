    use super::*;

    #[test]
    fn infers_fb2_lottery_type_from_topic_hint() {
        assert_eq!(
            infer_lottery_type(Some("今天竞彩怎么看")),
            Some("JingCai".into())
        );
        assert_eq!(infer_lottery_type(Some("北单赛事")), Some("BeiDan".into()));
        assert_eq!(infer_lottery_type(Some("足球比赛")), None);
    }

    #[test]
    fn context_log_helpers_extract_observability_fields() {
        let context = json!({
            "source": "fb2:/api/main-project/context/today-matches",
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "context_quality": {
                "warnings": ["missing_context_pack", "missing_tool_contract"],
                "tool_readiness": {"status": "partial"}
            }
        });

        assert!(context_fallback_used(&context));
        assert_eq!(context_quality_warning_count(&context), 2);
        assert_eq!(context_tool_readiness_status(&context), "partial");
        assert_eq!(
            context_answer_policy_schema(&context),
            "fb2.answer_policy.v1"
        );
    }

    #[test]
    fn readiness_annotation_updates_context_quality() {
        let mut context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "generated_at": "2026-06-22T11:30:00+08:00",
            "context_pack_version": "fb2-chat-pack-v1",
            "context_pack": "<fb2_context_pack>ok</fb2_context_pack>",
            "matches": [{"id": "m1"}],
            "tool_contract": {"tools": [{"name": "get_match_detail"}]},
            "metrics": {}
        });
        let readiness = json!({
            "schema": "external_app.live_context_readiness.v1",
            "status": "blocked",
            "warnings": ["fb2_readiness_blocked"]
        });

        annotate_context_with_readiness(&mut context, &readiness);

        assert_eq!(context["preflight_readiness"]["status"], "blocked");
        assert!(context["context_quality"]["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("fb2_readiness_blocked")));
    }
