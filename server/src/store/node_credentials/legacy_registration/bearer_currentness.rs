use anyhow::{bail, Result};
use rusqlite::{params, Transaction};

use super::normalize::NormalizedRegistrationRequest;
use crate::store::{hash_token, now};

pub(super) fn require_registration_bearer_current_on(
    transaction: &Transaction<'_>,
    request: &NormalizedRegistrationRequest<'_>,
) -> Result<()> {
    let Some(bearer_token) = request.current_bearer_token else {
        return Ok(());
    };
    let current = transaction.query_row(
        "SELECT EXISTS(
            SELECT 1
              FROM sessions s
              JOIN users u ON u.id=s.user_id
             WHERE s.token_hash=?1
               AND s.user_id=?2
               AND s.expires_at>?3
               AND s.revoked_at IS NULL
               AND u.status='active'
         )",
        params![hash_token(bearer_token), request.owner_user_id, now()],
        |row| row.get::<_, bool>(0),
    )?;
    if !current {
        bail!("NODE_ENDPOINT_BOOTSTRAP_BEARER_NOT_CURRENT");
    }
    Ok(())
}
