    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_user_presence_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn user_presence_settings_can_be_saved() {
        let store = temp_store();
        let user = store
            .create_user("presence@example.com", "secret1", None, None)
            .expect("user should be created");

        let default_presence = store
            .user_presence_settings(&user.id)
            .expect("default presence should load");
        assert_eq!(default_presence.status, "online");

        let updated = store
            .set_user_presence_settings(
                &user.id,
                "dnd",
                Some("  Coding  "),
                Some("Reviewing members"),
            )
            .expect("presence should update");
        assert_eq!(updated.status, "dnd");
        assert_eq!(updated.custom_status.as_deref(), Some("Coding"));
        assert_eq!(updated.activity.as_deref(), Some("Reviewing members"));

        let error = store
            .set_user_presence_settings(&user.id, "busy", None, None)
            .expect_err("invalid status should fail");
        assert!(error.to_string().contains("状态必须"));
    }

    #[test]
    fn presence_visibility_is_limited_to_friends_and_project_members() {
        let store = temp_store();
        let alice = store
            .create_user("presence-alice@example.com", "secret1", None, None)
            .expect("alice should be created");
        let bob = store
            .create_user("presence-bob@example.com", "secret1", None, None)
            .expect("bob should be created");
        let carol = store
            .create_user("presence-carol@example.com", "secret1", None, None)
            .expect("carol should be created");
        let dave = store
            .create_user("presence-dave@example.com", "secret1", None, None)
            .expect("dave should be created");

        assert!(!store
            .can_receive_presence(&alice.id, &bob.id)
            .expect("visibility should query"));

        store
            .add_friend(&alice.id, Some("email"), "presence-bob@example.com")
            .expect("friendship should be created");
        assert!(store
            .can_receive_presence(&alice.id, &bob.id)
            .expect("friends can see presence"));
        assert!(store
            .can_receive_presence(&bob.id, &alice.id)
            .expect("friendship is reciprocal"));

        let project = store
            .create_project(&alice.id, "Presence Visibility", None, None)
            .expect("project should be created")
            .project;
        store
            .add_project_member_by_account(&project.id, "presence-carol@example.com", "member")
            .expect("project member should be added");
        assert!(store
            .can_receive_presence(&carol.id, &alice.id)
            .expect("project members can see presence"));
        assert!(!store
            .can_receive_presence(&dave.id, &alice.id)
            .expect("unrelated users cannot see presence"));
    }
