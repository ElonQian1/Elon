use super::*;

fn seed(conn: &Connection) {
    conn.execute_batch(
        r#"CREATE TABLE friend_group_members(group_id TEXT, user_id TEXT);
         INSERT INTO friend_group_members VALUES ('g', 'author'), ('g', 'reader'), ('other', 'reader');
         CREATE TABLE friend_group_messages(
           id TEXT PRIMARY KEY, group_id TEXT, sender_user_id TEXT, content TEXT,
           created_at TEXT, recalled_at TEXT, attachments_json TEXT);
         INSERT INTO friend_group_messages VALUES
           ('m', 'g', 'author', '明天八点开会
请带材料 🐲', '2026-09-01T08:00:00Z', NULL, '[{"kind":"image","url":"/original.png"}]');"#,
    ).unwrap();
    migrate(conn);
}

fn migrate(conn: &Connection) {
    let migration = crate::store_migrations::MIGRATIONS
        .iter()
        .find(|entry| entry.1 == "群聊消息追加式修订历史")
        .unwrap();
    (migration.2)(conn).unwrap();
}

fn fixture() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    seed(&conn);
    conn
}

#[test]
fn concurrent_devices_cannot_overwrite_each_other() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "elon-revisions-{}-{stamp}.sqlite",
        std::process::id()
    ));
    let conn = Connection::open(&path).unwrap();
    seed(&conn);
    drop(conn);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = ["设备一", "设备二"]
        .into_iter()
        .map(|text| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut conn = Connection::open(path).unwrap();
                conn.busy_timeout(std::time::Duration::from_secs(10))
                    .unwrap();
                barrier.wait();
                edit(&mut conn, "author", "g", "m", 1, text)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| r
                .as_ref()
                .err()
                .and_then(|e| e.downcast_ref::<RevisionError>())
                == Some(&RevisionError::Conflict))
            .count(),
        1
    );
    let conn = Connection::open(&path).unwrap();
    let versions = history(&conn, "reader", "g", "m", None, 50).unwrap();
    assert_eq!(versions.current_revision, 2);
    assert_eq!(versions.revisions.len(), 2);
    drop(conn);
    std::fs::remove_file(path).unwrap();
}

fn assert_error<T: std::fmt::Debug>(result: Result<T>, expected: RevisionError) {
    assert_eq!(
        result.unwrap_err().downcast_ref::<RevisionError>(),
        Some(&expected)
    );
}

#[test]
fn edits_keep_all_versions_original_time_and_attachments() {
    let mut conn = fixture();
    let original = history(&conn, "reader", "g", "m", None, 50).unwrap();
    assert_eq!(original.current_revision, 1);
    let second = edit(
        &mut conn,
        "author",
        "g",
        "m",
        1,
        "明天九点开会\n请带材料 🐲",
    )
    .unwrap();
    assert_eq!(second.revision, 2);
    let third = edit(&mut conn, "author", "g", "m", 2, "后天九点开会").unwrap();
    assert_eq!(third.revision, 3);
    let page = history(&conn, "reader", "g", "m", None, 2).unwrap();
    assert_eq!(
        page.revisions
            .iter()
            .map(|r| r.revision)
            .collect::<Vec<_>>(),
        vec![3, 2]
    );
    let last = history(&conn, "reader", "g", "m", page.next_before_revision, 2).unwrap();
    assert_eq!(last.revisions[0].content, original.revisions[0].content);
    assert_eq!(last.revisions[0].created_at, "2026-09-01T08:00:00Z");
    assert!(last.next_before_revision.is_none());
    let stored: (String, String, String) = conn.query_row(
        "SELECT content, created_at, attachments_json FROM friend_group_messages WHERE id = 'm'", [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).unwrap();
    assert_eq!(stored.0, third.content);
    assert_eq!(stored.1, "2026-09-01T08:00:00Z");
    assert!(stored.2.contains("/original.png"));
}

#[test]
fn permissions_apply_to_history_and_edits_in_the_same_group() {
    let mut conn = fixture();
    assert_error(
        edit(&mut conn, "reader", "g", "m", 1, "他人修改"),
        RevisionError::Forbidden,
    );
    assert_error(
        edit(&mut conn, "outsider", "g", "m", 1, "越权修改"),
        RevisionError::Forbidden,
    );
    assert_error(
        history(&conn, "outsider", "g", "m", None, 50),
        RevisionError::Forbidden,
    );
    assert_error(
        history(&conn, "reader", "other", "m", None, 50),
        RevisionError::NotFound,
    );
    conn.execute(
        "DELETE FROM friend_group_members WHERE user_id = 'reader'",
        [],
    )
    .unwrap();
    assert_error(
        history(&conn, "reader", "g", "m", None, 50),
        RevisionError::Forbidden,
    );
}

#[test]
fn retries_do_not_duplicate_history_and_stale_edits_cannot_overwrite() {
    let mut conn = fixture();
    edit(&mut conn, "author", "g", "m", 1, "新版本").unwrap();
    let retry = edit(&mut conn, "author", "g", "m", 1, "新版本").unwrap();
    assert!(!retry.changed);
    assert_eq!(retry.revision, 2);
    assert_error(
        edit(&mut conn, "author", "g", "m", 1, "另一设备的旧草稿"),
        RevisionError::Conflict,
    );
    assert_eq!(
        history(&conn, "reader", "g", "m", None, 50)
            .unwrap()
            .revisions
            .len(),
        2
    );
}

#[test]
fn invalid_edits_do_not_create_audit_rows() {
    let mut conn = fixture();
    assert_error(
        edit(&mut conn, "author", "g", "m", 1, " \n "),
        RevisionError::Invalid,
    );
    assert_error(
        edit(&mut conn, "author", "g", "m", 1, &"字".repeat(4001)),
        RevisionError::Invalid,
    );
    assert_error(
        edit(&mut conn, "author", "g", "m", 0, "新文字"),
        RevisionError::Invalid,
    );
    assert_error(
        edit(&mut conn, "author", "g", "m", 1, "【一龙项目卡片】伪造"),
        RevisionError::Invalid,
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM friend_group_message_revisions",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn failed_current_text_write_rolls_back_both_revision_rows() {
    let mut conn = fixture();
    conn.execute_batch("CREATE TRIGGER reject_edit BEFORE UPDATE ON friend_group_messages BEGIN SELECT RAISE(ABORT, 'disk/write failure'); END;").unwrap();
    assert!(edit(&mut conn, "author", "g", "m", 1, "不能部分保存").is_err());
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM friend_group_message_revisions",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        history(&conn, "reader", "g", "m", None, 50)
            .unwrap()
            .current_revision,
        1
    );
}

#[test]
fn history_is_append_only_and_recall_does_not_expose_old_content() {
    let mut conn = fixture();
    edit(&mut conn, "author", "g", "m", 1, "更正").unwrap();
    assert!(conn
        .execute(
            "UPDATE friend_group_message_revisions SET content='篡改'",
            []
        )
        .is_err());
    assert!(conn
        .execute("DELETE FROM friend_group_message_revisions", [])
        .is_err());
    conn.execute(
        "UPDATE friend_group_messages SET recalled_at='2026-09-15T00:00:00Z'",
        [],
    )
    .unwrap();
    assert_error(
        history(&conn, "reader", "g", "m", None, 50),
        RevisionError::Recalled,
    );
    assert_error(
        edit(&mut conn, "author", "g", "m", 2, "撤回后编辑"),
        RevisionError::Recalled,
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM friend_group_message_revisions",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
}

#[test]
fn migration_is_repeatable_and_preserves_existing_versions() {
    let mut conn = fixture();
    edit(&mut conn, "author", "g", "m", 1, "修正文字").unwrap();
    migrate(&conn);
    assert_eq!(
        history(&conn, "reader", "g", "m", None, 50)
            .unwrap()
            .current_revision,
        2
    );
    assert_eq!(
        history(&conn, "reader", "g", "m", None, 50)
            .unwrap()
            .revisions
            .len(),
        2
    );
}

#[test]
fn edit_rate_is_bounded_without_losing_the_latest_saved_version() {
    let mut conn = fixture();
    for revision in 1..=30 {
        edit(
            &mut conn,
            "author",
            "g",
            "m",
            revision,
            &format!("第 {revision} 次修改"),
        )
        .unwrap();
    }
    assert_error(
        edit(&mut conn, "author", "g", "m", 31, "太快"),
        RevisionError::RateLimited,
    );
    assert_eq!(
        history(&conn, "reader", "g", "m", None, 50)
            .unwrap()
            .current_revision,
        31
    );
}
