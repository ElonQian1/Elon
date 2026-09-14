use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use super::{
    model::{Error, GrantResponse, IssueRequest, MAX_LIFETIME},
    signer::Signer,
};

/// The read transaction is the issuance boundary; no writes or session extension.
pub(super) fn issue_on(
    conn: &mut Connection,
    token: &str,
    request: &IssueRequest,
    signer: &Signer,
    clock: impl FnOnce() -> i64,
) -> Result<GrantResponse, Error> {
    let scopes = request.validate()?;
    let tx = conn.transaction().map_err(|_| Error::Unavailable)?;
    // Deferred BEGIN has not acquired a SQLite read lock yet. Read the session
    // first, then take the clock so an external writer cannot age a queued grant.
    let (user, parent_expiry) = session_identity(&tx, token)?;
    let at = clock();
    if at <= 0 || parent_expiry <= at {
        return Err(Error::Unauthorized);
    }
    let expiry = at
        .checked_add(MAX_LIFETIME)
        .ok_or(Error::Unavailable)?
        .min(parent_expiry);
    let response = signer.issue(&user, scopes, at, expiry)?;
    tx.commit().map_err(|_| Error::Unavailable)?;
    Ok(response)
}

pub(super) fn session_on(conn: &Connection, token: &str, at: i64) -> Result<(String, i64), Error> {
    let (user, expiry) = session_identity(conn, token)?;
    if at <= 0 || expiry <= at {
        return Err(Error::Unauthorized);
    }
    Ok((user, expiry))
}

fn session_identity(conn: &Connection, token: &str) -> Result<(String, i64), Error> {
    if token.is_empty() || token.len() > 8192 || token.trim() != token {
        return Err(Error::Unauthorized);
    }
    let hash = hex::encode(Sha256::digest(token.as_bytes()));
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT u.id,s.expires_at FROM sessions s JOIN users u ON s.user_id=u.id
          WHERE s.token_hash=?1 AND s.revoked_at IS NULL AND u.status='active'
            AND u.id<>'local-owner'",
            params![hash],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Error::Unavailable)?;
    let (user, expires) = row.ok_or(Error::Unauthorized)?;
    let expiry = DateTime::parse_from_rfc3339(&expires)
        .map_err(|_| Error::Unauthorized)?
        .timestamp();
    if user.trim().is_empty() {
        return Err(Error::Unauthorized);
    }
    Ok((user, expiry))
}

#[cfg(test)]
#[path = "issue_tests.rs"]
mod tests;
