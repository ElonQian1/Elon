use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_artifact_signing_key::{
        canonical_signing_key_activation_json_and_digest,
        canonical_signing_key_record_json_and_digest,
        canonical_signing_key_revocation_json_and_digest, validate_signing_key_activation_receipt,
        validate_signing_key_record, validate_signing_key_revocation_receipt,
        ExternalPoolAdapterArtifactSigningKeyActivationReceipt,
        ExternalPoolAdapterArtifactSigningKeyRecord,
        ExternalPoolAdapterArtifactSigningKeyRevocationReceipt, SIGNING_KEY_STATUS_ACTIVE,
        SIGNING_KEY_STATUS_PENDING_ACTIVATION, SIGNING_KEY_STATUS_REVOKED,
    },
    store::Store,
};

use super::types::{
    CurrentExternalPoolAdapterArtifactSigningKeyAuthority,
    ExternalPoolAdapterArtifactSigningKeyCurrentnessReceipt, StoredSigningKeyActivation,
    StoredSigningKeyRecord, StoredSigningKeyRevocation, CURRENTNESS_SCHEMA,
};

pub(super) fn record_by_id_on(
    conn: &Connection,
    key_record_id: &str,
) -> Result<Option<StoredSigningKeyRecord>> {
    record_on(conn, "key_record_id=?1", params![key_record_id])
}

pub(super) fn record_by_key_id_on(
    conn: &Connection,
    key_id: &str,
) -> Result<Option<StoredSigningKeyRecord>> {
    record_on(conn, "key_id=?1", params![key_id])
}

pub(super) fn record_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSigningKeyRecord>> {
    record_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn record_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredSigningKeyRecord>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT key_record_json
                   FROM compute_external_pool_adapter_artifact_signing_keys
                  WHERE {filter}"
            ),
            values,
            |row| decode_record(row.get(0)?),
        )
        .optional()?;
    stored.map(|value| audit_record(conn, value)).transpose()
}

fn decode_record(record_json: String) -> rusqlite::Result<StoredSigningKeyRecord> {
    let record: ExternalPoolAdapterArtifactSigningKeyRecord = serde_json::from_str(&record_json)
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
        })?;
    Ok(StoredSigningKeyRecord {
        record,
        record_json,
    })
}

fn audit_record(
    conn: &Connection,
    stored: StoredSigningKeyRecord,
) -> Result<StoredSigningKeyRecord> {
    validate_signing_key_record(&stored.record)?;
    let (json, digest) = canonical_signing_key_record_json_and_digest(&stored.record)?;
    let registration = &stored.record.registration;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_artifact_signing_keys
              WHERE key_record_id=?1 AND key_record_schema=?2 AND key_record_digest=?3
                AND key_record_json=?4 AND registration_material_digest=?5
                AND canonicalization=?6 AND digest_algorithm=?7 AND source_operator=?8
                AND key_id=?9 AND algorithm=?10 AND public_key_pem=?11 AND actor_kind=?12
                AND created_by_admin_user_id=?13 AND confirmation=?14
                AND idempotency_scope=?15 AND idempotency_key=?16 AND created_at=?17
                AND recorded_at=?18 AND currentness_effect=?19
                AND artifact_signature_effect=?20 AND adapter_effect=?21 AND route_effect=?22",
            params![
                stored.record.key_record_id,
                stored.record.schema,
                stored.record.key_record_digest,
                stored.record_json,
                stored.record.registration_material_digest,
                stored.record.canonicalization,
                stored.record.digest_algorithm,
                registration.source_operator,
                registration.key_id,
                registration.algorithm,
                registration.public_key_pem,
                registration.actor_kind,
                registration.created_by_admin_user_id,
                registration.confirmation,
                registration.idempotency_scope,
                registration.idempotency_key,
                registration.created_at,
                registration.recorded_at,
                registration.currentness_effect,
                registration.artifact_signature_effect,
                registration.adapter_effect,
                registration.route_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if json != stored.record_json || digest != stored.record.key_record_digest || !exact {
        bail!("signing-key record failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn activation_by_key_on(
    conn: &Connection,
    key_record_id: &str,
) -> Result<Option<StoredSigningKeyActivation>> {
    activation_on(conn, "key_record_id=?1", params![key_record_id])
}

pub(super) fn activation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSigningKeyActivation>> {
    activation_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn activation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredSigningKeyActivation>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT activation_receipt_json
                   FROM compute_external_pool_adapter_artifact_signing_key_activations
                  WHERE {filter}"
            ),
            values,
            |row| {
                let receipt_json: String = row.get(0)?;
                let receipt: ExternalPoolAdapterArtifactSigningKeyActivationReceipt =
                    serde_json::from_str(&receipt_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                    })?;
                Ok(StoredSigningKeyActivation {
                    receipt,
                    receipt_json,
                })
            },
        )
        .optional()?;
    stored
        .map(|value| audit_activation(conn, value))
        .transpose()
}

fn audit_activation(
    conn: &Connection,
    stored: StoredSigningKeyActivation,
) -> Result<StoredSigningKeyActivation> {
    validate_signing_key_activation_receipt(&stored.receipt)?;
    let (json, digest) = canonical_signing_key_activation_json_and_digest(&stored.receipt)?;
    let activation = &stored.receipt.activation;
    let root = record_by_id_on(conn, &activation.key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("signing-key activation lost its root"))?;
    if activation.key_record_digest != root.record.key_record_digest
        || activation.key_id != root.record.registration.key_id
        || activation.source_operator != root.record.registration.source_operator
        || activation.activated_by_admin_user_id
            == root.record.registration.created_by_admin_user_id
        || activation.occurred_at < root.record.registration.created_at
    {
        bail!("signing-key activation lineage drifted");
    }
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_artifact_signing_key_activations
              WHERE activation_receipt_id=?1 AND activation_receipt_digest=?2
                AND activation_receipt_json=?3 AND activation_material_digest=?4
                AND key_record_id=?5 AND key_record_digest=?6 AND key_id=?7
                AND source_operator=?8 AND activated_by_admin_user_id=?9
                AND confirmation=?10 AND idempotency_scope=?11 AND idempotency_key=?12
                AND occurred_at=?13 AND recorded_at=?14",
            params![
                stored.receipt.activation_receipt_id,
                stored.receipt.activation_receipt_digest,
                stored.receipt_json,
                stored.receipt.activation_material_digest,
                activation.key_record_id,
                activation.key_record_digest,
                activation.key_id,
                activation.source_operator,
                activation.activated_by_admin_user_id,
                activation.confirmation,
                activation.idempotency_scope,
                activation.idempotency_key,
                activation.occurred_at,
                activation.recorded_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if json != stored.receipt_json || digest != stored.receipt.activation_receipt_digest || !exact {
        bail!("signing-key activation failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn revocation_by_key_on(
    conn: &Connection,
    key_record_id: &str,
) -> Result<Option<StoredSigningKeyRevocation>> {
    revocation_on(conn, "key_record_id=?1", params![key_record_id])
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSigningKeyRevocation>> {
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
) -> Result<Option<StoredSigningKeyRevocation>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT revocation_receipt_json
                   FROM compute_external_pool_adapter_artifact_signing_key_revocations
                  WHERE {filter}"
            ),
            values,
            |row| {
                let receipt_json: String = row.get(0)?;
                let receipt: ExternalPoolAdapterArtifactSigningKeyRevocationReceipt =
                    serde_json::from_str(&receipt_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                    })?;
                Ok(StoredSigningKeyRevocation {
                    receipt,
                    receipt_json,
                })
            },
        )
        .optional()?;
    stored
        .map(|value| audit_revocation(conn, value))
        .transpose()
}

fn audit_revocation(
    conn: &Connection,
    stored: StoredSigningKeyRevocation,
) -> Result<StoredSigningKeyRevocation> {
    validate_signing_key_revocation_receipt(&stored.receipt)?;
    let (json, digest) = canonical_signing_key_revocation_json_and_digest(&stored.receipt)?;
    let revocation = &stored.receipt.revocation;
    let root = record_by_id_on(conn, &revocation.key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("signing-key revocation lost its root"))?;
    let activation = activation_by_key_on(conn, &revocation.key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("signing-key revocation lost its activation"))?;
    if revocation.key_record_digest != root.record.key_record_digest
        || revocation.key_id != root.record.registration.key_id
        || revocation.source_operator != root.record.registration.source_operator
        || revocation.occurred_at < activation.receipt.activation.occurred_at
    {
        bail!("signing-key revocation lineage drifted");
    }
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_artifact_signing_key_revocations
              WHERE revocation_receipt_id=?1 AND revocation_receipt_digest=?2
                AND revocation_receipt_json=?3 AND revocation_material_digest=?4
                AND key_record_id=?5 AND key_record_digest=?6 AND key_id=?7
                AND source_operator=?8 AND revoked_by_admin_user_id=?9 AND reason=?10
                AND confirmation=?11 AND idempotency_scope=?12 AND idempotency_key=?13
                AND occurred_at=?14 AND recorded_at=?15",
            params![
                stored.receipt.revocation_receipt_id,
                stored.receipt.revocation_receipt_digest,
                stored.receipt_json,
                stored.receipt.revocation_material_digest,
                revocation.key_record_id,
                revocation.key_record_digest,
                revocation.key_id,
                revocation.source_operator,
                revocation.revoked_by_admin_user_id,
                revocation.reason,
                revocation.confirmation,
                revocation.idempotency_scope,
                revocation.idempotency_key,
                revocation.occurred_at,
                revocation.recorded_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if json != stored.receipt_json || digest != stored.receipt.revocation_receipt_digest || !exact {
        bail!("signing-key revocation failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn currentness_on(
    conn: &Connection,
    key_record_id: &str,
) -> Result<Option<ExternalPoolAdapterArtifactSigningKeyCurrentnessReceipt>> {
    let Some(record) = record_by_id_on(conn, key_record_id)? else {
        return Ok(None);
    };
    let activation = activation_by_key_on(conn, key_record_id)?;
    let revocation = revocation_by_key_on(conn, key_record_id)?;
    let expected_status = if revocation.is_some() {
        SIGNING_KEY_STATUS_REVOKED
    } else if activation.is_some() {
        SIGNING_KEY_STATUS_ACTIVE
    } else {
        SIGNING_KEY_STATUS_PENDING_ACTIVATION
    };
    let view_status: String = conn.query_row(
        "SELECT current_status
           FROM compute_external_pool_adapter_artifact_signing_key_current
          WHERE key_record_id=?1 AND key_record_digest=?2 AND key_id=?3",
        params![
            record.record.key_record_id,
            record.record.key_record_digest,
            record.record.registration.key_id
        ],
        |row| row.get(0),
    )?;
    if view_status != expected_status {
        bail!("signing-key current view failed exact audit");
    }
    Ok(Some(
        ExternalPoolAdapterArtifactSigningKeyCurrentnessReceipt {
            schema: CURRENTNESS_SCHEMA,
            key_record: record.summary(),
            current_status: expected_status.to_string(),
            activation: activation.as_ref().map(StoredSigningKeyActivation::summary),
            revocation: revocation.as_ref().map(StoredSigningKeyRevocation::summary),
        },
    ))
}

pub(in crate::store) fn current_external_pool_adapter_artifact_signing_key_authority_on(
    conn: &Connection,
    key_record_id: &str,
    expected_key_record_digest: &str,
    expected_key_id: &str,
) -> Result<Option<CurrentExternalPoolAdapterArtifactSigningKeyAuthority>> {
    let Some(currentness) = currentness_on(conn, key_record_id)? else {
        return Ok(None);
    };
    if currentness.key_record.key_record_digest != expected_key_record_digest
        || currentness.key_record.key_id != expected_key_id
        || currentness.current_status != SIGNING_KEY_STATUS_ACTIVE
    {
        bail!("signing-key authority is not current and exact");
    }
    let record = record_by_id_on(conn, key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("signing-key root disappeared"))?;
    Ok(Some(
        CurrentExternalPoolAdapterArtifactSigningKeyAuthority::new(&record),
    ))
}

impl Store {
    pub(crate) fn external_pool_adapter_artifact_signing_key_currentness(
        &self,
        key_record_id: &str,
    ) -> Result<Option<ExternalPoolAdapterArtifactSigningKeyCurrentnessReceipt>> {
        validate_exact(key_record_id, "key record ID", 160)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction()?;
        let receipt = currentness_on(&transaction, key_record_id)?;
        transaction.commit()?;
        Ok(receipt)
    }
}

pub(super) fn validate_exact(value: &str, label: &str, maximum: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}
