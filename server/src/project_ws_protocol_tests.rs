    use super::*;

    #[test]
    fn parses_project_attachment_refs() {
        let request = parse_project_message(
            r#"{
                "op":"run",
                "task_id":"tsk_legacy",
                "trace_id":"ui_123",
                "client_request_id":"req_123",
                "message":"please inspect this file",
                "attachments":[
                    {
                        "kind":"image",
                        "attachment_id":"att_123",
                        "display_name":"screenshot.png",
                        "file_name":"screenshot.png",
                        "mime_type":"image/png",
                        "path":"D:/workspace/attachments/c1/screenshot.png",
                        "sha256":"abc123",
                        "size_bytes":128,
                        "image_width":640,
                        "image_height":480,
                        "annotations":[
                            {
                                "x":0.1,
                                "y":0.2,
                                "width":0.3,
                                "height":0.4,
                                "note":"看这里",
                                "icon_x":0.42,
                                "icon_y":0.58,
                                "icon_width":0.06,
                                "icon_height":0.06
                            }
                        ]
                    }
                ]
            }"#,
        );

        let attachment = request
            .attachments
            .as_ref()
            .and_then(|items| items.first())
            .expect("attachment ref should be parsed");
        assert_eq!(request.op.as_deref(), Some("run"));
        assert_eq!(request.task_id.as_deref(), Some("tsk_legacy"));
        assert_eq!(request.trace_id.as_deref(), Some("ui_123"));
        assert_eq!(request.client_request_id.as_deref(), Some("req_123"));
        assert_eq!(request.message, "please inspect this file");
        assert_eq!(attachment.kind.as_deref(), Some("image"));
        assert_eq!(attachment.attachment_id.as_deref(), Some("att_123"));
        assert_eq!(attachment.display_name.as_deref(), Some("screenshot.png"));
        assert_eq!(
            attachment.path.as_deref(),
            Some("D:/workspace/attachments/c1/screenshot.png")
        );
        assert_eq!(attachment.sha256.as_deref(), Some("abc123"));
        assert_eq!(attachment.size_bytes, Some(128));
        assert_eq!(attachment.image_width, Some(640));
        assert_eq!(attachment.image_height, Some(480));
        let annotation = attachment
            .annotations
            .first()
            .expect("annotation should parse");
        assert_eq!(annotation.note, "看这里");
        assert_eq!(annotation.icon_x, Some(0.42));
    }

    #[test]
    fn parses_project_chat_runtime_route_aliases() {
        let request = serde_json::from_str::<ProjectChatRequest>(
            r#"{"message":"run","runtimeRoute":"server-runtime"}"#,
        )
        .expect("request should parse");

        assert_eq!(
            request.pc_runtime_route().expect("route should parse"),
            Some(PcRuntimeRoutePreference::RouteC)
        );
    }

    #[test]
    fn rejects_unknown_project_chat_runtime_route() {
        let request = serde_json::from_str::<ProjectChatRequest>(
            r#"{"message":"run","runtimeRoute":"remote-neighbor"}"#,
        )
        .expect("request should parse");

        assert!(request.pc_runtime_route().is_err());
    }

    #[test]
    fn derives_stable_client_request_id_from_trace() {
        let request = parse_project_message(
            r#"{
                "trace_id":"ui_123_abc",
                "message":"build apk"
            }"#,
        );

        let id =
            project_client_request_id(&request, "project", "user", "conversation", "build apk");

        assert_eq!(id, "ui_123_abc");
    }

    #[test]
    fn derives_fallback_client_request_id_when_trace_missing() {
        let request = parse_project_message(r#"{"message":"build apk"}"#);

        let first =
            project_client_request_id(&request, "project", "user", "conversation", "build apk");
        let second =
            project_client_request_id(&request, "project", "user", "conversation", "build apk");

        assert!(first.starts_with("auto_"));
        assert_eq!(first, second);
    }

    #[test]
    fn enriches_project_ws_event_with_task_id_and_event() {
        let raw = WsMessage::progress("running").to_json();
        let enriched = enrich_project_ws_event(raw, "tsk_123");
        let value: serde_json::Value =
            serde_json::from_str(&enriched).expect("enriched payload should be valid json");
        assert_eq!(value["task_id"], "tsk_123");
        assert_eq!(value["event"], "progress");
        assert!(value["emitted_at_ms"].as_u64().is_some());
    }

    #[test]
    fn server_message_details_uses_text_for_assistant_chunks() {
        let details = server_message_details(
            &serde_json::json!({
                "type": "assistant_chunk",
                "text": "chunk preview"
            }),
            42,
        );

        assert_eq!(details["message_chars"], 13);
        assert_eq!(details["message_preview"], "chunk preview");
    }

    #[test]
    fn terminal_backlog_appends_done_when_replay_window_lacks_terminal() {
        let task = TaskSnapshot {
            id: "tsk_1".into(),
            project_id: "project".into(),
            user_id: "user".into(),
            conversation_id: Some("conversation".into()),
            message: "build apk".into(),
            status: "done".into(),
            apk_url: Some("http://example.test/app.apk".into()),
            error: None,
        };
        let events = (0..PROJECT_WS_BACKLOG_LIMIT)
            .map(|step| WsMessage::progress(format!("step {step}")).to_json())
            .collect::<Vec<_>>();

        let backlog = terminal_backlog_from_task(&task, events);

        assert_eq!(backlog.len(), PROJECT_WS_BACKLOG_LIMIT);
        assert!(is_terminal_project_ws_message(backlog.last().unwrap()));
        assert!(!backlog.iter().any(|raw| raw.contains("step 0")));
    }

    #[test]
    fn terminal_backlog_keeps_existing_terminal_event() {
        let task = TaskSnapshot {
            id: "tsk_1".into(),
            project_id: "project".into(),
            user_id: "user".into(),
            conversation_id: Some("conversation".into()),
            message: "build apk".into(),
            status: "done".into(),
            apk_url: Some("http://example.test/app.apk".into()),
            error: None,
        };
        let done = WsMessage::Done {
            message: "finished".into(),
            apk_url: task.apk_url.clone(),
            image_url: None,
            model_used: None,
            node_id: None,
        }
        .to_json();

        let backlog = terminal_backlog_from_task(&task, vec![done.clone()]);

        assert_eq!(backlog, vec![done]);
    }
