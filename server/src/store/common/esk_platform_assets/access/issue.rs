use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::{
    audit, clock, equal, hash_token, new_id, random_secret, scopes_from_json, session_expiry_on,
    timestamp,
};
use crate::{esk_asset::platform::access::*, store::Store};

impl Store {
    pub(crate) fn authorize_asset_access(
        &self,
        user_id: &str,
        session_token: &str,
        body: &AuthorizeBody,
        public_url: &str,
    ) -> Result<AuthorizationCode> {
        let mut conn = self.conn()?;
        authorize_on(&mut conn, user_id, session_token, body, public_url)
    }

    pub(crate) fn exchange_asset_access_code(
        &self,
        body: &TokenBody,
        public_url: &str,
    ) -> Result<AccessToken> {
        let mut conn = self.conn()?;
        exchange_on(&mut conn, body, public_url)
    }
}

pub(super) fn authorize_on(
    conn: &mut Connection,
    user_id: &str,
    session_token: &str,
    body: &AuthorizeBody,
    public_url: &str,
) -> Result<AuthorizationCode> {
    validate_authorize(body, public_url)?;
    if session_token.is_empty() || session_token.len() > 8192 {
        return Err(AccessError::Unauthorized.into());
    }
    let at = clock()?;
    let parent_hash = hash_token(session_token);
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let parent_expiry = session_expiry_on(&tx, user_id, &parent_hash, at)?;
    let outstanding: i64 = tx.query_row(
        "SELECT COUNT(*) FROM asset_access_grants WHERE user_id=?1
               AND revoked_at_unix IS NULL AND expires_at_unix>?2",
        params![user_id, at],
        |row| row.get(0),
    )?;
    if outstanding >= 64 {
        return Err(AccessError::Capacity.into());
    }
    let subject = tx
        .query_row(
            "SELECT subject FROM asset_access_subjects WHERE user_id=?1 AND client_id=?2",
            params![user_id, body.client_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let subject = match subject {
        Some(value) => value,
        None => {
            let value = random_secret("aas_")?;
            tx.execute(
                "INSERT INTO asset_access_subjects(user_id,client_id,subject) VALUES (?1,?2,?3)",
                params![user_id, body.client_id, value],
            )?;
            value
        }
    };
    let expires = at
        .checked_add(body.expires_in)
        .ok_or(AccessError::InvalidInput)?
        .min(parent_expiry);
    if expires <= at {
        return Err(AccessError::Unauthorized.into());
    }
    let code_expires = expires.min(at + CODE_LIFETIME_SECONDS);
    let grant_id = new_id("aag");
    let mut scopes = body.scopes.clone();
    scopes.sort_unstable();
    tx.execute(
        "INSERT INTO asset_access_grants(grant_id,user_id,client_id,subject,parent_session_hash,
                scopes_json,created_at_unix,expires_at_unix) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            grant_id,
            user_id,
            body.client_id,
            subject,
            parent_hash,
            serde_json::to_string(&scopes)?,
            at,
            expires
        ],
    )?;
    let code = random_secret("aac_")?;
    tx.execute(
        "INSERT INTO asset_access_codes(code_hash,grant_id,redirect_uri,state_hash,
            code_challenge,created_at_unix,expires_at_unix) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            hash_token(&code),
            grant_id,
            body.redirect_uri,
            hash_token(&body.state),
            body.code_challenge,
            at,
            code_expires
        ],
    )?;
    audit(&tx, &grant_id, "authorized", at)?;
    tx.commit()?;
    Ok(AuthorizationCode {
        schema: "yilong.asset_access.authorization_code.v1",
        code,
        state: body.state.clone(),
        client_id: body.client_id.clone(),
        redirect_uri: body.redirect_uri.clone(),
        code_expires_at: timestamp(code_expires)?,
        grant_id,
        expires_at: timestamp(expires)?,
        scopes,
    })
}

pub(super) fn exchange_on(
    conn: &mut Connection,
    body: &TokenBody,
    public_url: &str,
) -> Result<AccessToken> {
    exchange_at(conn, body, public_url, clock()?)
}

pub(super) fn exchange_at(
    conn: &mut Connection,
    body: &TokenBody,
    public_url: &str,
    at: i64,
) -> Result<AccessToken> {
    validate_exchange(body, public_url)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let row = tx.query_row(
            "SELECT c.grant_id,c.redirect_uri,c.state_hash,c.code_challenge,
                g.user_id,g.client_id,g.subject,g.parent_session_hash,g.scopes_json,g.expires_at_unix,
                g.created_at_unix
             FROM asset_access_codes c JOIN asset_access_grants g ON g.grant_id=c.grant_id
             JOIN asset_access_subjects sub ON sub.user_id=g.user_id AND sub.client_id=g.client_id AND sub.subject=g.subject
             WHERE c.code_hash=?1 AND c.consumed_at_unix IS NULL AND c.expires_at_unix>?2
               AND c.created_at_unix<=?2 AND c.expires_at_unix<=g.expires_at_unix
               AND g.revoked_at_unix IS NULL AND g.expires_at_unix>?2 AND g.created_at_unix<=?2",
            params![hash_token(&body.code),at], |row| Ok((
                row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,
                row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,
                row.get::<_,String>(6)?,row.get::<_,String>(7)?,row.get::<_,String>(8)?,
                row.get::<_,i64>(9)?,row.get::<_,i64>(10)?,
            ))
        ).optional()?.ok_or(AccessError::InvalidGrant)?;
    let (
        grant_id,
        redirect,
        state_hash,
        expected_challenge,
        user_id,
        client_id,
        subject,
        parent_hash,
        scopes_json,
        expires,
        created,
    ) = row;
    if redirect != body.redirect_uri
        || client_id != body.client_id
        || !equal(&state_hash, &hash_token(&body.state))
        || !equal(&expected_challenge, &challenge(&body.code_verifier)?)
        || expires - created > MAX_GRANT_SECONDS
    {
        return Err(AccessError::InvalidGrant.into());
    }
    let parent_expiry = session_expiry_on(&tx, &user_id, &parent_hash, at)
        .map_err(|_| AccessError::InvalidGrant)?;
    if parent_expiry < expires {
        return Err(AccessError::InvalidGrant.into());
    }
    let scopes = scopes_from_json(&scopes_json).map_err(|_| AccessError::InvalidGrant)?;
    let updated = tx.execute(
        "UPDATE asset_access_codes SET consumed_at_unix=?1
            WHERE code_hash=?2 AND consumed_at_unix IS NULL AND expires_at_unix>?1",
        params![at, hash_token(&body.code)],
    )?;
    if updated != 1 {
        return Err(AccessError::InvalidGrant.into());
    }
    let access_token = random_secret("aat_")?;
    tx.execute(
        "INSERT INTO asset_access_tokens(token_hash,grant_id,created_at_unix,expires_at_unix)
            VALUES (?1,?2,?3,?4)",
        params![hash_token(&access_token), grant_id, at, expires],
    )?;
    audit(&tx, &grant_id, "exchanged", at)?;
    tx.commit()?;
    Ok(AccessToken {
        schema: "yilong.asset_access.token.v1",
        token_type: "Bearer",
        access_token,
        audience: AUDIENCE,
        subject,
        client_id,
        grant_id,
        expires_in: expires - at,
        expires_at: timestamp(expires)?,
        scopes,
    })
}
