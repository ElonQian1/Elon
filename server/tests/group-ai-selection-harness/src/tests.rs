use super::*;
use rusqlite::params;

struct Fixture {
    store: Store,
}
impl Fixture {
    fn new() -> Self {
        let store = Store {
            connection: std::sync::Mutex::new(Connection::open_in_memory().unwrap()),
        };
        let conn = store.conn().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE users(id TEXT PRIMARY KEY,nickname TEXT,email TEXT);
            CREATE TABLE friend_groups(id TEXT PRIMARY KEY);
            CREATE TABLE friend_group_members(group_id TEXT,user_id TEXT);
            CREATE TABLE friend_group_messages(id TEXT PRIMARY KEY,group_id TEXT,sender_user_id TEXT,
                content TEXT,created_at TEXT,recalled_at TEXT,attachments_json TEXT,revision INTEGER NOT NULL DEFAULT 1);
            INSERT INTO users VALUES('u','user','test@example.com'),('v','member','member@example.com');
            INSERT INTO friend_groups VALUES('g'),('foreign');
            INSERT INTO friend_group_members VALUES('g','u'),('g','v');
            INSERT INTO friend_group_messages(id,group_id,sender_user_id,content,created_at) VALUES
              ('a','g','u','SELECTED_A','2026-09-20'),('b','g','v','EXCLUDED_B','2026-09-20'),
              ('c','g','v','SELECTED_C','2026-09-20'),('d','foreign','v','FOREIGN','2026-09-20');
            CREATE TABLE group_ai_reply_requests (
                id TEXT PRIMARY KEY,group_id TEXT NOT NULL REFERENCES friend_groups(id),
                trigger_message_id TEXT NOT NULL,requester_id TEXT NOT NULL REFERENCES users(id),
                state TEXT NOT NULL,result_message_id TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL,
                engine TEXT NOT NULL DEFAULT 'server_api',operation_hash TEXT UNIQUE,context_prompt TEXT,
                web_provider TEXT NOT NULL DEFAULT 'chatgpt_web', UNIQUE(group_id,trigger_message_id));
            CREATE TABLE group_ai_work_options (
                request_id TEXT PRIMARY KEY REFERENCES group_ai_reply_requests(id) ON DELETE CASCADE,
                agent TEXT,allow_fallback INTEGER NOT NULL);
            INSERT INTO group_ai_reply_requests(id,group_id,trigger_message_id,requester_id,state,created_at,updated_at)
                VALUES('legacy','g','b','u','dispatched','now','now');
            INSERT INTO group_ai_work_options VALUES('legacy','model',1);").unwrap();
        migration::migrate(&conn).unwrap();
        context::migrate(&conn).unwrap();
        drop(conn);
        Self { store }
    }
    fn input(ids: &[&str]) -> selection::GroupAiSelection {
        selection::GroupAiSelection {
            message_ids: ids.iter().map(|s| s.to_string()).collect(),
            message_revisions: ids.iter().map(|s| (s.to_string(), 1)).collect(),
            question: "compare".into(),
            allow_continue: false,
            attachment_transport_version: 1,
        }
    }
    fn prepare(
        &self,
        op: &str,
        input: &selection::GroupAiSelection,
    ) -> Result<web::WebGroupRequest> {
        self.store
            .prepare_group_ai_selection("u", "g", "a", op, input)
    }
}
fn op() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[test]
fn attachment_manifest_preserves_scope_and_requires_updated_client() {
    let f = Fixture::new();
    let raw = serde_json::json!([{"attachment_id":"fixture", "display_name":"chart.png", "mime_type":"image/png",
        "size_bytes":123, "url":"https://platform.example/api/user/u/chat-attachments/g/chart.png"}]).to_string();
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET attachments_json=?1,content=?2 WHERE id='a'",
            params![
                raw,
                "> quote\n```rust\nlet n = 1;\n```\n| A | B |\n|---|---|\n| 1 | 2 |"
            ],
        )
        .unwrap();
    let mut input = Fixture::input(&["a"]);
    input.attachment_transport_version = 0;
    assert!(f.prepare(&op(), &input).is_err());
    input.attachment_transport_version = 1;
    let token = op();
    let request = f.prepare(&token, &input).unwrap();
    assert_eq!(request.attachments.len(), 1);
    assert_eq!(request.attachments[0].message_id, "a");
    assert!(request.prompt.contains("group_01_chart.png"));
    assert!(request.prompt.contains("```rust"));
    assert!(request.prompt.contains("| 1 | 2 |"));
    assert!(!request.prompt.contains("platform.example"));
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET revision=2 WHERE id='a'",
            [],
        )
        .unwrap();
    assert!(f
        .store
        .group_web_ai_action("u", "g", &request.id, &token, "dispatch")
        .is_err());
}

#[test]
fn image_only_selection_retains_attachment_and_dispatches_without_text() {
    let f = Fixture::new();
    let raw = serde_json::json!([{"attachment_id":"fixture", "display_name":"chart.jpg", "mime_type":"image/jpeg",
        "size_bytes":128238, "url":"https://platform.example/api/user/u/chat-attachments/g/chart.jpg"}]).to_string();
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET content='',attachments_json=?1 WHERE id='a'",
            [raw],
        )
        .unwrap();
    let token = op();
    let mut input = Fixture::input(&["a"]);
    input.attachment_transport_version = 0;
    assert!(f.prepare(&token, &input).is_err());
    assert!(f.store.prepare_group_web_ai("u", "g", "a", &token).is_err());
    input.attachment_transport_version = 1;
    let request = f.prepare(&token, &input).unwrap();
    assert_eq!(request.attachments.len(), 1);
    assert_eq!(request.attachments[0].size_bytes, 128238);
    assert!(
        f.store
            .group_web_ai_action("u", "g", &request.id, &token, "dispatch")
            .unwrap()
            .dispatch_permit
    );
    assert!(
        !f.store
            .group_web_ai_action("u", "g", &request.id, &token, "dispatch")
            .unwrap()
            .dispatch_permit
    );
    assert!(f
        .store
        .group_web_ai_action("v", "g", &request.id, &token, "status")
        .is_err());
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET recalled_at='recalled' WHERE id='a'",
            [],
        )
        .unwrap();
    assert!(f
        .store
        .group_web_ai_action("u", "g", &request.id, &token, "status")
        .is_err());
}

#[test]
fn migration_preserves_legacy_ownership_and_work_options() {
    let f = Fixture::new();
    let conn = f.store.conn().unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT agent FROM group_ai_work_options WHERE request_id='legacy'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "model"
    );
    assert_eq!(
        conn.query_row(
            "SELECT context_scope FROM group_ai_reply_requests WHERE id='legacy'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "recent"
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    assert!(conn.execute("INSERT INTO group_ai_reply_requests(id,group_id,trigger_message_id,requester_id,state,created_at,updated_at) VALUES('bad','g','b','v','prepared','now','now')",[]).is_err());
}

#[test]
fn selected_only_ordered_idempotent_and_multi_user() {
    let f = Fixture::new();
    let operation = op();
    let mut input = Fixture::input(&["c", "a"]);
    let request = f.prepare(&operation, &input).unwrap();
    assert!(!request.prompt.contains("EXCLUDED_B"));
    assert!(
        request.prompt.find("SELECTED_A").unwrap() < request.prompt.find("SELECTED_C").unwrap()
    );
    input.message_ids.reverse();
    assert_eq!(request.id, f.prepare(&operation, &input).unwrap().id);
    input.question = "different".into();
    assert!(f.prepare(&operation, &input).is_err());
    let other = f
        .store
        .prepare_group_ai_selection("v", "g", "a", &op(), &input)
        .unwrap();
    assert_ne!(request.id, other.id);
    let dispatched = f
        .store
        .group_web_ai_action("u", "g", &request.id, &operation, "dispatch")
        .unwrap();
    assert!(dispatched.dispatch_permit);
    assert!(
        !f.store
            .group_web_ai_action("u", "g", &request.id, &operation, "dispatch")
            .unwrap()
            .dispatch_permit
    );
    assert!(f
        .store
        .group_web_ai_action("v", "g", &request.id, &operation, "status")
        .is_err());
}

#[test]
fn invalid_selection_cannot_be_dispatched_or_truncated() {
    let f = Fixture::new();
    for ids in [
        &[][..],
        &["a", "a"][..],
        &["a", "d"][..],
        &["a", "missing"][..],
    ] {
        assert!(f.prepare(&op(), &Fixture::input(ids)).is_err());
    }
    let mut input = Fixture::input(&["a"]);
    input.message_revisions.insert("a".into(), 2);
    assert!(f.prepare(&op(), &input).is_err());
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET content=?1 WHERE id='a'",
            ["x".repeat(21_000)],
        )
        .unwrap();
    assert!(f.prepare(&op(), &Fixture::input(&["a"])).is_err());
}

#[test]
fn edits_recalls_membership_and_fallback_are_checked() {
    for mutation in [
        "UPDATE friend_group_messages SET revision=2 WHERE id='c'",
        "UPDATE friend_group_messages SET recalled_at='now' WHERE id='c'",
        "DELETE FROM friend_group_members WHERE user_id='u'",
    ] {
        let f = Fixture::new();
        let operation = op();
        let req = f.prepare(&operation, &Fixture::input(&["a", "c"])).unwrap();
        assert!(f
            .store
            .group_web_ai_action("u", "g", &req.id, &operation, "fallback")
            .is_err());
        f.store.conn().unwrap().execute(mutation, []).unwrap();
        assert!(f
            .store
            .group_web_ai_action("u", "g", &req.id, &operation, "dispatch")
            .is_err());
        assert!(selection::validate_sources(&f.store.conn().unwrap(), "u", "g", &req.id).is_err());
    }
}

#[test]
fn recent_request_cannot_adopt_selected_operation() {
    let f = Fixture::new();
    let operation = op();
    let req = f.prepare(&operation, &Fixture::input(&["a"])).unwrap();
    assert!(f
        .store
        .prepare_group_web_ai("u", "g", "a", &operation)
        .is_err());
    let recent = f.store.prepare_group_web_ai("u", "g", "a", &op()).unwrap();
    assert_ne!(req.id, recent.id);
    assert_eq!(recent.context_scope, "recent");
    assert!(f
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE group_ai_reply_requests SET context_scope='bad' WHERE id=?1",
            params![req.id]
        )
        .is_err());
}

#[test]
fn reply_share_contains_only_selection_and_requires_consent_and_preserves_original_edits() {
    let f = Fixture::new();
    let operation = op();
    let req = f.prepare(&operation, &Fixture::input(&["a", "c"])).unwrap();
    let conn = f.store.conn().unwrap();
    conn.execute("INSERT INTO friend_group_messages(id,group_id,sender_user_id,content,created_at) VALUES('gai_reply','g','u','**ANSWER**','2026-09-20T00:00:00Z')",[]).unwrap();
    conn.execute("UPDATE group_ai_reply_requests SET state='completed',result_message_id='gai_reply' WHERE id=?1",[&req.id]).unwrap();
    drop(conn);
    let draft = f
        .store
        .group_ai_context_share_draft("u", "g", "gai_reply")
        .unwrap();
    assert_eq!(draft["document"]["messages"].as_array().unwrap().len(), 4);
    let text = draft.to_string();
    assert!(
        text.contains("SELECTED_A") && text.contains("SELECTED_C") && text.contains("**ANSWER**")
    );
    assert!(!text.contains("EXCLUDED_B") && !text.contains("operation_hash"));
    assert!(f
        .store
        .group_ai_context_share_draft("v", "g", "gai_reply")
        .is_err());
    assert!(f
        .store
        .group_ai_context_share_draft("u", "foreign", "gai_reply")
        .is_err());
    f.store.conn().unwrap().execute(
        "UPDATE friend_group_messages SET revision=2,content='EDITED_AFTER_ANSWER' WHERE id='c'",
        [],
    )
    .unwrap();
    let frozen = f
        .store
        .group_ai_context_share_draft("u", "g", "gai_reply")
        .unwrap()
        .to_string();
    assert!(frozen.contains("SELECTED_C") && !frozen.contains("EDITED_AFTER_ANSWER"));
    assert!(f
        .store
        .set_group_ai_continuation("v", "g", "gai_reply", true, 1)
        .is_err());
    f.store
        .set_group_ai_continuation("u", "g", "gai_reply", true, 1)
        .unwrap();
    assert!(f
        .store
        .group_ai_context_share_draft("v", "g", "gai_reply")
        .is_ok());
    assert!(f
        .store
        .set_group_ai_continuation("u", "g", "gai_reply", false, 1)
        .is_err());
    f.store
        .set_group_ai_continuation("u", "g", "gai_reply", false, 2)
        .unwrap();
    assert!(f
        .store
        .group_ai_context_share_draft("v", "g", "gai_reply")
        .is_err());
    assert!(f
        .store
        .group_ai_reply_sources("v", "g", "gai_reply")
        .is_ok());
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET recalled_at='now' WHERE id='c'",
            [],
        )
        .unwrap();
    assert!(!f
        .store
        .group_ai_reply_sources("v", "g", "gai_reply")
        .unwrap()
        .to_string()
        .contains("SELECTED_C"));
    f.store
        .conn()
        .unwrap()
        .execute("DELETE FROM friend_group_members WHERE user_id='v'", [])
        .unwrap();
    assert!(f
        .store
        .group_ai_reply_sources("v", "g", "gai_reply")
        .is_err());
}

#[test]
fn rich_sources_are_frozen_with_a_separate_private_attachment_manifest() {
    let f = Fixture::new();
    let conn = f.store.conn().unwrap();
    conn.execute(
        "UPDATE friend_group_messages SET attachments_json=?1 WHERE id='a'",
        [r#"[{"kind":"image","attachment_id":"ref","url":"/api/user/u/chat-attachments/g/reference.png","display_name":"Reference","mime_type":"image/png","size_bytes":123}]"#],
    )
    .unwrap();
    drop(conn);
    let mut input = Fixture::input(&["c", "a"]);
    input.allow_continue = true;
    let operation = op();
    let req = f.prepare(&operation, &input).unwrap();
    assert!(!req
        .prompt
        .contains("/api/user/u/chat-attachments/g/reference.png"));
    assert_eq!(req.attachments.len(), 1);
    assert_eq!(req.attachments[0].message_id, "a");
    input.allow_continue = false;
    assert!(f.prepare(&operation, &input).is_err());
    let conn = f.store.conn().unwrap();
    conn.execute("INSERT INTO friend_group_messages(id,group_id,sender_user_id,content,created_at) VALUES('gai_rich','g','u','**ANSWER**','2026-09-20T00:00:00Z')",[]).unwrap();
    conn.execute("UPDATE group_ai_reply_requests SET state='completed',result_message_id='gai_rich' WHERE id=?1",[&req.id]).unwrap();
    conn.execute("UPDATE friend_group_messages SET attachments_json=NULL,content='Changed',revision=2 WHERE id='a'",[]).unwrap();
    drop(conn);
    let sources = f
        .store
        .group_ai_reply_sources("v", "g", "gai_rich")
        .unwrap();
    assert_eq!(sources["sources"][0]["id"], "a");
    assert_eq!(sources["sources"][0]["content"], "SELECTED_A");
    assert_eq!(
        sources["sources"][0]["attachments"][0]["display_name"],
        "Reference"
    );
    assert!(f
        .store
        .group_ai_context_share_draft("v", "g", "gai_rich")
        .is_ok());
    let mut messages = vec![store::FriendGroupMessage {
        id: "gai_rich".into(),
        recalled_at: None,
        ai_reply: None,
    }];
    let conn = f.store.conn().unwrap();
    context::decorate(&conn, &mut messages).unwrap();
    assert_eq!(messages[0].ai_reply.as_ref().unwrap()["source_count"], 2);
    conn.execute(
        "UPDATE friend_group_messages SET recalled_at='now' WHERE id='a'",
        [],
    )
    .unwrap();
    context::decorate(&conn, &mut messages).unwrap();
    assert!(!messages[0]
        .ai_reply
        .as_ref()
        .unwrap()
        .to_string()
        .contains("SELECTED_A"));
    conn.execute(
        "UPDATE friend_group_messages SET recalled_at='now' WHERE id='gai_rich'",
        [],
    )
    .unwrap();
    drop(conn);
    assert!(f
        .store
        .group_ai_reply_sources("u", "g", "gai_rich")
        .is_err());
}

#[test]
fn pre_snapshot_owner_export_is_still_supported_without_expanding_access() {
    let f = Fixture::new();
    let req = f.prepare(&op(), &Fixture::input(&["a"])).unwrap();
    let conn = f.store.conn().unwrap();
    conn.execute(
        "DELETE FROM group_ai_reply_contexts WHERE request_id=?1",
        [&req.id],
    )
    .unwrap();
    conn.execute("INSERT INTO friend_group_messages(id,group_id,sender_user_id,content,created_at) VALUES('gai_old','g','u','Legacy answer','2026-09-20T00:00:00Z')",[]).unwrap();
    conn.execute("UPDATE group_ai_reply_requests SET state='completed',result_message_id='gai_old' WHERE id=?1",[&req.id]).unwrap();
    drop(conn);
    assert!(f
        .store
        .group_ai_context_share_draft("u", "g", "gai_old")
        .is_ok());
    assert!(f
        .store
        .group_ai_context_share_draft("v", "g", "gai_old")
        .is_err());
    assert!(f
        .store
        .set_group_ai_continuation("u", "g", "gai_old", true, 0)
        .is_err());
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET revision=2 WHERE id='a'",
            [],
        )
        .unwrap();
    assert!(f
        .store
        .group_ai_context_share_draft("u", "g", "gai_old")
        .is_err());
}
