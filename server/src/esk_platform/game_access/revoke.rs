use super::{authority::*, model::*, policy::*};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

fn revision(request: &RevokeRequest) -> Result<i64> {
    if request.schema != "esk.game.access.revoke.v1" || request.expected_revision != "1" {
        return Err(Error::InvalidInput.into());
    }
    Ok(1)
}
fn revoke_on(conn: &Connection, id: &str, expected: i64, at: i64) -> Result<()> {
    let (current, revoked): (i64, Option<i64>) = conn.query_row(
        "SELECT revision,revoked_ms FROM game_access_grants WHERE grant_id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if revoked.is_some() && current == expected + 1 {
        return Ok(());
    }
    if current != expected {
        return Err(Error::RevisionConflict.into());
    }
    conn.execute("UPDATE game_access_grants SET revoked_ms=?1,revision=revision+1 WHERE grant_id=?2 AND revision=?3 AND revoked_ms IS NULL",
        params![at,id,expected])?;
    audit(conn, id, "revoked", at)
}
pub fn revoke_owner(
    conn: &mut Connection,
    user: &str,
    parent_token: &str,
    id: &str,
    request: &RevokeRequest,
    clock: Clock<'_>,
) -> Result<()> {
    let expected = revision(request)?;
    if !identifier(id) || parent_token.is_empty() || parent_token.len() > 8192 {
        return Err(Error::InvalidInput.into());
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let at = clock()?;
    parent(&tx, user, &hash(parent_token), at)?;
    let owner: Option<String> = tx
        .query_row(
            "SELECT user_id FROM game_access_grants WHERE grant_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    if owner.as_deref() != Some(user) {
        return Err(Error::Unauthorized.into());
    }
    revoke_on(&tx, id, expected, at)?;
    tx.commit()?;
    Ok(())
}
pub fn revoke_self(
    conn: &mut Connection,
    policy: &Policy,
    service_secret: &str,
    token: &str,
    request: &RevokeRequest,
    clock: Clock<'_>,
) -> Result<()> {
    policy.check_service(service_secret)?;
    let expected = revision(request)?;
    if !secret_shape(token, "egt_") {
        return Err(Error::Unauthorized.into());
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let at = clock()?;
    // Revocation remains idempotent after parent logout/expiry; it grants no read or write authority.
    let id: Option<String> = tx
        .query_row(
            "SELECT grant_id FROM game_access_grants WHERE token_hash=?1 AND policy_digest=?2",
            params![hash(token), policy.digest],
            |r| r.get(0),
        )
        .optional()?;
    revoke_on(&tx, &id.ok_or(Error::Unauthorized)?, expected, at)?;
    tx.commit()?;
    Ok(())
}
