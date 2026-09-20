use super::*;
use rusqlite::params;

struct Fixture {
    store: Store,
}
impl Fixture {
    fn new() -> Self {
        let store = Store {
            path: std::env::temp_dir().join(format!("group-selection-{}.db", uuid::Uuid::new_v4())),
        };
        let conn = store.conn().unwrap();
        conn.execute_batch("CREATE TABLE users(id TEXT PRIMARY KEY,nickname TEXT,email TEXT);
            CREATE TABLE friend_groups(id TEXT PRIMARY KEY);
            CREATE TABLE friend_group_members(group_id TEXT,user_id TEXT);
            CREATE TABLE friend_group_messages(id TEXT PRIMARY KEY,group_id TEXT,sender_user_id TEXT,
                content TEXT,created_at TEXT,recalled_at TEXT,revision INTEGER NOT NULL DEFAULT 1);
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
        Self { store }
    }
    fn input(ids: &[&str]) -> selection::GroupAiSelection {
        selection::GroupAiSelection {
            message_ids: ids.iter().map(|s| s.to_string()).collect(),
            message_revisions: ids.iter().map(|s| (s.to_string(), 1)).collect(),
            question: "compare".into(),
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
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.store.path);
    }
}
fn op() -> String {
    uuid::Uuid::new_v4().to_string()
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
fn reply_share_contains_only_selection_and_requires_owner_and_unchanged_sources() {
    let f = Fixture::new();
    let operation = op();
    let req = f.prepare(&operation, &Fixture::input(&["a", "c"])).unwrap();
    let conn = f.store.conn().unwrap();
    conn.execute("INSERT INTO friend_group_messages(id,group_id,sender_user_id,content,created_at) VALUES('gai_reply','g','u','**ANSWER**','2026-09-20T00:00:00Z')",[]).unwrap();
    conn.execute("UPDATE group_ai_reply_requests SET state='completed',result_message_id='gai_reply' WHERE id=?1",[&req.id]).unwrap();
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
    conn.execute(
        "UPDATE friend_group_messages SET revision=2 WHERE id='c'",
        [],
    )
    .unwrap();
    assert!(f
        .store
        .group_ai_context_share_draft("u", "g", "gai_reply")
        .is_err());
}
