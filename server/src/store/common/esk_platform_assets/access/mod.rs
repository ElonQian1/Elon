use anyhow::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use rusqlite::{params, Connection, OptionalExtension};

use super::{hash_token, new_id, now};
use crate::esk_asset::platform::access::*;

mod issue;
mod read;
mod revoke;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_recovery;

/// Only this module can construct an authority; callers use it inside the same read transaction.
pub(super) struct AuthorizedAssetRead {
    user_id: String,
    subject: String,
    grant_id: String,
    client_id: String,
    scopes: Vec<AccessScope>,
    expires_at: String,
}

impl AuthorizedAssetRead {
    pub(crate) fn user_id(&self) -> &str {
        &self.user_id
    }
    pub(crate) fn subject(&self) -> &str {
        &self.subject
    }
    pub(crate) fn grant_id(&self) -> &str {
        &self.grant_id
    }
    pub(crate) fn client_id(&self) -> &str {
        &self.client_id
    }
    pub(crate) fn scopes(&self) -> &[AccessScope] {
        &self.scopes
    }
    pub(crate) fn expires_at(&self) -> &str {
        &self.expires_at
    }
}

pub(super) use read::verify_read_on;

fn clock() -> Result<i64> {
    DateTime::parse_from_rfc3339(&now())
        .map(|value| value.timestamp())
        .map_err(|_| AccessError::Unavailable.into())
}

fn timestamp(value: i64) -> Result<String> {
    DateTime::<Utc>::from_timestamp(value, 0)
        .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true))
        .ok_or_else(|| AccessError::Corrupt.into())
}

fn random_secret(prefix: &str) -> Result<String> {
    let mut bytes = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| AccessError::Unavailable)?;
    Ok(format!("{prefix}{}", hex::encode(bytes)))
}

fn equal(left: &str, right: &str) -> bool {
    // Inputs are fixed-size hashes or PKCE challenges, never attacker-selected allocation sizes.
    ring::constant_time::verify_slices_are_equal(left.as_bytes(), right.as_bytes()).is_ok()
}

fn session_expiry_on(conn: &Connection, user: &str, parent_hash: &str, at: i64) -> Result<i64> {
    let expires: Option<String> = conn
        .query_row(
            "SELECT s.expires_at FROM users u JOIN sessions s ON s.user_id=u.id
          WHERE u.id=?1 AND u.id<>'local-owner' AND u.status='active'
            AND s.token_hash=?2 AND s.revoked_at IS NULL
            AND julianday(s.expires_at)>julianday(?3,'unixepoch')",
            params![user, parent_hash, at],
            |row| row.get(0),
        )
        .optional()?;
    let expires = expires.ok_or(AccessError::Unauthorized)?;
    let value = DateTime::parse_from_rfc3339(&expires)
        .map_err(|_| AccessError::Unauthorized)?
        .timestamp();
    if value <= at {
        return Err(AccessError::Unauthorized.into());
    }
    Ok(value)
}

fn scopes_from_json(value: &str) -> Result<Vec<AccessScope>> {
    let scopes: Vec<AccessScope> = serde_json::from_str(value).map_err(|_| AccessError::Corrupt)?;
    if !valid_scopes(&scopes) {
        return Err(AccessError::Corrupt.into());
    }
    Ok(scopes)
}

fn audit(conn: &Connection, grant_id: &str, action: &str, at: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO asset_access_audit(audit_id,grant_id,action,created_at_unix)
        VALUES (?1,?2,?3,?4)",
        params![new_id("aaa"), grant_id, action, at],
    )?;
    Ok(())
}
