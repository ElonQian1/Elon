use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    esk_asset::platform::{PlatformError, PlatformPolicy},
    store::Store,
};

use super::{hash_token, new_id, now};

mod cancel;
mod history;
mod read;
mod reconciliation;
mod sellback;
mod write;

fn ensure_admin(conn: &Connection, actor_user_id: &str, session_token: &str) -> Result<()> {
    ensure_session(conn, actor_user_id, session_token, true)
}

impl Store {
    pub(crate) fn validate_esk_platform_session(
        &self,
        user_id: &str,
        session_token: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        ensure_session(&conn, user_id, session_token, false)
    }
}

fn ensure_session(
    conn: &Connection,
    user_id: &str,
    session_token: &str,
    require_admin: bool,
) -> Result<()> {
    if user_id == "local-owner" || session_token.trim().is_empty() {
        return Err(PlatformError::Unauthorized.into());
    }
    let active: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users u JOIN sessions s ON s.user_id = u.id
          WHERE u.id = ?1 AND u.status = 'active'
            AND (?4 = 0 OR u.role IN ('admin', 'owner'))
            AND s.token_hash = ?2 AND s.revoked_at IS NULL
            AND julianday(s.expires_at) IS NOT NULL
            AND julianday(s.expires_at) > julianday(?3))",
        params![
            user_id,
            hash_token(session_token.trim()),
            now(),
            require_admin
        ],
        |row| row.get(0),
    )?;
    if !active {
        return Err(PlatformError::Unauthorized.into());
    }
    Ok(())
}

fn ensure_active_user(conn: &Connection, user_id: &str) -> Result<()> {
    let active: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = ?1 AND status = 'active')",
        params![user_id],
        |row| row.get(0),
    )?;
    if !active || user_id == "local-owner" {
        return Err(PlatformError::UserUnavailable.into());
    }
    Ok(())
}

fn policy_on(conn: &Connection) -> Result<Option<PlatformPolicy>> {
    let stored = conn
        .query_row(
            "SELECT source_json, source_fingerprint, policy_digest, issuance_limit_base_units
               FROM esk_platform_policy WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?;
    stored
        .map(
            |(source, source_fingerprint, policy_digest, issuance_limit_base_units)| {
                Ok(PlatformPolicy {
                    source: serde_json::from_str(&source)
                        .map_err(|_| PlatformError::CorruptLedger)?,
                    source_fingerprint,
                    policy_digest,
                    issuance_limit_base_units,
                })
            },
        )
        .transpose()
}

fn require_same_policy(stored: &PlatformPolicy, current: &PlatformPolicy) -> Result<()> {
    if stored.source != current.source
        || stored.policy_digest != current.policy_digest
        || stored.source_fingerprint != current.source_fingerprint
        || stored.issuance_limit_base_units != current.issuance_limit_base_units
    {
        return Err(PlatformError::PolicyChanged.into());
    }
    Ok(())
}
