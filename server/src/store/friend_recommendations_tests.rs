use super::*;

fn fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE users(id TEXT PRIMARY KEY, phone TEXT, email TEXT, nickname TEXT,
           avatar_data_url TEXT, status TEXT NOT NULL DEFAULT 'active',
           password_hash TEXT NOT NULL DEFAULT 'registered', created_at TEXT NOT NULL DEFAULT '2026-01-01');
         CREATE TABLE user_friends(user_id TEXT, friend_user_id TEXT);
         CREATE TABLE user_presence_settings(user_id TEXT PRIMARY KEY, status TEXT);
         INSERT INTO users(id,nickname) VALUES ('viewer','Viewer'),('social_ai','AI');",
    ).unwrap();
    conn
}

fn list(conn: &Connection, online: &[&str]) -> Vec<FriendRecommendation> {
    list_recommendations(
        conn,
        "viewer",
        "social_ai",
        &online.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
    )
    .unwrap()
}

#[test]
fn online_strangers_precede_offline_users_without_typing() {
    let conn = fixture();
    conn.execute_batch("INSERT INTO users(id,nickname) VALUES ('offline','AAA'),('online','ZZZ');")
        .unwrap();
    let rows = list(&conn, &["online"]);
    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        ["online", "offline"]
    );
    assert!(rows[0].is_online);
    assert!(!rows[0].already_friend);
    assert!(!rows[1].is_online);
}

#[test]
fn presence_snapshot_honors_invisible_and_disconnects() {
    let conn = fixture();
    conn.execute_batch(
        "INSERT INTO users(id,nickname) VALUES
      ('hidden','Hidden'),('idle','Idle'),('busy','Busy'),('away','Away');
      INSERT INTO user_presence_settings(user_id,status) VALUES
      ('hidden','invisible'),('idle','idle'),('busy','dnd'),('away','online');",
    )
    .unwrap();
    let rows = list(&conn, &["hidden", "idle", "busy"]);
    for row in rows {
        assert_eq!(
            row.is_online,
            row.id == "idle" || row.id == "busy",
            "{}",
            row.id
        );
    }
    assert!(list(&conn, &[]).iter().all(|row| !row.is_online));
}

#[test]
fn ranking_happens_before_candidate_cap() {
    let conn = fixture();
    for index in 0..501 {
        conn.execute(
            "INSERT INTO users(id,nickname) VALUES (?1,?2)",
            params![format!("offline_{index:03}"), format!("A{index:03}")],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO users(id,nickname) VALUES ('last_online','ZZZ')",
        [],
    )
    .unwrap();
    let rows = list(&conn, &["last_online"]);
    assert_eq!(rows.len(), 500);
    assert_eq!(rows[0].id, "last_online");
    assert!(rows[0].is_online);
}

#[test]
fn excludes_self_ai_devices_and_inactive_accounts_even_when_connected() {
    let conn = fixture();
    conn.execute_batch(
        "INSERT INTO users(id,status,password_hash) VALUES
      ('device','active','device-user'),('inactive','disabled','registered'),
      ('valid','active','registered');",
    )
    .unwrap();
    let rows = list(
        &conn,
        &["viewer", "social_ai", "device", "inactive", "valid"],
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "valid");
}

#[test]
fn relationship_and_mutual_friend_context_remain_stable() {
    let conn = fixture();
    conn.execute_batch(
        "INSERT INTO users(id,nickname) VALUES
      ('friend','AAA'),('plain','BBB'),('mutual','ZZZ');
      INSERT INTO user_friends(user_id,friend_user_id) VALUES
      ('viewer','friend'),('friend','viewer'),('mutual','friend');",
    )
    .unwrap();
    let rows = list(&conn, &["friend", "plain", "mutual"]);
    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        ["mutual", "plain", "friend"]
    );
    assert_eq!(rows[0].mutual_friend_count, 1);
    assert!(rows[2].already_friend);
    assert!(rows.iter().all(|row| row.is_online));
}

#[test]
fn online_identifiers_are_bound_as_data_and_results_are_deterministic() {
    let conn = fixture();
    conn.execute_batch("INSERT INTO users(id,nickname) VALUES ('z','Same'),('a','Same');")
        .unwrap();
    let rows = list(&conn, &["x') OR 1=1 --"]);
    assert!(rows.iter().all(|row| !row.is_online));
    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        ["a", "z"]
    );
    let json = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(json["is_online"], false);
    assert!(json.get("password_hash").is_none());
}

#[test]
fn recommendation_payload_masks_phone_email_and_login_accounts() {
    let conn = fixture();
    conn.execute_batch(
        "INSERT INTO users(id,phone,email,nickname) VALUES
      ('phone','13900009650',NULL,'Phone User'),
      ('email',NULL,'private@personal.example','Email User'),
      ('login','private_login',NULL,'Login User');",
    )
    .unwrap();
    let rows = list(&conn, &[]);
    let wire = serde_json::to_string(&rows).unwrap();
    for secret in [
        "13900009650",
        "private@personal.example",
        "personal.example",
        "private_login",
    ] {
        assert!(!wire.contains(secret), "unmasked account leaked: {secret}");
    }
    for row in rows {
        assert_eq!(row.account, row.account_hint);
        assert!(row.phone.is_none());
        assert!(row.nickname.unwrap().ends_with("User"));
        if row.id == "phone" {
            assert_eq!(row.account, "手机尾号 9650");
        }
    }
}

#[test]
fn account_fallback_and_account_duplicated_as_nickname_are_also_masked() {
    let conn = fixture();
    conn.execute_batch(
        "INSERT INTO users(id,phone,email,nickname) VALUES
      ('phone','13900009650',NULL,'13900009650'),
      ('email',NULL,'private@personal.example','PRIVATE@PERSONAL.EXAMPLE'),
      ('blank',NULL,'blank@personal.example',' '),
      ('missing','13900001111',NULL,NULL);",
    )
    .unwrap();
    for row in list(&conn, &[]) {
        assert_eq!(row.nickname.as_deref(), Some(row.account_hint.as_str()));
        assert!(row.phone.is_none());
    }
}
