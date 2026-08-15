use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_provider_runtime_readiness::*;

use super::{audit::*, types::*};

pub(super) fn readiness_by_id_on(
    conn: &Connection,
    readiness_receipt_id: &str,
) -> Result<Option<StoredProviderRuntimeReadiness>> {
    readiness_on(
        conn,
        "readiness_receipt_id=?1",
        params![readiness_receipt_id],
    )
}

pub(super) fn readiness_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredProviderRuntimeReadiness>> {
    readiness_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn readiness_head_by_binding_on(
    conn: &Connection,
    provider_binding_id: &str,
) -> Result<Option<StoredProviderRuntimeReadiness>> {
    readiness_on(
        conn,
        "provider_binding_id=?1 ORDER BY sequence DESC LIMIT 1",
        params![provider_binding_id],
    )
}

fn readiness_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredProviderRuntimeReadiness>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT readiness_receipt_json
                   FROM compute_external_pool_adapter_provider_runtime_readiness_receipts
                  WHERE {filter}"
            ),
            values,
            |row| decode(row, 0),
        )
        .optional()?
        .map(|(receipt, receipt_json)| StoredProviderRuntimeReadiness {
            receipt,
            receipt_json,
        });
    stored
        .map(|stored| audit_readiness(conn, stored))
        .transpose()
}

pub(super) fn revocation_by_readiness_on(
    conn: &Connection,
    readiness_receipt_id: &str,
) -> Result<Option<StoredProviderRuntimeReadinessRevocation>> {
    revocation_on(
        conn,
        "readiness_receipt_id=?1",
        params![readiness_receipt_id],
    )
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredProviderRuntimeReadinessRevocation>> {
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
) -> Result<Option<StoredProviderRuntimeReadinessRevocation>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT revocation_receipt_json
                   FROM compute_external_pool_adapter_provider_runtime_readiness_revocations
                  WHERE {filter}"
            ),
            values,
            |row| decode(row, 0),
        )
        .optional()?
        .map(
            |(receipt, receipt_json)| StoredProviderRuntimeReadinessRevocation {
                receipt,
                receipt_json,
            },
        );
    stored
        .map(|stored| audit_revocation(conn, stored))
        .transpose()
}

fn audit_readiness(
    conn: &Connection,
    stored: StoredProviderRuntimeReadiness,
) -> Result<StoredProviderRuntimeReadiness> {
    validate_provider_runtime_readiness_receipt(&stored.receipt)?;
    if canonical_provider_runtime_readiness_receipt_json_and_digest(&stored.receipt)?.0
        != stored.receipt_json
    {
        bail!("provider runtime readiness receipt JSON is not canonical")
    }
    audit_readiness_projection(conn, &stored.receipt, &stored.receipt_json)?;
    Ok(stored)
}

fn audit_revocation(
    conn: &Connection,
    stored: StoredProviderRuntimeReadinessRevocation,
) -> Result<StoredProviderRuntimeReadinessRevocation> {
    validate_provider_runtime_readiness_revocation_receipt(&stored.receipt)?;
    if canonical_provider_runtime_readiness_revocation_json_and_digest(&stored.receipt)?.0
        != stored.receipt_json
    {
        bail!("provider runtime readiness revocation JSON is not canonical")
    }
    audit_revocation_projection(conn, &stored.receipt, &stored.receipt_json)?;
    Ok(stored)
}

fn decode<T: serde::de::DeserializeOwned>(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<(T, String)> {
    let json: String = row.get(index)?;
    let receipt = serde_json::from_str(&json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })?;
    Ok((receipt, json))
}

pub(super) fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("provider runtime readiness identifier is invalid")
    }
    Ok(())
}
