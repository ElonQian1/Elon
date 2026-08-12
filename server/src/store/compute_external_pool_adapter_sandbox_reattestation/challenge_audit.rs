use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_sandbox_reattestation::{
    canonical_sandbox_reattestation_json, sandbox_reattestation_challenge,
    validate_sandbox_reattestation_binding, ExternalPoolAdapterSandboxReattestationBinding,
    ExternalPoolAdapterSandboxReattestationChallenge,
};

struct StoredChallenge {
    challenge: ExternalPoolAdapterSandboxReattestationChallenge,
    json: String,
    nonce: String,
    nonce_digest: String,
    message: String,
    message_digest: String,
    release_id: String,
    release_digest: String,
    release_material_digest: String,
    vulnerability_id: String,
    vulnerability_digest: String,
    vulnerability_material_digest: String,
    verifier_id: String,
    verifier_digest: String,
    verifier_key_id: String,
    sequence: i64,
    predecessor_id: Option<String>,
    predecessor_digest: Option<String>,
    issued_at: String,
    expires_at: String,
}

pub(super) fn challenge_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterSandboxReattestationChallenge>> {
    conn.query_row(
        "SELECT challenge_nonce_base64,challenge_nonce_digest,signature_message_base64,
                signature_message_digest,registry_release_id,registry_release_digest,
                registry_release_material_digest,vulnerability_reattestation_receipt_id,
                vulnerability_reattestation_receipt_digest,vulnerability_reattestation_material_digest,
                sandbox_verifier_key_record_id,sandbox_verifier_key_record_digest,
                sandbox_verifier_key_id,sequence,predecessor_receipt_id,predecessor_receipt_digest,
                challenge_json,issued_at,expires_at
           FROM compute_external_pool_adapter_sandbox_reattestation_challenges
          WHERE challenge_id=?1",
        params![id],
        |row| {
            let json: String = row.get(16)?;
            let value: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(16, Type::Text, Box::new(error))
            })?;
            let binding: ExternalPoolAdapterSandboxReattestationBinding = serde_json::from_value(
                value.get("binding").cloned().unwrap_or_default(),
            )
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(16, Type::Text, Box::new(error))
            })?;
            let challenge = sandbox_reattestation_challenge(binding).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(16, Type::Text, error.into())
            })?;
            Ok(StoredChallenge {
                challenge,
                json,
                nonce: row.get(0)?,
                nonce_digest: row.get(1)?,
                message: row.get(2)?,
                message_digest: row.get(3)?,
                release_id: row.get(4)?,
                release_digest: row.get(5)?,
                release_material_digest: row.get(6)?,
                vulnerability_id: row.get(7)?,
                vulnerability_digest: row.get(8)?,
                vulnerability_material_digest: row.get(9)?,
                verifier_id: row.get(10)?,
                verifier_digest: row.get(11)?,
                verifier_key_id: row.get(12)?,
                sequence: row.get(13)?,
                predecessor_id: row.get(14)?,
                predecessor_digest: row.get(15)?,
                issued_at: row.get(17)?,
                expires_at: row.get(18)?,
            })
        },
    )
    .optional()?
    .map(|stored| audit(stored, id))
    .transpose()
}

fn audit(
    stored: StoredChallenge,
    expected_id: &str,
) -> Result<ExternalPoolAdapterSandboxReattestationChallenge> {
    validate_sandbox_reattestation_binding(&stored.challenge.binding)?;
    let b = &stored.challenge.binding;
    if b.challenge_id != expected_id
        || canonical_sandbox_reattestation_json(&stored.challenge)? != stored.json
        || b.challenge_nonce_base64 != stored.nonce
        || b.challenge_nonce_digest != stored.nonce_digest
        || stored.challenge.signature_message_base64 != stored.message
        || stored.challenge.signature_message_digest != stored.message_digest
        || b.registry_release_id != stored.release_id
        || b.registry_release_digest != stored.release_digest
        || b.registry_release_material_digest != stored.release_material_digest
        || b.vulnerability_reattestation_receipt_id != stored.vulnerability_id
        || b.vulnerability_reattestation_receipt_digest != stored.vulnerability_digest
        || b.vulnerability_reattestation_material_digest != stored.vulnerability_material_digest
        || b.sandbox_verifier_key_record_id != stored.verifier_id
        || b.sandbox_verifier_key_record_digest != stored.verifier_digest
        || b.sandbox_verifier_key_id != stored.verifier_key_id
        || i64::try_from(b.sequence)? != stored.sequence
        || b.predecessor_receipt_id != stored.predecessor_id
        || b.predecessor_receipt_digest != stored.predecessor_digest
        || b.challenge_issued_at != stored.issued_at
        || b.challenge_expires_at != stored.expires_at
    {
        bail!("sandbox re-attestation challenge failed exact readback audit");
    }
    Ok(stored.challenge)
}
