    use crate::store::Store;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_social_ai_pending_test_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn finds_latest_unanswered_friend_and_group_mentions() {
        let store = temp_store();
        let alice = store
            .create_user("pending-alice@example.com", "secret1", Some("Alice"), None)
            .expect("alice should be created");
        let bob = store
            .create_user("pending-bob@example.com", "secret1", Some("Bob"), None)
            .expect("bob should be created");
        store
            .add_friend(&alice.id, Some("email"), "pending-bob@example.com")
            .expect("alice can add bob");

        store
            .send_friend_message(&alice.id, &bob.id, "how to remove fleas?", None)
            .expect("question should be stored");
        let trigger = store
            .send_friend_message(&alice.id, &bob.id, "@EL", None)
            .expect("mention should be stored");
        let pending = store
            .latest_unanswered_friend_social_ai_mention(&alice.id, &bob.id)
            .expect("pending lookup should work")
            .expect("friend mention should be pending");
        assert_eq!(pending.trigger_message_id, trigger.id);
        store
            .insert_friend_social_ai_reply(&alice.id, &bob.id, "use a flea treatment plan")
            .expect("ai reply should be stored");
        assert!(store
            .latest_unanswered_friend_social_ai_mention(&alice.id, &bob.id)
            .expect("pending lookup should work")
            .is_none());

        let group = store
            .create_friend_group(&alice.id, Some("Pending Test"), &[bob.id.clone()])
            .expect("group should be created");
        let trigger = store
            .send_friend_group_message(&alice.id, &group.id, "＠EL explain this", None)
            .expect("group mention should be stored");
        let pending = store
            .latest_unanswered_group_social_ai_mention(&alice.id, &group.id)
            .expect("pending lookup should work")
            .expect("group mention should be pending");
        assert_eq!(pending.trigger_message_id, trigger.id);
        store
            .insert_group_social_ai_reply(&group.id, "group answer")
            .expect("group ai reply should be stored");
        assert!(store
            .latest_unanswered_group_social_ai_mention(&alice.id, &group.id)
            .expect("pending lookup should work")
            .is_none());
        store
            .insert_group_social_ai_reply(&group.id, "EL fallback asks user to retry @EL")
            .expect("group ai fallback reply should be stored");
        assert!(store
            .latest_unanswered_group_social_ai_mention(&alice.id, &group.id)
            .expect("pending lookup should work")
            .is_none());
    }
