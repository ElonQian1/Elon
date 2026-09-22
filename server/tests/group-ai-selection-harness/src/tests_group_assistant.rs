use super::{
    group_assistant::{
        self,
        model::{Share, Sync, Update},
    },
    Store,
};
use rusqlite::Connection;

fn fixture() -> Store {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;
      CREATE TABLE users(id TEXT PRIMARY KEY,nickname TEXT);
      CREATE TABLE friend_groups(id TEXT PRIMARY KEY,owner_user_id TEXT,name TEXT);
      CREATE TABLE friend_group_members(group_id TEXT,user_id TEXT,PRIMARY KEY(group_id,user_id));
      INSERT INTO users VALUES('owner','Owner'),('reader','Reader'),('outsider','Other'),('admin','Admin');
      INSERT INTO friend_groups VALUES('group','admin','Original'),('other','admin','Other');
      INSERT INTO friend_group_members VALUES('group','owner'),('group','reader'),('group','admin'),('other','reader');").unwrap();
    group_assistant::migrate(&conn).unwrap();
    group_assistant::migrate(&conn).unwrap();
    Store {
        connection: std::sync::Mutex::new(conn),
    }
}
fn share() -> Share {
    Share {
        task_id: "task_one".into(),
        account_scope: "a".repeat(64),
        title: "Public topic".into(),
        consent: true,
    }
}
fn update(id: &str) -> Sync {
    Sync { task_id: "task_one".into(), account_scope: "a".repeat(64), state: "ready".into(),
      update: Some(Update { id:id.into(),created_at:"2026-09-01T00:00:00Z".into(),content:"# Result\n**Public** [source](https://example.com/news)\n\n| A | B |\n|---|---|\n|1|2|".into() }) }
}
fn bind(store: &Store) -> String {
    store
        .group_assistant_share("owner", "group", &share())
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn assistant_requires_consent_membership_and_private_scope() {
    let s = fixture();
    let mut body = share();
    body.consent = false;
    assert!(s.group_assistant_share("owner", "group", &body).is_err());
    assert!(s
        .group_assistant_share("outsider", "group", &share())
        .is_err());
    let id = bind(&s);
    let own = s.group_assistant_list("owner", "group").unwrap();
    assert_eq!(own["items"][0]["task_id"], "task_one");
    let view = s.group_assistant_list("reader", "group").unwrap();
    assert!(view["items"][0]["task_id"].is_null());
    assert!(view["items"][0]["account_scope"].is_null());
    assert!(s.group_assistant_list("outsider", "group").is_err());
    assert!(s
        .group_assistant_updates("reader", "other", &id, 0)
        .is_err());
    assert!(s
        .group_assistant_sync("reader", "group", &id, &update("r1"))
        .is_err());
}
#[test]
fn assistant_binding_survives_group_rename_not_account_change() {
    let s = fixture();
    let id = bind(&s);
    assert_eq!(
        s.group_assistant_share("owner", "group", &share()).unwrap()["id"],
        id
    );
    s.conn()
        .unwrap()
        .execute("UPDATE friend_groups SET name='Renamed'", [])
        .unwrap();
    assert_eq!(
        s.group_assistant_list("reader", "group").unwrap()["items"][0]["id"],
        id
    );
    let mut body = update("r1");
    body.account_scope = "b".repeat(64);
    assert!(s
        .group_assistant_sync("owner", "group", &id, &body)
        .is_err());
    body.account_scope = "a".repeat(64);
    body.task_id = "task_other".into();
    assert!(s
        .group_assistant_sync("owner", "group", &id, &body)
        .is_err());
}
#[test]
fn assistant_retries_and_offline_checks_preserve_one_result() {
    let s = fixture();
    let id = bind(&s);
    let mut body = update("r1");
    assert_eq!(
        s.group_assistant_sync("owner", "group", &id, &body)
            .unwrap()["inserted"],
        true
    );
    body.update.as_mut().unwrap().content = "Changed retry".into();
    assert_eq!(
        s.group_assistant_sync("owner", "group", &id, &body)
            .unwrap()["inserted"],
        false
    );
    body.update = None;
    body.state = "unavailable".into();
    s.group_assistant_sync("owner", "group", &id, &body)
        .unwrap();
    let page = s
        .group_assistant_updates("reader", "group", &id, 0)
        .unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert!(page["items"][0]["content"]
        .as_str()
        .unwrap()
        .contains("**Public**"));
}
#[test]
fn assistant_revoke_blocks_late_upload_and_requires_new_consent() {
    let s = fixture();
    let id = bind(&s);
    s.group_assistant_sync("owner", "group", &id, &update("r1"))
        .unwrap();
    assert!(s.group_assistant_revoke("reader", "group", &id).is_err());
    s.group_assistant_revoke("owner", "group", &id).unwrap();
    s.group_assistant_revoke("owner", "group", &id).unwrap();
    assert!(s
        .group_assistant_sync("owner", "group", &id, &update("r2"))
        .is_err());
    assert!(s
        .group_assistant_updates("reader", "group", &id, 0)
        .is_err());
    let next = bind(&s);
    assert_ne!(id, next);
    assert!(s
        .group_assistant_updates("reader", "group", &next, 0)
        .unwrap()["items"]
        .as_array()
        .unwrap()
        .is_empty());
}
#[test]
fn assistant_leaving_and_rejoining_does_not_restore_authorization() {
    let s = fixture();
    let id = bind(&s);
    s.group_assistant_sync("owner", "group", &id, &update("r1"))
        .unwrap();
    s.conn()
        .unwrap()
        .execute(
            "DELETE FROM friend_group_members WHERE group_id='group' AND user_id='owner'",
            [],
        )
        .unwrap();
    s.conn()
        .unwrap()
        .execute(
            "INSERT INTO friend_group_members VALUES('group','owner')",
            [],
        )
        .unwrap();
    assert!(s.group_assistant_list("reader", "group").unwrap()["items"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(s
        .group_assistant_sync("owner", "group", &id, &update("r2"))
        .is_err());
    assert_eq!(
        s.conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM group_assistant_updates", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn assistant_reader_leaving_revokes_access_but_not_owners_shares() {
    let s = fixture();
    let id = bind(&s);
    s.conn()
        .unwrap()
        .execute(
            "DELETE FROM friend_group_members WHERE user_id='reader'",
            [],
        )
        .unwrap();
    assert!(s
        .group_assistant_updates("reader", "group", &id, 0)
        .is_err());
    assert_eq!(
        s.group_assistant_list("owner", "group").unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    s.group_assistant_revoke("admin", "group", &id).unwrap();
}
#[test]
fn assistant_pages_are_stable_and_storage_bounded() {
    let s = fixture();
    let id = bind(&s);
    for i in 0..105 {
        s.group_assistant_sync("owner", "group", &id, &update(&format!("r{i}")))
            .unwrap();
    }
    assert_eq!(
        s.group_assistant_sync("owner", "group", &id, &update("r0"))
            .unwrap()["inserted"],
        false
    );
    let first = s
        .group_assistant_updates("reader", "group", &id, 0)
        .unwrap();
    let before = first["next_cursor"].as_i64().unwrap();
    let second = s
        .group_assistant_updates("reader", "group", &id, before)
        .unwrap();
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    assert_ne!(first["items"][0]["id"], second["items"][0]["id"]);
    assert_eq!(
        s.group_assistant_list("reader", "group").unwrap()["items"][0]["update_count"],
        100
    );
}
#[test]
fn assistant_rejects_credentials_private_urls_and_bad_contracts() {
    let s = fixture();
    let id = bind(&s);
    for text in [
        "https://chatgpt.com/backend-api/files/private",
        "https://example.com/?access_token=abc",
        "https://127.0.0.1/a",
        "Bearer abcdefghijklmnop1234567890123456",
    ] {
        let mut body = update("r1");
        body.update.as_mut().unwrap().content = text.into();
        assert!(
            s.group_assistant_sync("owner", "group", &id, &body)
                .is_err(),
            "{text}"
        );
    }
    let mut body = update("r1");
    body.state = "no_update".into();
    assert!(s
        .group_assistant_sync("owner", "group", &id, &body)
        .is_err());
    assert!(serde_json::from_value::<Share>(serde_json::json!({"task_id":"t","account_scope":"a".repeat(64),"title":"T","consent":true,"cookie":"never"})).is_err());
}
