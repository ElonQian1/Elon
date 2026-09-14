use super::*;

fn fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE users(id TEXT PRIMARY KEY, phone TEXT, email TEXT, nickname TEXT,
         avatar_data_url TEXT, status TEXT DEFAULT 'active', password_hash TEXT DEFAULT 'hash',
         created_at TEXT DEFAULT '2026-01-01');
         CREATE TABLE user_friends(user_id TEXT, friend_user_id TEXT, created_at TEXT);
         INSERT INTO users(id,phone,nickname,avatar_data_url) VALUES
          ('usr_old','testaccount','同名用户','data:image/png;base64,old'),
          ('usr_target','13900009650','同名用户','data:image/png;base64,target'),
          ('usr_self','13900001111','同名用户',NULL);
         INSERT INTO users(id,email,nickname) VALUES ('usr_email','private@example.test','邮箱用户');
         INSERT INTO users(id,nickname,status) VALUES ('usr_inactive','同名用户','disabled');
         INSERT INTO users(id,nickname,password_hash) VALUES ('usr_device','同名用户','device-user');
         INSERT INTO user_friends VALUES ('usr_self','usr_old','2026-01-02');"
    ).unwrap();
    conn
}

#[test]
fn duplicate_names_return_distinct_ids_with_masked_accounts_and_per_row_status() {
    let conn = fixture();
    let found = search_candidates(&conn, "usr_self", CandidateField::Nickname, "同名用户").unwrap();
    assert_eq!(found.results.len(), 3);
    assert!(!found.has_more);
    let target = found
        .results
        .iter()
        .find(|item| item.id == "usr_target")
        .unwrap();
    assert_eq!(target.account_hint, "手机尾号 9650");
    assert_eq!(
        target.avatar_data_url.as_deref(),
        Some("data:image/png;base64,target")
    );
    assert!(!target.is_self && !target.already_friend);
    assert!(
        found
            .results
            .iter()
            .find(|item| item.id == "usr_self")
            .unwrap()
            .is_self
    );
    assert!(
        found
            .results
            .iter()
            .find(|item| item.id == "usr_old")
            .unwrap()
            .already_friend
    );
    let json = serde_json::to_string(&found).unwrap();
    assert!(!json.contains("13900009650") && !json.contains("testaccount"));
    assert!(
        !json.contains("\"phone\"") && !json.contains("\"account\"") && !json.contains("\"email\"")
    );
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM user_friends", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "search must never create a relationship");
}

#[test]
fn phone_email_and_selected_account_id_are_exact_and_private() {
    let conn = fixture();
    for (field, query, id) in [
        (CandidateField::Phone, "13900009650", "usr_target"),
        (CandidateField::AccountId, "usr_target", "usr_target"),
        (CandidateField::Email, "private@example.test", "usr_email"),
    ] {
        let found = search_candidates(&conn, "usr_self", field, query).unwrap();
        assert_eq!(found.results.len(), 1);
        assert_eq!(found.results[0].id, id);
        assert!(!serde_json::to_string(&found)
            .unwrap()
            .contains("private@example.test"));
    }
    for (field, query) in [
        (CandidateField::Phone, "9650"),
        (CandidateField::Nickname, "同名"),
        (CandidateField::Nickname, "%' OR 1=1 --"),
    ] {
        assert!(search_candidates(&conn, "usr_self", field, query)
            .unwrap()
            .results
            .is_empty());
    }
    assert!(search_candidates(&conn, "usr_self", CandidateField::Nickname, "同").is_err());
}

#[test]
fn candidate_limit_is_explicit_and_order_is_stable() {
    let conn = fixture();
    for index in 0..25 {
        conn.execute(
            "INSERT INTO users(id,nickname) VALUES (?1,'更多同名')",
            [format!("usr_{index:03}")],
        )
        .unwrap();
    }
    let found = search_candidates(&conn, "viewer", CandidateField::Nickname, "更多同名").unwrap();
    assert!(found.has_more);
    assert_eq!(found.results.len(), 20);
    assert_eq!(found.results[0].id, "usr_000");
    assert_eq!(found.results[19].id, "usr_019");
}

#[test]
fn unicode_short_and_missing_identifiers_are_safe() {
    assert_eq!(account_hint(Some("昵称账号"), None, "id"), "账号 昵***号");
    assert_eq!(account_hint(Some("x"), None, "id"), "账号 x***");
    assert_eq!(account_hint(None, None, "usr_unknown"), "账号 u***n");
}
