use super::{Challenge, Error, Grant, Observation};
use sha2::{Digest, Sha256};

pub(super) fn id(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
    {
        return Err(Error::InvalidContract);
    }
    Ok(())
}
pub(super) fn hex(value: &str, len: usize) -> Result<Vec<u8>, Error> {
    if value.len() != len * 2
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::InvalidContract);
    }
    Ok(value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let n = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
            n(pair[0]) * 16 + n(pair[1])
        })
        .collect())
}
pub(super) fn units(value: &str) -> Result<u64, Error> {
    if value.is_empty()
        || value.len() > 19
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error::InvalidContract);
    }
    let n: u64 = value.parse().map_err(|_| Error::InvalidContract)?;
    if n > i64::MAX as u64 {
        return Err(Error::InvalidContract);
    }
    Ok(n)
}
fn canonical(values: &[&str]) -> Vec<u8> {
    serde_json::to_vec(values).expect("string array")
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn challenge_digest(challenge: &Challenge) -> Result<String, Error> {
    if challenge.domain != "esk.game.session.challenge.v1"
        || challenge.audience != "esk-game"
        || challenge.stage != "platform_recorded"
    {
        return Err(Error::InvalidContract);
    }
    id(&challenge.main_issuer)?;
    hex(&challenge.nonce, 32)?;
    hex(&challenge.credential_digest, 32)?;
    let action = challenge.action.values();
    for value in &action {
        id(value)?;
    }
    let action_digest = digest(&canonical(&action));
    Ok(digest(&canonical(&[
        &challenge.domain,
        &challenge.main_issuer,
        &challenge.audience,
        &challenge.stage,
        &challenge.nonce,
        &challenge.credential_digest,
        &action_digest,
    ])))
}

fn validate_grant(grant: &Grant) -> Result<(), Error> {
    id(&grant.main_user_id)?;
    id(&grant.main_session_id)?;
    id(&grant.grant_id)?;
    let revision = units(&grant.revision)?;
    let from = units(&grant.not_before_ms)?;
    let until = units(&grant.expires_at_ms)?;
    if revision == 0 || revision > u32::MAX.into() || until <= from || until - from > 900_000 {
        return Err(Error::InvalidContract);
    }
    if grant.scopes.is_empty() || grant.scopes.len() > 4 || grant.scopes[0] != "play" {
        return Err(Error::InvalidContract);
    }
    let order = ["play", "inventory_read", "redeem", "principal_withdraw"];
    let mut previous = None;
    for scope in &grant.scopes {
        let index = order
            .iter()
            .position(|s| s == scope)
            .ok_or(Error::InvalidContract)?;
        if previous.is_some_and(|prev| index <= prev) {
            return Err(Error::InvalidContract);
        }
        previous = Some(index);
    }
    Ok(())
}

pub fn observation_message(observation: &Observation) -> Result<Vec<u8>, Error> {
    id(&observation.key_id)?;
    hex(&observation.signature_hex, 64)?;
    units(&observation.observed_at_ms)?;
    let challenge_digest = challenge_digest(&observation.challenge)?;
    let g = &observation.grant;
    validate_grant(g)?;
    let scopes = g.scopes.join(",");
    Ok(canonical(&[
        "esk.game.session.observation.v1",
        &observation.key_id,
        &challenge_digest,
        &g.main_user_id,
        &g.main_session_id,
        &g.grant_id,
        &g.revision,
        &g.not_before_ms,
        &g.expires_at_ms,
        &scopes,
        &observation.observed_at_ms,
    ]))
}

pub fn authorization_digest(observation: &Observation) -> Result<String, Error> {
    let message_digest = digest(&observation_message(observation)?);
    Ok(digest(&canonical(&[
        "esk.game.session.evidence.v1",
        &message_digest,
        &observation.signature_hex,
    ])))
}
