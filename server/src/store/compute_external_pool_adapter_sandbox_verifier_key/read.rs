use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_sandbox_verifier_key::{
        sandbox_verifier_key_record_json_and_digest,
        sandbox_verifier_key_transition_json_and_digest, validate_sandbox_verifier_key_record,
        validate_sandbox_verifier_key_transition, ExternalPoolAdapterSandboxVerifierKeyRecord,
        ExternalPoolAdapterSandboxVerifierKeyTransitionReceipt, SANDBOX_VERIFIER_KEY_STATUS_ACTIVE,
        SANDBOX_VERIFIER_KEY_STATUS_PENDING, SANDBOX_VERIFIER_KEY_STATUS_REVOKED,
    },
    store::Store,
};

use super::types::*;

pub(super) fn record_by_id_on(conn: &Connection, id: &str) -> Result<Option<StoredKeyRecord>> {
    record_on(conn, "key_record_id=?1", params![id])
}

pub(super) fn record_by_key_id_on(conn: &Connection, id: &str) -> Result<Option<StoredKeyRecord>> {
    record_on(conn, "key_id=?1", params![id])
}

pub(super) fn record_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredKeyRecord>> {
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
) -> Result<Option<StoredKeyRecord>> {
    conn.query_row(
        &format!("SELECT key_record_json FROM compute_external_pool_adapter_sandbox_verifier_keys WHERE {filter}"),
        values,
        |row| decode_record(row.get(0)?),
    )
    .optional()?
    .map(|stored| audit_record(conn, stored))
    .transpose()
}

fn decode_record(json: String) -> rusqlite::Result<StoredKeyRecord> {
    let record: ExternalPoolAdapterSandboxVerifierKeyRecord =
        serde_json::from_str(&json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
        })?;
    Ok(StoredKeyRecord { record, json })
}

fn audit_record(conn: &Connection, stored: StoredKeyRecord) -> Result<StoredKeyRecord> {
    validate_sandbox_verifier_key_record(&stored.record)?;
    let (json, digest) = sandbox_verifier_key_record_json_and_digest(&stored.record)?;
    let item = &stored.record.registration;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_keys
         WHERE key_record_id=?1 AND key_record_digest=?2 AND key_record_json=?3
           AND registration_material_digest=?4 AND verifier_operator=?5 AND verifier_product=?6
           AND key_id=?7 AND algorithm=?8 AND public_key_pem=?9
           AND created_by_admin_user_id=?10 AND confirmation=?11
           AND idempotency_scope=?12 AND idempotency_key=?13
           AND created_at=?14 AND recorded_at=?15",
            params![
                stored.record.key_record_id,
                stored.record.key_record_digest,
                stored.json,
                stored.record.registration_material_digest,
                item.verifier_operator,
                item.verifier_product,
                item.key_id,
                item.algorithm,
                item.public_key_pem,
                item.created_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.created_at,
                item.recorded_at
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if json != stored.json || digest != stored.record.key_record_digest || !exact {
        bail!("sandbox-verifier-key record failed exact readback audit");
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
        "key_record_id=?1 AND transition_kind=?2",
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
        &format!("SELECT transition_receipt_json FROM compute_external_pool_adapter_sandbox_verifier_key_transitions WHERE {filter}"),
        values,
        |row| {
            let json: String = row.get(0)?;
            let receipt: ExternalPoolAdapterSandboxVerifierKeyTransitionReceipt =
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
    validate_sandbox_verifier_key_transition(&stored.receipt)?;
    let (json, digest) = sandbox_verifier_key_transition_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.transition;
    let kind = if item.reason.is_some() {
        "revocation"
    } else {
        "activation"
    };
    let root = record_by_id_on(conn, &item.key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("sandbox-verifier-key transition lost its root"))?;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_key_transitions
         WHERE transition_receipt_id=?1 AND transition_receipt_digest=?2
           AND transition_receipt_json=?3 AND transition_material_digest=?4
           AND transition_kind=?5 AND key_record_id=?6 AND key_record_digest=?7 AND key_id=?8
           AND verifier_operator=?9 AND verifier_product=?10 AND actor_user_id=?11
           AND COALESCE(reason,'')=COALESCE(?12,'') AND confirmation=?13
           AND idempotency_scope=?14 AND idempotency_key=?15
           AND occurred_at=?16 AND recorded_at=?17",
            params![
                stored.receipt.transition_receipt_id,
                stored.receipt.transition_receipt_digest,
                stored.json,
                stored.receipt.transition_material_digest,
                kind,
                item.key_record_id,
                item.key_record_digest,
                item.key_id,
                item.verifier_operator,
                item.verifier_product,
                item.actor_user_id,
                item.reason,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.occurred_at,
                item.recorded_at
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    let activation = transition_time_by_kind_on_unchecked(conn, &item.key_record_id, "activation")?;
    if item.key_record_digest != root.record.key_record_digest
        || item.key_id != root.record.registration.key_id
        || item.verifier_operator != root.record.registration.verifier_operator
        || item.verifier_product != root.record.registration.verifier_product
        || (kind == "activation"
            && item.actor_user_id == root.record.registration.created_by_admin_user_id)
        || item.occurred_at < root.record.registration.created_at
        || (kind == "revocation"
            && activation
                .as_deref()
                .is_none_or(|activated_at| item.occurred_at.as_str() < activated_at))
        || json != stored.json
        || digest != stored.receipt.transition_receipt_digest
        || !exact
    {
        bail!("sandbox-verifier-key transition failed exact readback audit");
    }
    Ok(stored)
}

fn transition_time_by_kind_on_unchecked(
    conn: &Connection,
    id: &str,
    kind: &str,
) -> Result<Option<String>> {
    Ok(conn.query_row(
        "SELECT occurred_at FROM compute_external_pool_adapter_sandbox_verifier_key_transitions WHERE key_record_id=?1 AND transition_kind=?2",
        params![id, kind], |row| row.get(0),
    ).optional()?)
}

pub(super) fn currentness_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterSandboxVerifierKeyCurrentnessReceipt>> {
    let Some(root) = record_by_id_on(conn, id)? else {
        return Ok(None);
    };
    let activation = transition_by_kind_on(conn, id, "activation")?;
    let revocation = transition_by_kind_on(conn, id, "revocation")?;
    let expected = if revocation.is_some() {
        SANDBOX_VERIFIER_KEY_STATUS_REVOKED
    } else if activation.is_some() {
        SANDBOX_VERIFIER_KEY_STATUS_ACTIVE
    } else {
        SANDBOX_VERIFIER_KEY_STATUS_PENDING
    };
    let actual: String = conn.query_row(
        "SELECT current_status FROM compute_external_pool_adapter_sandbox_verifier_key_current WHERE key_record_id=?1 AND key_record_digest=?2",
        params![root.record.key_record_id,root.record.key_record_digest], |row| row.get(0)
    )?;
    if actual != expected {
        bail!("sandbox-verifier-key current view failed exact audit");
    }
    Ok(Some(
        ExternalPoolAdapterSandboxVerifierKeyCurrentnessReceipt {
            schema: CURRENTNESS_SCHEMA,
            key_record: root.summary(),
            current_status: expected.into(),
            activation: activation.as_ref().map(StoredTransition::summary),
            revocation: revocation.as_ref().map(StoredTransition::summary),
        },
    ))
}

pub(in crate::store) fn current_sandbox_verifier_key_authority_on(
    conn: &Connection,
    id: &str,
    expected_digest: &str,
    expected_key_id: &str,
) -> Result<Option<CurrentExternalPoolAdapterSandboxVerifierKeyAuthority>> {
    let Some(current) = currentness_on(conn, id)? else {
        return Ok(None);
    };
    if current.current_status != SANDBOX_VERIFIER_KEY_STATUS_ACTIVE
        || current.key_record.key_record_digest != expected_digest
        || current.key_record.key_id != expected_key_id
    {
        bail!("sandbox-verifier-key authority is not current and exact");
    }
    Ok(record_by_id_on(conn, id)?
        .map(|root| CurrentExternalPoolAdapterSandboxVerifierKeyAuthority::new(&root)))
}

pub(in crate::store) fn sandbox_verifier_key_record_authority_on(
    conn: &Connection,
    id: &str,
    expected_digest: &str,
    expected_key_id: &str,
) -> Result<Option<ExternalPoolAdapterSandboxVerifierKeyRecordAuthority>> {
    let Some(root) = record_by_id_on(conn, id)? else {
        return Ok(None);
    };
    if root.record.key_record_digest != expected_digest
        || root.record.registration.key_id != expected_key_id
    {
        bail!("sandbox-verifier-key historical root is not exact");
    }
    Ok(Some(
        ExternalPoolAdapterSandboxVerifierKeyRecordAuthority::new(&root),
    ))
}

impl Store {
    pub(crate) fn external_pool_adapter_sandbox_verifier_key_currentness(
        &self,
        id: &str,
    ) -> Result<Option<ExternalPoolAdapterSandboxVerifierKeyCurrentnessReceipt>> {
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
        bail!("sandbox-verifier-key input is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("sandbox-verifier-key digest is invalid");
    }
    Ok(())
}
