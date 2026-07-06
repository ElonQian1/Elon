    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_conversation_router_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn scope_aliases_route_to_expected_system_entry() {
        assert_eq!(
            ConversationEntryKind::from_scope(Some("chat")),
            ConversationEntryKind::ChatMemory
        );
        assert_eq!(
            ConversationEntryKind::from_scope(Some("normal_chat")),
            ConversationEntryKind::ChatMemory
        );
        assert_eq!(
            ConversationEntryKind::from_scope(Some("phone_control")),
            ConversationEntryKind::PhoneControl
        );
        assert_eq!(
            ConversationEntryKind::from_scope(None),
            ConversationEntryKind::PhoneControl
        );
    }

    #[test]
    fn system_routes_create_distinct_project_archives() {
        let store = temp_store();
        let user = store
            .create_user("conversation-router@example.com", "secret1", None, None)
            .expect("user should be created");

        let phone = resolve_system_conversation_route(
            &store,
            &user.id,
            ConversationEntryKind::PhoneControl,
        )
        .expect("phone route should resolve");
        let chat =
            resolve_system_conversation_route(&store, &user.id, ConversationEntryKind::ChatMemory)
                .expect("chat route should resolve");

        assert_ne!(phone.project_id, chat.project_id);
        assert_eq!(phone.entry_key, "phone_control");
        assert_eq!(chat.entry_key, "chat_memory");
        assert!(phone.project_created);
        assert!(chat.project_created);
        assert_eq!(
            phone.memory_scope_id.as_deref(),
            Some(phone.project_id.as_str())
        );
        assert_eq!(
            chat.memory_scope_id.as_deref(),
            Some(chat.project_id.as_str())
        );
    }

    #[test]
    fn project_route_uses_project_memory_scope() {
        let store = temp_store();
        let user = store
            .create_user("project-route@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "项目会话", None, None)
            .expect("project should create")
            .project;

        let route = resolve_project_conversation_route(&store, &user.id, &project.id)
            .expect("project route should resolve");

        assert_eq!(route.project_id, project.id);
        assert_eq!(route.entry_key, "project");
        assert_eq!(route.memory_scope_type, MEMORY_SCOPE_PROJECT);
        assert_eq!(route.memory_scope_id.as_deref(), Some(project.id.as_str()));
    }
