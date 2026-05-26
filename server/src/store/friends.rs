use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{normalize_account, now, AddFriendResult, FriendProfile, FriendSearchResult, Store};

impl Store {
    pub fn search_friend_by_phone(
        &self,
        user_id: &str,
        phone: &str,
    ) -> Result<Option<FriendSearchResult>> {
        let phone = normalize_friend_phone(phone)?;
        let conn = self.conn()?;
        let mut profile = match conn
            .query_row(
                "SELECT id, phone, email, nickname
                 FROM users
                 WHERE phone = ?1 AND status = 'active' AND password_hash != 'device-user'",
                params![phone],
                |row| {
                    let id: String = row.get(0)?;
                    let phone: Option<String> = row.get(1)?;
                    let email: Option<String> = row.get(2)?;
                    let account = phone.clone().or(email).unwrap_or_else(|| id.clone());
                    Ok(FriendProfile {
                        id,
                        account,
                        nickname: row.get(3)?,
                        phone,
                        friend_since: None,
                        last_message: None,
                        last_message_at: None,
                        unread_count: 0,
                    })
                },
            )
            .optional()?
        {
            Some(profile) => profile,
            None => return Ok(None),
        };

        let is_self = profile.id == user_id;
        let friend_since = if is_self {
            None
        } else {
            conn.query_row(
                "SELECT created_at
                 FROM user_friends
                 WHERE user_id = ?1 AND friend_user_id = ?2",
                params![user_id, profile.id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        };
        let already_friend = friend_since.is_some();
        profile.friend_since = friend_since;

        Ok(Some(FriendSearchResult {
            user: profile,
            already_friend,
            is_self,
        }))
    }

    pub fn add_friend_by_phone(&self, user_id: &str, phone: &str) -> Result<AddFriendResult> {
        let search = self
            .search_friend_by_phone(user_id, phone)?
            .ok_or_else(|| anyhow!("未找到已注册用户"))?;
        if search.is_self {
            return Err(anyhow!("不能添加自己"));
        }

        let conn = self.conn()?;
        let created_at = now();
        let tx = conn.unchecked_transaction()?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO user_friends (user_id, friend_user_id, created_at)
             VALUES (?1, ?2, ?3)",
            params![user_id, search.user.id, created_at],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO user_friends (user_id, friend_user_id, created_at)
             VALUES (?1, ?2, ?3)",
            params![search.user.id, user_id, created_at],
        )?;
        tx.commit()?;

        Ok(AddFriendResult {
            friend: FriendProfile {
                friend_since: Some(created_at),
                ..search.user
            },
            already_friend: inserted == 0,
        })
    }

    pub fn list_friends(&self, user_id: &str) -> Result<Vec<FriendProfile>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT u.id, u.phone, u.email, u.nickname, f.created_at,
                    lm.content,
                    lm.created_at,
                    (
                        SELECT COUNT(*)
                        FROM friend_messages unread
                        LEFT JOIN friend_read_states read_state
                          ON read_state.user_id = ?1
                         AND read_state.friend_user_id = u.id
                        WHERE unread.sender_user_id = u.id
                          AND unread.receiver_user_id = ?1
                          AND (
                              read_state.last_read_at IS NULL
                              OR unread.created_at > read_state.last_read_at
                          )
                    ) AS unread_count
             FROM user_friends f
             JOIN users u ON u.id = f.friend_user_id
             LEFT JOIN friend_messages lm
               ON lm.id = (
                   SELECT latest.id
                   FROM friend_messages latest
                   WHERE (
                       (latest.sender_user_id = ?1 AND latest.receiver_user_id = u.id)
                       OR (latest.sender_user_id = u.id AND latest.receiver_user_id = ?1)
                   )
                   ORDER BY latest.created_at DESC
                   LIMIT 1
               )
             WHERE f.user_id = ?1 AND u.status = 'active'
             ORDER BY COALESCE(lm.created_at, f.created_at) DESC",
        )?;
        let friends = stmt
            .query_map(params![user_id], |row| {
                let id: String = row.get(0)?;
                let phone: Option<String> = row.get(1)?;
                let email: Option<String> = row.get(2)?;
                let account = phone.clone().or(email).unwrap_or_else(|| id.clone());
                Ok(FriendProfile {
                    id,
                    account,
                    nickname: row.get(3)?,
                    phone,
                    friend_since: row.get(4)?,
                    last_message: row.get(5)?,
                    last_message_at: row.get(6)?,
                    unread_count: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(friends)
    }
}

fn normalize_friend_phone(phone: &str) -> Result<String> {
    let phone = normalize_account(phone)?;
    if phone.contains('@') {
        return Err(anyhow!("请输入手机号"));
    }
    Ok(phone)
}
