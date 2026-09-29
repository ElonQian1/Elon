//! First-party onboarding membership, applied only while a human account is created.
use rusqlite::{params, Transaction};

// The existing 杀蟑螂 group. Bind its identity, never an editable/non-unique name.
const DEFAULT_GROUP_ID: &str = "grp_68682805eec5436fb68bf989668923b6";

pub(in crate::store) fn join_new_user(
    tx: &Transaction<'_>,
    user_id: &str,
    created_at: &str,
) -> rusqlite::Result<()> {
    // Fresh installations need not contain this deployment's group. Do not create
    // a replacement or enroll users in a different group with the same name.
    tx.execute(
        "INSERT INTO friend_group_members (group_id, user_id, created_at, last_read_at)
         SELECT id, ?2, ?3, ?3 FROM friend_groups WHERE id = ?1
         ON CONFLICT(group_id, user_id) DO NOTHING",
        params![DEFAULT_GROUP_ID, user_id, created_at],
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "registration_groups_tests.rs"]
mod tests;
