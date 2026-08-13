use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_provider_activation_candidate::{
        validate_activation_candidate_receipt, validate_activation_delegation_receipt,
        validate_activation_delegation_revocation_receipt,
    },
    store::{
        compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
        Store,
    },
};

use super::types::*;
use super::{
    audit_candidate::audit_candidate,
    audit_delegation::audit_delegation,
    audit_revocation::audit_revocation,
    roots::{audit_candidate_derived_identity, audit_delegation_derived_identity},
};

pub(super) fn delegation_by_id_on(conn: &Connection, id: &str) -> Result<Option<StoredDelegation>> {
    delegation_on(conn, "delegation_id=?1", params![id])
}

pub(super) fn delegation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredDelegation>> {
    delegation_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn delegation_head_by_binding_on(
    conn: &Connection,
    provider_binding_id: &str,
) -> Result<Option<StoredDelegation>> {
    delegation_on(
        conn,
        "provider_binding_id=?1 ORDER BY sequence DESC LIMIT 1",
        params![provider_binding_id],
    )
}

fn delegation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredDelegation>> {
    conn.query_row(
        &format!("SELECT delegation_json FROM compute_external_pool_provider_activation_delegations WHERE {filter}"),
        values,
        |row| decode(row, 0).map(|(receipt, receipt_json)| StoredDelegation { receipt, receipt_json }),
    )
    .optional()?
    .map(|stored| {
        validate_activation_delegation_receipt(&stored.receipt)?;
        audit_delegation_derived_identity(&stored.receipt)?;
        audit_delegation(conn, stored)
    })
    .transpose()
}

pub(super) fn candidate_by_id_on(conn: &Connection, id: &str) -> Result<Option<StoredCandidate>> {
    candidate_on(conn, "candidate_id=?1", params![id])
}

pub(super) fn candidate_by_delegation_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredCandidate>> {
    candidate_on(conn, "delegation_id=?1", params![id])
}

pub(super) fn candidate_head_by_binding_on(
    conn: &Connection,
    provider_binding_id: &str,
) -> Result<Option<StoredCandidate>> {
    candidate_on(
        conn,
        "provider_binding_id=?1 ORDER BY sequence DESC LIMIT 1",
        params![provider_binding_id],
    )
}

fn candidate_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredCandidate>> {
    conn.query_row(
        &format!("SELECT candidate_json FROM compute_external_pool_provider_activation_candidates WHERE {filter}"),
        values,
        |row| decode(row, 0).map(|(receipt, receipt_json)| StoredCandidate { receipt, receipt_json }),
    )
    .optional()?
    .map(|stored| {
        validate_activation_candidate_receipt(&stored.receipt)?;
        audit_candidate_derived_identity(&stored.receipt)?;
        audit_candidate(conn, stored)
    })
    .transpose()
}

pub(super) fn revocation_by_delegation_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRevocation>> {
    revocation_on(conn, "delegation_id=?1", params![id])
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
    conn.query_row(
        &format!("SELECT revocation_json FROM compute_external_pool_provider_activation_delegation_revocations WHERE {filter}"),
        values,
        |row| decode(row, 0).map(|(receipt, receipt_json)| StoredRevocation { receipt, receipt_json }),
    )
    .optional()?
    .map(|stored| {
        validate_activation_delegation_revocation_receipt(&stored.receipt)?;
        audit_revocation(conn, stored)
    })
    .transpose()
}

impl Store {
    pub(crate) fn external_pool_provider_activation_candidate_audit_target(
        &self,
        candidate_id: &str,
    ) -> Result<Option<ExternalPoolProviderActivationCandidateAuditTarget>> {
        validate_identifier(candidate_id)?;
        let conn = self.conn()?;
        let Some(candidate) = candidate_by_id_on(&conn, candidate_id)? else {
            return Ok(None);
        };
        audit_target_on(&conn, &candidate)
    }

    pub(crate) fn external_pool_provider_activation_delegation_audit_target(
        &self,
        delegation_id: &str,
    ) -> Result<Option<ExternalPoolProviderActivationCandidateAuditTarget>> {
        validate_identifier(delegation_id)?;
        let conn = self.conn()?;
        let Some(candidate) = candidate_by_delegation_on(&conn, delegation_id)? else {
            return Ok(None);
        };
        audit_target_on(&conn, &candidate)
    }
}

fn audit_target_on(
    conn: &Connection,
    candidate: &StoredCandidate,
) -> Result<Option<ExternalPoolProviderActivationCandidateAuditTarget>> {
    let c = &candidate.receipt.candidate;
    let Some(installation) = external_pool_adapter_installation_receipt_authority_on(
        conn,
        &c.installation_receipt_id,
        &c.installation_receipt_digest,
    )?
    else {
        return Ok(None);
    };
    Ok(Some(ExternalPoolProviderActivationCandidateAuditTarget {
        candidate_id: candidate.receipt.candidate_id.clone(),
        candidate_digest: candidate.receipt.candidate_digest.clone(),
        provider_binding_id: c.provider_binding_id.clone(),
        provider_binding_digest: c.provider_binding_digest.clone(),
        provider_owner_account_id: c.provider_owner_account_id.clone(),
        installation_binding: installation.receipt().installation.binding.clone(),
    }))
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
        || value.chars().count() > 200
        || value.chars().any(char::is_control)
    {
        bail!("activation candidate identifier is invalid");
    }
    Ok(())
}
