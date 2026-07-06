    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_store() -> Store {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("elon_friend_recommendations_{suffix}.db"));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn recommendations_include_registered_users_with_relationship_context() {
        let store = temp_store();
        let alice = store
            .create_user(
                "alice-recommend@example.com",
                "password123",
                Some("Alice"),
                None,
            )
            .expect("alice should be created");
        let bob = store
            .create_user(
                "bob-recommend@example.com",
                "password123",
                Some("Bob"),
                None,
            )
            .expect("bob should be created");
        let carol = store
            .create_user(
                "carol-recommend@example.com",
                "password123",
                Some("Carol"),
                None,
            )
            .expect("carol should be created");
        let dave = store
            .create_user(
                "dave-recommend@example.com",
                "password123",
                Some("Dave"),
                None,
            )
            .expect("dave should be created");

        store
            .add_friend(&alice.id, Some("account_id"), &bob.id)
            .expect("alice and bob should be friends");
        store
            .add_friend(&bob.id, Some("account_id"), &carol.id)
            .expect("bob and carol should be friends");

        let recommendations = store
            .list_friend_recommendations(&alice.id)
            .expect("recommendations should load");
        let bob_row = recommendations
            .iter()
            .find(|item| item.id == bob.id)
            .expect("existing friend should still be represented");
        let carol_row = recommendations
            .iter()
            .find(|item| item.id == carol.id)
            .expect("registered non-friend should be represented");
        let dave_row = recommendations
            .iter()
            .find(|item| item.id == dave.id)
            .expect("another registered user should be represented");

        assert!(bob_row.already_friend);
        assert!(!carol_row.already_friend);
        assert!(!dave_row.already_friend);
        assert_eq!(carol_row.mutual_friend_count, 1);
        assert!(!recommendations.iter().any(|item| item.id == alice.id));
    }
