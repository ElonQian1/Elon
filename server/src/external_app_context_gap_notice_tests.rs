    use super::ensure_fb2_context_gap_notice;
    use serde_json::json;

    #[test]
    fn appends_gap_notice_for_blocked_fb2_context() {
        let context = json!({
            "app_id": "fb2",
            "source": "fb2:/api/main-project/context/pack",
            "status": "ready",
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "metrics": {"budget_status": "empty"},
            "_context_budget": {"trimmed": true},
            "preflight_readiness": {
                "status": "blocked",
                "warnings": [{"code": "fb2_readiness_blocked"}]
            },
            "context_quality": {
                "warnings": ["fb2_budget_empty", "missing_context_pack"]
            },
            "context_pack": ""
        });

        let reply = ensure_fb2_context_gap_notice(
            "数据事实：暂时只能看到部分摘要。\nAI推断：先保守看。\n风险边界：不保证命中。",
            Some(&context),
        );

        assert!(reply.contains("数据缺口："));
        assert!(reply.contains("readiness 被阻断"));
        assert!(reply.contains("业务上下文为空"));
        assert!(reply.contains("缺少可引用 Context Pack"));
        assert!(reply.contains("不能把缺失数据编造成比赛、赔率、订单或群友观点事实"));
    }

    #[test]
    fn keeps_ready_fb2_context_unchanged() {
        let context = json!({
            "app_id": "fb2",
            "status": "ready",
            "metrics": {"budget_status": "ok"},
            "preflight_readiness": {"status": "ready"},
            "context_quality": {"warnings": []},
            "context_pack": "<fb2_context_pack>match_id M1</fb2_context_pack>"
        });
        let reply = "数据事实：match_id M1。\nAI推断：谨慎。\n风险边界：不保证命中。";

        assert_eq!(ensure_fb2_context_gap_notice(reply, Some(&context)), reply);
    }

    #[test]
    fn does_not_duplicate_existing_gap_notice() {
        let context = json!({
            "app_id": "fb2",
            "preflight_readiness": {"status": "blocked"},
            "context_pack": ""
        });
        let reply = "数据缺口：fb2 当前没有返回订单。";

        assert_eq!(ensure_fb2_context_gap_notice(reply, Some(&context)), reply);
    }

    #[test]
    fn ignores_non_fb2_context() {
        let context = json!({
            "app_id": "other",
            "preflight_readiness": {"status": "blocked"},
            "context_pack": ""
        });
        let reply = "普通回答";

        assert_eq!(ensure_fb2_context_gap_notice(reply, Some(&context)), reply);
    }
