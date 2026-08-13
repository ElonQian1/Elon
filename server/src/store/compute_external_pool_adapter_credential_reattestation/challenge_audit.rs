use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_credential_reattestation::{
    canonical_credential_reattestation_json, credential_reattestation_challenge,
    validate_credential_reattestation_binding, ExternalPoolAdapterCredentialReattestationBinding,
    ExternalPoolAdapterCredentialReattestationChallenge,
};

struct StoredChallenge {
    challenge: ExternalPoolAdapterCredentialReattestationChallenge,
    json: String,
    columns: ChallengeColumns,
}

struct ChallengeColumns {
    nonce: String,
    nonce_digest: String,
    message: String,
    message_digest: String,
    provider_binding_id: String,
    provider_binding_digest: String,
    provider_binding_material_digest: String,
    release_id: String,
    release_digest: String,
    release_material_digest: String,
    verifier_key_record_id: String,
    verifier_key_record_digest: String,
    verifier_key_id: String,
    observed_provider_revision: i64,
    observed_provider_digest: String,
    observed_provider_status: String,
    sequence: i64,
    predecessor_id: Option<String>,
    predecessor_digest: Option<String>,
    issued_at: String,
    expires_at: String,
}

pub(super) fn challenge_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterCredentialReattestationChallenge>> {
    conn.query_row(
        "SELECT challenge_nonce_base64,challenge_nonce_digest,signature_message_base64,
                signature_message_digest,provider_binding_id,provider_binding_digest,
                provider_binding_material_digest,registry_release_id,registry_release_digest,
                registry_release_material_digest,credential_verifier_key_record_id,
                credential_verifier_key_record_digest,credential_verifier_key_id,
                observed_provider_policy_revision,observed_provider_digest,
                observed_provider_status,sequence,predecessor_receipt_id,
                predecessor_receipt_digest,challenge_json,issued_at,expires_at
           FROM compute_external_pool_adapter_credential_reattestation_challenges
          WHERE challenge_id=?1",
        params![id],
        |row| {
            let json: String = row.get(19)?;
            let value: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(19, Type::Text, Box::new(error))
            })?;
            let binding: ExternalPoolAdapterCredentialReattestationBinding =
                serde_json::from_value(value.get("binding").cloned().unwrap_or_default()).map_err(
                    |error| {
                        rusqlite::Error::FromSqlConversionFailure(19, Type::Text, Box::new(error))
                    },
                )?;
            let challenge = credential_reattestation_challenge(binding).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(19, Type::Text, error.into())
            })?;
            Ok(StoredChallenge {
                challenge,
                json,
                columns: ChallengeColumns {
                    nonce: row.get(0)?,
                    nonce_digest: row.get(1)?,
                    message: row.get(2)?,
                    message_digest: row.get(3)?,
                    provider_binding_id: row.get(4)?,
                    provider_binding_digest: row.get(5)?,
                    provider_binding_material_digest: row.get(6)?,
                    release_id: row.get(7)?,
                    release_digest: row.get(8)?,
                    release_material_digest: row.get(9)?,
                    verifier_key_record_id: row.get(10)?,
                    verifier_key_record_digest: row.get(11)?,
                    verifier_key_id: row.get(12)?,
                    observed_provider_revision: row.get(13)?,
                    observed_provider_digest: row.get(14)?,
                    observed_provider_status: row.get(15)?,
                    sequence: row.get(16)?,
                    predecessor_id: row.get(17)?,
                    predecessor_digest: row.get(18)?,
                    issued_at: row.get(20)?,
                    expires_at: row.get(21)?,
                },
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
) -> Result<ExternalPoolAdapterCredentialReattestationChallenge> {
    validate_credential_reattestation_binding(&stored.challenge.binding)?;
    let b = &stored.challenge.binding;
    let c = &stored.columns;
    if b.challenge_id != expected_id
        || canonical_credential_reattestation_json(&stored.challenge)? != stored.json
        || b.challenge_nonce_base64 != c.nonce
        || b.challenge_nonce_digest != c.nonce_digest
        || stored.challenge.signature_message_base64 != c.message
        || stored.challenge.signature_message_digest != c.message_digest
        || b.provider_binding_id != c.provider_binding_id
        || b.provider_binding_digest != c.provider_binding_digest
        || b.provider_binding_material_digest != c.provider_binding_material_digest
        || b.registry_release_id != c.release_id
        || b.registry_release_digest != c.release_digest
        || b.registry_release_material_digest != c.release_material_digest
        || b.credential_verifier_key_record_id != c.verifier_key_record_id
        || b.credential_verifier_key_record_digest != c.verifier_key_record_digest
        || b.credential_verifier_key_id != c.verifier_key_id
        || b.observed_provider_policy_revision != c.observed_provider_revision
        || b.observed_provider_digest != c.observed_provider_digest
        || b.observed_provider_status != c.observed_provider_status
        || i64::try_from(b.sequence)? != c.sequence
        || b.predecessor_receipt_id != c.predecessor_id
        || b.predecessor_receipt_digest != c.predecessor_digest
        || b.challenge_issued_at != c.issued_at
        || b.challenge_expires_at != c.expires_at
    {
        bail!("credential re-attestation challenge failed exact readback audit");
    }
    Ok(stored.challenge)
}
