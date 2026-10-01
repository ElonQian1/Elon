//! First-party onboarding membership, applied only while a human account is created.
use rusqlite::{params, Transaction};

// The existing 杀蟑螂 group. Bind its identity, never an editable/non-unique name.
const DEFAULT_GROUP_ID: &str = "grp_68682805eec5436fb68bf989668923b6";
const NOTICE_SENDER_ID: &str = "usr_group_notifications";

pub(in crate::store) fn join_new_user(
    tx: &Transaction<'_>,
    user_id: &str,
    created_at: &str,
) -> rusqlite::Result<()> {
    // Fresh installations need not contain this deployment's group. Do not create
    // a replacement or enroll users in a different group with the same name.
    let inserted = tx.execute(
        "INSERT INTO friend_group_members (group_id, user_id, created_at, last_read_at)
         SELECT id, ?2, ?3, ?3 FROM friend_groups WHERE id = ?1
         ON CONFLICT(group_id, user_id) DO NOTHING",
        params![DEFAULT_GROUP_ID, user_id, created_at],
    )?;
    if inserted == 0 {
        return Ok(());
    }
    // A disabled, non-member identity makes notices readable by existing clients
    // without attributing generated text to the new member or inflating the roster.
    tx.execute(
        "INSERT INTO users (id, password_hash, nickname, status, password_login_enabled, created_at, updated_at)
         VALUES (?1, '', '群通知', 'disabled', 0, ?2, ?2) ON CONFLICT(id) DO NOTHING",
        params![NOTICE_SENDER_ID, created_at],
    )?;
    let name: String = tx.query_row(
        "SELECT COALESCE(NULLIF(TRIM(nickname), ''), '新用户') FROM users WHERE id = ?1",
        params![user_id],
        |row| row.get(0),
    )?;
    let name: String = name.chars().filter(|c| !c.is_control()).take(80).collect();
    tx.execute(
        "INSERT INTO friend_group_messages (id, group_id, sender_user_id, content, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            crate::store::new_id("gjoin"),
            DEFAULT_GROUP_ID,
            NOTICE_SENDER_ID,
            format!("{} 加入了群聊", name),
            created_at
        ],
    )?;
    tx.execute(
        "UPDATE friend_groups SET updated_at = ?2 WHERE id = ?1",
        params![DEFAULT_GROUP_ID, created_at],
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "registration_groups_tests.rs"]
mod tests;
