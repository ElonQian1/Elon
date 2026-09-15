use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FriendRecommendation {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub avatar_data_url: Option<String>,
    pub mutual_friend_count: i64,
    pub already_friend: bool,
    /// 当前是否在线（由 API 层在返回前注入，store 层默认 false）
    pub is_online: bool,
}

pub fn list_recommendations(
    conn: &Connection,
    user_id: &str,
    social_ai_user_id: &str,
) -> Result<Vec<FriendRecommendation>> {
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
                ) AS mutual_friend_count
         FROM users u
         WHERE u.status = 'active'
           AND u.password_hash != 'device-user'
           AND u.id != ?1
           AND u.id != ?2
         ORDER BY already_friend ASC,
                  mutual_friend_count DESC,
                  COALESCE(u.nickname, u.email, u.phone, u.id) COLLATE NOCASE ASC,
                  u.created_at DESC
         LIMIT 500",
    )?;
    let recommendations = stmt
        .query_map(params![user_id, social_ai_user_id], |row| {
            let id: String = row.get(0)?;
            let phone: Option<String> = row.get(1)?;
            let email: Option<String> = row.get(2)?;
            let account = phone.clone().or(email).unwrap_or_else(|| id.clone());
            Ok(FriendRecommendation {
                id,
                account,
                nickname: row.get(3)?,
                phone,
                avatar_data_url: row.get(4)?,
                already_friend: row.get::<_, i64>(5)? != 0,
                mutual_friend_count: row.get(6)?,
                is_online: false,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(recommendations)
}
