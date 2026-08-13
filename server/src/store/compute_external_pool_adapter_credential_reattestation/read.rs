use anyhow::{bail, Result};
use chrono::DateTime;
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_credential_reattestation::{
        credential_reattestation_revocation_receipt_json_and_digest,
        validate_credential_reattestation_revocation_receipt,
    },
    store::Store,
};

use super::{
    audit::audit_receipt, receipt_projection_audit::exact_revocation_projection, types::*,
};

pub(super) fn receipt_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredCredentialReattestation>> {
    receipt_on(conn, "reattestation_receipt_id=?1", params![id])
}

pub(super) fn receipt_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredCredentialReattestation>> {
    receipt_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn receipt_by_challenge_on(
    conn: &Connection,
    challenge_id: &str,
) -> Result<Option<StoredCredentialReattestation>> {
    receipt_on(conn, "challenge_id=?1", params![challenge_id])
}

pub(super) fn receipt_by_report_on(
    conn: &Connection,
    verifier_report_id: &str,
) -> Result<Option<StoredCredentialReattestation>> {
    receipt_on(conn, "verifier_report_id=?1", params![verifier_report_id])
}

pub(super) fn head_by_provider_binding_on(
    conn: &Connection,
    provider_binding_id: &str,
) -> Result<Option<StoredCredentialReattestation>> {
    let (count, id): (i64, Option<String>) = conn.query_row(
        "SELECT COUNT(*),MIN(reattestation_receipt_id)
           FROM compute_external_pool_adapter_credential_reattestation_receipts candidate
          WHERE provider_binding_id=?1 AND NOT EXISTS(
                SELECT 1
                  FROM compute_external_pool_adapter_credential_reattestation_receipts successor
                 WHERE successor.predecessor_receipt_id=candidate.reattestation_receipt_id)",
        params![provider_binding_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if count > 1 {
        bail!("credential re-attestation history has multiple immutable heads");
    }
    id.map(|value| {
        receipt_by_id_on(conn, &value)?.ok_or_else(|| {
            anyhow::anyhow!("credential re-attestation head disappeared during read")
        })
    })
    .transpose()
}

fn receipt_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredCredentialReattestation>> {
    conn.query_row(
        &format!(
            "SELECT receipt_json FROM compute_external_pool_adapter_credential_reattestation_receipts WHERE {filter}"
        ),
        values,
        |row| decode_receipt(row.get(0)?),
    )
    .optional()?
    .map(|stored| audit_receipt(conn, stored))
    .transpose()
}

fn decode_receipt(json: String) -> rusqlite::Result<StoredCredentialReattestation> {
    let receipt = serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
    })?;
    Ok(StoredCredentialReattestation {
        receipt,
        receipt_json: json,
    })
}

pub(in crate::store) fn historical_external_pool_adapter_credential_reattestation_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_digest: &str,
) -> Result<Option<HistoricalExternalPoolAdapterCredentialReattestationAuthority>> {
    let Some(stored) = receipt_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.reattestation_receipt_digest != expected_digest {
        bail!("credential re-attestation history is not exact");
    }
    Ok(Some(
        HistoricalExternalPoolAdapterCredentialReattestationAuthority::new(stored.receipt),
    ))
}

pub(super) fn revocation_by_receipt_on(
    conn: &Connection,
    receipt_id: &str,
) -> Result<Option<StoredCredentialReattestationRevocation>> {
    revocation_on(conn, "reattestation_receipt_id=?1", params![receipt_id])
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredCredentialReattestationRevocation>> {
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
) -> Result<Option<StoredCredentialReattestationRevocation>> {
    conn.query_row(
        &format!(
            "SELECT receipt_json FROM compute_external_pool_adapter_credential_reattestation_revocations WHERE {filter}"
        ),
        values,
        |row| {
            let receipt_json: String = row.get(0)?;
            let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
            })?;
            Ok(StoredCredentialReattestationRevocation {
                receipt,
                receipt_json,
            })
        },
    )
    .optional()?
    .map(|stored| audit_revocation(conn, stored))
    .transpose()
}

fn audit_revocation(
    conn: &Connection,
    stored: StoredCredentialReattestationRevocation,
) -> Result<StoredCredentialReattestationRevocation> {
    validate_credential_reattestation_revocation_receipt(&stored.receipt)?;
    let (json, digest) =
        credential_reattestation_revocation_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.revocation;
    let target = receipt_by_id_on(conn, &item.reattestation_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("credential re-attestation revocation lost target"))?;
    let binding = &target.receipt.reattestation.binding;
    if json != stored.receipt_json
        || digest != stored.receipt.revocation_receipt_digest
        || item.reattestation_receipt_digest != target.receipt.reattestation_receipt_digest
        || item.provider_binding_id != binding.provider_binding_id
        || item.provider_binding_digest != binding.provider_binding_digest
        || DateTime::parse_from_rfc3339(&item.revoked_at)?
            < DateTime::parse_from_rfc3339(&target.receipt.reattestation.verified_at)?
        || !exact_revocation_projection(conn, &stored)?
    {
        bail!("credential re-attestation revocation failed exact historical audit");
    }
    Ok(stored)
}

impl Store {
    pub(crate) fn external_pool_adapter_credential_reattestation_challenge_exists(
        &self,
        challenge_id: &str,
        provider_binding_id: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        Ok(conn
            .query_row(
                "SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_challenges
                  WHERE challenge_id=?1 AND provider_binding_id=?2",
                params![challenge_id, provider_binding_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    pub(crate) fn external_pool_adapter_credential_reattestation_exists(
        &self,
        receipt_id: &str,
        provider_binding_id: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        Ok(conn
            .query_row(
                "SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts
                  WHERE reattestation_receipt_id=?1 AND provider_binding_id=?2",
                params![receipt_id, provider_binding_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }
}
