    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_project_execution_sessions_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn latest_session_tracks_workspace_finish() {
        let store = temp_store();
        let user = store
            .create_user("project-execution@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "执行会话项目", None, Some("android"))
            .expect("project should be created")
            .project;
        store
            .record_project_execution_started(ProjectExecutionSessionStart {
                project_id: &project.id,
                conversation_id: "conv-a",
                user_id: &user.id,
                node_id: "node-a",
                request_id: "req-a",
                requested_workspace_path: Some("D:/repo"),
                model: Some("codex"),
            })
            .expect("start should record");
        store
            .record_project_execution_finished(ProjectExecutionSessionFinish {
                request_id: "req-a",
                base_workspace_path: Some("D:/repo"),
                active_workspace_path: Some("D:/wt"),
                branch: Some("ai/session/prj-a/conv-a"),
                isolated: true,
                status: "done",
                merge_status: Some("merged"),
                last_error: None,
                model: Some("gpt-5"),
                prompt_tokens: Some(100),
                cached_input_tokens: Some(20),
                completion_tokens: Some(30),
                reasoning_tokens: Some(5),
                total_tokens: Some(130),
                token_usage_event_id: Some("tok-a"),
                billing_event_id: Some("bev-a"),
            })
            .expect("finish should update");

        let latest = store
            .latest_project_execution_session(&project.id)
            .expect("latest should query")
            .expect("latest should exist");
        assert_eq!(latest.status, "done");
        assert_eq!(latest.active_workspace_path.as_deref(), Some("D:/wt"));
        assert!(latest.isolated);
        assert_eq!(latest.model.as_deref(), Some("gpt-5"));
        assert_eq!(latest.prompt_tokens, 100);
        assert_eq!(latest.cached_input_tokens, 20);
        assert_eq!(latest.completion_tokens, 30);
        assert_eq!(latest.reasoning_tokens, 5);
        assert_eq!(latest.total_tokens, 130);
        assert_eq!(latest.token_usage_event_id.as_deref(), Some("tok-a"));
        assert_eq!(latest.billing_event_id.as_deref(), Some("bev-a"));

        let by_request = store
            .get_project_execution_session_by_request_id("req-a")
            .expect("request lookup should query")
            .expect("request lookup should find session");
        assert_eq!(by_request.id, latest.id);
        assert_eq!(
            by_request.branch.as_deref(),
            Some("ai/session/prj-a/conv-a")
        );
    }

    #[test]
    fn startup_interrupts_running_project_execution_sessions() {
        let store = temp_store();
        let user = store
            .create_user(
                "project-execution-restart@example.com",
                "secret1",
                None,
                None,
            )
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "执行会话重启项目", None, Some("android"))
            .expect("project should be created")
            .project;
        store
            .record_project_execution_started(ProjectExecutionSessionStart {
                project_id: &project.id,
                conversation_id: "conv-restart",
                user_id: &user.id,
                node_id: "node-a",
                request_id: "req-restart",
                requested_workspace_path: Some("D:/repo"),
                model: Some("codex"),
            })
            .expect("start should record");

        assert_eq!(
            store
                .mark_interrupted_running_project_execution_sessions()
                .expect("running sessions should be interrupted"),
            1
        );
        let session = store
            .get_project_execution_session_by_request_id("req-restart")
            .expect("request lookup should query")
            .expect("request lookup should find session");
        assert_eq!(session.status, "failed");
        assert_eq!(session.merge_status.as_deref(), Some("interrupted"));
        assert_eq!(
            session.last_error.as_deref(),
            Some("server restarted before PC CLI terminal event")
        );
    }
