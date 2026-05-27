use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{normalize_account, now, AddFriendResult, FriendProfile, FriendSearchResult, Store};

impl Store {
    pub fn search_friend(
        &self,
        user_id: &str,
        search_type: Option<&str>,
        query: &str,
    ) -> Result<Option<FriendSearchResult>> {
        let query = query.trim();
        if query.is_empty() {
            return Err(anyhow!("请输入搜索内容"));
        }
        let search_type = FriendSearchType::from_input(search_type)?;
        let conn = self.conn()?;
        let mut profile = match search_profile(&conn, search_type, query)? {
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

    pub fn add_friend(
        &self,
        user_id: &str,
        search_type: Option<&str>,
        query: &str,
    ) -> Result<AddFriendResult> {
        let search = self
            .search_friend(user_id, search_type, query)?
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
            "SELECT u.id, u.phone, u.email, u.nickname, u.avatar_data_url, f.created_at,
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
                    avatar_data_url: row.get(4)?,
                    friend_since: row.get(5)?,
                    last_message: row.get(6)?,
                    last_message_at: row.get(7)?,
                    unread_count: row.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(friends)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum FriendSearchType {
    Auto,
    Phone,
    Email,
    AccountId,
    Nickname,
}

impl FriendSearchType {
    fn from_input(value: Option<&str>) -> Result<Self> {
        match value.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
            "" | "auto" => Ok(Self::Auto),
            "phone" => Ok(Self::Phone),
            "email" => Ok(Self::Email),
            "account" | "account_id" | "id" => Ok(Self::AccountId),
            "nickname" | "name" => Ok(Self::Nickname),
            _ => Err(anyhow!("不支持的好友搜索方式")),
        }
    }
}

fn search_profile(
    conn: &rusqlite::Connection,
    search_type: FriendSearchType,
    query: &str,
) -> Result<Option<FriendProfile>> {
    match search_type {
        FriendSearchType::Auto => search_profile_auto(conn, query),
        FriendSearchType::Phone => {
            search_profile_by_column(conn, "phone", &normalize_phone(query)?)
        }
        FriendSearchType::Email => {
            search_profile_by_column(conn, "email", &normalize_email(query)?)
        }
        FriendSearchType::AccountId => {
            search_profile_by_column(conn, "id", &normalize_account(query)?)
        }
        FriendSearchType::Nickname => search_profile_by_nickname(conn, query),
    }
}

fn search_profile_auto(conn: &rusqlite::Connection, query: &str) -> Result<Option<FriendProfile>> {
    if query.contains('@') {
        return search_profile_by_column(conn, "email", &normalize_email(query)?);
    }

    if query.trim().to_ascii_lowercase().starts_with("usr_") {
        let account = normalize_account(query)?;
        return search_profile_by_column(conn, "id", &account);
    }

    if looks_like_phone(query) {
        if let Some(profile) = search_profile_by_column(conn, "phone", &normalize_phone(query)?)? {
            return Ok(Some(profile));
        }
    }

    search_profile_by_nickname(conn, query)
}

fn search_profile_by_column(
    conn: &rusqlite::Connection,
    column: &str,
    value: &str,
) -> Result<Option<FriendProfile>> {
    let sql = format!(
        "SELECT id, phone, email, nickname, avatar_data_url
         FROM users
         WHERE {column} = ?1 AND status = 'active' AND password_hash != 'device-user'"
    );
    conn.query_row(&sql, params![value], friend_profile_from_row)
        .optional()
        .map_err(Into::into)
}

fn search_profile_by_nickname(
    conn: &rusqlite::Connection,
    query: &str,
) -> Result<Option<FriendProfile>> {
    let nickname = query.trim();
    if nickname.chars().count() < 2 {
        return Err(anyhow!("昵称至少输入 2 个字符"));
    }

    let mut stmt = conn.prepare(
        "SELECT id, phone, email, nickname, avatar_data_url
         FROM users
         WHERE nickname = ?1 AND status = 'active' AND password_hash != 'device-user'
         ORDER BY created_at DESC
         LIMIT 2",
    )?;
    let matches = stmt
        .query_map(params![nickname], friend_profile_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if matches.len() > 1 {
        return Err(anyhow!(
            "找到多个同名用户，请改用手机号、邮箱或账号 ID 搜索"
        ));
    }
    Ok(matches.into_iter().next())
}

fn friend_profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FriendProfile> {
    let id: String = row.get(0)?;
    let phone: Option<String> = row.get(1)?;
    let email: Option<String> = row.get(2)?;
    let account = phone.clone().or(email).unwrap_or_else(|| id.clone());
    Ok(FriendProfile {
        id,
        account,
        nickname: row.get(3)?,
        phone,
        avatar_data_url: row.get(4)?,
        friend_since: None,
        last_message: None,
        last_message_at: None,
        unread_count: 0,
    })
}

fn normalize_phone(phone: &str) -> Result<String> {
    let phone = normalize_account(&compact_phone(phone))?;
    if phone.contains('@') {
        return Err(anyhow!("请输入手机号"));
    }
    Ok(phone)
}

fn normalize_email(email: &str) -> Result<String> {
    let email = normalize_account(email)?;
    if !email.contains('@') {
        return Err(anyhow!("请输入邮箱"));
    }
    Ok(email)
}

fn compact_phone(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '-' | '(' | ')'))
        .collect()
}

fn looks_like_phone(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | ' ' | '-' | '(' | ')'))
}
