use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_credential_verifier_key::{
        record_json_and_digest, revocation_json_and_digest, validate_record, validate_revocation,
        CredentialVerifierKeyRecord, CredentialVerifierKeyRevocationReceipt, STATUS_ACTIVE,
        STATUS_REVOKED,
    },
    store::{
        compute_external_pool_adapter_credential_verifier::credential_verifier_is_current_exact_on,
        Store,
    },
};

use super::types::*;

pub(super) fn record_by_id_on(conn: &Connection, id: &str) -> Result<Option<StoredKeyRecord>> {
    record_on(conn, "key_record_id=?1", params![id])
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
        &format!("SELECT key_record_json FROM compute_external_pool_adapter_credential_verifier_keys WHERE {filter}"),
        values,
        |row| {
            let json: String = row.get(0)?;
            let record: CredentialVerifierKeyRecord = serde_json::from_str(&json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0,Type::Text,Box::new(e)))?;
            Ok(StoredKeyRecord { record, json })
        },
    ).optional()?.map(|x| audit_record(conn,x)).transpose()
}

fn audit_record(conn: &Connection, stored: StoredKeyRecord) -> Result<StoredKeyRecord> {
    validate_record(&stored.record)?;
    let (json, digest) = record_json_and_digest(&stored.record)?;
    let item = &stored.record.registration;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_verifier_keys
         WHERE key_record_id=?1 AND key_record_digest=?2 AND key_record_json=?3
           AND verifier_record_id=?4 AND verifier_record_digest=?5 AND verification_kind=?6
           AND verifier_id=?7 AND verifier_revision=?8 AND verifier_digest=?9 AND key_id=?10
           AND public_key_pem=?11 AND created_by_admin_user_id=?12
           AND idempotency_scope=?13 AND idempotency_key=?14 AND created_at=?15",
            params![
                stored.record.key_record_id,
                stored.record.key_record_digest,
                stored.json,
                item.verifier_record_id,
                item.verifier_record_digest,
                item.verification_kind,
                item.verifier_id,
                item.verifier_revision,
                item.verifier_digest,
                item.key_id,
                item.public_key_pem,
                item.created_by_admin_user_id,
                item.idempotency_scope,
                item.idempotency_key,
                item.created_at
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if json != stored.json || digest != stored.record.key_record_digest || !exact {
        bail!("credential-verifier-key failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn revocation_by_key_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRevocation>> {
    revocation_on(conn, "key_record_id=?1", params![id])
}
pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRevocation>> {
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
) -> Result<Option<StoredRevocation>> {
    conn.query_row(&format!("SELECT revocation_receipt_json FROM compute_external_pool_adapter_credential_verifier_key_revocations WHERE {filter}"),values,|row|{
        let json:String=row.get(0)?;
        let receipt:CredentialVerifierKeyRevocationReceipt=serde_json::from_str(&json)
            .map_err(|e|rusqlite::Error::FromSqlConversionFailure(0,Type::Text,Box::new(e)))?;
        Ok(StoredRevocation{receipt,json})
    }).optional()?.map(|x|audit_revocation(conn,x)).transpose()
}
fn audit_revocation(conn: &Connection, stored: StoredRevocation) -> Result<StoredRevocation> {
    validate_revocation(&stored.receipt)?;
    let (json, digest) = revocation_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.revocation;
    let root = record_by_id_on(conn, &item.key_record_id)?
        .ok_or_else(|| anyhow::anyhow!("credential-verifier-key revocation lost root"))?;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_credential_verifier_key_revocations
      WHERE revocation_receipt_id=?1 AND revocation_receipt_digest=?2 AND revocation_receipt_json=?3
        AND key_record_id=?4 AND key_record_digest=?5 AND verifier_record_id=?6
        AND verifier_record_digest=?7 AND key_id=?8 AND revoked_by_admin_user_id=?9
        AND reason=?10 AND idempotency_scope=?11 AND idempotency_key=?12 AND revoked_at=?13",
            params![
                stored.receipt.revocation_receipt_id,
                stored.receipt.revocation_receipt_digest,
                stored.json,
                item.key_record_id,
                item.key_record_digest,
                item.verifier_record_id,
                item.verifier_record_digest,
                item.key_id,
                item.revoked_by_admin_user_id,
                item.reason,
                item.idempotency_scope,
                item.idempotency_key,
                item.revoked_at
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    let registration = &root.record.registration;
    if item.key_record_digest != root.record.key_record_digest
        || item.verifier_record_id != registration.verifier_record_id
        || item.verifier_record_digest != registration.verifier_record_digest
        || item.key_id != registration.key_id
        || item.revoked_at < registration.created_at
        || json != stored.json
        || digest != stored.receipt.revocation_receipt_digest
        || !exact
    {
        bail!("credential-verifier-key revocation failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn currentness_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<CredentialVerifierKeyCurrentnessReceipt>> {
    let Some(root) = record_by_id_on(conn, id)? else {
        return Ok(None);
    };
    let item = &root.record.registration;
    let verifier_current = credential_verifier_is_current_exact_on(
        conn,
        &item.verifier_record_id,
        &item.verifier_record_digest,
        &item.verification_kind,
        &item.verifier_id,
        item.verifier_revision,
        &item.verifier_digest,
    )?;
    let revocation = revocation_by_key_on(conn, id)?;
    let expected = if !verifier_current {
        "verifier_not_current"
    } else if revocation.is_some() {
        STATUS_REVOKED
    } else {
        STATUS_ACTIVE
    };
    let actual:String=conn.query_row("SELECT current_status FROM compute_external_pool_adapter_credential_verifier_key_current WHERE key_record_id=?1",[id],|r|r.get(0))?;
    if actual != expected {
        bail!("credential-verifier-key current view failed exact audit")
    }
    Ok(Some(CredentialVerifierKeyCurrentnessReceipt {
        schema: CURRENTNESS_SCHEMA,
        key_record: root.summary(),
        current_status: expected.into(),
        revocation: revocation.as_ref().map(StoredRevocation::summary),
    }))
}

pub(in crate::store) fn current_credential_verifier_key_authority_on(
    conn: &Connection,
    id: &str,
    expected_digest: &str,
    expected_key_id: &str,
) -> Result<Option<CurrentCredentialVerifierKeyAuthority>> {
    let Some(current) = currentness_on(conn, id)? else {
        return Ok(None);
    };
    if current.current_status != STATUS_ACTIVE
        || current.key_record.key_record_digest != expected_digest
        || current.key_record.key_id != expected_key_id
    {
        bail!("credential-verifier-key authority is not current and exact");
    }
    Ok(record_by_id_on(conn, id)?.map(|x| CurrentCredentialVerifierKeyAuthority::new(&x)))
}

impl Store {
    pub(crate) fn external_pool_adapter_credential_verifier_key_currentness(
        &self,
        id: &str,
    ) -> Result<Option<CredentialVerifierKeyCurrentnessReceipt>> {
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
        bail!("credential-verifier-key input is invalid")
    };
    Ok(())
}
pub(super) fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        bail!("credential-verifier-key digest is invalid")
    };
    Ok(())
}
