use chrono::{DateTime, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{model::*, policy::*, protocol::Grant};
pub type Clock<'a> = &'a dyn Fn() -> Result<i64>;

pub fn now_ms() -> Result<i64> {
    let at = Utc::now().timestamp_millis();
    if at <= 0 {
        return Err(Error::Unavailable.into());
    }
    Ok(at)
}
pub(super) fn secret(prefix: &str) -> Result<String> {
    let mut bytes = [0u8; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    Ok(format!("{prefix}{}", hex::encode(bytes)))
}
pub(super) fn audit(conn: &Connection, id: &str, action: &str, at: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO game_access_audit VALUES(?1,?2,?3)",
        params![id, action, at],
    )?;
    Ok(())
}

pub(super) struct Parent {
    pub id: String,
    pub expires_ms: i64,
}
pub(super) fn parent(conn: &Connection, user: &str, token_hash: &str, at: i64) -> Result<Parent> {
    if !identifier(user) || !lower_hex(token_hash, 32) {
        return Err(Error::Unauthorized.into());
    }
    let row: Option<(String, String)> = conn.query_row(
        "SELECT s.id,s.expires_at FROM sessions s JOIN users u ON u.id=s.user_id
         WHERE u.id=?1 AND u.id<>'local-owner' AND u.status='active' AND s.token_hash=?2 AND s.revoked_at IS NULL",
        params![user, token_hash], |r| Ok((r.get(0)?,r.get(1)?)),
    ).optional()?;
    let (id, expiry) = row.ok_or(Error::Unauthorized)?;
    let expires_ms = DateTime::parse_from_rfc3339(&expiry)
        .map_err(|_| Error::Unauthorized)?
        .timestamp_millis();
    if !identifier(&id) || expires_ms <= at {
        return Err(Error::Unauthorized.into());
    }
    Ok(Parent { id, expires_ms })
}

pub(super) struct StoredGrant {
    pub id: String,
    pub user: String,
    pub session: String,
    pub parent_hash: String,
    pub policy_digest: String,
    pub scopes_json: String,
    pub created: i64,
    pub expires: i64,
    pub revision: i64,
    pub revoked: Option<i64>,
}
pub(super) const COLUMNS: &str = "grant_id,user_id,session_id,parent_hash,policy_digest,scopes_json,created_ms,expires_ms,revision,revoked_ms";
impl StoredGrant {
    pub fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get(0)?,
            user: r.get(1)?,
            session: r.get(2)?,
            parent_hash: r.get(3)?,
            policy_digest: r.get(4)?,
            scopes_json: r.get(5)?,
            created: r.get(6)?,
            expires: r.get(7)?,
            revision: r.get(8)?,
            revoked: r.get(9)?,
        })
    }
    pub fn verify(&self, conn: &Connection, policy: &Policy, at: i64) -> Result<Grant> {
        if self.policy_digest != policy.digest
            || self.revoked.is_some()
            || self.revision != 1
            || at < self.created
            || at >= self.expires
            || self
                .expires
                .checked_sub(self.created)
                .is_none_or(|d| d <= 0 || d > 900_000)
        {
            return Err(Error::InvalidGrant.into());
        }
        let parent = parent(conn, &self.user, &self.parent_hash, at)?;
        if parent.id != self.session || parent.expires_ms < self.expires {
            return Err(Error::InvalidGrant.into());
        }
        let scopes: Vec<String> =
            serde_json::from_str(&self.scopes_json).map_err(|_| Error::Corrupt)?;
        if !valid_scopes(&scopes) {
            return Err(Error::Corrupt.into());
        }
        Ok(Grant {
            main_user_id: self.user.clone(),
            main_session_id: self.session.clone(),
            grant_id: self.id.clone(),
            revision: self.revision.to_string(),
            not_before_ms: self.created.to_string(),
            expires_at_ms: self.expires.to_string(),
            scopes,
        })
    }
}
pub(super) fn token_grant(
    conn: &Connection,
    policy: &Policy,
    token: &str,
    at: i64,
) -> Result<(StoredGrant, Grant)> {
    if !secret_shape(token, "egt_") {
        return Err(Error::Unauthorized.into());
    }
    let stored = conn.query_row(
        &format!("SELECT {COLUMNS} FROM game_access_grants WHERE token_hash=?1 AND consumed_ms IS NOT NULL"),
        [hash(token)], StoredGrant::from_row,
    ).optional()?.ok_or(Error::Unauthorized)?;
    let grant = stored.verify(conn, policy, at)?;
    Ok((stored, grant))
}
