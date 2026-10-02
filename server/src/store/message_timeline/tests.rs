use super::*;
use rusqlite::{params, Connection};

pub(super) fn store(count: usize) -> Store {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE users(id TEXT PRIMARY KEY,nickname TEXT,email TEXT,phone TEXT);
        CREATE TABLE user_friends(user_id TEXT,friend_user_id TEXT);
        CREATE TABLE friend_read_states(user_id TEXT,friend_user_id TEXT,last_read_at TEXT,PRIMARY KEY(user_id,friend_user_id));
        CREATE TABLE friend_group_members(group_id TEXT,user_id TEXT,last_read_at TEXT);
        CREATE TABLE friend_group_messages(id TEXT PRIMARY KEY,group_id TEXT,sender_user_id TEXT,content TEXT,attachments_json TEXT,created_at TEXT,recalled_at TEXT,recalled_by TEXT,revision INTEGER DEFAULT 1,edited_at TEXT);
        CREATE TABLE friend_messages(id TEXT PRIMARY KEY,sender_user_id TEXT,receiver_user_id TEXT,context_user_id TEXT,content TEXT,attachments_json TEXT,created_at TEXT,recalled_at TEXT,recalled_by TEXT);
        CREATE TABLE messages(id TEXT PRIMARY KEY,project_id TEXT,conversation_id TEXT,created_at TEXT);
        CREATE TABLE project_channel_messages(id TEXT PRIMARY KEY,project_id TEXT,channel_id TEXT,created_at TEXT,task_id TEXT);
        CREATE TABLE tasks(id TEXT PRIMARY KEY,status TEXT,error TEXT,apk_url TEXT,codex_thread_id TEXT);
        CREATE TABLE group_ai_selected_sources(request_id TEXT,message_id TEXT,revision INTEGER);
        CREATE TABLE group_ai_reply_requests(id TEXT,state TEXT,result_message_id TEXT,requester_id TEXT,web_provider TEXT);
        CREATE TABLE group_ai_reply_contexts(request_id TEXT,sources_json TEXT,allow_continue INTEGER,version INTEGER);
        INSERT INTO users VALUES('a','Alice',NULL,NULL),('b','Bob',NULL,NULL),('usr_elon_ai','EL',NULL,NULL);
        INSERT INTO friend_group_members VALUES('g','a',NULL),('g','b',NULL),('h','a',NULL);
        INSERT INTO user_friends VALUES('a','b'),('b','a');").unwrap();
    crate::store::friend_messages::social_quotes::migrate(&conn).unwrap();
    // Simulate an existing large history before upgrading. No body backfill into the journal.
    let tx = conn.transaction().unwrap();
    for i in 0..count {
        tx.execute("INSERT INTO friend_group_messages(id,group_id,sender_user_id,content,created_at) VALUES(?1,'g','b','body','2026-10-01T00:00:00Z')", [format!("m{i:06}")]).unwrap();
    }
    tx.commit().unwrap();
    schema::migrate(&conn).unwrap();
    schema::migrate(&conn).unwrap();
    reading::schema::migrate(&conn).unwrap();
    Store {
        conn: std::sync::Mutex::new(conn),
    }
}

fn request() -> TimelineRequest {
    TimelineRequest {
        kind: "group".into(),
        id: "g".into(),
        project: String::new(),
        before: None,
        sync: None,
        limit: None,
    }
}

#[test]
fn message_timeline_window_recovery_keeps_old_position_and_rechecks_bodies_and_access() {
    let store = store(1_000);
    let request = request();
    let ids = vec!["m000020".into(), "m000021".into(), "m000022".into()];
    {
        let conn = store.conn().unwrap();
        conn.execute(
            "UPDATE friend_group_messages SET content='edited',revision=2 WHERE id='m000020'",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM friend_group_messages WHERE id='m000021'", [])
            .unwrap();
        conn.execute("UPDATE message_timeline_epoch SET value='replacement'", [])
            .unwrap();
    }
    let recovered = store
        .recover_message_timeline_window("a", &request, &ids)
        .unwrap();
    assert_eq!(recovered.messages.len(), 2);
    assert_eq!(recovered.messages[0]["id"], "m000020");
    assert_eq!(recovered.messages[0]["content"], "edited");
    assert_eq!(recovered.removed_ids, vec!["m000021"]);
    let mut older = request.clone();
    older.before = recovered.before;
    assert_eq!(
        store
            .read_message_timeline("a", &older)
            .unwrap()
            .messages
            .last()
            .unwrap()["id"],
        "m000019"
    );
    let mut live = request.clone();
    live.sync = recovered.sync;
    assert!(!store.read_message_timeline("a", &live).unwrap().reset);
    assert!(store
        .recover_message_timeline_window("a", &request, &vec!["m000020".into(); 301])
        .is_err());
    store
        .conn()
        .unwrap()
        .execute("DELETE FROM friend_group_members WHERE user_id='a'", [])
        .unwrap();
    assert!(store
        .recover_message_timeline_window("a", &request, &ids)
        .is_err());
}

#[test]
fn message_timeline_large_history_same_timestamp_is_bounded_and_complete() {
    let store = store(100_005);
    let mut request = request();
    request.limit = Some(100_000);
    let first = store.read_message_timeline("a", &request).unwrap();
    assert_eq!(first.messages.len(), 100);
    assert_eq!(first.messages[0]["id"], "m099905");
    assert!(first.has_more);
    request.before = first.before;
    let second = store.read_message_timeline("a", &request).unwrap();
    assert_eq!(second.messages.len(), 100);
    assert_eq!(second.messages.last().unwrap()["id"], "m099904");
    assert!(
        second.sync.is_none(),
        "historical reads must not advance live checkpoint"
    );
    let conn = store.conn().unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM message_timeline_changes", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}

#[test]
fn message_timeline_reconnect_replays_edits_deletes_and_multiple_pages() {
    let store = store(3);
    let mut request = request();
    let initial = store.read_message_timeline("a", &request).unwrap();
    request.sync = initial.sync;
    {
        let conn = store.conn().unwrap();
        conn.execute(
            "UPDATE friend_group_messages SET content='edited',revision=2 WHERE id='m000001'",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM friend_group_messages WHERE id='m000000'", [])
            .unwrap();
        for i in 0..125 {
            conn.execute("INSERT INTO friend_group_messages(id,group_id,sender_user_id,content,created_at) VALUES(?1,'g','b','new','2026-10-01T00:00:00Z')",[format!("new{i:03}")]).unwrap();
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut removed = vec![];
    let mut pages = 0;
    loop {
        let page = store.read_message_timeline("a", &request).unwrap();
        assert!(page.messages.len() <= 50);
        for m in page.messages {
            if m["id"] == "m000001" {
                assert_eq!(m["content"], "edited");
            }
            ids.insert(m["id"].as_str().unwrap().to_owned());
        }
        removed.extend(page.removed_ids);
        pages += 1;
        request.sync = page.sync;
        if !page.has_more {
            break;
        }
    }
    assert_eq!(pages, 3);
    assert_eq!(ids.len(), 126);
    assert_eq!(removed, vec!["m000000"]);
    let empty = store.read_message_timeline("a", &request).unwrap();
    assert!(empty.messages.is_empty());
    assert!(!empty.has_more);
}

#[test]
fn message_timeline_scope_revocation_and_receipts_fail_closed() {
    let store = store(4);
    let mut r = request();
    let first = store.read_message_timeline("a", &r).unwrap();
    r.sync = first.sync.clone();
    assert!(store.read_message_timeline("b", &r).is_err());
    r.id = "h".into();
    assert!(store.read_message_timeline("a", &r).is_err());
    r.id = "g".into();
    r.sync = None;
    store.mark_timeline_read("a", &r, "m000001").unwrap();
    assert!(store.mark_timeline_read("a", &r, "unknown").is_err());
    store
        .conn()
        .unwrap()
        .execute("DELETE FROM friend_group_members WHERE user_id='a'", [])
        .unwrap();
    assert!(store.read_message_timeline("a", &r).is_err());
    assert!(store.mark_timeline_read("a", &r, "m000001").is_err());
}

#[test]
fn message_timeline_friend_ai_bridges_preserve_recipient_privacy() {
    let store = store(0);
    let mut r = request();
    r.kind = "friend".into();
    r.id = "b".into();
    let first = store.read_message_timeline("a", &r).unwrap();
    let mut b = r.clone();
    b.id = "a".into();
    b.sync = store.read_message_timeline("b", &b).unwrap().sync;
    store.conn().unwrap().execute("INSERT INTO friend_messages VALUES('private','usr_elon_ai','a','b','private',NULL,'2026',NULL,NULL)",[]).unwrap();
    r.sync = first.sync;
    assert_eq!(
        store.read_message_timeline("a", &r).unwrap().messages.len(),
        1
    );
    let hidden = store.read_message_timeline("b", &b).unwrap();
    assert!(hidden.messages.is_empty());
    assert!(hidden.removed_ids.is_empty());
}

#[test]
fn message_timeline_expired_checkpoint_requests_reset_and_rollbacks_leave_no_changes() {
    let store = store(1);
    let mut r = request();
    r.sync = store.read_message_timeline("a", &r).unwrap().sync;
    {
        let mut conn = store.conn().unwrap();
        {
            let tx = conn.transaction().unwrap();
            tx.execute("UPDATE friend_group_messages SET content='rollback'", [])
                .unwrap();
        }
        assert_eq!(
            conn.query_row("SELECT count(*) FROM message_timeline_changes", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
        for i in 0..3 {
            conn.execute(
                "UPDATE friend_group_messages SET content=?1",
                params![format!("change{i}")],
            )
            .unwrap();
        }
        conn.execute("DELETE FROM message_timeline_changes WHERE seq<3", [])
            .unwrap();
    }
    assert!(store.read_message_timeline("a", &r).unwrap().reset);
}

#[test]
fn message_timeline_related_quote_source_and_consent_changes_are_replayed() {
    let store = store(3);
    {
        let conn = store.conn().unwrap();
        let quote = crate::store::friend_messages::social_quotes::SocialQuote {
            message_id: "m000000".into(),
            sender_name: "Bob".into(),
            content: "body".into(),
            attachments: vec![],
            revision: 1,
            unavailable: false,
        };
        crate::store::friend_messages::social_quotes::save(&conn, "group", "m000001", Some(&quote))
            .unwrap();
        conn.execute_batch("INSERT INTO group_ai_reply_requests VALUES('r','completed','m000002','a','chatgpt_web'); INSERT INTO group_ai_reply_contexts VALUES('r','[]',0,1); INSERT INTO group_ai_selected_sources VALUES('r','m000000',1);").unwrap();
    }
    let mut r = request();
    r.sync = store.read_message_timeline("a", &r).unwrap().sync;
    store.conn().unwrap().execute_batch("UPDATE group_ai_reply_contexts SET allow_continue=1,version=2; UPDATE friend_group_messages SET recalled_at='now' WHERE id='m000000';").unwrap();
    let updated = store.read_message_timeline("a", &r).unwrap();
    assert_eq!(updated.messages.len(), 3);
    assert_eq!(updated.messages[1]["quote"]["unavailable"], true);
    assert_eq!(updated.messages[2]["ai_reply"]["version"], 2);
}

#[test]
fn message_timeline_long_offline_gap_refreshes_window_instead_of_downloading_all_history() {
    let store = store(1);
    let mut r = request();
    r.sync = store.read_message_timeline("a", &r).unwrap().sync;
    {
        let mut conn = store.conn().unwrap();
        let tx = conn.transaction().unwrap();
        for _ in 0..1001 {
            tx.execute("UPDATE friend_group_messages SET content='changed'", [])
                .unwrap();
        }
        tx.commit().unwrap();
    }
    let page = store.read_message_timeline("a", &r).unwrap();
    assert!(page.reset);
    assert!(page.messages.is_empty());
    r.sync = None;
    assert_eq!(
        store.read_message_timeline("a", &r).unwrap().messages[0]["content"],
        "changed"
    );
}
