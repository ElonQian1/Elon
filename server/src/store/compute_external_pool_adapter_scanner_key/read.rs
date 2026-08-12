use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_scanner_key::{
        scanner_key_activation_json_and_digest, scanner_key_record_json_and_digest,
        scanner_key_revocation_json_and_digest, validate_scanner_key_activation,
        validate_scanner_key_record, validate_scanner_key_revocation,
        ExternalPoolAdapterScannerKeyActivationReceipt, ExternalPoolAdapterScannerKeyRecord,
        ExternalPoolAdapterScannerKeyRevocationReceipt, SCANNER_KEY_STATUS_ACTIVE,
        SCANNER_KEY_STATUS_PENDING, SCANNER_KEY_STATUS_REVOKED,
    },
    store::Store,
};

use super::types::*;

pub(super) fn record_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredScannerKeyRecord>> {
    record_on(conn, "key_record_id=?1", params![id])
}

pub(super) fn record_by_key_id_on(
    conn: &Connection,
    key_id: &str,
) -> Result<Option<StoredScannerKeyRecord>> {
    record_on(conn, "key_id=?1", params![key_id])
}

pub(super) fn record_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredScannerKeyRecord>> {
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
) -> Result<Option<StoredScannerKeyRecord>> {
    conn.query_row(
        &format!(
            "SELECT key_record_json FROM compute_external_pool_adapter_scanner_keys WHERE {filter}"
        ),
        values,
        |row| {
            let json: String = row.get(0)?;
            let record: ExternalPoolAdapterScannerKeyRecord =
                serde_json::from_str(&json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                })?;
            Ok(StoredScannerKeyRecord { record, json })
        },
    )
    .optional()?
    .map(|item| audit_record(conn, item))
    .transpose()
}

fn audit_record(
    conn: &Connection,
    stored: StoredScannerKeyRecord,
) -> Result<StoredScannerKeyRecord> {
    validate_scanner_key_record(&stored.record)?;
    let (json, digest) = scanner_key_record_json_and_digest(&stored.record)?;
    let item = &stored.record.registration;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_scanner_keys
          WHERE key_record_id=?1 AND key_record_digest=?2 AND key_record_json=?3
            AND registration_material_digest=?4 AND scanner_operator=?5 AND scanner_product=?6
            AND key_id=?7 AND algorithm=?8 AND public_key_pem=?9
            AND created_by_admin_user_id=?10 AND confirmation=?11
            AND idempotency_scope=?12 AND idempotency_key=?13
            AND created_at=?14 AND recorded_at=?15",
            params![
                stored.record.key_record_id,
                stored.record.key_record_digest,
                stored.json,
                stored.record.registration_material_digest,
                item.scanner_operator,
                item.scanner_product,
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
        bail!("scanner-key record failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn activation_by_key_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredScannerKeyActivation>> {
    activation_on(conn, "key_record_id=?1", params![id])
}

pub(super) fn activation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredScannerKeyActivation>> {
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
) -> Result<Option<StoredScannerKeyActivation>> {
    conn.query_row(
        &format!("SELECT activation_receipt_json FROM compute_external_pool_adapter_scanner_key_activations WHERE {filter}"),
        values,
        |row| decode_activation(row.get(0)?),
    ).optional()?.map(|item| audit_activation(conn, item)).transpose()
}

fn decode_activation(json: String) -> rusqlite::Result<StoredScannerKeyActivation> {
    let receipt = serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
    })?;
    Ok(StoredScannerKeyActivation { receipt, json })
}

fn audit_activation(
    conn: &Connection,
    stored: StoredScannerKeyActivation,
) -> Result<StoredScannerKeyActivation> {
    validate_scanner_key_activation(&stored.receipt)?;
    let (json, digest) = scanner_key_activation_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.activation;
    let root = record_by_id_on(conn, &item.key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("scanner-key activation lost its root"))?;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_scanner_key_activations
          WHERE activation_receipt_id=?1 AND activation_receipt_digest=?2
            AND activation_receipt_json=?3 AND activation_material_digest=?4
            AND key_record_id=?5 AND key_record_digest=?6 AND key_id=?7
            AND scanner_operator=?8 AND scanner_product=?9
            AND activated_by_admin_user_id=?10 AND confirmation=?11
            AND idempotency_scope=?12 AND idempotency_key=?13
            AND occurred_at=?14 AND recorded_at=?15",
            params![
                stored.receipt.activation_receipt_id,
                stored.receipt.activation_receipt_digest,
                stored.json,
                stored.receipt.activation_material_digest,
                item.key_record_id,
                item.key_record_digest,
                item.key_id,
                item.scanner_operator,
                item.scanner_product,
                item.activated_by_admin_user_id,
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
    if item.key_record_digest != root.record.key_record_digest
        || item.key_id != root.record.registration.key_id
        || item.scanner_operator != root.record.registration.scanner_operator
        || item.scanner_product != root.record.registration.scanner_product
        || item.activated_by_admin_user_id == root.record.registration.created_by_admin_user_id
        || item.occurred_at < root.record.registration.created_at
        || json != stored.json
        || digest != stored.receipt.activation_receipt_digest
        || !exact
    {
        bail!("scanner-key activation failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn revocation_by_key_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredScannerKeyRevocation>> {
    revocation_on(conn, "key_record_id=?1", params![id])
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredScannerKeyRevocation>> {
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
) -> Result<Option<StoredScannerKeyRevocation>> {
    conn.query_row(
        &format!("SELECT revocation_receipt_json FROM compute_external_pool_adapter_scanner_key_revocations WHERE {filter}"),
        values,
        |row| {
            let json: String = row.get(0)?;
            let receipt: ExternalPoolAdapterScannerKeyRevocationReceipt = serde_json::from_str(&json)
                .map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error)))?;
            Ok(StoredScannerKeyRevocation { receipt, json })
        },
    ).optional()?.map(|item| audit_revocation(conn, item)).transpose()
}

fn audit_revocation(
    conn: &Connection,
    stored: StoredScannerKeyRevocation,
) -> Result<StoredScannerKeyRevocation> {
    validate_scanner_key_revocation(&stored.receipt)?;
    let (json, digest) = scanner_key_revocation_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.revocation;
    let root = record_by_id_on(conn, &item.key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("scanner-key revocation lost its root"))?;
    let active = activation_by_key_on(conn, &item.key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("scanner-key revocation lost its activation"))?;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_scanner_key_revocations
          WHERE revocation_receipt_id=?1 AND revocation_receipt_digest=?2
            AND revocation_receipt_json=?3 AND revocation_material_digest=?4
            AND key_record_id=?5 AND key_record_digest=?6 AND key_id=?7
            AND scanner_operator=?8 AND scanner_product=?9
            AND revoked_by_admin_user_id=?10 AND reason=?11 AND confirmation=?12
            AND idempotency_scope=?13 AND idempotency_key=?14
            AND occurred_at=?15 AND recorded_at=?16",
            params![
                stored.receipt.revocation_receipt_id,
                stored.receipt.revocation_receipt_digest,
                stored.json,
                stored.receipt.revocation_material_digest,
                item.key_record_id,
                item.key_record_digest,
                item.key_id,
                item.scanner_operator,
                item.scanner_product,
                item.revoked_by_admin_user_id,
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
    if item.key_record_digest != root.record.key_record_digest
        || item.key_id != root.record.registration.key_id
        || item.scanner_operator != root.record.registration.scanner_operator
        || item.scanner_product != root.record.registration.scanner_product
        || item.occurred_at < active.receipt.activation.occurred_at
        || json != stored.json
        || digest != stored.receipt.revocation_receipt_digest
        || !exact
    {
        bail!("scanner-key revocation failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn currentness_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<ExternalPoolAdapterScannerKeyCurrentnessReceipt>> {
    let Some(root) = record_by_id_on(conn, id)? else {
        return Ok(None);
    };
    let activation = activation_by_key_on(conn, id)?;
    let revocation = revocation_by_key_on(conn, id)?;
    let expected = if revocation.is_some() {
        SCANNER_KEY_STATUS_REVOKED
    } else if activation.is_some() {
        SCANNER_KEY_STATUS_ACTIVE
    } else {
        SCANNER_KEY_STATUS_PENDING
    };
    let actual: String = conn.query_row(
        "SELECT current_status FROM compute_external_pool_adapter_scanner_key_current WHERE key_record_id=?1 AND key_record_digest=?2",
        params![root.record.key_record_id,root.record.key_record_digest], |row| row.get(0)
    )?;
    if actual != expected {
        bail!("scanner-key current view failed exact audit")
    }
    Ok(Some(ExternalPoolAdapterScannerKeyCurrentnessReceipt {
        schema: CURRENTNESS_SCHEMA,
        key_record: root.summary(),
        current_status: expected.to_string(),
        activation: activation.as_ref().map(StoredScannerKeyActivation::summary),
        revocation: revocation.as_ref().map(StoredScannerKeyRevocation::summary),
    }))
}

pub(in crate::store) fn current_scanner_key_authority_on(
    conn: &Connection,
    id: &str,
    expected_digest: &str,
    expected_key_id: &str,
) -> Result<Option<CurrentExternalPoolAdapterScannerKeyAuthority>> {
    let Some(currentness) = currentness_on(conn, id)? else {
        return Ok(None);
    };
    if currentness.current_status != SCANNER_KEY_STATUS_ACTIVE
        || currentness.key_record.key_record_digest != expected_digest
        || currentness.key_record.key_id != expected_key_id
    {
        bail!("scanner-key authority is not current and exact");
    }
    let root = record_by_id_on(conn, id)?
        .ok_or_else(|| anyhow::anyhow!("scanner-key root disappeared"))?;
    Ok(Some(CurrentExternalPoolAdapterScannerKeyAuthority::new(
        &root,
    )))
}

pub(in crate::store) fn scanner_key_record_authority_on(
    conn: &Connection,
    id: &str,
    expected_digest: &str,
    expected_key_id: &str,
) -> Result<Option<ExternalPoolAdapterScannerKeyRecordAuthority>> {
    let Some(root) = record_by_id_on(conn, id)? else {
        return Ok(None);
    };
    if root.record.key_record_digest != expected_digest
        || root.record.registration.key_id != expected_key_id
    {
        bail!("scanner-key historical root is not exact");
    }
    Ok(Some(ExternalPoolAdapterScannerKeyRecordAuthority::new(
        &root,
    )))
}

impl Store {
    pub(crate) fn external_pool_adapter_scanner_key_currentness(
        &self,
        id: &str,
    ) -> Result<Option<ExternalPoolAdapterScannerKeyCurrentnessReceipt>> {
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
        bail!("scanner-key input is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("scanner-key digest is invalid");
    }
    Ok(())
}
