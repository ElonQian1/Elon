    use super::Store;
    use rusqlite::Connection;

    fn store_with_slots() -> Store {
        let conn = Connection::open_in_memory().expect("db");
        conn.execute_batch(
            r#"
            CREATE TABLE user_codex_credentials (
              user_id TEXT PRIMARY KEY,
              auth_mode TEXT NOT NULL,
              account_hint_hash TEXT,
              source_device TEXT,
              ciphertext_b64 TEXT NOT NULL,
              nonce_b64 TEXT NOT NULL,
              credential_version INTEGER NOT NULL DEFAULT 1,
              last_backup_at TEXT,
              last_lease_at TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            CREATE TABLE user_codex_credential_slots (
              slot_id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL,
              auth_mode TEXT NOT NULL,
              account_hint_hash TEXT,
              source_device TEXT,
              ciphertext_b64 TEXT NOT NULL,
              nonce_b64 TEXT NOT NULL,
              credential_version INTEGER NOT NULL DEFAULT 1,
              status TEXT NOT NULL DEFAULT 'active',
              priority INTEGER NOT NULL DEFAULT 100,
              failure_count INTEGER NOT NULL DEFAULT 0,
              last_backup_at TEXT,
              last_lease_at TEXT,
              last_failure_at TEXT,
              last_success_at TEXT,
              last_error TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            CREATE TABLE user_codex_credential_events (
              id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL,
              event_type TEXT NOT NULL,
              node_id TEXT,
              success INTEGER NOT NULL DEFAULT 1,
              error TEXT,
              created_at TEXT NOT NULL
            );
            "#,
        )
        .expect("schema");
        Store {
            conn: std::sync::Mutex::new(conn),
        }
    }

    #[test]
    fn codex_vault_slots_keep_multiple_accounts_and_avoid_failed_hint() {
        let store = store_with_slots();
        store
            .upsert_user_codex_credential(
                "u1",
                "chatgpt",
                Some("hint-a"),
                Some("pc"),
                "cipher-a",
                "nonce-a",
            )
            .expect("first slot");
        store
            .upsert_user_codex_credential(
                "u1",
                "chatgpt",
                Some("hint-b"),
                Some("pc"),
                "cipher-b",
                "nonce-b",
            )
            .expect("second slot");

        let slots = store.list_user_codex_credential_slots("u1").expect("slots");
        assert_eq!(slots.len(), 2);

        let selected = store
            .select_user_codex_credential_slot("u1", Some("hint-a"))
            .expect("select")
            .expect("fallback slot");
        assert_eq!(selected.account_hint_hash.as_deref(), Some("hint-b"));

        assert!(store
            .mark_user_codex_credential_slot_failed("u1", Some("hint-a"), "usage limit reached")
            .expect("mark failed"));
    }
