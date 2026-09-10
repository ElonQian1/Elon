use super::{wire, *};
use ed25519_dalek::{Signature, VerifyingKey};

pub fn verify_observation_json(
    input: &[u8],
    expected: &Challenge,
    authority: &Authority,
    now_ms: u64,
) -> Result<VerifiedObservation, Error> {
    if input.len() > 16_384 {
        return Err(Error::InvalidContract);
    }
    let observation = serde_json::from_slice(input).map_err(|_| Error::InvalidContract)?;
    verify_observation(&observation, expected, authority, now_ms)
}

pub fn verify_observation(
    observation: &Observation,
    expected: &Challenge,
    authority: &Authority,
    now_ms: u64,
) -> Result<VerifiedObservation, Error> {
    wire::id(&authority.main_issuer)?;
    wire::id(&authority.key_id)?;
    if now_ms > i64::MAX as u64 {
        return Err(Error::InvalidContract);
    }
    if challenge_digest(&observation.challenge)? != challenge_digest(expected)? {
        return Err(Error::ChallengeMismatch);
    }
    if expected.main_issuer != authority.main_issuer || observation.key_id != authority.key_id {
        return Err(Error::AuthorityMismatch);
    }
    let message = observation_message(observation)?;
    let now = u128::from(now_ms);
    let observed = u128::from(wire::units(&observation.observed_at_ms)?);
    let from = u128::from(wire::units(&observation.grant.not_before_ms)?);
    let until = u128::from(wire::units(&observation.grant.expires_at_ms)?);
    if now < from
        || now >= until
        || observed < from
        || observed >= until
        || observed > now + 2_000
        || now >= observed + 10_000
    {
        return Err(Error::ObservationExpired);
    }
    if !observation
        .grant
        .scopes
        .iter()
        .any(|s| s == expected.action.scope())
    {
        return Err(Error::ScopeMissing);
    }
    let key =
        VerifyingKey::from_bytes(&authority.public_key).map_err(|_| Error::SignatureInvalid)?;
    let signature = Signature::from_slice(&wire::hex(&observation.signature_hex, 64)?)
        .map_err(|_| Error::SignatureInvalid)?;
    key.verify_strict(&message, &signature)
        .map_err(|_| Error::SignatureInvalid)?;
    Ok(VerifiedObservation {
        main_user_id: observation.grant.main_user_id.clone(),
        main_session_id: observation.grant.main_session_id.clone(),
        grant_id: observation.grant.grant_id.clone(),
        revision: observation.grant.revision.clone(),
        valid_until_ms: until.min(observed + 10_000).to_string(),
        authorization_digest: authorization_digest(observation)?,
        wallet_bound: false,
        funds_moved: false,
    })
}
