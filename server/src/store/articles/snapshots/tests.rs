use super::*;
use serde_json::json;

pub(super) fn fixture() -> Store {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE users(id TEXT PRIMARY KEY,nickname TEXT,email TEXT,phone TEXT);
        CREATE TABLE friend_groups(id TEXT PRIMARY KEY,updated_at TEXT);
        CREATE TABLE friend_group_members(group_id TEXT,user_id TEXT,last_read_at TEXT);
        CREATE TABLE friend_group_messages(id TEXT PRIMARY KEY,group_id TEXT,sender_user_id TEXT,
            content TEXT,attachments_json TEXT,created_at TEXT,recalled_at TEXT,recalled_by TEXT,
            revision INTEGER NOT NULL DEFAULT 1,edited_at TEXT);
        INSERT INTO users VALUES('author','Author',NULL,NULL),('reader','Reader',NULL,NULL),('other','Other',NULL,NULL);
        INSERT INTO friend_groups VALUES('g1','before'),('g2','before');
        INSERT INTO friend_group_members VALUES('g1','author','before'),('g1','reader','before'),('g2','author','before');").unwrap();
    super::super::migration::migrate(&conn).unwrap();
    migration::migrate(&conn).unwrap();
    migration::migrate(&conn).unwrap();
    orphan_media::migrate(&conn).unwrap();
    orphan_media::migrate(&conn).unwrap();
    Store {
        conn: std::sync::Mutex::new(conn),
    }
}

pub(super) fn document() -> SnapshotDocument {
    serde_json::from_value(json!({"schema":SCHEMA,"provider":"chatgpt","title":"Selected discussion",
        "summary":"Two selected messages", "messages":[
        {"id":"public-input-1","role":"user","content":"Explain this table.","created_at_ms":1000,"gap_before":false,"parts":[]},
        {"id":"public-input-2","role":"assistant","content":"| Item | Value |\n| --- | --- |\n| A | 1 |",
         "created_at_ms":2000,"gap_before":true,"parts":[]}
    ]})).unwrap()
}

fn create(store: &Store) -> SnapshotCreated {
    store
        .create_ai_snapshot("author", "g1", "operation-1", document())
        .unwrap()
}

#[test]
fn selected_snapshot_regenerates_ids_and_preserves_order_and_gaps() {
    let store = fixture();
    let result = create(&store);
    let view = store
        .read_ai_snapshot("reader", "g1", &result.snapshot_id)
        .unwrap();
    assert_eq!(view.document.messages.len(), 2);
    assert_eq!(view.owner_name, "Author");
    assert_eq!(
        view.document.messages[1].content,
        document().messages[1].content
    );
    assert!(view.document.messages[1].gap_before);
    assert!(view.document.messages[0].id.starts_with("shared_message_"));
    assert!(!serde_json::to_string(&view)
        .unwrap()
        .contains("public-input-"));
    assert!(!result.message.content.contains("Explain this"));
    let card: SnapshotCard =
        serde_json::from_str(result.message.content.strip_prefix(CARD_PREFIX).unwrap()).unwrap();
    assert_eq!(card.snapshot_id, result.snapshot_id);
    assert_eq!(card.message_count, 2);
    assert_eq!(card.sender_name, "Author");
    assert_eq!(
        message_preview(&result.message.content).unwrap(),
        "[AI\u{5bf9}\u{8bdd}] Selected discussion"
    );
    assert!(store
        .list_articles("reader", Some("g1"), 0)
        .unwrap()
        .items
        .is_empty());
    assert!(store
        .read_article("author", &result.snapshot_id, 1, false)
        .is_err());
    assert!(store.article_draft("author", &result.snapshot_id).is_err());
}

#[test]
fn retries_do_not_resend_or_allow_payload_changes() {
    let store = fixture();
    let first = create(&store);
    let second = create(&store);
    assert!(!first.replayed);
    assert!(second.replayed);
    assert_eq!(first.snapshot_id, second.snapshot_id);
    assert_eq!(first.message.id, second.message.id);
    let mut changed = document();
    changed.messages[0].content.push_str("Changed");
    let error = store
        .create_ai_snapshot("author", "g1", "operation-1", changed)
        .err()
        .unwrap();
    assert_eq!(
        error
            .downcast_ref::<super::super::ArticleFault>()
            .unwrap()
            .0,
        409
    );
    let conn = store.conn().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM friend_group_messages", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
    let persisted: String = conn
        .query_row("SELECT key_hash FROM social_snapshot_operations", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_ne!(persisted, "operation-1");
}

#[test]
fn missing_membership_and_failure_at_last_insert_leave_no_partial_send() {
    let store = fixture();
    assert!(store
        .create_ai_snapshot("other", "g1", "operation-1", document())
        .is_err());
    store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_operation BEFORE INSERT ON social_snapshot_operations
        BEGIN SELECT RAISE(ABORT,'fixture write failure'); END;",
        )
        .unwrap();
    assert!(store
        .create_ai_snapshot("author", "g1", "operation-1", document())
        .is_err());
    let conn = store.conn().unwrap();
    for table in [
        "social_contents",
        "social_content_revisions",
        "social_content_distributions",
        "social_snapshot_operations",
        "friend_group_messages",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "partial commit in {table}");
    }
    let time: String = conn
        .query_row(
            "SELECT updated_at FROM friend_groups WHERE id='g1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(time, "before");
}

#[test]
fn wrong_group_outsiders_departure_and_owner_revoke_are_guarded() {
    let store = fixture();
    let created = create(&store);
    let id = &created.snapshot_id;
    assert!(store.read_ai_snapshot("other", "g1", id).is_err());
    assert!(store.read_ai_snapshot("author", "g2", id).is_err());
    assert!(store.revoke_ai_snapshot("reader", "g1", id).is_err());
    assert!(store.revoke_ai_snapshot("author", "g2", id).is_err());
    store
        .conn()
        .unwrap()
        .execute(
            "DELETE FROM friend_group_members WHERE user_id='author'",
            [],
        )
        .unwrap();
    assert!(store.read_ai_snapshot("author", "g1", id).is_err());
    assert!(store
        .create_ai_snapshot("author", "g1", "operation-1", document())
        .is_err());
    store.revoke_ai_snapshot("author", "g1", id).unwrap();
    store.revoke_ai_snapshot("author", "g1", id).unwrap();
    assert!(store.read_ai_snapshot("reader", "g1", id).is_err());
}

#[test]
fn recall_deletion_and_revocation_cannot_replay_or_republish() {
    for action in ["recall", "delete", "revoke"] {
        let store = fixture();
        let created = create(&store);
        match action {
            "recall" => {
                store
                    .conn()
                    .unwrap()
                    .execute(
                        "UPDATE friend_group_messages SET recalled_at='now' WHERE id=?1",
                        [&created.message.id],
                    )
                    .unwrap();
            }
            "delete" => {
                store
                    .conn()
                    .unwrap()
                    .execute(
                        "DELETE FROM friend_group_messages WHERE id=?1",
                        [&created.message.id],
                    )
                    .unwrap();
            }
            _ => store
                .revoke_ai_snapshot("author", "g1", &created.snapshot_id)
                .unwrap(),
        }
        assert!(store
            .read_ai_snapshot("reader", "g1", &created.snapshot_id)
            .is_err());
        assert!(store
            .create_ai_snapshot("author", "g1", "operation-1", document())
            .is_err());
        assert_eq!(
            store
                .conn()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM social_snapshot_operations", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}

#[test]
fn sqlite_enforces_immutable_payload_and_terminal_revocation() {
    let store = fixture();
    let result = create(&store);
    {
        let conn = store.conn().unwrap();
        assert!(conn
            .execute("UPDATE social_content_revisions SET document_json='{}'", [])
            .is_err());
        assert!(conn
            .execute("UPDATE social_contents SET draft_json='changed'", [])
            .is_err());
        assert!(conn
            .execute(
                "UPDATE social_snapshot_operations SET request_hash='changed'",
                []
            )
            .is_err());
    }
    store
        .revoke_ai_snapshot("author", "g1", &result.snapshot_id)
        .unwrap();
    assert!(store
        .conn()
        .unwrap()
        .execute("UPDATE social_contents SET status='published'", [])
        .is_err());
}

#[test]
fn snapshot_card_cannot_be_edited_through_generic_group_editor() {
    let store = fixture();
    let result = create(&store);
    assert!(store
        .edit_group_message("author", "g1", &result.message.id, 1, "changed")
        .is_err());
    assert!(store
        .send_friend_group_message("reader", "g1", &result.message.content, None)
        .is_err());
    let error = {
        let conn = store.conn().unwrap();
        crate::store::groups::send::insert_message(
            &conn,
            "reader",
            "g1",
            &result.message.content,
            None,
        )
        .err()
        .unwrap()
    };
    assert!(error.to_string().contains("Reserved AI snapshot card"));
    let ordinary = store
        .send_friend_group_message("author", "g1", "Ordinary", None)
        .unwrap();
    assert!(store
        .edit_group_message("author", "g1", &ordinary.id, 1, &result.message.content)
        .is_err());
}

#[test]
fn concurrent_retries_publish_exactly_once() {
    let store = std::sync::Arc::new(fixture());
    let mut threads = Vec::new();
    for _ in 0..8 {
        let store = store.clone();
        threads.push(std::thread::spawn(move || create(&store)));
    }
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| !r.replayed).count(), 1);
    assert!(results
        .iter()
        .all(|r| r.snapshot_id == results[0].snapshot_id));
}
