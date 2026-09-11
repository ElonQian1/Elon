use super::{model::*, policy};
use rusqlite::{params, Connection, OptionalExtension};

/// Called inside the same transaction as every protected read/write, and again at its end.
pub fn session(conn: &Connection, user: &str, token: &str, admin: bool, at: i64) -> Result<()> {
    if !policy::id(user)
        || user == "local-owner"
        || token.is_empty()
        || token.len() > 8192
        || at <= 0
    {
        return Err(Error::Unauthorized.into());
    }
    let active: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users u JOIN sessions s ON s.user_id=u.id
         WHERE u.id=?1 AND u.status='active' AND (?3=0 OR u.role IN ('admin','owner'))
         AND s.token_hash=?2 AND s.revoked_at IS NULL AND julianday(s.expires_at) IS NOT NULL
         AND julianday(s.expires_at)>julianday(?4 / 1000.0,'unixepoch'))",
        params![user, policy::hash(token), admin, at],
        |r| r.get(0),
    )?;
    if !active {
        return Err(Error::Unauthorized.into());
    }
    Ok(())
}
pub fn user(conn: &Connection, id: &str) -> Result<()> {
    if !policy::id(id) || id == "local-owner" {
        return Err(Error::Unauthorized.into());
    }
    let active: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?1 AND status='active')",
        [id],
        |r| r.get(0),
    )?;
    if !active {
        return Err(Error::Unauthorized.into());
    }
    Ok(())
}
pub fn pin(conn: &Connection, policy: &policy::Policy, create: bool) -> Result<()> {
    let stored: Option<(String, String)> = conn
        .query_row(
            "SELECT digest,source_json FROM game_reward_policy WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let json = serde_json::to_string(&policy.input)?;
    match stored {
        Some((digest, source)) if digest == policy.digest && source == json => Ok(()),
        None if create => {
            conn.execute(
                "INSERT INTO game_reward_policy VALUES(1,?1,?2)",
                params![policy.digest, json],
            )?;
            Ok(())
        }
        None => Ok(()),
        _ => Err(Error::Conflict.into()),
    }
}
