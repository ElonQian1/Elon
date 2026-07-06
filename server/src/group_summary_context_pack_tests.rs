    use super::*;

    fn source_message(id: &str, sender_name: &str) -> GroupSummarySourceMessage {
        GroupSummarySourceMessage {
            id: id.to_string(),
            group_id: "official".to_string(),
            sender_user_id: format!("user-{sender_name}"),
            sender_name: sender_name.to_string(),
            content: "今天比赛怎么看".to_string(),
            created_at: "2026-06-22T09:20:00Z".to_string(),
        }
    }

    #[test]
    fn summary_feedback_validation_sources_include_local_group_message_ids() {
        let citations = group_summary_feedback_validation_sources(
            "gsp-summary-1",
            &[
                source_message("gmsg-1", "用户A"),
                source_message("gmsg-2", "用户B"),
            ],
        );

        let ids = citations
            .iter()
            .filter_map(|source| source.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(ids.contains(&"gsp-summary-1"));
        assert!(ids.contains(&"gmsg-1"));
        assert!(ids.contains(&"gmsg-2"));
    }

    #[test]
    fn fb2_summary_shape_sanitizes_unmatched_source_ids_and_appends_audit_source() {
        let context = serde_json::json!({
            "group_id": "ext_fb2_official",
            "external_app_context": {
                "app_id": "fb2",
                "context_audit_id": "ebb6bc08-47f4-491d-b8ec-40e6f48ef078",
                "citation_sources": [
                    {"kind": "match", "id": "EXT-2589467", "label": "西班牙 vs 意大利"},
                    {"kind": "context_audit", "id": "ebb6bc08-47f4-491d-b8ec-40e6f48ef078", "label": "上下文审计"}
                ]
            }
        })
        .to_string();

        let shaped = ensure_fb2_summary_policy_shape(
            "## 数据事实\n- 引用 EXT-2589467 和 ebb6bc08-47f4-491d-b8ec-40e6f48ef078，也误写了 EXT-2589477 与 d4a61f25-e6ec-4192-8c27-471662e15a63。\n\n## 群友观点\n- 有讨论。\n\n## AI推断\n- 只做推断。\n\n## 风险边界\n- 不保证命中。",
            &context,
            None,
        );

        assert!(shaped.contains("EXT-2589467"));
        assert!(shaped.contains("ebb6bc08-47f4-491d-b8ec-40e6f48ef078"));
        assert!(!shaped.contains("EXT-2589477"));
        assert!(!shaped.contains("d4a61f25-e6ec-4192-8c27-471662e15a63"));
        assert!(shaped.contains("未核验来源编号"));
    }

    #[test]
    fn compact_summary_context_skips_small_context() {
        let context = serde_json::json!({
            "group_id": "ext_fb2_official",
            "selected_messages": [{
                "id": "gmsg-1",
                "sender_name": "用户A",
                "content": "今天比赛怎么看"
            }]
        })
        .to_string();

        assert!(compact_group_summary_context_pack(&context).is_none());
    }

    #[test]
    fn compact_summary_context_preserves_source_ids_and_truncates_large_payload() {
        let long_text = "今天比赛怎么看，赔率有什么变化。".repeat(1200);
        let context = serde_json::json!({
            "group_id": "ext_fb2_official",
            "task": "summary_post",
            "source_message_count": 40,
            "selected_messages": [{
                "id": "gmsg-source-1",
                "sender_user_id": "usr-1",
                "sender_name": "用户A",
                "created_at": "2026-06-22T11:42:00Z",
                "content": long_text
            }],
            "external_app_context": {
                "app_id": "fb2",
                "context_audit_id": "audit-1",
                "payload": "赔率与订单摘要".repeat(2200)
            },
            "output_contract": {
                "required_sections": ["数据事实", "AI推断", "风险边界"]
            }
        })
        .to_string();

        let compact = compact_group_summary_context_pack(&context).expect("compact context");

        assert!(compact.len() < context.len());
        assert!(compact.contains("group_summary.compact_context_pack.v1"));
        assert!(compact.contains("gmsg-source-1"));
        assert!(compact.contains("audit-1"));
        assert!(compact.contains("message_id"));
        assert!(compact.contains("风险边界"));
        assert!(compact.contains("truncated"));
    }
