use super::*;

#[test]
fn text_login_accounts_are_found_even_when_the_display_name_differs() {
    let conn = fixture();
    conn.execute_batch(
        "INSERT INTO users(id,phone,nickname) VALUES
         ('usr_login','tester1999','Display Name'),
         ('usr_same_name','another-account','tester1999');",
    )
    .unwrap();
    for query in ["tester1999", "TESTER1999", "  tester1999  "] {
        let found = search_text_candidates(&conn, "viewer", query).unwrap();
        assert_eq!(found.results.len(), 1);
        assert_eq!(
            found.results[0].id, "usr_login",
            "login identity takes priority over another user's nickname"
        );
        assert_eq!(found.results[0].nickname, "Display Name");
        assert_eq!(
            text_login_account_id(&conn, query).unwrap().as_deref(),
            Some("usr_login")
        );
        assert!(!serde_json::to_string(&found)
            .unwrap()
            .contains("tester1999"));
    }
    let explicit_name =
        search_candidates(&conn, "viewer", CandidateField::Nickname, "tester1999").unwrap();
    assert_eq!(explicit_name.results[0].id, "usr_same_name");
}

#[test]
fn text_account_misses_fall_back_to_nicknames_without_partial_account_matches() {
    let conn = fixture();
    assert_eq!(
        search_text_candidates(&conn, "usr_self", "同名用户")
            .unwrap()
            .results
            .len(),
        3
    );
    conn.execute(
        "INSERT INTO users(id,phone,nickname) VALUES ('usr_short', 'other-login', 'ab')",
        [],
    )
    .unwrap();
    assert_eq!(
        search_text_candidates(&conn, "viewer", "ab")
            .unwrap()
            .results[0]
            .id,
        "usr_short"
    );
    assert!(search_text_candidates(&conn, "viewer", "testacc")
        .unwrap()
        .results
        .is_empty());
    assert!(search_text_candidates(&conn, "viewer", "missing1999")
        .unwrap()
        .results
        .is_empty());
}

#[test]
fn inactive_and_device_login_accounts_stay_hidden_and_search_does_not_add_friends() {
    let conn = fixture();
    conn.execute(
        "UPDATE users SET phone='inactive1999' WHERE id='usr_inactive'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE users SET phone='device1999' WHERE id='usr_device'",
        [],
    )
    .unwrap();
    for query in ["inactive1999", "device1999"] {
        assert!(text_login_account_id(&conn, query).unwrap().is_none());
        assert!(search_text_candidates(&conn, "viewer", query)
            .unwrap()
            .results
            .is_empty());
    }
    let found = search_text_candidates(&conn, "usr_self", "testaccount").unwrap();
    assert!(found.results[0].already_friend);
    let own = search_text_candidates(&conn, "usr_old", "testaccount").unwrap();
    assert!(own.results[0].is_self);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM user_friends", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

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
