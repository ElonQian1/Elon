use super::{authority::*, model::*, policy::*};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

pub fn authorize(
    conn: &mut Connection,
    policy: &Policy,
    user: &str,
    parent_token: &str,
    request: &AuthorizeRequest,
    clock: Clock<'_>,
) -> Result<AuthorizationCode> {
    if request.schema != "esk.game.access.authorize.v1"
        || request.client_id != CLIENT
        || request.redirect_uri != policy.redirect
        || request.code_challenge_method != "S256"
        || !unreserved(&request.state, 43, 128)
        || !valid_challenge(&request.code_challenge)
        || !valid_scopes(&request.scopes)
        || !(1..=900).contains(&request.expires_in_seconds)
        || !request.explicit_consent
        || request.confirmation != CONSENT
        || parent_token.is_empty()
        || parent_token.len() > 8192
    {
        return Err(Error::InvalidInput.into());
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let at = clock()?;
    let parent_hash = hash(parent_token);
    let parent = parent(&tx, user, &parent_hash, at)?;
    let count: i64 = tx.query_row("SELECT count(*) FROM game_access_grants WHERE user_id=?1 AND revoked_ms IS NULL AND expires_ms>?2",
        params![user,at], |r|r.get(0))?;
    if count >= 64 {
        return Err(Error::Capacity.into());
    }
    let expires = at
        .checked_add(i64::from(request.expires_in_seconds) * 1000)
        .ok_or(Error::InvalidInput)?
        .min(parent.expires_ms);
    let code_expires = at
        .checked_add(120_000)
        .ok_or(Error::InvalidInput)?
        .min(expires);
    let id = format!("egg_{}", uuid::Uuid::new_v4().simple());
    let code = secret("egc_")?;
    tx.execute("INSERT INTO game_access_grants(grant_id,user_id,session_id,parent_hash,policy_digest,scopes_json,
        created_ms,expires_ms,code_hash,state_hash,pkce,code_expires_ms) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![id,user,parent.id,parent_hash,policy.digest,serde_json::to_string(&request.scopes)?,at,expires,
            hash(&code),hash(&request.state),request.code_challenge,code_expires])?;
    audit(&tx, &id, "authorized", at)?;
    tx.commit()?;
    Ok(AuthorizationCode {
        schema: "esk.game.access.code.v1",
        code,
        state: request.state.clone(),
        redirect_uri: policy.redirect.clone(),
        grant_id: id,
        code_expires_at_ms: code_expires.to_string(),
        expires_at_ms: expires.to_string(),
        scopes: request.scopes.clone(),
    })
}

pub fn exchange(
    conn: &mut Connection,
    policy: &Policy,
    service_secret: &str,
    request: &ExchangeRequest,
    clock: Clock<'_>,
) -> Result<GameToken> {
    policy.check_service(service_secret)?;
    if request.schema != "esk.game.access.exchange.v1"
        || request.grant_type != "authorization_code"
        || request.client_id != CLIENT
        || request.redirect_uri != policy.redirect
        || !unreserved(&request.state, 43, 128)
        || !secret_shape(&request.code, "egc_")
    {
        return Err(Error::InvalidGrant.into());
    }
    let challenge = pkce(&request.code_verifier).map_err(|_| Error::InvalidGrant)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let at = clock()?;
    let row = tx
        .query_row(
            &format!(
                "SELECT {COLUMNS},state_hash,pkce FROM game_access_grants
        WHERE code_hash=?1 AND consumed_ms IS NULL AND code_expires_ms>?2"
            ),
            params![hash(&request.code), at],
            |r| {
                Ok((
                    StoredGrant::from_row(r)?,
                    r.get::<_, String>(10)?,
                    r.get::<_, String>(11)?,
                ))
            },
        )
        .optional()?
        .ok_or(Error::InvalidGrant)?;
    let (stored, state_hash, expected_pkce) = row;
    let grant = stored.verify(&tx, policy, at)?;
    if !equal(&state_hash, &hash(&request.state)) || !equal(&expected_pkce, &challenge) {
        return Err(Error::InvalidGrant.into());
    }
    let token = secret("egt_")?;
    let changed = tx.execute(
        "UPDATE game_access_grants SET consumed_ms=?1,token_hash=?2
        WHERE grant_id=?3 AND consumed_ms IS NULL AND revoked_ms IS NULL AND code_expires_ms>?1",
        params![at, hash(&token), stored.id],
    )?;
    if changed != 1 {
        return Err(Error::InvalidGrant.into());
    }
    audit(&tx, &stored.id, "exchanged", at)?;
    tx.commit()?;
    Ok(GameToken {
        schema: "esk.game.access.token.v1",
        token_type: "Bearer",
        audience: "esk-game",
        access_token: token,
        grant_id: stored.id,
        expires_at_ms: grant.expires_at_ms,
        scopes: grant.scopes,
    })
}
