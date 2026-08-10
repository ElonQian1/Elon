use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::store::{hash_token, verify_password};

pub(crate) struct CurrentOwnerAccountSource {
    account_session_id: String,
    owner_user_id: String,
    token_hash: String,
    session_created_at: String,
    session_expires_at: String,
    session_revoked_at: Option<String>,
    role: String,
    user_status: String,
    password_login_enabled: bool,
    password_hash: String,
    password_changed_at: Option<String>,
    user_updated_at: String,
}

pub(super) fn verify_current_owner_account_on(
    transaction: &Transaction<'_>,
    bearer_token: &str,
    current_password: &str,
    checked_at: DateTime<Utc>,
) -> Result<CurrentOwnerAccountSource> {
    let source = transaction
        .query_row(
            "SELECT s.id, s.user_id, s.token_hash, s.created_at, s.expires_at, s.revoked_at,
                    u.role, u.status, u.password_login_enabled, u.password_hash,
                    u.password_changed_at, u.updated_at
               FROM sessions s
               JOIN users u ON u.id=s.user_id
              WHERE s.token_hash=?1",
            params![hash_token(bearer_token)],
            |row| {
                Ok(CurrentOwnerAccountSource {
                    account_session_id: row.get(0)?,
                    owner_user_id: row.get(1)?,
                    token_hash: row.get(2)?,
                    session_created_at: row.get(3)?,
                    session_expires_at: row.get(4)?,
                    session_revoked_at: row.get(5)?,
                    role: row.get(6)?,
                    user_status: row.get(7)?,
                    password_login_enabled: row.get::<_, i64>(8)? == 1,
                    password_hash: row.get(9)?,
                    password_changed_at: row.get(10)?,
                    user_updated_at: row.get(11)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_ACCOUNT_AUTHENTICATION_FAILED"))?;

    let expires_at = DateTime::parse_from_rfc3339(&source.session_expires_at)
        .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_OWNER_ACCOUNT_SESSION_EXPIRY_INVALID"))?
        .with_timezone(&Utc);
    if source.session_revoked_at.is_some() || checked_at >= expires_at {
        bail!("NODE_ENDPOINT_OWNER_ACCOUNT_SESSION_NOT_CURRENT");
    }
    if source.user_status != "active" {
        bail!("NODE_ENDPOINT_OWNER_ACCOUNT_NOT_ACTIVE");
    }
    if !source.password_login_enabled {
        bail!("NODE_ENDPOINT_OWNER_PASSWORD_FACTOR_DISABLED");
    }
    if !verify_password(current_password, &source.password_hash) {
        bail!("NODE_ENDPOINT_OWNER_ACCOUNT_AUTHENTICATION_FAILED");
    }
    Ok(source)
}

impl CurrentOwnerAccountSource {
    pub(crate) fn account_session_id(&self) -> &str {
        &self.account_session_id
    }

    pub(crate) fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }

    pub(crate) fn token_hash(&self) -> &str {
        &self.token_hash
    }

    pub(crate) fn session_created_at(&self) -> &str {
        &self.session_created_at
    }

    pub(crate) fn session_expires_at(&self) -> &str {
        &self.session_expires_at
    }

    pub(crate) fn session_revoked_at(&self) -> Option<&str> {
        self.session_revoked_at.as_deref()
    }

    pub(crate) fn role(&self) -> &str {
        &self.role
    }

    pub(crate) fn user_status(&self) -> &str {
        &self.user_status
    }

    pub(crate) fn password_login_enabled(&self) -> bool {
        self.password_login_enabled
    }

    pub(crate) fn password_hash(&self) -> &str {
        &self.password_hash
    }

    pub(crate) fn password_changed_at(&self) -> Option<&str> {
        self.password_changed_at.as_deref()
    }

    pub(crate) fn user_updated_at(&self) -> &str {
        &self.user_updated_at
    }
}
