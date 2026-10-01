use super::{membership::MembershipCommand, roster::RosterQuery, Store};
use rusqlite::{params, Connection};
use serde_json::json;

fn fixture(count: usize) -> Store {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    crate::store_schema::apply_migrations(&conn).unwrap();
    super::membership_schema::migrate(&conn).unwrap();
    for i in 0..count + 3 {
        let id = format!("u{i:04}");
        conn.execute("INSERT INTO users(id,email,password_hash,nickname,status,password_login_enabled,created_at,updated_at)
            VALUES(?1,?2,'',?3,'active',0,'2026-10-01','2026-10-01')",params![id,format!("{id}@example.test"),format!("成员{i:04}")]).unwrap();
    }
    conn.execute("INSERT INTO friend_groups(id,name,owner_user_id,created_at,updated_at) VALUES('g','测试群','u0000','2026-10-01','2026-10-01')",[]).unwrap();
    for i in 0..count {
        conn.execute(
            "INSERT INTO friend_group_members(group_id,user_id,created_at) VALUES('g',?1,?2)",
            params![format!("u{i:04}"), format!("2026-10-01T{:04}", i)],
        )
        .unwrap();
    }
    Store {
        conn: std::sync::Mutex::new(conn),
    }
}

fn action(
    store: &Store,
    actor: &str,
    value: serde_json::Value,
) -> anyhow::Result<super::membership::MembershipReceipt> {
    let mut value = value;
    if value.get("request_id").is_none() {
        value["request_id"] = json!(super::new_id("testaction"));
    }
    store.group_membership_command(
        actor,
        "g",
        &serde_json::from_value::<MembershipCommand>(value).unwrap(),
    )
}

fn friend(store: &Store, actor: &str, user: &str) {
    store.conn().unwrap().execute("INSERT INTO user_friends(user_id,friend_user_id,created_at) VALUES(?1,?2,'2026-10-01')",params![actor,user]).unwrap();
}

#[test]
fn group_roster_paginates_1000_members_searches_beyond_preview_and_binds_cursor() {
    let store = fixture(1000);
    let mut q = RosterQuery::default();
    let mut ids = std::collections::BTreeSet::new();
    loop {
        let page = store.group_roster("u0000", "g", &q).unwrap();
        assert_eq!(page.total_count, 1000);
        for member in page.members {
            assert!(ids.insert(member.id));
        }
        if page.next_cursor.is_none() {
            break;
        }
        q.cursor = page.next_cursor;
    }
    assert_eq!(ids.len(), 1000);
    q.cursor = None;
    q.q = "成员0999".into();
    let page = store.group_roster("u0000", "g", &q).unwrap();
    assert_eq!(page.matched_count, 1);
    assert_eq!(page.members[0].id, "u0999");
    q.q.clear();
    let first = store.group_roster("u0000", "g", &q).unwrap();
    q.cursor = first.next_cursor;
    action(
        &store,
        "u0000",
        json!({"action":"role","user_ids":["u0001"],"role":"admin"}),
    )
    .unwrap();
    assert!(store
        .group_roster("u0000", "g", &q)
        .unwrap_err()
        .to_string()
        .contains("ROSTER_CHANGED"));
}

#[test]
fn group_roster_outsiders_cannot_read_or_mutate_and_admins_cannot_escalate() {
    let store = fixture(5);
    assert!(store
        .group_roster("u0006", "g", &RosterQuery::default())
        .is_err());
    assert!(action(
        &store,
        "u0001",
        json!({"action":"role","user_ids":["u0001"],"role":"admin"})
    )
    .is_err());
    action(
        &store,
        "u0000",
        json!({"action":"role","user_ids":["u0001"],"role":"admin"}),
    )
    .unwrap();
    action(
        &store,
        "u0000",
        json!({"action":"role","user_ids":["u0002"],"role":"admin"}),
    )
    .unwrap();
    assert!(action(
        &store,
        "u0001",
        json!({"action":"remove","user_ids":["u0002"]})
    )
    .is_err());
    assert!(action(
        &store,
        "u0001",
        json!({"action":"transfer","user_ids":["u0003"]})
    )
    .is_err());
    assert!(action(
        &store,
        "u0001",
        json!({"action":"remove","user_ids":["u0000"]})
    )
    .is_err());
    action(
        &store,
        "u0001",
        json!({"action":"remove","user_ids":["u0003"]}),
    )
    .unwrap();
    assert!(store
        .group_roster("u0003", "g", &RosterQuery::default())
        .is_err());
}

#[test]
fn membership_batch_is_atomic_and_retries_do_not_duplicate_notices() {
    let store = fixture(5);
    let command =
        json!({"request_id":"remove-once","action":"remove","user_ids":["u0001","u0000"]});
    assert!(action(&store, "u0000", command).is_err());
    assert_eq!(
        store
            .group_roster("u0000", "g", &RosterQuery::default())
            .unwrap()
            .total_count,
        5
    );
    let command = json!({"request_id":"remove-once","action":"remove","user_ids":["u0001"]});
    action(&store, "u0000", command.clone()).unwrap();
    action(&store, "u0000", command).unwrap();
    let count: i64 = store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM friend_group_messages WHERE group_id='g'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert!(action(
        &store,
        "u0000",
        json!({"request_id":"remove-once","action":"remove","user_ids":["u0002"]})
    )
    .is_err());
}

#[test]
fn membership_owner_transfer_leave_and_dissolve_preserve_authority() {
    let store = fixture(3);
    assert!(action(&store, "u0000", json!({"action":"leave"})).is_err());
    action(
        &store,
        "u0000",
        json!({"action":"transfer","user_ids":["u0001"]}),
    )
    .unwrap();
    let page = store
        .group_roster("u0001", "g", &RosterQuery::default())
        .unwrap();
    assert_eq!(page.viewer_role, "owner");
    let leave = json!({"action":"leave","request_id":"leave-retry"});
    assert!(action(&store, "u0000", leave.clone()).unwrap().exited);
    assert!(action(&store, "u0000", leave).unwrap().exited);
    assert!(action(&store, "u0002", json!({"action":"dissolve"})).is_err());
    action(&store, "u0001", json!({"action":"dissolve"})).unwrap();
    assert!(store
        .group_roster("u0002", "g", &RosterQuery::default())
        .is_err());
    assert!(store.list_friend_groups("u0002").unwrap().is_empty());
}

#[test]
fn membership_invitation_rules_apply_to_legacy_and_approval_flows() {
    let store = fixture(3);
    friend(&store, "u0001", "u0003");
    action(
        &store,
        "u0000",
        json!({"action":"policy","invitation_policy":"admins"}),
    )
    .unwrap();
    assert!(store
        .add_group_members("u0001", "g", &["u0003".into()])
        .is_err());
    action(
        &store,
        "u0000",
        json!({"action":"policy","invitation_policy":"approval"}),
    )
    .unwrap();
    assert!(store
        .add_group_members("u0001", "g", &["u0003".into()])
        .is_err());
    assert_eq!(
        action(
            &store,
            "u0001",
            json!({"action":"invite","user_ids":["u0003"]})
        )
        .unwrap()
        .changed,
        0
    );
    assert!(store.group_pending_invitations("u0001", "g", 0).is_err());
    let pending = store.group_pending_invitations("u0000", "g", 0).unwrap();
    let id = pending["requests"][0]["id"].as_str().unwrap();
    action(
        &store,
        "u0000",
        json!({"action":"approve","invitation_id":id}),
    )
    .unwrap();
    assert_eq!(
        store
            .group_roster("u0003", "g", &RosterQuery::default())
            .unwrap()
            .total_count,
        4
    );
    assert_eq!(
        store.group_pending_invitations("u0000", "g", 0).unwrap()["total_count"],
        0
    );
}

#[test]
fn membership_invites_do_not_partially_apply_or_repeat_existing_people() {
    let store = fixture(3);
    friend(&store, "u0001", "u0003");
    assert!(action(
        &store,
        "u0001",
        json!({"action":"invite","user_ids":["u0003","u0004"]})
    )
    .is_err());
    assert_eq!(
        store
            .group_roster("u0000", "g", &RosterQuery::default())
            .unwrap()
            .total_count,
        3
    );
    action(
        &store,
        "u0001",
        json!({"action":"invite","user_ids":["u0003","u0003"]}),
    )
    .unwrap();
    assert_eq!(
        action(
            &store,
            "u0001",
            json!({"action":"invite","user_ids":["u0003"]})
        )
        .unwrap()
        .changed,
        0
    );
    assert_eq!(
        store
            .group_roster("u0000", "g", &RosterQuery::default())
            .unwrap()
            .total_count,
        4
    );
}
