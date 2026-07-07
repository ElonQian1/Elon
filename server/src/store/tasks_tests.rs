    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_store_test_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    fn temp_task(store: &Store) -> String {
        let user = store
            .create_user("events@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Task Events", None, None)
            .expect("project should be created")
            .project;
        store
            .create_task(&project.id, &user.id, Some("conv"), "run task")
            .expect("task should be created")
    }

    fn event_message(raw: &str) -> String {
        serde_json::from_str::<serde_json::Value>(raw)
            .expect("event should be json")
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn task_can_store_separate_display_message() {
        let store = temp_store();
        let user = store
            .create_user("display-message@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Display Message", None, None)
            .expect("project should be created")
            .project;
        let agent_message = "please inspect this\n\nUser uploaded real chat attachments for this project conversation (conversation_id=conv):\n- image.png [image; image/png; 1 bytes] -> /tmp/image.png\nThese attachments are part of the current message context, like images/files in a normal chat app. If the user asks about an uploaded image, inspect the exact local path listed above before answering.";
        let display_message = "please inspect this";

        let task_id = store
            .create_task_with_client_request_and_display_message(
                &project.id,
                &user.id,
                Some("conv"),
                Some("req-display"),
                agent_message,
                display_message,
            )
            .expect("task should be created");

        let task = store
            .get_task_by_client_request(&project.id, &user.id, Some("conv"), "req-display")
            .expect("task query should work")
            .expect("task should exist");
        assert_eq!(task.message, agent_message);

        let messages = store
            .list_user_conversation_messages(&project.id, &user.id, "conv", 20)
            .expect("messages should list");
        let user_message = messages
            .iter()
            .find(|message| message.task_id.as_deref() == Some(task_id.as_str()))
            .expect("display message should exist");
        assert_eq!(user_message.content, display_message);
    }

    #[test]
    fn recalled_user_conversation_message_hides_content_and_context() {
        let store = temp_store();
        let user = store
            .create_user("recall-message@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Recall Message", None, None)
            .expect("project should be created")
            .project;

        let task_id = store
            .create_task_with_client_request_and_display_message(
                &project.id,
                &user.id,
                Some("conv"),
                Some("req-recall"),
                "secret prompt for ai context",
                "secret visible message",
            )
            .expect("task should be created");

        let message_id = store
            .list_user_conversation_messages(&project.id, &user.id, "conv", 20)
            .expect("messages should list")
            .into_iter()
            .find(|message| message.task_id.as_deref() == Some(task_id.as_str()))
            .expect("user message should exist")
            .id;

        store
            .recall_user_conversation_message(&project.id, &user.id, "conv", &message_id)
            .expect("message should recall");

        let recalled = store
            .list_user_conversation_messages(&project.id, &user.id, "conv", 20)
            .expect("messages should list")
            .into_iter()
            .find(|message| message.id == message_id)
            .expect("recalled message should still list");
        assert_eq!(recalled.content, "");
        assert_eq!(recalled.recalled_by.as_deref(), Some(user.id.as_str()));
        assert!(recalled.recalled_at.is_some());

        let recent = store
            .list_recent_conversation_messages(&project.id, Some("conv"), 20)
            .expect("recent messages should list");
        assert!(recent
            .iter()
            .all(|message| message.content != "secret prompt for ai context"));
    }

    #[test]
    fn lists_latest_task_events_in_chronological_order() {
        let store = temp_store();
        let task_id = temp_task(&store);

        for step in 0..5 {
            store
                .record_task_event(
                    &task_id,
                    &format!(r#"{{"type":"progress","message":"step {step}"}}"#),
                )
                .expect("event should be recorded");
        }

        let messages = store
            .list_task_events(&task_id, 3)
            .expect("events should list")
            .into_iter()
            .map(|raw| event_message(&raw))
            .collect::<Vec<_>>();

        assert_eq!(messages, vec!["step 2", "step 3", "step 4"]);
    }

    #[test]
    fn prunes_old_task_events_per_task() {
        let store = temp_store();
        let task_id = temp_task(&store);

        for step in 0..(MAX_TASK_EVENTS_PER_TASK + 5) {
            store
                .record_task_event(
                    &task_id,
                    &format!(r#"{{"type":"progress","message":"step {step}"}}"#),
                )
                .expect("event should be recorded");
        }

        let events = store
            .list_task_events(&task_id, MAX_TASK_EVENTS_PER_TASK as usize + 100)
            .expect("events should list");

        assert_eq!(events.len(), MAX_TASK_EVENTS_PER_TASK as usize);
        assert_eq!(event_message(events.first().unwrap()), "step 5");
        assert_eq!(event_message(events.last().unwrap()), "step 1004");
    }

    #[test]
    fn lists_task_events_after_stable_rowid_cursor() {
        let store = temp_store();
        let task_id = temp_task(&store);

        for step in 0..4 {
            store
                .record_task_event(
                    &task_id,
                    &format!(r#"{{"type":"progress","message":"step {step}"}}"#),
                )
                .expect("event should be recorded");
        }

        let first_page = store
            .list_task_events_after(&task_id, 0, 2)
            .expect("events should list");
        assert_eq!(first_page.len(), 2);
        assert_eq!(event_message(&first_page[0].event_json), "step 0");
        assert_eq!(event_message(&first_page[1].event_json), "step 1");

        let second_page = store
            .list_task_events_after(&task_id, first_page[1].seq, 10)
            .expect("events after cursor should list");
        let messages = second_page
            .iter()
            .map(|event| event_message(&event.event_json))
            .collect::<Vec<_>>();
        assert_eq!(messages, vec!["step 2", "step 3"]);
        assert_eq!(
            store
                .latest_task_event_seq(&task_id)
                .expect("latest seq should load"),
            second_page.last().expect("event should exist").seq
        );
    }

    #[test]
    fn channel_task_snapshot_requires_channel_task_link() {
        let store = temp_store();
        let user = store
            .create_user("channel-task-snapshot@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Channel Task Snapshot", None, None)
            .expect("project should be created")
            .project;
        let channels = store
            .list_project_space_channels(&user.id, &project.id)
            .expect("channels should list");
        let ai_channel = channels
            .iter()
            .find(|channel| channel.kind == "ai_development")
            .expect("ai channel should exist");
        let discussion_channel = channels
            .iter()
            .find(|channel| channel.kind == "discussion")
            .expect("discussion channel should exist");
        let task_id = store
            .create_task(&project.id, &user.id, Some("channel-dev"), "继续修复")
            .expect("task should create");

        assert!(store
            .get_channel_task_snapshot(&project.id, &ai_channel.id, &task_id)
            .expect("snapshot query should work")
            .is_none());
        store
            .insert_project_channel_message(
                &project.id,
                &ai_channel.id,
                Some(&user.id),
                "ai_task",
                "发起 AI 开发任务：继续修复",
                Some(&task_id),
                None,
            )
            .expect("task message should insert");

        let snapshot = store
            .get_channel_task_snapshot(&project.id, &ai_channel.id, &task_id)
            .expect("snapshot should query")
            .expect("linked task should be visible");
        assert_eq!(snapshot.id, task_id);
        assert_eq!(snapshot.status, "running");
        assert!(store
            .get_channel_task_snapshot(&project.id, &discussion_channel.id, &snapshot.id)
            .expect("snapshot query should work")
            .is_none());
    }
