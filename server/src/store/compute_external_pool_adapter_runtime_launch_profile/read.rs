use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_runtime_launch_profile::{
        validate_runtime_launch_profile_receipt, validate_runtime_launch_profile_revocation_receipt,
    },
    store::{
        compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
        Store,
    },
};

use super::{audit::*, types::*};

pub(super) fn profile_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRuntimeLaunchProfile>> {
    profile_on(conn, "profile_id=?1", params![id])
}

pub(super) fn profile_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRuntimeLaunchProfile>> {
    profile_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn profile_head_by_binding_on(
    conn: &Connection,
    provider_binding_id: &str,
) -> Result<Option<StoredRuntimeLaunchProfile>> {
    profile_on(
        conn,
        "provider_binding_id=?1 ORDER BY sequence DESC LIMIT 1",
        params![provider_binding_id],
    )
}

fn profile_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredRuntimeLaunchProfile>> {
    conn.query_row(
        &format!(
            "SELECT profile_json FROM compute_external_pool_adapter_runtime_launch_profiles WHERE {filter}"
        ),
        values,
        |row| {
            decode(row, 0).map(|(receipt, receipt_json)| StoredRuntimeLaunchProfile {
                receipt,
                receipt_json,
            })
        },
    )
    .optional()?
    .map(|stored| {
        validate_runtime_launch_profile_receipt(&stored.receipt)?;
        audit_profile(conn, stored)
    })
    .transpose()
}

pub(super) fn revocation_by_profile_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRuntimeLaunchProfileRevocation>> {
    revocation_on(conn, "profile_id=?1", params![id])
}

pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRuntimeLaunchProfileRevocation>> {
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
) -> Result<Option<StoredRuntimeLaunchProfileRevocation>> {
    conn.query_row(
        &format!(
            "SELECT revocation_json FROM compute_external_pool_adapter_runtime_launch_profile_revocations WHERE {filter}"
        ),
        values,
        |row| {
            decode(row, 0).map(|(receipt, receipt_json)| {
                StoredRuntimeLaunchProfileRevocation {
                    receipt,
                    receipt_json,
                }
            })
        },
    )
    .optional()?
    .map(|stored| {
        validate_runtime_launch_profile_revocation_receipt(&stored.receipt)?;
        audit_revocation(conn, stored)
    })
    .transpose()
}

impl Store {
    pub(crate) fn external_pool_adapter_runtime_launch_profile_audit_target(
        &self,
        profile_id: &str,
    ) -> Result<Option<ExternalPoolAdapterRuntimeLaunchProfileAuditTarget>> {
        validate_identifier(profile_id)?;
        let conn = self.conn()?;
        let Some(profile) = profile_by_id_on(&conn, profile_id)? else {
            return Ok(None);
        };
        audit_target_on(&conn, &profile)
    }
}

fn audit_target_on(
    conn: &Connection,
    profile: &StoredRuntimeLaunchProfile,
) -> Result<Option<ExternalPoolAdapterRuntimeLaunchProfileAuditTarget>> {
    let p = &profile.receipt.profile;
    let Some(installation) = external_pool_adapter_installation_receipt_authority_on(
        conn,
        &p.installation_receipt_id,
        &p.installation_receipt_digest,
    )?
    else {
        return Ok(None);
    };
    Ok(Some(ExternalPoolAdapterRuntimeLaunchProfileAuditTarget {
        profile_id: profile.receipt.profile_id.clone(),
        profile_digest: profile.receipt.profile_digest.clone(),
        candidate_id: p.candidate_id.clone(),
        candidate_digest: p.candidate_digest.clone(),
        provider_binding_id: p.provider_binding_id.clone(),
        provider_owner_account_id: p.provider_owner_account_id.clone(),
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
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("runtime launch profile identifier is invalid");
    }
    Ok(())
}
