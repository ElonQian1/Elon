use super::*;
use serde_json::json;

fn fixture() -> Store {
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
    migration::migrate(&conn).unwrap();
    migration::migrate(&conn).unwrap();
    Store {
        conn: std::sync::Mutex::new(conn),
    }
}
fn document() -> Document {
    serde_json::from_value(json!({"schema":SCHEMA,"source":"wechat","title":"Imported records","raw_text":"original export",
        "messages":[
          {"id":"one","sender":"A","time":"2026-09-27 12:01","kind":"forward","text":"[Chat history]"},
          {"id":"two","parent_id":"one","sender":"B","time":"2026-09-26 12:00","kind":"text","text":"nested\ntext"},
          {"id":"three","sender":"A","time":"2026-09-27 12:02","kind":"text","text":"original"}
        ]})).unwrap()
}
fn create(store: &Store) -> Created {
    store
        .create_chat_record("author", "g1", "operation-123", document())
        .unwrap()
}
#[test]
fn tree_and_original_text_survive_and_retry_publishes_once() {
    let store = fixture();
    let a = create(&store);
    let b = create(&store);
    assert!(!a.replayed);
    assert!(b.replayed);
    assert_eq!(a.message.id, b.message.id);
    let v = store
        .read_chat_record("reader", "g1", &a.card.record_id)
        .unwrap();
    assert_eq!(v.document.raw_text, "original export");
    assert_eq!(v.document.messages[1].parent_id.as_deref(), Some("one"));
    assert_eq!(v.card.message_count, 2);
    assert_eq!(v.card.total_count, 3);
    let mut changed = document();
    changed.title = "Changed".into();
    assert!(store
        .create_chat_record("author", "g1", "operation-123", changed)
        .is_err());
    assert_eq!(
        store
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM friend_group_messages", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn assets_are_deduplicated_bound_to_group_and_revocable() {
    let s = fixture();
    let bytes = b"fixture attachment";
    let asset = s.upload_chat_record_asset("author", "g1", bytes).unwrap();
    assert_eq!(
        asset.asset_id,
        s.upload_chat_record_asset("author", "g1", bytes)
            .unwrap()
            .asset_id
    );
    assert!(s.upload_chat_record_asset("other", "g1", bytes).is_err());
    let mut doc = document();
    doc.messages[2].kind = "file".into();
    doc.messages[2].asset_id = Some(asset.asset_id.clone());
    assert!(s
        .create_chat_record("author", "g2", "operation-456", doc.clone())
        .is_err());
    let r = s
        .create_chat_record("author", "g1", "operation-456", doc)
        .unwrap();
    let id = &r.card.record_id;
    assert_eq!(
        s.read_chat_record_asset("reader", "g1", id, &asset.asset_id)
            .unwrap()
            .1,
        bytes
    );
    assert!(s
        .read_chat_record_asset("other", "g1", id, &asset.asset_id)
        .is_err());
    assert!(s.read_chat_record("author", "g2", id).is_err());
    assert!(s.revoke_chat_record("reader", "g1", id).is_err());
    s.revoke_chat_record("author", "g1", id).unwrap();
    assert!(s
        .read_chat_record_asset("reader", "g1", id, &asset.asset_id)
        .is_err());
}
#[test]
fn recalled_deleted_or_revoked_records_cannot_be_replayed() {
    for action in ["recall", "delete", "revoke"] {
        let s = fixture();
        let r = create(&s);
        match action {
            "recall" => {
                s.conn()
                    .unwrap()
                    .execute("UPDATE friend_group_messages SET recalled_at='now'", [])
                    .unwrap();
            }
            "delete" => {
                s.conn()
                    .unwrap()
                    .execute("DELETE FROM friend_group_messages", [])
                    .unwrap();
            }
            _ => s
                .revoke_chat_record("author", "g1", &r.card.record_id)
                .unwrap(),
        }
        assert!(s
            .read_chat_record("reader", "g1", &r.card.record_id)
            .is_err());
        assert!(s
            .create_chat_record("author", "g1", "operation-123", document())
            .is_err());
    }
}
#[test]
fn invalid_trees_and_forged_or_edited_cards_are_rejected() {
    let mut doc = document();
    doc.messages[1].parent_id = Some("missing".into());
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.messages[0].kind = "text".into();
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.messages[2].id = "one".into();
    assert!(doc.validate().is_err());
    let s = fixture();
    let r = create(&s);
    assert!(s
        .send_friend_group_message("reader", "g1", &r.message.content, None)
        .is_err());
    assert!(s
        .edit_group_message("author", "g1", &r.message.id, 1, "changed")
        .is_err());
    s.conn()
        .unwrap()
        .execute_batch("DELETE FROM friend_group_members WHERE user_id='reader'")
        .unwrap();
    assert!(s
        .read_chat_record("reader", "g1", &r.card.record_id)
        .is_err());
}
