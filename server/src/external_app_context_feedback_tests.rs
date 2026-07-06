    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn payload_mentions_sources_by_id_or_label() {
        let context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "context_audit_id": "audit-1",
            "citation_sources": [
                {"kind": "match", "id": "match-1234", "label": "中央海岸 vs 奥克兰FC"},
                {"kind": "order", "id": "order-5678", "label": "用户票据"}
            ]
        });

        let payload = generated_answer_feedback_payload(
            &context,
            Some("fb2-user-1"),
            "ext_fb2_official",
            "social_group_message:m1",
            "group_mention",
            "这场中央海岸 vs 奥克兰FC 风险偏高，可参考 match-1234。",
            None,
            &[],
        )
        .expect("payload");

        assert_eq!(payload["context_audit_id"], "audit-1");
        assert_eq!(payload["group_id"], "official");
        assert_eq!(payload["external_user_id"], "fb2-user-1");
        assert_eq!(payload["cited_source_count"], 1);
        assert_eq!(payload["cited_sources"][0]["id"], "match-1234");
    }

    #[test]
    fn payload_uses_extra_selected_message_for_answer_validation_without_feedback_citation_pollution(
    ) {
        let context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "context_audit_id": "audit-1",
            "citation_sources": [
                {"kind": "match", "id": "match-1234", "label": "中央海岸 vs 奥克兰FC"}
            ]
        });

        let payload = generated_answer_feedback_payload(
            &context,
            Some("fb2-user-1"),
            "ext_fb2_official",
            "social_group_selected_message:m1",
            "selected_message_ai_reply",
            "只引用了原消息 gmsg-selected-1，没有引用比赛来源。",
            None,
            &[json!({
                "kind": "selected_message",
                "id": "gmsg-selected-1",
                "label": "被长按的群聊消息"
            })],
        )
        .expect("payload");

        assert_eq!(payload["cited_source_count"], 0);
        assert_eq!(payload["wrong_context"], false);
        assert_eq!(payload["answer_source_validation"]["status"], "ok");
        assert_eq!(
            payload["answer_source_validation"]["matched_source_ids"][0],
            "gmsg-selected-1"
        );
    }

    #[test]
    fn payload_reports_tool_sources_only_when_present_in_context_audit() {
        let context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "context_audit_id": "audit-1",
            "citation_sources": [
                {"kind": "match", "id": "match-tool-1", "label": "工具命中的比赛"}
            ]
        });
        let tool_results = json!({
            "results": [
                {
                    "tool_name": "match_analysis_brief",
                    "success": true,
                    "status": "ready",
                    "grounding": {"status": "grounded"},
                    "source_ids": ["match-tool-1", "order-tool-1"]
                }
            ]
        });

        let payload = generated_answer_feedback_payload(
            &context,
            Some("fb2-user-1"),
            "ext_fb2_official",
            "social_group_message:m1",
            "group_mention",
            "本次分析引用 match-tool-1，并补充查看 order-tool-1 的当前用户票据。",
            Some(&tool_results),
            &[],
        )
        .expect("payload");

        assert_eq!(payload["cited_source_count"], 1);
        assert_eq!(payload["cited_sources"][0]["kind"], "match");
        assert_eq!(payload["cited_sources"][0]["id"], "match-tool-1");
    }

    #[test]
    fn payload_ignores_unsafe_or_unmentioned_tool_sources() {
        let context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "context_audit_id": "audit-1",
            "citation_sources": []
        });
        let tool_results = json!({
            "results": [
                {
                    "tool_name": "search_user_orders",
                    "success": true,
                    "status": "ready",
                    "grounding": {"status": "unsafe"},
                    "source_ids": ["order-unsafe-1"]
                },
                {
                    "tool_name": "search_matches",
                    "success": true,
                    "status": "ready",
                    "grounding": {"status": "grounded"},
                    "source_ids": ["match-unmentioned-1"]
                }
            ]
        });

        let payload = generated_answer_feedback_payload(
            &context,
            Some("fb2-user-1"),
            "ext_fb2_official",
            "social_group_message:m1",
            "group_mention",
            "这里只做概括，不引用具体工具来源。",
            Some(&tool_results),
            &[],
        )
        .expect("payload");

        assert_eq!(payload["cited_source_count"], 0);
        assert_eq!(payload["missing_context"], true);
        assert_eq!(
            payload["answer_source_validation"]["status"],
            "no_explicit_source_ids"
        );
        assert_eq!(
            payload["answer_source_validation"]["has_missing_explicit_sources"],
            true
        );
    }

    #[test]
    fn payload_marks_wrong_context_when_reply_mentions_unmatched_source_id() {
        let context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "context_audit_id": "audit-1",
            "citation_sources": [
                {"kind": "match", "id": "match-1234", "label": "工具命中的比赛"}
            ]
        });

        let payload = generated_answer_feedback_payload(
            &context,
            Some("fb2-user-1"),
            "ext_fb2_official",
            "social_group_message:m1",
            "group_mention",
            "数据事实引用 match-1234，但也写出了不存在的 order-404。",
            None,
            &[],
        )
        .expect("payload");

        assert_eq!(payload["cited_source_count"], 1);
        assert_eq!(payload["missing_context"], false);
        assert_eq!(payload["wrong_context"], true);
        assert_eq!(payload["answer_source_validation"]["status"], "unmatched");
        assert_eq!(
            payload["answer_source_validation"]["unmatched_source_ids"][0],
            "order-404"
        );
        assert!(payload["note"]
            .as_str()
            .unwrap()
            .contains("source_validation=unmatched"));
    }

    #[test]
    fn payload_allows_grounded_tool_source_without_feedback_citation_pollution() {
        let context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "context_audit_id": "audit-1",
            "citation_sources": []
        });
        let tool_results = json!({
            "results": [{
                "tool_name": "match_analysis_brief",
                "success": true,
                "status": "ready",
                "grounding": {"status": "grounded"},
                "source_ids": ["order-tool-1"]
            }]
        });

        let payload = generated_answer_feedback_payload(
            &context,
            Some("fb2-user-1"),
            "ext_fb2_official",
            "social_group_message:m1",
            "group_mention",
            "用户订单：引用 order-tool-1 作为当前用户票据来源。",
            Some(&tool_results),
            &[],
        )
        .expect("payload");

        assert_eq!(payload["cited_source_count"], 0);
        assert_eq!(payload["missing_context"], false);
        assert_eq!(payload["wrong_context"], false);
        assert_eq!(
            payload["answer_source_validation"]["schema"],
            "external_app.answer_source_validation.v1"
        );
        assert_eq!(payload["answer_source_validation"]["status"], "ok");
        assert_eq!(
            payload["answer_source_validation"]["matched_tool_source_ids"][0],
            "order-tool-1"
        );
        assert_eq!(
            payload["answer_source_validation"]["allowed_tool_source_ids"][0],
            "order-tool-1"
        );
        assert_eq!(payload["answer_source_validation"]["cited_source_count"], 0);
        assert!(payload["note"]
            .as_str()
            .unwrap()
            .contains("source_validation=ok"));
    }

    #[test]
    fn payload_allows_extra_validation_source_without_feedback_citation_pollution() {
        let context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "context_audit_id": "audit-1",
            "citation_sources": []
        });
        let extra_sources = vec![json!({
            "kind": "group_message",
            "id": "gmsg_summary_1",
            "message_id": "gmsg_summary_1"
        })];

        let payload = generated_answer_feedback_payload(
            &context,
            Some("fb2-user-1"),
            "ext_fb2_official",
            "social_group_summary_post:gsp-1",
            "group_summary_post",
            "相关发言引用 message_id gmsg_summary_1，只作为总结帖本地消息来源。",
            None,
            &extra_sources,
        )
        .expect("payload");

        assert_eq!(payload["cited_source_count"], 0);
        assert_eq!(payload["missing_context"], false);
        assert_eq!(payload["wrong_context"], false);
        assert_eq!(payload["answer_source_validation"]["status"], "ok");
        assert_eq!(
            payload["answer_source_validation"]["matched_source_ids"][0],
            "gmsg_summary_1"
        );
    }

    #[test]
    fn payload_requires_ready_fb2_context_pack_audit() {
        let context = json!({
            "app_id": "fb2",
            "status": "ready",
            "source": "fb2:/api/main-project/context/today-matches"
        });

        assert!(!is_fb2_context_pack(&context));
        assert!(generated_answer_feedback_payload(
            &context,
            None,
            "group",
            "request",
            "trigger",
            "reply",
            None,
            &[]
        )
        .is_none());
    }

    #[test]
    fn feedback_scope_detects_platform_order_context_only() {
        assert!(feedback_context_needs_platform_order_scope(&json!({
            "platform_order_summary": {"visibility": "privileged_summary"}
        })));
        assert!(feedback_context_needs_platform_order_scope(&json!({
            "citation_sources": [
                {"kind": "platform_order_summary", "id": "platform_order_summary:2026-06-22"}
            ]
        })));
        assert!(feedback_context_needs_platform_order_scope(&json!({
            "context_pack": "<fb2_context_pack><platform_order_summary>匿名汇总</platform_order_summary></fb2_context_pack>"
        })));
        assert!(!feedback_context_needs_platform_order_scope(&json!({
            "platform_order_summary": null,
            "citation_sources": [
                {"kind": "order", "id": "order-1"},
                {"kind": "match", "id": "match-1"}
            ]
        })));
    }

    #[test]
    fn opinion_memory_ids_require_grounded_tool_and_reply_reference() {
        let tool_results = json!({
            "results": [
                {
                    "tool_name": "opinion_memories",
                    "success": true,
                    "grounding": {"status": "grounded"},
                    "source_ids": ["opinion-memory-1"],
                    "data": {
                        "memories": [
                            {"id": "opinion-memory-2", "source_message_id": "group-msg-9999"},
                            {"id": "unmentioned-memory", "source_message_id": "group-msg-0000"}
                        ]
                    }
                },
                {
                    "tool_name": "opinion_memories",
                    "success": true,
                    "grounding": {"status": "ungrounded"},
                    "source_ids": ["unsafe-memory"]
                }
            ]
        });

        let ids = mentioned_opinion_memory_ids(
            &json!({"cited_sources": []}),
            Some(&tool_results),
            "AI 采纳了 opinion-memory-1，也参考了 group-msg-9999 的历史观点。",
        );

        assert_eq!(ids, vec!["opinion-memory-1", "opinion-memory-2"]);
    }

    #[test]
    fn opinion_memory_ids_ignore_unmentioned_sources() {
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["opinion-memory-1"],
                "data": {"memories": [{"id": "opinion-memory-2", "source_message_id": "group-msg-9999"}]}
            }]
        });

        let ids = mentioned_opinion_memory_ids(
            &json!({"cited_sources": []}),
            Some(&tool_results),
            "这里只总结群观点，不引用具体来源。",
        );

        assert!(ids.is_empty());
    }

    #[test]
    fn opinion_memory_ids_include_context_citation_sources() {
        let feedback_payload = json!({
            "cited_sources": [
                {
                    "kind": "opinion_memory",
                    "id": "opinion-memory-context-1",
                    "label": "群友A赛前观点",
                    "source_message_id": "group-msg-context-1"
                },
                {
                    "kind": "match",
                    "id": "match-1",
                    "label": "比赛事实"
                }
            ]
        });

        let ids = mentioned_opinion_memory_ids(
            &feedback_payload,
            None,
            "本次回答采纳了 group-msg-context-1 的群友观点，并结合 match-1。",
        );

        assert_eq!(ids, vec!["opinion-memory-context-1"]);
    }

    #[test]
    fn opinion_memory_ids_ignore_unmentioned_context_sources() {
        let feedback_payload = json!({
            "cited_sources": [{
                "kind": "group_opinion_memory",
                "id": "opinion-memory-context-1",
                "label": "群友A赛前观点"
            }]
        });

        let ids = mentioned_opinion_memory_ids(&feedback_payload, None, "这里只做普通回答。");

        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn feedback_request_retries_with_fresh_client_after_transport_error() {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");

        let server = tokio::spawn(async move {
            let (mut first_stream, _) = listener.accept().await.expect("first request");
            let mut first_buffer = [0_u8; 1024];
            let _ = first_stream.read(&mut first_buffer).await;
            drop(first_stream);

            let (mut second_stream, _) = listener.accept().await.expect("retry request");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = second_stream.read(&mut chunk).await.expect("read retry");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let request_text = String::from_utf8_lossy(&request);
            let lower_request = request_text.to_ascii_lowercase();
            assert!(request_text.starts_with("POST /feedback HTTP/1.1"));
            assert!(lower_request.contains("x-fb2-ai-center-token: test-token"));
            assert!(lower_request.contains("x-fb2-ai-context-user-id: fb2-user-1"));
            assert!(lower_request.contains("x-fb2-ai-context-scope: platform_order_summary"));

            let body = r#"{"success":true}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            second_stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let client = reqwest::Client::builder().build().expect("client");
        let response = send_feedback_request_with_client(
            &client,
            &format!("http://{addr}/feedback"),
            "test-token",
            Some("fb2-user-1"),
            true,
            &json!({"status": "ready"}),
        )
        .await
        .expect("retried response");

        assert!(response.status().is_success());
        assert_eq!(response.text().await.expect("body"), r#"{"success":true}"#);
        server.await.expect("server task");
    }
