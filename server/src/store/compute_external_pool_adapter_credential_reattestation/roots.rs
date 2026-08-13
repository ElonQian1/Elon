use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::compute_federation::{
    external_pool_adapter_credential_verifier::{
        credential_verifier_record_json_and_digest, credential_verifier_transition_json_and_digest,
        validate_credential_verifier_record, validate_credential_verifier_transition,
        ExternalPoolAdapterCredentialVerifierRecord,
        ExternalPoolAdapterCredentialVerifierTransitionReceipt,
    },
    external_pool_adapter_credential_verifier_key::{
        record_json_and_digest as key_record_json_and_digest,
        validate_record as validate_key_record, CredentialVerifierKeyRecord,
    },
    provider::ComputeProvider,
};
use crate::store::compute_provider_registry::registered_provider_version_on;

pub(super) fn historical_observed_provider_on(
    conn: &Connection,
    provider_id: &str,
    policy_revision: i64,
    expected_digest: &str,
) -> Result<Option<ComputeProvider>> {
    let Some(receipt) = registered_provider_version_on(conn, provider_id, policy_revision)? else {
        return Ok(None);
    };
    if receipt.provider_digest != expected_digest {
        bail!("credential re-attestation observed Provider history is not exact");
    }
    Ok(Some(receipt.provider))
}

pub(super) fn historical_credential_verifier_on(
    conn: &Connection,
    record_id: &str,
    expected_digest: &str,
) -> Result<Option<ExternalPoolAdapterCredentialVerifierRecord>> {
    conn.query_row(
        "SELECT verifier_record_json FROM compute_external_pool_adapter_credential_verifiers
          WHERE verifier_record_id=?1",
        params![record_id],
        |row| {
            let json: String = row.get(0)?;
            let record = serde_json::from_str(&json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok((json, record))
        },
    )
    .optional()?
    .map(|(json, record)| {
        audit_verifier_projection(conn, &json, &record, expected_digest)?;
        audit_verifier_activation(conn, &record)?;
        Ok(record)
    })
    .transpose()
}

pub(super) fn historical_credential_verifier_key_on(
    conn: &Connection,
    record_id: &str,
    expected_digest: &str,
    expected_key_id: &str,
) -> Result<Option<CredentialVerifierKeyRecord>> {
    conn.query_row(
        "SELECT key_record_json FROM compute_external_pool_adapter_credential_verifier_keys
          WHERE key_record_id=?1",
        params![record_id],
        |row| {
            let json: String = row.get(0)?;
            let record = serde_json::from_str(&json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok((json, record))
        },
    )
    .optional()?
    .map(|(json, record)| {
        audit_key_projection(conn, &json, &record, expected_digest, expected_key_id)?;
        Ok(record)
    })
    .transpose()
}

fn audit_verifier_projection(
    conn: &Connection,
    json: &str,
    record: &ExternalPoolAdapterCredentialVerifierRecord,
    expected_digest: &str,
) -> Result<()> {
    validate_credential_verifier_record(record)?;
    let (canonical, digest) = credential_verifier_record_json_and_digest(record)?;
    let item = &record.registration;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_verifiers
          WHERE verifier_record_id=?1 AND verifier_record_schema=?2
            AND verifier_record_digest=?3 AND verifier_record_json=?4
            AND registration_material_digest=?5 AND canonicalization=?6
            AND digest_algorithm=?7 AND verifier_operator=?8 AND verifier_product=?9
            AND verification_kind=?10 AND verifier_id=?11 AND verifier_revision=?12
            AND verifier_digest=?13 AND actor_kind=?14 AND created_by_admin_user_id=?15
            AND confirmation=?16 AND idempotency_scope=?17 AND idempotency_key=?18
            AND created_at=?19 AND recorded_at=?20 AND currentness_effect=?21
            AND credential_receipt_effect=?22 AND adapter_adoption_effect=?23
            AND route_effect=?24 AND execution_effect=?25",
            params![
                record.verifier_record_id,
                record.schema,
                record.verifier_record_digest,
                json,
                record.registration_material_digest,
                record.canonicalization,
                record.digest_algorithm,
                item.verifier_operator,
                item.verifier_product,
                item.verification_kind,
                item.verifier_id,
                item.verifier_revision,
                item.verifier_digest,
                item.actor_kind,
                item.created_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.created_at,
                item.recorded_at,
                item.currentness_effect,
                item.credential_receipt_effect,
                item.adapter_adoption_effect,
                item.route_effect,
                item.execution_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if canonical != json
        || digest != record.verifier_record_digest
        || record.verifier_record_digest != expected_digest
        || !exact
    {
        bail!("credential re-attestation V241 history failed exact audit");
    }
    Ok(())
}

fn audit_verifier_activation(
    conn: &Connection,
    record: &ExternalPoolAdapterCredentialVerifierRecord,
) -> Result<()> {
    let (json, transition): (
        String,
        ExternalPoolAdapterCredentialVerifierTransitionReceipt,
    ) = conn.query_row(
        "SELECT transition_receipt_json
               FROM compute_external_pool_adapter_credential_verifier_transitions
              WHERE verifier_record_id=?1 AND transition_kind='activation'",
        params![record.verifier_record_id],
        |row| {
            let json: String = row.get(0)?;
            let receipt = serde_json::from_str(&json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok((json, receipt))
        },
    )?;
    validate_credential_verifier_transition(&transition)?;
    let (canonical, digest) = credential_verifier_transition_json_and_digest(&transition)?;
    let item = &transition.transition;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_verifier_transitions
          WHERE transition_receipt_id=?1 AND transition_receipt_schema=?2
            AND transition_receipt_digest=?3 AND transition_receipt_json=?4
            AND transition_material_digest=?5 AND transition_kind='activation'
            AND canonicalization=?6 AND digest_algorithm=?7 AND verifier_record_id=?8
            AND verifier_record_digest=?9 AND verification_kind=?10 AND verifier_id=?11
            AND verifier_revision=?12 AND verifier_digest=?13 AND verifier_operator=?14
            AND verifier_product=?15 AND actor_kind=?16 AND actor_user_id=?17
            AND reason IS ?18 AND confirmation=?19 AND idempotency_scope=?20
            AND idempotency_key=?21 AND occurred_at=?22 AND recorded_at=?23
            AND currentness_effect=?24 AND credential_receipt_effect=?25
            AND adapter_adoption_effect=?26 AND route_effect=?27 AND execution_effect=?28",
            params![
                transition.transition_receipt_id,
                transition.schema,
                transition.transition_receipt_digest,
                json,
                transition.transition_material_digest,
                transition.canonicalization,
                transition.digest_algorithm,
                item.verifier_record_id,
                item.verifier_record_digest,
                item.verification_kind,
                item.verifier_id,
                item.verifier_revision,
                item.verifier_digest,
                item.verifier_operator,
                item.verifier_product,
                item.actor_kind,
                item.actor_user_id,
                item.reason,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.occurred_at,
                item.recorded_at,
                item.currentness_effect,
                item.credential_receipt_effect,
                item.adapter_adoption_effect,
                item.route_effect,
                item.execution_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if canonical != json
        || digest != transition.transition_receipt_digest
        || item.verifier_record_id != record.verifier_record_id
        || item.verifier_record_digest != record.verifier_record_digest
        || item.verification_kind != record.registration.verification_kind
        || item.verifier_id != record.registration.verifier_id
        || item.verifier_revision != record.registration.verifier_revision
        || item.verifier_digest != record.registration.verifier_digest
        || !exact
    {
        bail!("credential re-attestation V241 activation history failed exact audit");
    }
    Ok(())
}

fn audit_key_projection(
    conn: &Connection,
    json: &str,
    record: &CredentialVerifierKeyRecord,
    expected_digest: &str,
    expected_key_id: &str,
) -> Result<()> {
    validate_key_record(record)?;
    let (canonical, digest) = key_record_json_and_digest(record)?;
    let item = &record.registration;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_verifier_keys
          WHERE key_record_id=?1 AND key_record_schema=?2 AND key_record_digest=?3
            AND key_record_json=?4 AND registration_material_digest=?5
            AND canonicalization=?6 AND digest_algorithm=?7 AND verifier_record_id=?8
            AND verifier_record_digest=?9 AND verifier_operator=?10 AND verifier_product=?11
            AND verification_kind=?12 AND verifier_id=?13 AND verifier_revision=?14
            AND verifier_digest=?15 AND key_id=?16 AND algorithm=?17 AND public_key_pem=?18
            AND actor_kind=?19 AND created_by_admin_user_id=?20 AND confirmation=?21
            AND idempotency_scope=?22 AND idempotency_key=?23 AND created_at=?24
            AND recorded_at=?25 AND currentness_effect=?26
            AND credential_receipt_effect=?27 AND adapter_effect=?28 AND route_effect=?29",
            params![
                record.key_record_id,
                record.schema,
                record.key_record_digest,
                json,
                record.registration_material_digest,
                record.canonicalization,
                record.digest_algorithm,
                item.verifier_record_id,
                item.verifier_record_digest,
                item.verifier_operator,
                item.verifier_product,
                item.verification_kind,
                item.verifier_id,
                item.verifier_revision,
                item.verifier_digest,
                item.key_id,
                item.algorithm,
                item.public_key_pem,
                item.actor_kind,
                item.created_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.created_at,
                item.recorded_at,
                item.currentness_effect,
                item.credential_receipt_effect,
                item.adapter_effect,
                item.route_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if canonical != json
        || digest != record.key_record_digest
        || record.key_record_digest != expected_digest
        || item.key_id != expected_key_id
        || !exact
    {
        bail!("credential re-attestation V242 key history failed exact audit");
    }
    Ok(())
}
