use super::*;

fn database() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE users(id TEXT PRIMARY KEY,nickname TEXT,email TEXT,phone TEXT);
        CREATE TABLE friend_group_members(group_id TEXT,user_id TEXT);
        CREATE TABLE friend_group_messages(id TEXT PRIMARY KEY,group_id TEXT,sender_user_id TEXT,content TEXT,attachments_json TEXT,revision INTEGER,recalled_at TEXT);
        CREATE TABLE friend_messages(id TEXT PRIMARY KEY,sender_user_id TEXT,receiver_user_id TEXT,context_user_id TEXT,content TEXT,attachments_json TEXT,recalled_at TEXT);
        INSERT INTO users VALUES('a','Alice',NULL,NULL),('b','Bob',NULL,NULL),('c','Carol',NULL,NULL);
        INSERT INTO friend_group_members VALUES('g','a'),('g','b');
        INSERT INTO friend_group_messages VALUES('source','g','b','original',NULL,2,NULL);
        INSERT INTO friend_messages VALUES('direct','b','a',NULL,'private',NULL,NULL);").unwrap();
    migrate(&conn).unwrap();
    conn
}
fn source(revision: i64) -> QuoteSource {
    QuoteSource {
        message_id: "source".into(),
        revision: Some(revision),
    }
}

#[test]
fn quotes_preserve_body_and_version_without_nested_metadata() {
    let conn = database();
    let quote = prepare(&conn, "a", "group", "g", Some(&source(2)))
        .unwrap()
        .unwrap();
    save(&conn, "group", "reply", Some(&quote)).unwrap();
    conn.execute(
        "UPDATE friend_group_messages SET content='edited',revision=3 WHERE id='source'",
        [],
    )
    .unwrap();
    let loaded = read(&conn, "group", "reply", false).unwrap().unwrap();
    assert_eq!(loaded.content, "original");
    assert_eq!(loaded.sender_name, "Bob");
    assert_eq!(loaded.revision, 2);
    assert!(!loaded.unavailable);
    assert!(read(&conn, "group", "plain", false).unwrap().is_none());
    assert!(read(&conn, "group", "reply", true).unwrap().is_none());
    let context = context_text(&conn, "group", "reply", "reply body".into()).unwrap();
    assert!(context.starts_with("reply body\n"));
    assert!(context.contains("original") && context.contains("Bob"));
    assert!(!context.contains("edited"));
}

#[test]
fn quotes_reject_other_scopes_nonmembers_stale_versions_and_recall() {
    let conn = database();
    assert!(prepare(&conn, "a", "group", "other", Some(&source(2))).is_err());
    assert!(prepare(&conn, "c", "group", "g", Some(&source(2))).is_err());
    assert!(prepare(&conn, "a", "group", "g", Some(&source(1))).is_err());
    conn.execute("UPDATE friend_group_messages SET recalled_at='now'", [])
        .unwrap();
    assert!(prepare(&conn, "a", "group", "g", Some(&source(2))).is_err());
}

#[test]
fn quotes_hide_recalled_or_removed_source_and_respect_friend_pair() {
    let conn = database();
    let quote = prepare(&conn, "a", "group", "g", Some(&source(2)))
        .unwrap()
        .unwrap();
    save(&conn, "group", "reply", Some(&quote)).unwrap();
    conn.execute("DELETE FROM friend_group_messages", [])
        .unwrap();
    let loaded = read(&conn, "group", "reply", false).unwrap().unwrap();
    assert!(loaded.unavailable && loaded.content.is_empty() && loaded.attachments.is_empty());
    let context = context_text(&conn, "group", "reply", "reply body".into()).unwrap();
    assert!(!context.contains("original"));
    let direct = QuoteSource {
        message_id: "direct".into(),
        revision: Some(1),
    };
    assert!(prepare(&conn, "a", "friend", "b", Some(&direct))
        .unwrap()
        .is_some());
    assert!(prepare(&conn, "c", "friend", "b", Some(&direct)).is_err());
    assert!(prepare(&conn, "a", "friend", "c", Some(&direct)).is_err());
}

#[test]
fn quotes_rollback_with_message_transaction_and_migrate_idempotently() {
    let mut conn = database();
    migrate(&conn).unwrap();
    {
        let tx = conn.transaction().unwrap();
        let quote = prepare(&tx, "a", "group", "g", Some(&source(2)))
            .unwrap()
            .unwrap();
        save(&tx, "group", "reply", Some(&quote)).unwrap();
    }
    assert!(read(&conn, "group", "reply", false).unwrap().is_none());
    assert!(prepare(&conn, "a", "group", "g", None).unwrap().is_none());
}
