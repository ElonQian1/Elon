use super::super::v2;
use super::*;

#[test]
fn message_timeline_reading_upgrades_existing_v310_database() {
    let s = super::super::tests::store(10);
    {
        let conn = s.conn().unwrap();
        conn.execute_batch(
            "DROP TABLE reading_bookmarks; DROP TABLE reading_progress;
            DROP TABLE reading_candidates; DROP TABLE reading_operations;
            CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
            INSERT INTO schema_migrations VALUES(310, '2026-10-01T00:00:00Z');",
        )
        .unwrap();
        crate::store_schema::apply_migrations(&conn).unwrap();
        crate::store_schema::apply_migrations(&conn).unwrap();
        let applied: i64 = conn
            .query_row(
                "SELECT count(*) FROM schema_migrations WHERE version=311",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applied, 1);
        let retained: i64 = conn
            .query_row("SELECT count(*) FROM friend_group_messages", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(retained, 10);
    }
    s.reading_command("a", &command("create", "upgrade-bookmark", 1, "m000005"))
        .unwrap();
    assert_eq!(
        s.reading_list("a", &scope(), "").unwrap()["bookmarks"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

fn scope() -> Scope {
    Scope {
        kind: "group".into(),
        id: "g".into(),
        project: String::new(),
    }
}
fn command(action: &str, bookmark: &str, seq: i64, message: &str) -> Command {
    Command {
        scope: scope(),
        action: action.into(),
        bookmark_id: bookmark.into(),
        operation_id: format!("operation-{action}-{bookmark}-{seq}"),
        position: Position {
            message_id: message.into(),
            ..Default::default()
        },
        device_id: "device-one".into(),
        device_seq: seq,
        ..Default::default()
    }
}
#[test]
fn message_timeline_reading_direct_seek_bidirectional_and_scope() {
    let s = super::super::tests::store(100_000);
    let mut r = v2::Request {
        scope: scope(),
        around: Some("m050000".into()),
        ..Default::default()
    };
    let p = s.read_message_timeline_v2("a", &r).unwrap();
    assert_eq!(p["messages"].as_array().unwrap().len(), 50);
    assert_eq!(p["messages"][24]["id"], "m050000");
    assert_eq!(p["messages"][0]["id"], "m049976");
    r.around = None;
    r.after = p["after"].as_str().map(str::to_owned);
    let next = s.read_message_timeline_v2("a", &r).unwrap();
    assert_eq!(next["messages"][0]["id"], "m050026");
    r.before = next["before"].as_str().map(str::to_owned);
    r.after = None;
    let previous = s.read_message_timeline_v2("a", &r).unwrap();
    assert_eq!(previous["messages"], p["messages"]);
    assert!(s.read_message_timeline_v2("outsider", &r).is_err());
    r.after = r.before.take();
    assert!(s.read_message_timeline_v2("a", &r).is_err());
}
#[test]
fn message_timeline_reading_bookmarks_independent_progress_and_fixed_anchor() {
    let s = super::super::tests::store(1000);
    for (id, anchor, current) in [("bookmark-a", 10, 20), ("bookmark-b", 100, 110)] {
        let create = command("create", id, 1, &format!("m{anchor:06}"));
        let first = s.reading_command("a", &create).unwrap();
        assert_eq!(s.reading_command("a", &create).unwrap(), first);
        let progress = command("progress", id, 2, &format!("m{current:06}"));
        s.reading_command("a", &progress).unwrap();
        let r = v2::Request {
            scope: scope(),
            bookmark: Some(id.into()),
            resume: true,
            ..Default::default()
        };
        let p = s.read_message_timeline_v2("a", &r).unwrap();
        assert_eq!(p["target"]["resolved_id"], format!("m{current:06}"));
        let r = v2::Request { resume: false, ..r };
        assert_eq!(
            s.read_message_timeline_v2("a", &r).unwrap()["target"]["resolved_id"],
            format!("m{anchor:06}")
        );
    }
    assert_eq!(
        s.reading_list("a", &scope(), "").unwrap()["bookmarks"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(s.reading_list("b", &scope(), "").unwrap()["bookmarks"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(s
        .reading_command("b", &command("progress", "bookmark-a", 8, "m000030"))
        .is_err());
}
#[test]
fn message_timeline_reading_conflict_preserves_both_positions_and_user_backtracking() {
    let s = super::super::tests::store(1000);
    s.reading_command("a", &command("create", "bookmark-a", 1, "m000010"))
        .unwrap();
    s.reading_command("a", &command("progress", "bookmark-a", 2, "m000100"))
        .unwrap();
    let mut other = command("progress", "bookmark-a", 3, "m000050");
    other.device_id = "device-two".into();
    let conflict = s.reading_command("a", &other).unwrap();
    assert_eq!(conflict["conflict"], true);
    assert_eq!(conflict["progress"]["position"]["message_id"], "m000100");
    assert_eq!(
        conflict["candidates"][0]["position"]["message_id"],
        "m000050"
    );
    let mut resolve = command("resolve", "bookmark-a", 4, "m000050");
    resolve.device_id = "device-two".into();
    resolve.base_revision = 1;
    let resolved = s.reading_command("a", &resolve).unwrap();
    assert_eq!(resolved["progress"]["position"]["message_id"], "m000050");
    assert_eq!(resolved["progress"]["furthest"]["message_id"], "m000100");
    let saved = s.reading_list("a", &scope(), "").unwrap();
    assert!(saved["bookmarks"][0]["candidates"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!saved["bookmarks"][0]["previous_positions"]
        .as_array()
        .unwrap()
        .is_empty());
}
#[test]
fn message_timeline_reading_deleted_anchor_nearby_and_deleted_bookmark_never_revives() {
    let s = super::super::tests::store(1000);
    let create = command("create", "bookmark-a", 1, "m000010");
    s.reading_command("a", &create).unwrap();
    s.conn()
        .unwrap()
        .execute("DELETE FROM friend_group_messages WHERE id='m000010'", [])
        .unwrap();
    let r = v2::Request {
        scope: scope(),
        bookmark: Some("bookmark-a".into()),
        ..Default::default()
    };
    let page = s.read_message_timeline_v2("a", &r).unwrap();
    assert_eq!(page["target"]["resolved_id"], "m000011");
    assert_eq!(page["target"]["status"], "nearby");
    let mut delete = command("delete", "bookmark-a", 2, "");
    delete.base_revision = 1;
    s.reading_command("a", &delete).unwrap();
    assert!(s.reading_command("a", &create).is_err());
    assert!(s
        .reading_command("a", &command("progress", "bookmark-a", 3, "m000020"))
        .is_err());
    assert!(s.reading_list("a", &scope(), "").unwrap()["bookmarks"]
        .as_array()
        .unwrap()
        .is_empty());
}
#[test]
fn message_timeline_reading_revoked_access_and_operation_reuse() {
    let s = super::super::tests::store(30);
    let mut create = command("create", "bookmark-a", 1, "m000010");
    s.reading_command("a", &create).unwrap();
    create.title = "different".into();
    assert!(s.reading_command("a", &create).is_err());
    s.conn()
        .unwrap()
        .execute("DELETE FROM friend_group_members WHERE user_id='a'", [])
        .unwrap();
    assert!(s.reading_list("a", &scope(), "").is_err());
    assert!(s
        .reading_command("a", &command("progress", "bookmark-a", 2, "m000020"))
        .is_err());
}
