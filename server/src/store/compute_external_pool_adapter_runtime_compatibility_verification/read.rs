use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_runtime_compatibility_verification::*,
    store::{
        compute_external_pool_adapter_sandbox_verifier_key::sandbox_verifier_key_record_authority_on,
        Store,
    },
};

use super::{
    error::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError as StoreError, types::*,
};

pub(super) fn challenge_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRuntimeCompatibilityChallenge>> {
    challenge_on(conn, "challenge_id=?1", params![id])
}

pub(super) fn challenge_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRuntimeCompatibilityChallenge>> {
    challenge_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn challenge_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredRuntimeCompatibilityChallenge>> {
    conn.query_row(
        &format!("SELECT challenge_json FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges WHERE {filter}"),
        values,
        |row| decode(row, 0).map(|(receipt, receipt_json)| StoredRuntimeCompatibilityChallenge { receipt, receipt_json }),
    )
    .optional()?
    .map(audit_challenge)
    .transpose()
}

pub(super) fn run_observation_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRuntimeCompatibilityRunObservation>> {
    observation_on(conn, "run_observation_id=?1", params![id])
}

pub(super) fn run_observation_by_challenge_on(
    conn: &Connection,
    challenge_id: &str,
) -> Result<Option<StoredRuntimeCompatibilityRunObservation>> {
    observation_on(conn, "challenge_id=?1", params![challenge_id])
}

fn observation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredRuntimeCompatibilityRunObservation>> {
    conn.query_row(
        &format!("SELECT run_observation_json FROM compute_external_pool_adapter_runtime_compatibility_verification_run_observations WHERE {filter}"),
        values,
        |row| decode(row, 0).map(|(receipt, receipt_json)| StoredRuntimeCompatibilityRunObservation { receipt, receipt_json }),
    )
    .optional()?
    .map(|stored| audit_observation(conn, stored))
    .transpose()
}

pub(super) fn verification_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRuntimeCompatibilityVerification>> {
    verification_on(conn, "verification_receipt_id=?1", params![id])
}

pub(super) fn verification_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRuntimeCompatibilityVerification>> {
    verification_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn verification_head_by_release_on(
    conn: &Connection,
    release_id: &str,
) -> Result<Option<StoredRuntimeCompatibilityVerification>> {
    verification_on(
        conn,
        "registry_release_id=?1 ORDER BY sequence DESC LIMIT 1",
        params![release_id],
    )
}

fn verification_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredRuntimeCompatibilityVerification>> {
    conn.query_row(
        &format!("SELECT verification_receipt_json FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts WHERE {filter}"),
        values,
        |row| decode(row, 0).map(|(receipt, receipt_json)| StoredRuntimeCompatibilityVerification { receipt, receipt_json }),
    )
    .optional()?
    .map(|stored| audit_verification(conn, stored))
    .transpose()
}

pub(super) fn revocation_by_verification_on(
    conn: &Connection,
    verification_id: &str,
) -> Result<Option<StoredRuntimeCompatibilityRevocation>> {
    revocation_on(conn, "verification_receipt_id=?1", params![verification_id])
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRuntimeCompatibilityRevocation>> {
    revocation_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn revocation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredRuntimeCompatibilityRevocation>> {
    conn.query_row(
        &format!("SELECT revocation_receipt_json FROM compute_external_pool_adapter_runtime_compatibility_verification_revocations WHERE {filter}"),
        values,
        |row| decode(row, 0).map(|(receipt, receipt_json)| StoredRuntimeCompatibilityRevocation { receipt, receipt_json }),
    )
    .optional()?
    .map(|stored| audit_revocation(conn, stored))
    .transpose()
}

fn audit_challenge(
    stored: StoredRuntimeCompatibilityChallenge,
) -> Result<StoredRuntimeCompatibilityChallenge> {
    validate_runtime_compatibility_challenge_receipt(&stored.receipt)?;
    if runtime_compatibility_challenge_json_and_digest(&stored.receipt)?.0 != stored.receipt_json {
        bail!("stored V268 challenge is not canonical");
    }
    Ok(stored)
}

fn audit_observation(
    conn: &Connection,
    stored: StoredRuntimeCompatibilityRunObservation,
) -> Result<StoredRuntimeCompatibilityRunObservation> {
    validate_runtime_compatibility_run_observation_receipt(&stored.receipt)?;
    let challenge = challenge_by_id_on(conn, &stored.receipt.observation.challenge_id)?
        .ok_or_else(|| anyhow::anyhow!("stored V268 observation lost its challenge"))?;
    validate_runtime_compatibility_observation_against_challenge(
        &stored.receipt.observation,
        &challenge.receipt,
    )?;
    if runtime_compatibility_observation_json_and_digest(&stored.receipt)?.0 != stored.receipt_json
    {
        bail!("stored V268 observation is not canonical");
    }
    Ok(stored)
}

fn audit_verification(
    conn: &Connection,
    stored: StoredRuntimeCompatibilityVerification,
) -> Result<StoredRuntimeCompatibilityVerification> {
    let material = &stored.receipt.verification;
    let challenge = challenge_by_id_on(conn, &material.challenge_id)?
        .ok_or_else(|| anyhow::anyhow!("stored V268 verification lost its challenge"))?;
    let observation = run_observation_by_id_on(conn, &material.run_observation_id)?
        .ok_or_else(|| anyhow::anyhow!("stored V268 verification lost its observation"))?;
    validate_runtime_compatibility_verification_receipt(
        &stored.receipt,
        &challenge.receipt,
        &observation.receipt,
    )?;
    let key = sandbox_verifier_key_record_authority_on(
        conn,
        &material.sandbox_verifier_key_record_id,
        &material.sandbox_verifier_key_record_digest,
        &material.sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("stored V268 verification lost its V237 key"))?;
    let signature_challenge =
        runtime_compatibility_signature_challenge(&challenge.receipt, &observation.receipt)?;
    if key.verifier_operator() != material.sandbox_verifier_operator
        || key.verifier_product() != material.sandbox_verifier_product
    {
        bail!("stored V268 verification V237 operator/product roots drifted");
    }
    verify_runtime_compatibility_signature(
        key.public_key_pem(),
        &signature_challenge,
        &material.signature_base64,
    )?;
    if runtime_compatibility_verification_receipt_json_and_digest(&stored.receipt)?.0
        != stored.receipt_json
    {
        bail!("stored V268 verification is not canonical");
    }
    Ok(stored)
}

fn audit_revocation(
    conn: &Connection,
    stored: StoredRuntimeCompatibilityRevocation,
) -> Result<StoredRuntimeCompatibilityRevocation> {
    validate_runtime_compatibility_revocation_receipt(&stored.receipt)?;
    let verification =
        verification_by_id_on(conn, &stored.receipt.revocation.verification_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("stored V268 revocation lost its verification"))?;
    let revocation = &stored.receipt.revocation;
    let sealed = &verification.receipt.verification;
    if revocation.verification_receipt_digest != verification.receipt.verification_receipt_digest
        || revocation.registry_release_id != sealed.registry_release.registry_release_id
        || revocation.registry_release_digest != sealed.registry_release.registry_release_digest
        || runtime_compatibility_revocation_receipt_json_and_digest(&stored.receipt)?.0
            != stored.receipt_json
    {
        bail!("stored V268 revocation roots are not exact");
    }
    Ok(stored)
}

fn decode<T: serde::de::DeserializeOwned>(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<(T, String)> {
    let json: String = row.get(index)?;
    let value = serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })?;
    Ok((value, json))
}

impl Store {
    pub(crate) fn external_pool_adapter_runtime_compatibility_verification_challenge_exists(
        &self,
        challenge_id: &str,
        registry_release_id: &str,
    ) -> std::result::Result<bool, StoreError> {
        identifier(challenge_id).map_err(StoreError::conflict)?;
        identifier(registry_release_id).map_err(StoreError::conflict)?;
        let conn = self.conn().map_err(StoreError::storage)?;
        Ok(challenge_by_id_on(&conn, challenge_id)
            .map_err(StoreError::storage)?
            .is_some_and(|stored| {
                stored
                    .receipt
                    .challenge
                    .registry_release
                    .registry_release_id
                    == registry_release_id
            }))
    }

    pub(crate) fn external_pool_adapter_runtime_compatibility_verification_run_observation_exists(
        &self,
        run_observation_id: &str,
        registry_release_id: &str,
    ) -> std::result::Result<bool, StoreError> {
        identifier(run_observation_id).map_err(StoreError::conflict)?;
        identifier(registry_release_id).map_err(StoreError::conflict)?;
        let conn = self.conn().map_err(StoreError::storage)?;
        Ok(run_observation_by_id_on(&conn, run_observation_id)
            .map_err(StoreError::storage)?
            .is_some_and(|stored| {
                stored
                    .receipt
                    .observation
                    .registry_release
                    .registry_release_id
                    == registry_release_id
            }))
    }

    pub(crate) fn external_pool_adapter_runtime_compatibility_verification_exists(
        &self,
        verification_receipt_id: &str,
        registry_release_id: &str,
    ) -> std::result::Result<bool, StoreError> {
        identifier(verification_receipt_id).map_err(StoreError::conflict)?;
        identifier(registry_release_id).map_err(StoreError::conflict)?;
        let conn = self.conn().map_err(StoreError::storage)?;
        Ok(verification_by_id_on(&conn, verification_receipt_id)
            .map_err(StoreError::storage)?
            .is_some_and(|stored| {
                stored
                    .receipt
                    .verification
                    .registry_release
                    .registry_release_id
                    == registry_release_id
            }))
    }
}

pub(super) fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("V268 identifier is invalid");
    }
    Ok(())
}
