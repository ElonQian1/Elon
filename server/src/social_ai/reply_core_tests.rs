    use super::super::{contains_el_mention, social_ai_fallback_message};
    use super::{
        ensure_fb2_grounded_answer_shape, ensure_fb2_opinion_memory_source,
        format_external_context, latest_request_user_text, social_ai_base_prompt,
    };
    use crate::store::SocialAiHistoryMessage;
    use serde_json::json;

    #[test]
    fn detects_half_and_full_width_mentions() {
        assert!(contains_el_mention("@EL 帮我看看"));
        assert!(contains_el_mention("＠el 这是什么意思"));
        assert!(!contains_el_mention("普通聊天"));
    }

    #[test]
    fn mention_only_uses_previous_user_question() {
        let history = vec![
            SocialAiHistoryMessage {
                speaker: "我".into(),
                content: "这句话是什么意思？".into(),
                from_request_user: true,
            },
            SocialAiHistoryMessage {
                speaker: "我".into(),
                content: "@EL".into(),
                from_request_user: true,
            },
        ];
        assert_eq!(
            latest_request_user_text(&history).as_deref(),
            Some("这句话是什么意思？")
        );
    }

    #[test]
    fn latest_request_user_text_removes_mention_for_topic_hint() {
        let history = vec![SocialAiHistoryMessage {
            speaker: "我".into(),
            content: "@EL 帮我分析今天比赛和我的票".into(),
            from_request_user: true,
        }];
        assert_eq!(
            latest_request_user_text(&history).as_deref(),
            Some("帮我分析今天比赛和我的票")
        );
    }

    #[test]
    fn base_prompt_requires_fb2_source_references() {
        let prompt = social_ai_base_prompt();
        assert!(prompt.contains("fb2 外部上下文"));
        assert!(prompt.contains("来源 ID"));
        assert!(prompt.contains("context_audit_id"));
        assert!(prompt.contains("opinion_memory_id"));
        assert!(prompt.contains("source_message_id"));
        assert!(prompt.contains("数据事实："));
        assert!(prompt.contains("AI推断："));
        assert!(prompt.contains("风险边界："));
        assert!(prompt.contains("不保证命中"));
    }

    #[test]
    fn external_context_prompt_includes_fb2_domain_scenario_guidance() {
        let context = json!({
            "app_id": "fb2",
            "source": "fb2",
            "status": "ready",
            "context_audit_id": "audit-social-1",
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "context_pack": "<fb2_context_pack>比赛和订单摘要</fb2_context_pack>"
        });
        let tools = json!({
            "app_id": "fb2",
            "plan": {
                "topic_hint": "今天比赛怎么看，顺便帮我分析我的票",
                "planned_tools": [
                    {"name": "match_analysis_brief"},
                    {"name": "search_user_orders"}
                ]
            },
            "results": []
        });

        let block = format_external_context(Some(&context), Some(&tools));

        assert!(block.contains("fb2.domain_scenario_prompt.v1"));
        assert!(block.contains("scenario=today_matches_analysis"));
        assert!(block.contains("scenario=my_ticket_analysis"));
        assert!(block.contains("order_id/ticket_id/match_id"));
    }

    #[test]
    fn external_context_prompt_surfaces_quality_readiness_budget_and_tool_gap() {
        let context = json!({
            "app_id": "fb2",
            "source": "fb2:/api/main-project/context/pack",
            "status": "ready",
            "generated_at": "2026-06-22T12:00:00+08:00",
            "context_pack": "<fb2_context_pack>数据缺口样本</fb2_context_pack>",
            "context_pack_version": "fb2-chat-pack-v1",
            "context_audit_id": "audit-gap",
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "metrics": {"budget_status": "empty"},
            "_context_budget": {"trimmed": true},
            "preflight_readiness": {
                "status": "blocked",
                "warnings": ["fb2_readiness_blocked"]
            },
            "context_quality": {
                "warnings": ["fb2_readiness_blocked", "fb2_budget_empty", "empty_matches"],
                "tool_readiness": {"status": "partial"}
            },
            "matches": [],
            "user_orders": [],
            "group_messages": []
        });
        let tool_results = json!({
            "schema": "external_app.executed_tools.v1",
            "app_id": "fb2",
            "status": "skipped",
            "executed_at": "2026-06-22T12:01:00Z",
            "results": [{
                "tool_name": "search_matches",
                "status": "skipped",
                "success": false,
                "error": "fb2_readiness_blocked",
                "reason": "readiness blocked"
            }]
        });

        let block = format_external_context(Some(&context), Some(&tool_results));

        assert!(block.contains("context_quality="));
        assert!(block.contains("context_gap_summary="));
        assert!(block.contains("\"preflight_readiness\""));
        assert!(block.contains("context_budget="));
        assert!(block.contains("\"trimmed\":true"));
        assert!(block.contains("fb2_readiness_blocked"));
        assert!(block.contains("\"fact_answer_allowed\":false"));
        assert!(block.contains("<tool_gap_summary>"));
        assert!(block.contains("这只是数据缺口"));
        assert!(block.contains("不能编造成比赛、赔率、订单或群友观点事实"));
    }

    #[test]
    fn fb2_grounded_answer_shape_adds_required_labels() {
        let context = json!({"answer_policy": {"schema": "fb2.answer_policy.v1"}});
        let reply = ensure_fb2_grounded_answer_shape(
            "今天有比赛，来源：match_id EXT-1，context_audit_id audit-1",
            Some(&context),
        );

        assert!(reply.contains("数据事实："));
        assert!(reply.contains("AI推断："));
        assert!(reply.contains("风险边界："));
        assert!(reply.contains("不保证命中"));
        assert!(reply.contains("match_id EXT-1"));
    }

    #[test]
    fn fb2_grounded_answer_shape_keeps_plain_chat_unchanged() {
        let reply = "普通朋友聊天回复";

        assert_eq!(ensure_fb2_grounded_answer_shape(reply, None), reply);
    }

    #[test]
    fn fb2_opinion_memory_source_is_appended_when_reply_uses_group_opinion() {
        let context = json!({"answer_policy": {"schema": "fb2.answer_policy.v1"}});
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["opinion-memory-1"],
                "data": {
                    "memories": [{
                        "id": "opinion-memory-2",
                        "source_message_id": "gmsg-memory-2"
                    }]
                }
            }]
        });

        let reply = ensure_fb2_opinion_memory_source(
            "群友观点：我倾向采纳这个方向，但仍需看临场。",
            Some(&context),
            Some(&tool_results),
        );

        assert!(reply.contains("opinion_memory_id opinion-memory-2"));
        assert!(reply.contains("source_message_id gmsg-memory-2"));
    }

    #[test]
    fn fb2_opinion_memory_source_keeps_existing_reference() {
        let context = json!({"app_id": "fb2"});
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["opinion-memory-1"]
            }]
        });

        let reply = ensure_fb2_opinion_memory_source(
            "群友观点：参考 opinion-memory-1 后，我不建议重注。",
            Some(&context),
            Some(&tool_results),
        );

        assert_eq!(reply.matches("opinion-memory-1").count(), 1);
    }

    #[test]
    fn fb2_opinion_memory_source_ignores_ungrounded_tool_result() {
        let context = json!({"app_id": "fb2"});
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "weak"},
                "source_ids": ["opinion-memory-1"]
            }]
        });
        let reply = "群友观点：这里只能做轻量参考。";

        assert_eq!(
            ensure_fb2_opinion_memory_source(reply, Some(&context), Some(&tool_results)),
            reply
        );
    }

    #[test]
    fn fb2_generation_fallback_keeps_sources_and_opinion_memory() {
        let context = json!({
            "app_id": "fb2",
            "context_audit_id": "audit-fallback-1",
            "citation_sources": [
                {"kind": "match", "id": "EXT-2589467", "label": "西班牙 vs 意大利"},
                {"kind": "user_order", "id": "order-fallback-1", "label": "我的票"}
            ]
        });
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["memory-fallback-1"],
                "data": {
                    "memories": [{
                        "id": "memory-fallback-1",
                        "source_message_id": "gmsg-fallback-1"
                    }]
                }
            }]
        });

        let reply = social_ai_fallback_message(
            "群聊",
            "provider resource exhausted",
            Some(&context),
            Some(&tool_results),
        );

        assert!(reply.contains("数据事实："));
        assert!(reply.contains("群友观点："));
        assert!(reply.contains("AI推断："));
        assert!(reply.contains("风险边界："));
        assert!(reply.contains("context_audit_id audit-fallback-1"));
        assert!(reply.contains("match_id EXT-2589467"));
        assert!(reply.contains("order_id order-fallback-1"));
        assert!(reply.contains("opinion_memory_id memory-fallback-1"));
        assert!(reply.contains("source_message_id gmsg-fallback-1"));
    }
