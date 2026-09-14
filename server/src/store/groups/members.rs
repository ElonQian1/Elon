//! Complete member directory for mentions; the chat list deliberately keeps nine previews.
use super::{FriendGroupMemberPreview, Store};
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};

impl Store {
    pub fn list_friend_group_mention_members(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> Result<Vec<FriendGroupMemberPreview>> {
        let conn = self.conn()?;
        list_members(&conn, user_id, group_id)
    }
}

fn list_members(
    conn: &Connection,
    user_id: &str,
    group_id: &str,
) -> Result<Vec<FriendGroupMemberPreview>> {
    let allowed: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM friend_group_members WHERE group_id = ?1 AND user_id = ?2)",
        params![group_id, user_id],
        |row| row.get(0),
    )?;
    if !allowed {
        return Err(anyhow!("仅群成员可以查看群成员列表"));
    }
    let mut statement = conn.prepare(
        "SELECT u.id, COALESCE(NULLIF(u.nickname, ''), u.email, u.phone, u.id), u.avatar_data_url
         FROM friend_group_members gm JOIN users u ON u.id = gm.user_id
         WHERE gm.group_id = ?1 ORDER BY gm.created_at ASC, u.id ASC",
    )?;
    let rows = statement
        .query_map(params![group_id], |row| {
            Ok(FriendGroupMemberPreview {
                id: row.get(0)?,
                display_name: row.get(1)?,
                avatar_data_url: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE users (id TEXT PRIMARY KEY, nickname TEXT, email TEXT, phone TEXT, avatar_data_url TEXT);
            CREATE TABLE friend_group_members (group_id TEXT, user_id TEXT, created_at TEXT);").unwrap();
        for index in 0..115 {
            let id = format!("u{index:03}");
            conn.execute(
                "INSERT INTO users VALUES (?1, ?2, NULL, NULL, NULL)",
                params![id, format!("群友{index}")],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO friend_group_members VALUES ('group', ?1, '2026-09-15')",
                params![id],
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn mention_directory_includes_members_beyond_nine_previews() {
        let members = list_members(&fixture(), "u000", "group").unwrap();
        assert_eq!(members.len(), 115);
        assert!(members
            .iter()
            .any(|member| member.display_name == "群友114"));
    }

    #[test]
    fn outsiders_and_other_groups_cannot_read_directory() {
        let conn = fixture();
        assert!(list_members(&conn, "outsider", "group").is_err());
        assert!(list_members(&conn, "u000", "other-group").is_err());
        conn.execute(
            "DELETE FROM friend_group_members WHERE user_id = 'u000'",
            [],
        )
        .unwrap();
        assert!(list_members(&conn, "u000", "group").is_err());
    }
}
