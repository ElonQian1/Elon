use super::friend_candidates::account_hint;
use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FriendRecommendation {
    pub id: String,
    pub account: String,
    pub account_hint: String,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub avatar_data_url: Option<String>,
    pub mutual_friend_count: i64,
    pub already_friend: bool,
    /// 当前连接快照与用户展示状态共同决定；隐身用户按离线返回。
    pub is_online: bool,
}

pub fn list_recommendations(
    conn: &Connection,
    user_id: &str,
    social_ai_user_id: &str,
    online_user_ids: &[String],
) -> Result<Vec<FriendRecommendation>> {
    let online_json = serde_json::to_string(online_user_ids)?;
    let mut stmt = conn.prepare(
        "SELECT u.id,
                u.phone,
                u.email,
                u.nickname,
                u.avatar_data_url,
                EXISTS(
                    SELECT 1
                    FROM user_friends existing
                    WHERE existing.user_id = ?1
                      AND existing.friend_user_id = u.id
                ) AS already_friend,
                (
                    SELECT COUNT(*)
                    FROM user_friends mine
                    JOIN user_friends theirs
                      ON theirs.friend_user_id = mine.friend_user_id
                    WHERE mine.user_id = ?1
                      AND theirs.user_id = u.id
                      AND mine.friend_user_id != ?1
                      AND mine.friend_user_id != u.id
                      AND mine.friend_user_id != ?2
                ) AS mutual_friend_count,
                (u.id IN (SELECT value FROM json_each(?3))
                  AND COALESCE(lower(trim(ps.status)), 'online') != 'invisible') AS is_online
         FROM users u
         LEFT JOIN user_presence_settings ps ON ps.user_id = u.id
         WHERE u.status = 'active'
           AND u.password_hash != 'device-user'
           AND u.id != ?1
           AND u.id != ?2
         ORDER BY is_online DESC,
                  already_friend ASC,
                  mutual_friend_count DESC,
                  COALESCE(u.nickname, u.email, u.phone, u.id) COLLATE NOCASE ASC,
                  u.created_at DESC,
                  u.id ASC
         LIMIT 500",
    )?;
    let recommendations = stmt
        .query_map(params![user_id, social_ai_user_id, online_json], |row| {
            let id: String = row.get(0)?;
            let phone: Option<String> = row.get(1)?;
            let email: Option<String> = row.get(2)?;
            let account = account_hint(phone.as_deref(), email.as_deref(), &id);
            let nickname: Option<String> = row.get(3)?;
            let nickname = nickname.filter(|name| {
                !name.trim().is_empty()
                    && ![phone.as_deref(), email.as_deref()]
                        .into_iter()
                        .flatten()
                        .any(|value| name.trim().eq_ignore_ascii_case(value.trim()))
            });
            Ok(FriendRecommendation {
                id,
                account_hint: account.clone(),
                nickname: Some(nickname.unwrap_or_else(|| account.clone())),
                account,
                phone: None,
                avatar_data_url: row.get(4)?,
                already_friend: row.get::<_, i64>(5)? != 0,
                mutual_friend_count: row.get(6)?,
                is_online: row.get::<_, i64>(7)? != 0,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(recommendations)
}

#[cfg(test)]
#[path = "friend_recommendations_tests.rs"]
mod tests;
