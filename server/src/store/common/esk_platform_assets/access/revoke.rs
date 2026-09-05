use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::{audit, clock, hash_token, read::verify_token_on, session_expiry_on};
use crate::{esk_asset::platform::access::*, store::Store};

pub(super) fn revoke_on(conn: &Connection, grant_id: &str, at: i64) -> Result<()> {
    let changed = conn.execute(
        "UPDATE asset_access_grants SET revoked_at_unix=?1
        WHERE grant_id=?2 AND revoked_at_unix IS NULL",
        params![at, grant_id],
    )?;
    conn.execute(
        "UPDATE asset_access_tokens SET revoked_at_unix=COALESCE(revoked_at_unix,?1)
        WHERE grant_id=?2 AND revoked_at_unix IS NULL",
        params![at, grant_id],
    )?;
    if changed == 1 {
        audit(conn, grant_id, "revoked", at)?;
    }
    Ok(())
}

impl Store {
    pub(crate) fn revoke_asset_access_grant(
        &self,
        user_id: &str,
        session_token: &str,
        grant_id: &str,
    ) -> Result<()> {
        if !valid_grant_id(grant_id) {
            return Err(AccessError::NotFound.into());
        }
        let at = clock()?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        session_expiry_on(&tx, user_id, &hash_token(session_token), at)?;
        let own: Option<String> = tx
            .query_row(
                "SELECT grant_id FROM asset_access_grants WHERE grant_id=?1 AND user_id=?2",
                params![grant_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        if own.is_none() {
            return Err(AccessError::NotFound.into());
        }
        revoke_on(&tx, grant_id, at)?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn revoke_asset_access_token(&self, token: &str, client_id: &str) -> Result<()> {
        let at = clock()?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let read = verify_token_on(&tx, token, client_id, at, true)?;
        revoke_on(&tx, read.grant_id(), at)?;
        tx.commit()?;
        Ok(())
    }
}
