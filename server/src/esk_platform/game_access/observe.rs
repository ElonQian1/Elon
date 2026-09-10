use super::{
    authority::*,
    model::*,
    policy::*,
    protocol::{self, Challenge, Observation},
};
use ed25519_dalek::Signer;
use rusqlite::{params, Connection, TransactionBehavior};

pub fn observe(
    conn: &mut Connection,
    policy: &Policy,
    service_secret: &str,
    token: &str,
    challenge: &Challenge,
    clock: Clock<'_>,
) -> Result<Observation> {
    policy.check_service(service_secret)?;
    protocol::challenge_digest(challenge).map_err(|_| Error::InvalidInput)?;
    if challenge.main_issuer != policy.issuer || !equal(&challenge.credential_digest, &hash(token))
    {
        return Err(Error::InvalidInput.into());
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let at = clock()?;
    let (stored, grant) = token_grant(&tx, policy, token, at)?;
    if !grant
        .scopes
        .iter()
        .any(|s| s == protocol::required_scope(&challenge.action))
    {
        return Err(Error::ScopeMissing.into());
    }
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM game_access_nonces WHERE grant_id=?1 AND nonce=?2)",
        params![stored.id, challenge.nonce],
        |r| r.get(0),
    )?;
    if exists {
        return Err(Error::Replay.into());
    }
    let count: i64 = tx.query_row(
        "SELECT count(*) FROM game_access_nonces WHERE grant_id=?1",
        [&stored.id],
        |r| r.get(0),
    )?;
    if count >= 2048 {
        return Err(Error::Capacity.into());
    }
    let mut observation = Observation {
        challenge: challenge.clone(),
        grant,
        observed_at_ms: at.to_string(),
        key_id: policy.key_id.clone(),
        signature_hex: "00".repeat(64),
    };
    let message = protocol::observation_message(&observation).map_err(|_| Error::Corrupt)?;
    observation.signature_hex = hex::encode(policy.signing_key.sign(&message).to_bytes());
    tx.execute(
        "INSERT INTO game_access_nonces VALUES(?1,?2,?3)",
        params![stored.id, challenge.nonce, at],
    )?;
    // A slow or reversed clock cannot produce a fresh-looking response after a wait.
    let end = clock()?;
    if end < at || end >= stored.expires || end - at >= 5_000 {
        return Err(Error::Unavailable.into());
    }
    tx.commit()?;
    Ok(observation)
}
