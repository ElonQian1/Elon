use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_upstream_transport_target::{
        validate_upstream_transport_target_receipt,
        validate_upstream_transport_target_revocation_receipt,
        ExternalPoolAdapterUpstreamTransportTargetReceipt,
    },
    store::{
        compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
        Store,
    },
};

use super::{audit::*, types::*};

pub(super) fn target_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredUpstreamTransportTarget>> {
    target_on(conn, "target_id=?1", params![id])
}

pub(in crate::store) fn historical_external_pool_adapter_upstream_transport_target_authority_on(
    conn: &Connection,
    target_id: &str,
    expected_target_digest: &str,
) -> Result<Option<ExternalPoolAdapterUpstreamTransportTargetReceipt>> {
    let Some(stored) = target_by_id_on(conn, target_id)? else {
        return Ok(None);
    };
    if stored.receipt.target_digest != expected_target_digest {
        bail!("historical upstream transport target digest is not exact");
    }
    Ok(Some(stored.receipt))
}

pub(super) fn target_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredUpstreamTransportTarget>> {
    target_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn target_head_by_binding_on(
    conn: &Connection,
    provider_binding_id: &str,
) -> Result<Option<StoredUpstreamTransportTarget>> {
    target_on(
        conn,
        "provider_binding_id=?1 ORDER BY sequence DESC LIMIT 1",
        params![provider_binding_id],
    )
}

fn target_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredUpstreamTransportTarget>> {
    conn.query_row(
        &format!(
            "SELECT target_json FROM compute_external_pool_adapter_upstream_transport_targets WHERE {filter}"
        ),
        values,
        |row| {
            decode(row, 0).map(|(receipt, receipt_json)| StoredUpstreamTransportTarget {
                receipt,
                receipt_json,
            })
        },
    )
    .optional()?
    .map(|stored| {
        validate_upstream_transport_target_receipt(&stored.receipt)?;
        audit_target(conn, stored)
    })
    .transpose()
}

pub(super) fn revocation_by_target_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredUpstreamTransportTargetRevocation>> {
    revocation_on(conn, "target_id=?1", params![id])
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredUpstreamTransportTargetRevocation>> {
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
) -> Result<Option<StoredUpstreamTransportTargetRevocation>> {
    conn.query_row(
        &format!(
            "SELECT revocation_json FROM compute_external_pool_adapter_upstream_transport_target_revocations WHERE {filter}"
        ),
        values,
        |row| {
            decode(row, 0).map(|(receipt, receipt_json)| {
                StoredUpstreamTransportTargetRevocation {
                    receipt,
                    receipt_json,
                }
            })
        },
    )
    .optional()?
    .map(|stored| {
        validate_upstream_transport_target_revocation_receipt(&stored.receipt)?;
        audit_revocation(conn, stored)
    })
    .transpose()
}

impl Store {
    pub(crate) fn external_pool_adapter_upstream_transport_target_audit_target(
        &self,
        target_id: &str,
    ) -> Result<Option<ExternalPoolAdapterUpstreamTransportTargetAuditTarget>> {
        validate_identifier(target_id)?;
        let conn = self.conn()?;
        let Some(target) = target_by_id_on(&conn, target_id)? else {
            return Ok(None);
        };
        audit_target_on(&conn, &target)
    }
}

fn audit_target_on(
    conn: &Connection,
    target: &StoredUpstreamTransportTarget,
) -> Result<Option<ExternalPoolAdapterUpstreamTransportTargetAuditTarget>> {
    let t = &target.receipt.target;
    let Some(installation) = external_pool_adapter_installation_receipt_authority_on(
        conn,
        &t.installation_receipt_id,
        &t.installation_receipt_digest,
    )?
    else {
        return Ok(None);
    };
    Ok(Some(
        ExternalPoolAdapterUpstreamTransportTargetAuditTarget {
            target_id: target.receipt.target_id.clone(),
            target_digest: target.receipt.target_digest.clone(),
            profile_id: t.profile_id.clone(),
            profile_digest: t.profile_digest.clone(),
            candidate_id: t.candidate_id.clone(),
            provider_binding_id: t.provider_binding_id.clone(),
            provider_owner_account_id: t.provider_owner_account_id.clone(),
            installation_binding: installation.receipt().installation.binding.clone(),
        },
    ))
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

fn validate_identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("upstream transport target identifier is invalid");
    }
    Ok(())
}
