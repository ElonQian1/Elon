    use super::*;
    use serde_json::json;

    #[test]
    fn flags_unmatched_source_like_ids() {
        let context = json!({
            "context_audit_id": "audit-1",
            "citation_sources": [
                {"kind": "match", "id": "match-1", "label": "比赛1"}
            ]
        });
        let validation = validate_reply_sources(
            &context,
            None,
            "数据事实引用 match-1，但也写了不存在的 order-404。",
            &[],
            &[],
        );

        assert!(validation.has_unmatched_sources());
        assert_eq!(validation.unmatched_source_ids, vec!["order-404"]);
        assert_eq!(validation.matched_source_ids, vec!["match-1"]);
    }

    #[test]
    fn accepts_grounded_tool_source_ids_without_context_registry_match() {
        let context = json!({"context_audit_id": "audit-1", "citation_sources": []});
        let tool_results = json!({
            "results": [{
                "tool_name": "match_analysis_brief",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["order-tool-1"]
            }]
        });
        let validation = validate_reply_sources(
            &context,
            Some(&tool_results),
            "用户订单：引用 order-tool-1 作为当前用户票据来源。",
            &[],
            &[],
        );

        assert!(!validation.has_unmatched_sources());
        assert_eq!(validation.matched_source_ids, vec!["order-tool-1"]);
        assert_eq!(validation.matched_tool_source_ids, vec!["order-tool-1"]);
        assert_eq!(validation.allowed_tool_source_ids, vec!["order-tool-1"]);
        let summary = validation.answer_source_validation_summary("main-req-1", "audit-1", 0);
        assert_eq!(
            summary["schema"],
            "external_app.answer_source_validation.v1"
        );
        assert_eq!(summary["status"], "ok");
        assert_eq!(summary["main_request_id"], "main-req-1");
        assert_eq!(summary["context_audit_id"], "audit-1");
        assert_eq!(summary["matched_tool_source_ids"][0], "order-tool-1");
        assert_eq!(summary["cited_source_count"], 0);
    }

    #[test]
    fn ignores_dates_and_plain_field_names() {
        let context = json!({"context_audit_id": "audit-1"});
        let validation = validate_reply_sources(
            &context,
            None,
            "2026-06-23 的 match_id、source_message_id 与 opinion_memory_id 字段缺失，所以这里只能说明数据不足。",
            &[],
            &[],
        );

        assert!(!validation.has_unmatched_sources());
        assert!(validation.has_missing_explicit_sources());
        assert!(validation.candidate_source_ids.is_empty());
        let summary = validation.answer_source_validation_summary("main-req-1", "audit-1", 0);
        assert_eq!(summary["status"], "no_explicit_source_ids");
        assert_eq!(summary["has_missing_explicit_sources"], true);
        assert_eq!(summary["has_unmatched_sources"], false);
    }

    #[test]
    fn flags_fabricated_group_message_source_ids() {
        let context = json!({
            "context_audit_id": "audit-1",
            "citation_sources": [
                {"kind": "group_message", "id": "msg-1001"}
            ]
        });
        let validation = validate_reply_sources(
            &context,
            None,
            "群友观点：参考 msg-1001；但也写出了不存在的 message-404 和 source-message-500。",
            &[],
            &[],
        );

        assert!(validation.has_unmatched_sources());
        assert_eq!(validation.matched_source_ids, vec!["msg-1001"]);
        assert_eq!(
            validation.unmatched_source_ids,
            vec!["message-404", "source-message-500"]
        );
    }

    #[test]
    fn accepts_context_registered_group_message_aliases() {
        let context = json!({
            "context_audit_id": "audit-1",
            "citation_sources": [
                {
                    "kind": "group_message",
                    "id": "message-1001",
                    "message_id": "msg-1001",
                    "source_message_id": "source-message-1001"
                }
            ]
        });
        let validation = validate_reply_sources(
            &context,
            None,
            "群友观点：引用 msg-1001 和 source-message-1001 后，只能作为讨论线索。",
            &[],
            &[],
        );

        assert!(!validation.has_unmatched_sources());
        assert!(validation
            .matched_source_ids
            .contains(&"msg-1001".to_string()));
        assert!(validation
            .matched_source_ids
            .contains(&"source-message-1001".to_string()));
    }
