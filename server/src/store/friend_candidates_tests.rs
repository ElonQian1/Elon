use super::*;

#[test]
fn short_and_identifier_shaped_nicknames_are_searchable_ignoring_ascii_case() {
    let conn = fixture();
    for (index, nickname) in ["Q@", "Alias@Home", "USR_ALIAS", "12", "7", "中", "A"]
        .iter()
        .enumerate()
    {
        let id = format!("usr_nickname_{index}");
        conn.execute(
            "INSERT INTO users(id,nickname) VALUES (?1,?2)",
            params![id, nickname],
        )
        .unwrap();
        for query in [
            nickname.to_string(),
            nickname.to_ascii_lowercase(),
            format!(" {nickname} "),
        ] {
            let found = search_auto_candidates(&conn, "viewer", &query).unwrap();
            assert_eq!(
                found.results.len(),
                1,
                "nickname {query:?} must not be rejected as an account"
            );
            assert_eq!(found.results[0].id, id);
            assert_eq!(found.results[0].nickname, *nickname);
        }
    }
}

#[test]
fn real_identifiers_take_priority_and_formatted_phones_remain_exact() {
    let conn = fixture();
    conn.execute_batch(
        "INSERT INTO users(id,phone,nickname) VALUES
         ('usr_name_email',NULL,'PRIVATE@EXAMPLE.TEST'),
         ('usr_name_id',NULL,'USR_TARGET'),
         ('usr_name_phone',NULL,'13900009650'),
         ('usr_prefixed_login','usr_customer','Login Display'),
         ('usr_compacted_login','alphabeta','Compact Display'),
         ('usr_dash_nickname',NULL,'Alpha-Beta');",
    )
    .unwrap();
    for (query, id) in [
        ("PRIVATE@EXAMPLE.TEST", "usr_email"),
        ("USR_TARGET", "usr_target"),
        ("13900009650", "usr_target"),
        ("(139) 0000-9650", "usr_target"),
        ("USR_CUSTOMER", "usr_prefixed_login"),
        ("alpha-beta", "usr_dash_nickname"),
    ] {
        let found = search_auto_candidates(&conn, "viewer", query).unwrap();
        assert_eq!(found.results.len(), 1, "query {query}");
        assert_eq!(found.results[0].id, id, "query {query}");
    }
}

#[test]
fn case_variants_return_separate_candidates_and_empty_input_never_enumerates() {
    let conn = fixture();
    conn.execute_batch(
        "INSERT INTO users(id,nickname) VALUES ('usr_upper','Q@'),('usr_lower','q@');",
    )
    .unwrap();
    let found = search_auto_candidates(&conn, "usr_upper", "q@").unwrap();
    assert_eq!(found.results.len(), 2);
    assert!(found
        .results
        .iter()
        .any(|item| item.id == "usr_upper" && item.is_self));
    assert!(found
        .results
        .iter()
        .any(|item| item.id == "usr_lower" && !item.is_self));
    let explicit = search_candidates(&conn, "viewer", CandidateField::Nickname, "q@").unwrap();
    assert_eq!(explicit.results.len(), 2);
    assert!(search_auto_candidates(&conn, "viewer", "").is_err());
    assert!(search_auto_candidates(&conn, "viewer", "  ").is_err());
    assert!(
        search_auto_candidates(&conn, "viewer", "q")
            .unwrap()
            .results
            .is_empty(),
        "nickname matches must remain exact"
    );
}

#[test]
fn auto_identifier_resolution_never_exposes_inactive_or_device_accounts() {
    let conn = fixture();
    conn.execute(
        "UPDATE users SET email='inactive@example.test' WHERE id='usr_inactive'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE users SET email='device@example.test' WHERE id='usr_device'",
        [],
    )
    .unwrap();
    for query in [
        "inactive@example.test",
        "device@example.test",
        "usr_inactive",
        "usr_device",
    ] {
        assert!(auto_account_id(&conn, query).unwrap().is_none());
        assert!(search_auto_candidates(&conn, "viewer", query)
            .unwrap()
            .results
            .is_empty());
    }
}

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
        let found = search_auto_candidates(&conn, "viewer", query).unwrap();
        assert_eq!(found.results.len(), 1);
        assert_eq!(
            found.results[0].id, "usr_login",
            "login identity takes priority over another user's nickname"
        );
        assert_eq!(found.results[0].nickname, "Display Name");
        assert_eq!(
            auto_account_id(&conn, query).unwrap().as_deref(),
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
        search_auto_candidates(&conn, "usr_self", "同名用户")
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
        search_auto_candidates(&conn, "viewer", "ab")
            .unwrap()
            .results[0]
            .id,
        "usr_short"
    );
    assert!(search_auto_candidates(&conn, "viewer", "testacc")
        .unwrap()
        .results
        .is_empty());
    assert!(search_auto_candidates(&conn, "viewer", "missing1999")
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
        assert!(auto_account_id(&conn, query).unwrap().is_none());
        assert!(search_auto_candidates(&conn, "viewer", query)
            .unwrap()
            .results
            .is_empty());
    }
    let found = search_auto_candidates(&conn, "usr_self", "testaccount").unwrap();
    assert!(found.results[0].already_friend);
    let own = search_auto_candidates(&conn, "usr_old", "testaccount").unwrap();
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
    assert!(search_candidates(&conn, "usr_self", CandidateField::Nickname, "").is_err());
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
