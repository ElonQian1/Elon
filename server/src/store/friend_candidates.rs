use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

const MAX_CANDIDATES: usize = 20;

pub(super) enum CandidateField {
    Phone,
    Email,
    AccountId,
    Nickname,
}

#[derive(Debug, Serialize)]
pub struct FriendCandidate {
    pub id: String,
    pub nickname: String,
    pub account_hint: String,
    pub avatar_data_url: Option<String>,
    pub already_friend: bool,
    pub is_self: bool,
}

#[derive(Debug, Serialize)]
pub struct FriendCandidates {
    pub results: Vec<FriendCandidate>,
    pub has_more: bool,
}

/// Resolve actual registered identifiers; punctuation alone does not select a search mode.
/// Registration stores non-email login names in users.phone as well.
pub(super) fn auto_account_id(conn: &Connection, query: &str) -> Result<Option<String>> {
    let account = query.trim().to_ascii_lowercase();
    if account.is_empty() {
        return Ok(None);
    }
    let formatted_phone: Option<String> = account
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | ' ' | '-' | '(' | ')'))
        .then(|| {
            account
                .chars()
                .filter(|ch| !matches!(ch, ' ' | '-' | '(' | ')'))
                .collect()
        });
    conn.query_row(
        "SELECT id FROM users
         WHERE status = 'active' AND password_hash != 'device-user'
           AND (id = ?1 OR email = ?1 OR phone = ?1 OR phone = ?2)
         ORDER BY CASE WHEN id = ?1 THEN 0 WHEN email = ?1 THEN 1
                       WHEN phone = ?1 THEN 2 ELSE 3 END, id
         LIMIT 1",
        params![account, formatted_phone],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

/// Prefer an exact registered identifier, then an exact case-insensitive nickname.
pub(super) fn search_auto_candidates(
    conn: &Connection,
    viewer_id: &str,
    query: &str,
) -> Result<FriendCandidates> {
    match auto_account_id(conn, query)? {
        Some(id) => search_candidates(conn, viewer_id, CandidateField::AccountId, &id),
        None => search_candidates(conn, viewer_id, CandidateField::Nickname, query.trim()),
    }
}

/// Exact matches only. Raw phone/email values never leave this query boundary.
pub(super) fn search_candidates(
    conn: &Connection,
    viewer_id: &str,
    field: CandidateField,
    value: &str,
) -> Result<FriendCandidates> {
    let column = match field {
        CandidateField::Phone => "phone",
        CandidateField::Email => "email",
        CandidateField::AccountId => "id",
        CandidateField::Nickname => {
            if value.trim().is_empty() {
                return Err(anyhow!("请输入昵称"));
            }
            "nickname COLLATE NOCASE"
        }
    };
    let sql = format!(
        "SELECT u.id, u.phone, u.email, u.nickname, u.avatar_data_url,
                EXISTS(SELECT 1 FROM user_friends f
                       WHERE f.user_id = ?2 AND f.friend_user_id = u.id)
         FROM users u
         WHERE u.{column} = ?1 AND u.status = 'active'
           AND u.password_hash != 'device-user'
         ORDER BY u.created_at DESC, u.id ASC LIMIT ?3"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut results = stmt
        .query_map(params![value, viewer_id, MAX_CANDIDATES + 1], |row| {
            let id: String = row.get(0)?;
            let phone: Option<String> = row.get(1)?;
            let email: Option<String> = row.get(2)?;
            let nickname: Option<String> = row.get(3)?;
            let account_hint = account_hint(phone.as_deref(), email.as_deref(), &id);
            Ok(FriendCandidate {
                is_self: id == viewer_id,
                id,
                nickname: nickname
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| account_hint.clone()),
                account_hint,
                avatar_data_url: row.get(4)?,
                already_friend: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let has_more = results.len() > MAX_CANDIDATES;
    results.truncate(MAX_CANDIDATES);
    Ok(FriendCandidates { results, has_more })
}

pub(super) fn account_hint(phone: Option<&str>, email: Option<&str>, id: &str) -> String {
    if let Some(phone) = phone.filter(|value| !value.is_empty()) {
        // Older accounts can store a username in the phone column.
        if phone.chars().all(|ch| ch.is_ascii_digit() || ch == '+') && phone.len() >= 7 {
            let suffix: String = phone
                .chars()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            return format!("手机尾号 {suffix}");
        }
        return format!("账号 {}", masked_identifier(phone));
    }
    if let Some(email) = email.filter(|value| !value.is_empty()) {
        // Mask both parts: a personal email domain can itself identify a user.
        return format!("邮箱 {}", masked_identifier(email));
    }
    format!("账号 {}", masked_identifier(id))
}

fn masked_identifier(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    match chars.as_slice() {
        [] => "***".to_string(),
        [first] => format!("{first}***"),
        [first, .., last] => format!("{first}***{last}"),
    }
}

#[cfg(test)]
#[path = "friend_candidates_tests.rs"]
mod tests;
