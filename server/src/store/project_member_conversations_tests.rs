    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_member_conversation_test_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn member_conversation_messages_are_scoped_to_project_member() {
        let store = temp_store();
        let owner = store
            .create_user("owner@example.com", "secret1", Some("Owner"), None)
            .expect("owner should be created");
        let member = store
            .create_user("member@example.com", "secret1", Some("Member"), None)
            .expect("member should be created");
        let outsider = store
            .create_user("outsider@example.com", "secret1", Some("Outsider"), None)
            .expect("outsider should be created");
        let project = store
            .create_project(&owner.id, "Member Conversation Scope", None, None)
            .expect("project should be created")
            .project;
        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should be public");
        store
            .join_project(&member.id, &project.id)
            .expect("member should join project");

        store
            .create_task(&project.id, &owner.id, Some("default"), "owner request")
            .expect("owner task should be created");
        let member_task = store
            .create_task(&project.id, &member.id, Some("default"), "member request")
            .expect("member task should be created");
        store
            .finish_task(&member_task, "done", Some("member reply"), None, None)
            .expect("member task should finish");

        let conversations = store
            .list_project_member_conversations(&owner.id, &project.id, &member.id, 10)
            .expect("owner can inspect member project conversations");
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].message_count, 2);
        assert_eq!(
            conversations[0].last_message.as_deref(),
            Some("member reply")
        );

        let messages = store
            .list_project_member_conversation_messages(
                &owner.id,
                &project.id,
                &member.id,
                "default",
                10,
            )
            .expect("owner can inspect member project conversation messages");
        let contents = messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["member request", "member reply"]);
        let self_messages = store
            .list_project_member_conversation_messages(
                &member.id,
                &project.id,
                &member.id,
                "default",
                10,
            )
            .expect("member can inspect own project conversation messages");
        assert_eq!(self_messages.len(), 2);
        assert_eq!(self_messages[0].role, "user");
        assert!(self_messages[0].outgoing);
        assert_eq!(self_messages[1].role, "assistant");
        assert!(!self_messages[1].outgoing);

        assert!(store
            .list_project_member_conversations(&outsider.id, &project.id, &member.id, 10)
            .is_err());
    }

    #[test]
    fn project_member_can_discuss_another_members_conversation_without_creating_task() {
        let store = temp_store();
        let owner = store
            .create_user("owner2@example.com", "secret1", Some("Owner"), None)
            .expect("owner should be created");
        let member = store
            .create_user("member2@example.com", "secret1", Some("Member"), None)
            .expect("member should be created");
        let outsider = store
            .create_user("outsider2@example.com", "secret1", Some("Outsider"), None)
            .expect("outsider should be created");
        let project = store
            .create_project(&owner.id, "Member Conversation Discussion", None, None)
            .expect("project should be created")
            .project;
        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should be public");
        store
            .join_project(&member.id, &project.id)
            .expect("member should join project");

        store
            .create_task(
                &project.id,
                &owner.id,
                Some("default"),
                "owner default request",
            )
            .expect("owner task should be created");
        let member_task = store
            .create_task(&project.id, &member.id, Some("default"), "member request")
            .expect("member task should be created");
        store
            .finish_task(&member_task, "done", Some("member reply"), None, None)
            .expect("member task should finish");

        let discussion = store
            .insert_project_member_conversation_discussion_message(
                &owner.id,
                &project.id,
                &member.id,
                "default",
                "我来补充一个人工讨论点",
            )
            .expect("project owner can discuss member conversation");
        assert_eq!(discussion.user_id.as_deref(), Some(owner.id.as_str()));
        assert_eq!(discussion.sender_name.as_deref(), Some("Owner"));
        assert_eq!(discussion.role, "discussion");
        assert!(discussion.task_id.is_none());
        assert!(discussion.outgoing);

        let conversations = store
            .list_project_member_conversations(&owner.id, &project.id, &member.id, 10)
            .expect("owner can inspect member project conversations");
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].message_count, 3);
        assert_eq!(conversations[0].task_count, 1);
        assert_eq!(
            conversations[0].last_message.as_deref(),
            Some("我来补充一个人工讨论点")
        );
        assert_eq!(
            conversations[0].last_message_role.as_deref(),
            Some("discussion")
        );

        let messages = store
            .list_project_member_conversation_messages(
                &owner.id,
                &project.id,
                &member.id,
                "default",
                10,
            )
            .expect("owner can inspect the combined timeline");
        let contents = messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            contents,
            vec!["member request", "member reply", "我来补充一个人工讨论点"]
        );
        assert!(messages[0].user_id.as_deref() != Some(owner.id.as_str()));
        assert!(messages[2].outgoing);
        assert_eq!(messages[2].sender_name.as_deref(), Some("Owner"));

        assert!(store
            .insert_project_member_conversation_discussion_message(
                &outsider.id,
                &project.id,
                &member.id,
                "default",
                "outsider comment",
            )
            .is_err());
    }

    #[test]
    fn member_can_hide_own_project_conversation_from_other_members() {
        let store = temp_store();
        let owner = store
            .create_user("owner3@example.com", "secret1", Some("Owner"), None)
            .expect("owner should be created");
        let member = store
            .create_user("member3@example.com", "secret1", Some("Member"), None)
            .expect("member should be created");
        let project = store
            .create_project(&owner.id, "Member Conversation Visibility", None, None)
            .expect("project should be created")
            .project;
        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should be public");
        store
            .join_project(&member.id, &project.id)
            .expect("member should join project");

        let member_task = store
            .create_task(&project.id, &member.id, Some("default"), "member request")
            .expect("member task should be created");
        store
            .finish_task(&member_task, "done", Some("member reply"), None, None)
            .expect("member task should finish");

        let owner_view = store
            .list_project_member_conversations(&owner.id, &project.id, &member.id, 10)
            .expect("owner can inspect public member conversations");
        assert_eq!(owner_view.len(), 1);
        assert!(owner_view[0].is_public);

        let hidden = store
            .update_project_member_conversation_visibility(
                &member.id,
                &project.id,
                "default",
                false,
            )
            .expect("member can hide own conversation");
        assert!(!hidden.is_public);

        let owner_view = store
            .list_project_member_conversations(&owner.id, &project.id, &member.id, 10)
            .expect("owner can list visible member conversations");
        assert!(owner_view.is_empty());
        assert!(store
            .list_project_member_conversation_messages(
                &owner.id,
                &project.id,
                &member.id,
                "default",
                10,
            )
            .is_err());
        assert!(store
            .insert_project_member_conversation_discussion_message(
                &owner.id,
                &project.id,
                &member.id,
                "default",
                "owner comment",
            )
            .is_err());

        let self_view = store
            .list_project_member_conversations(&member.id, &project.id, &member.id, 10)
            .expect("member can still inspect own hidden conversation");
        assert_eq!(self_view.len(), 1);
        assert!(!self_view[0].is_public);
        store
            .list_project_member_conversation_messages(
                &member.id,
                &project.id,
                &member.id,
                "default",
                10,
            )
            .expect("member can still read own hidden conversation");

        let visible = store
            .update_project_member_conversation_visibility(&member.id, &project.id, "default", true)
            .expect("member can reopen own conversation");
        assert!(visible.is_public);
        let owner_view = store
            .list_project_member_conversations(&owner.id, &project.id, &member.id, 10)
            .expect("owner can inspect reopened member conversations");
        assert_eq!(owner_view.len(), 1);
        assert!(owner_view[0].is_public);
    }

    #[test]
    fn member_conversation_list_orders_by_real_message_time_not_background_updated_at() {
        let store = temp_store();
        let owner = store
            .create_user("owner4@example.com", "secret1", Some("Owner"), None)
            .expect("owner should be created");
        let member = store
            .create_user("member4@example.com", "secret1", Some("Member"), None)
            .expect("member should be created");
        let project = store
            .create_project(&owner.id, "Member Conversation Activity Order", None, None)
            .expect("project should be created")
            .project;
        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should be public");
        store
            .join_project(&member.id, &project.id)
            .expect("member should join project");

        store
            .create_task(&project.id, &member.id, Some("may"), "may request")
            .expect("old conversation task should be created");
        store
            .create_task(&project.id, &member.id, Some("june"), "june request")
            .expect("newer conversation task should be created");

        let conn = store.conn().expect("connection should open");
        conn.execute(
            "UPDATE conversations SET created_at = ?1, updated_at = ?2
             WHERE project_id = ?3 AND user_id = ?4 AND id = 'may'",
            params![
                "2026-05-01T00:00:00Z",
                "2026-07-06T00:00:00Z",
                project.id,
                member.id
            ],
        )
        .expect("old conversation timestamps should update");
        conn.execute(
            "UPDATE messages SET created_at = ?1
             WHERE project_id = ?2 AND conversation_id = 'may'",
            params!["2026-05-01T00:00:00Z", project.id],
        )
        .expect("old message timestamp should update");
        conn.execute(
            "UPDATE conversations SET created_at = ?1, updated_at = ?1
             WHERE project_id = ?2 AND user_id = ?3 AND id = 'june'",
            params!["2026-06-01T00:00:00Z", project.id, member.id],
        )
        .expect("newer conversation timestamps should update");
        conn.execute(
            "UPDATE messages SET created_at = ?1
             WHERE project_id = ?2 AND conversation_id = 'june'",
            params!["2026-06-01T00:00:00Z", project.id],
        )
        .expect("newer message timestamp should update");
        drop(conn);

        let conversations = store
            .list_project_member_conversations(&owner.id, &project.id, &member.id, 10)
            .expect("owner can inspect member conversations");

        assert_eq!(
            conversations
                .iter()
                .map(|conversation| conversation.id.as_str())
                .collect::<Vec<_>>(),
            vec!["june", "may"]
        );
        assert_eq!(conversations[1].updated_at.as_str(), "2026-07-06T00:00:00Z");
        assert_eq!(
            conversations[1].last_message_at.as_deref(),
            Some("2026-05-01T00:00:00Z")
        );
    }
