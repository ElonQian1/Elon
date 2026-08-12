use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_credential_verifier::{
        credential_verifier_record_json_and_digest, credential_verifier_transition_json_and_digest,
        validate_credential_verifier_record, validate_credential_verifier_transition,
        ExternalPoolAdapterCredentialVerifierRecord,
        ExternalPoolAdapterCredentialVerifierTransitionReceipt, CREDENTIAL_VERIFIER_STATUS_ACTIVE,
        CREDENTIAL_VERIFIER_STATUS_PENDING, CREDENTIAL_VERIFIER_STATUS_REVOKED,
    },
    store::Store,
};

use super::types::*;

pub(super) fn record_by_id_on(conn: &Connection, id: &str) -> Result<Option<StoredVerifierRecord>> {
    record_on(conn, "verifier_record_id=?1", params![id])
}

pub(super) fn record_by_identity_on(
    conn: &Connection,
    kind: &str,
    verifier_id: &str,
    revision: i64,
) -> Result<Option<StoredVerifierRecord>> {
    record_on(
        conn,
        "verification_kind=?1 AND verifier_id=?2 AND verifier_revision=?3",
        params![kind, verifier_id, revision],
    )
}

pub(super) fn record_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredVerifierRecord>> {
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
) -> Result<Option<StoredVerifierRecord>> {
    conn.query_row(
        &format!("SELECT verifier_record_json FROM compute_external_pool_adapter_credential_verifiers WHERE {filter}"),
        values,
        |row| decode_record(row.get(0)?),
    )
    .optional()?
    .map(|stored| audit_record(conn, stored))
    .transpose()
}

fn decode_record(json: String) -> rusqlite::Result<StoredVerifierRecord> {
    let record: ExternalPoolAdapterCredentialVerifierRecord =
        serde_json::from_str(&json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
        })?;
    Ok(StoredVerifierRecord { record, json })
}

fn audit_record(conn: &Connection, stored: StoredVerifierRecord) -> Result<StoredVerifierRecord> {
    validate_credential_verifier_record(&stored.record)?;
    let (json, digest) = credential_verifier_record_json_and_digest(&stored.record)?;
    let item = &stored.record.registration;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_verifiers
             WHERE verifier_record_id=?1 AND verifier_record_digest=?2 AND verifier_record_json=?3
               AND registration_material_digest=?4 AND verifier_operator=?5 AND verifier_product=?6
               AND verification_kind=?7 AND verifier_id=?8 AND verifier_revision=?9 AND verifier_digest=?10
               AND created_by_admin_user_id=?11 AND confirmation=?12
               AND idempotency_scope=?13 AND idempotency_key=?14
               AND created_at=?15 AND recorded_at=?16",
            params![
                stored.record.verifier_record_id,
                stored.record.verifier_record_digest,
                stored.json,
                stored.record.registration_material_digest,
                item.verifier_operator,
                item.verifier_product,
                item.verification_kind,
                item.verifier_id,
                item.verifier_revision,
                item.verifier_digest,
                item.created_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.created_at,
                item.recorded_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if json != stored.json || digest != stored.record.verifier_record_digest || !exact {
        bail!("credential-verifier record failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn transition_by_kind_on(
    conn: &Connection,
    id: &str,
    kind: &str,
) -> Result<Option<StoredTransition>> {
    transition_on(
        conn,
        "verifier_record_id=?1 AND transition_kind=?2",
        params![id, kind],
    )
}

pub(super) fn transition_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredTransition>> {
    transition_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn transition_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredTransition>> {
    conn.query_row(
        &format!("SELECT transition_receipt_json FROM compute_external_pool_adapter_credential_verifier_transitions WHERE {filter}"),
        values,
        |row| {
            let json: String = row.get(0)?;
            let receipt: ExternalPoolAdapterCredentialVerifierTransitionReceipt =
                serde_json::from_str(&json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                })?;
            Ok(StoredTransition { receipt, json })
        },
    )
    .optional()?
    .map(|stored| audit_transition(conn, stored))
    .transpose()
}

fn audit_transition(conn: &Connection, stored: StoredTransition) -> Result<StoredTransition> {
    validate_credential_verifier_transition(&stored.receipt)?;
    let (json, digest) = credential_verifier_transition_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.transition;
    let kind = if item.reason.is_some() {
        "revocation"
    } else {
        "activation"
    };
    let root = record_by_id_on(conn, &item.verifier_record_id)?
        .ok_or_else(|| anyhow::anyhow!("credential-verifier transition lost its root"))?;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_verifier_transitions
             WHERE transition_receipt_id=?1 AND transition_receipt_digest=?2
               AND transition_receipt_json=?3 AND transition_material_digest=?4
               AND transition_kind=?5 AND verifier_record_id=?6 AND verifier_record_digest=?7
               AND verification_kind=?8 AND verifier_id=?9 AND verifier_revision=?10 AND verifier_digest=?11
               AND verifier_operator=?12 AND verifier_product=?13 AND actor_user_id=?14
               AND COALESCE(reason,'')=COALESCE(?15,'') AND confirmation=?16
               AND idempotency_scope=?17 AND idempotency_key=?18
               AND occurred_at=?19 AND recorded_at=?20",
            params![
                stored.receipt.transition_receipt_id,
                stored.receipt.transition_receipt_digest,
                stored.json,
                stored.receipt.transition_material_digest,
                kind,
                item.verifier_record_id,
                item.verifier_record_digest,
                item.verification_kind,
                item.verifier_id,
                item.verifier_revision,
                item.verifier_digest,
                item.verifier_operator,
                item.verifier_product,
                item.actor_user_id,
                item.reason,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.occurred_at,
                item.recorded_at,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    let registration = &root.record.registration;
    let activation =
        transition_time_by_kind_on_unchecked(conn, &item.verifier_record_id, "activation")?;
    if item.verifier_record_digest != root.record.verifier_record_digest
        || item.verification_kind != registration.verification_kind
        || item.verifier_id != registration.verifier_id
        || item.verifier_revision != registration.verifier_revision
        || item.verifier_digest != registration.verifier_digest
        || item.verifier_operator != registration.verifier_operator
        || item.verifier_product != registration.verifier_product
        || (kind == "activation" && item.actor_user_id == registration.created_by_admin_user_id)
        || item.occurred_at < registration.created_at
        || (kind == "revocation"
            && activation
                .as_deref()
                .is_none_or(|activated_at| item.occurred_at.as_str() < activated_at))
        || json != stored.json
        || digest != stored.receipt.transition_receipt_digest
        || !exact
    {
        bail!("credential-verifier transition failed exact readback audit");
    }
    Ok(stored)
}

fn transition_time_by_kind_on_unchecked(
    conn: &Connection,
    id: &str,
    kind: &str,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT occurred_at FROM compute_external_pool_adapter_credential_verifier_transitions WHERE verifier_record_id=?1 AND transition_kind=?2",
            params![id, kind],
            |row| row.get(0),
        )
        .optional()?)
}

pub(super) fn currentness_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterCredentialVerifierCurrentnessReceipt>> {
    let Some(root) = record_by_id_on(conn, id)? else {
        return Ok(None);
    };
    let activation = transition_by_kind_on(conn, id, "activation")?;
    let revocation = transition_by_kind_on(conn, id, "revocation")?;
    let expected = if revocation.is_some() {
        CREDENTIAL_VERIFIER_STATUS_REVOKED
    } else if activation.is_some() {
        CREDENTIAL_VERIFIER_STATUS_ACTIVE
    } else {
        CREDENTIAL_VERIFIER_STATUS_PENDING
    };
    let actual: String = conn.query_row(
        "SELECT current_status FROM compute_external_pool_adapter_credential_verifier_current WHERE verifier_record_id=?1 AND verifier_record_digest=?2",
        params![root.record.verifier_record_id, root.record.verifier_record_digest],
        |row| row.get(0),
    )?;
    if actual != expected {
        bail!("credential-verifier current view failed exact audit");
    }
    Ok(Some(
        ExternalPoolAdapterCredentialVerifierCurrentnessReceipt {
            schema: CURRENTNESS_SCHEMA,
            verifier_record: root.summary(),
            current_status: expected.into(),
            activation: activation.as_ref().map(StoredTransition::summary),
            revocation: revocation.as_ref().map(StoredTransition::summary),
        },
    ))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::store) fn current_credential_verifier_authority_on(
    conn: &Connection,
    record_id: &str,
    expected_record_digest: &str,
    verification_kind: &str,
    verifier_id: &str,
    verifier_revision: i64,
    expected_verifier_digest: &str,
) -> Result<Option<CurrentExternalPoolAdapterCredentialVerifierAuthority>> {
    let Some(current) = currentness_on(conn, record_id)? else {
        return Ok(None);
    };
    let item = &current.verifier_record;
    if current.current_status != CREDENTIAL_VERIFIER_STATUS_ACTIVE
        || item.verifier_record_digest != expected_record_digest
        || item.verification_kind != verification_kind
        || item.verifier_id != verifier_id
        || item.verifier_revision != verifier_revision
        || item.verifier_digest != expected_verifier_digest
    {
        bail!("credential-verifier authority is not current and exact");
    }
    Ok(record_by_id_on(conn, record_id)?
        .map(|root| CurrentExternalPoolAdapterCredentialVerifierAuthority::new(&root)))
}

pub(in crate::store) fn credential_verifier_is_current_exact_on(
    conn: &Connection,
    record_id: &str,
    expected_record_digest: &str,
    verification_kind: &str,
    verifier_id: &str,
    verifier_revision: i64,
    expected_verifier_digest: &str,
) -> Result<bool> {
    let Some(current) = currentness_on(conn, record_id)? else {
        return Ok(false);
    };
    let item = &current.verifier_record;
    Ok(current.current_status == CREDENTIAL_VERIFIER_STATUS_ACTIVE
        && item.verifier_record_digest == expected_record_digest
        && item.verification_kind == verification_kind
        && item.verifier_id == verifier_id
        && item.verifier_revision == verifier_revision
        && item.verifier_digest == expected_verifier_digest)
}

impl Store {
    pub(crate) fn external_pool_adapter_credential_verifier_currentness(
        &self,
        id: &str,
    ) -> Result<Option<ExternalPoolAdapterCredentialVerifierCurrentnessReceipt>> {
        validate_exact(id, 160)?;
        let connection = self.conn()?;
        currentness_on(&connection, id)
    }
}

pub(super) fn validate_exact(value: &str, max: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("credential-verifier input is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("credential-verifier digest is invalid");
    }
    Ok(())
}
