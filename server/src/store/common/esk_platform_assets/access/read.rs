use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    clock, hash_token, scopes_from_json, session_expiry_on, timestamp, AuthorizedAssetRead,
};
use crate::{esk_asset::platform::access::*, store::Store};

pub(in crate::store::common::esk_platform_assets) fn verify_read_on(
    conn: &Connection,
    token: &str,
    client_id: &str,
    required_scope: &str,
) -> Result<AuthorizedAssetRead> {
    let authority = verify_token_on(conn, token, client_id, clock()?, false)?;
    if !authority
        .scopes
        .iter()
        .any(|scope| scope.as_str() == required_scope)
    {
        return Err(AccessError::InsufficientScope.into());
    }
    Ok(authority)
}

pub(super) fn verify_token_on(
    conn: &Connection,
    token: &str,
    client_id: &str,
    at: i64,
    allow_revoked: bool,
) -> Result<AuthorizedAssetRead> {
    if !valid_secret(token, "aat_") || !valid_client(client_id) {
        return Err(AccessError::Unauthorized.into());
    }
    let row = conn.query_row(
        "SELECT g.user_id,g.subject,g.grant_id,g.client_id,g.scopes_json,g.expires_at_unix,
             g.parent_session_hash,g.created_at_unix,t.created_at_unix,t.expires_at_unix,
             g.revoked_at_unix,t.revoked_at_unix
         FROM asset_access_tokens t JOIN asset_access_grants g ON g.grant_id=t.grant_id
         JOIN asset_access_subjects sub ON sub.user_id=g.user_id AND sub.client_id=g.client_id AND sub.subject=g.subject
         WHERE t.token_hash=?1 AND g.client_id=?2",
        params![hash_token(token),client_id], |row|Ok((
            row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,
            row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,i64>(5)?,
            row.get::<_,String>(6)?,row.get::<_,i64>(7)?,row.get::<_,i64>(8)?,row.get::<_,i64>(9)?,
            row.get::<_,Option<i64>>(10)?,row.get::<_,Option<i64>>(11)?,
        ))
    ).optional()?.ok_or(AccessError::Unauthorized)?;
    let (
        user_id,
        subject,
        grant_id,
        client_id,
        scope_json,
        expires,
        parent_hash,
        created,
        token_created,
        token_expires,
        revoked,
        token_revoked,
    ) = row;
    if expires <= at
        || token_expires != expires
        || created > token_created
        || token_created > at
        || created <= 0
        || expires - created > MAX_GRANT_SECONDS
        || (!allow_revoked && (revoked.is_some() || token_revoked.is_some()))
    {
        return Err(AccessError::Unauthorized.into());
    }
    let parent_expiry = session_expiry_on(conn, &user_id, &parent_hash, at)?;
    let scopes = scopes_from_json(&scope_json)?;
    Ok(AuthorizedAssetRead {
        user_id,
        subject,
        grant_id,
        client_id,
        scopes,
        expires_at: timestamp(expires.min(parent_expiry))?,
    })
}

pub(super) fn profile_on(conn: &Connection, read: &AuthorizedAssetRead) -> Result<Option<String>> {
    if !read.scopes.contains(&AccessScope::ProfileRead) {
        return Ok(None);
    }
    // Bound the SQLite result before allocation; never return control characters to a client UI.
    let nickname: String = conn.query_row(
        "SELECT substr(COALESCE(nickname,''),1,1024) FROM users WHERE id=?1",
        params![read.user_id],
        |row| row.get(0),
    )?;
    Ok(Some(
        nickname
            .chars()
            .filter(|value| !value.is_control())
            .take(128)
            .collect(),
    ))
}

impl Store {
    /// Resolves only a real session and never updates last_seen_at or the session expiry.
    pub(crate) fn asset_access_owner_id(&self, session_token: &str) -> Result<String> {
        if session_token.is_empty() || session_token.len() > 8192 {
            return Err(AccessError::Unauthorized.into());
        }
        let conn = self.conn()?;
        let hash = hash_token(session_token);
        let user: Option<String> = conn
            .query_row(
                "SELECT user_id FROM sessions WHERE token_hash=?1",
                params![hash],
                |row| row.get(0),
            )
            .optional()?;
        let user = user.ok_or(AccessError::Unauthorized)?;
        session_expiry_on(&conn, &user, &hash, clock()?)?;
        Ok(user)
    }

    pub(crate) fn asset_access_me(&self, token: &str, client_id: &str) -> Result<AccessIdentity> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let read = verify_read_on(&tx, token, client_id, "esk.summary.read")?;
        let nickname = profile_on(&tx, &read)?;
        let identity = AccessIdentity {
            schema: "yilong.asset_access.identity.v1",
            audience: AUDIENCE,
            subject: read.subject,
            client_id: read.client_id,
            grant_id: read.grant_id,
            expires_at: read.expires_at,
            scopes: read.scopes,
            nickname,
        };
        tx.commit()?;
        Ok(identity)
    }

    pub(crate) fn list_asset_access_grants(
        &self,
        user_id: &str,
        session_token: &str,
    ) -> Result<Vec<GrantOverview>> {
        let at = clock()?;
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        session_expiry_on(&tx, user_id, &hash_token(session_token), at)?;
        let mut stmt=tx.prepare("SELECT grant_id,client_id,subject,scopes_json,created_at_unix,expires_at_unix,
             revoked_at_unix,parent_session_hash FROM asset_access_grants WHERE user_id=?1
             ORDER BY (revoked_at_unix IS NULL AND expires_at_unix>?2) DESC,created_at_unix DESC,grant_id DESC LIMIT 100")?;
        let mut rows = stmt.query(params![user_id, at])?;
        let mut grants = Vec::new();
        while let Some(row) = rows.next()? {
            let created: i64 = row.get(4)?;
            let expires: i64 = row.get(5)?;
            let revoked: Option<i64> = row.get(6)?;
            let parent_hash: String = row.get(7)?;
            let status = if revoked.is_some() {
                "revoked"
            } else if expires <= at {
                "expired"
            } else if session_expiry_on(&tx, user_id, &parent_hash, at).is_err() {
                "session_invalid"
            } else {
                "active"
            };
            grants.push(GrantOverview {
                grant_id: row.get(0)?,
                client_id: row.get(1)?,
                subject: row.get(2)?,
                scopes: scopes_from_json(&row.get::<_, String>(3)?)?,
                created_at: timestamp(created)?,
                expires_at: timestamp(expires)?,
                status,
            });
        }
        drop(rows);
        drop(stmt);
        tx.commit()?;
        Ok(grants)
    }
}
