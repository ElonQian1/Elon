use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_runtime_launch_profile::{
        validate_runtime_launch_profile_receipt,
        validate_runtime_launch_profile_revocation_receipt,
        ExternalPoolAdapterRuntimeLaunchProfileReceipt,
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

pub(in crate::store) fn historical_external_pool_adapter_runtime_launch_profile_authority_on(
    conn: &Connection,
    profile_id: &str,
    expected_profile_digest: &str,
) -> Result<Option<ExternalPoolAdapterRuntimeLaunchProfileReceipt>> {
    let Some(profile) = profile_by_id_on(conn, profile_id)? else {
        return Ok(None);
    };
    if profile.receipt.profile_digest != expected_profile_digest {
        bail!("historical runtime launch profile digest is not exact");
    }
    let p = &profile.receipt.profile;
    let candidate = crate::store::compute_external_pool_provider_activation_candidate::historical_external_pool_provider_activation_candidate_authority_on(
        conn,
        &p.candidate_id,
        &p.candidate_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical runtime launch profile lost V254 candidate"))?;
    let c = &candidate.candidate().candidate;
    if c.delegation_id != p.delegation_id
        || c.delegation_digest != p.delegation_digest
        || c.provider_binding_id != p.provider_binding_id
        || c.provider_binding_digest != p.provider_binding_digest
        || c.registry_release_id != p.registry_release_id
        || c.registry_release_digest != p.registry_release_digest
        || c.installation_receipt_id != p.installation_receipt_id
        || c.installation_receipt_digest != p.installation_receipt_digest
        || c.installation_content_digest != p.installation_content_digest
        || c.route_adapter_projection_id != p.route_adapter_projection_id
        || c.provider_id != p.provider_id
        || c.provider_owner_account_id != p.provider_owner_account_id
        || c.provider_policy_revision != p.provider_policy_revision
        || c.provider_digest != p.provider_digest
        || c.provider_status != p.provider_status
        || c.logical_adapter_id != p.logical_adapter_id
        || c.release_version != p.release_version
        || c.adapter_config_revision != p.adapter_config_revision
        || c.adapter_config_digest != p.adapter_config_digest
        || c.implementation_digest != p.implementation_digest
        || c.capability_set_digest != p.capability_set_digest
        || c.credential_verifier_digest != p.credential_verifier_digest
        || c.service_actor_id != p.service_actor_id
    {
        bail!("historical runtime launch profile V254 roots are not exact");
    }
    let binding = crate::store::compute_external_pool_adapter_registry::historical_external_pool_adapter_registry_provider_binding_authority_on(
        conn,
        &p.provider_binding_id,
        &p.provider_binding_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical runtime launch profile lost V249 binding"))?;
    let release = crate::store::compute_external_pool_adapter_registry::historical_external_pool_adapter_registry_release_authority_on(
        conn,
        &p.registry_release_id,
        &p.registry_release_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("historical runtime launch profile lost V249 release"))?;
    let b = &binding.binding().binding;
    let r = &release.release().release;
    if b.registry_release_id != p.registry_release_id
        || b.registry_release_digest != p.registry_release_digest
        || b.route_adapter_projection_id != p.route_adapter_projection_id
        || b.installation_receipt_id != p.installation_receipt_id
        || b.installation_receipt_digest != p.installation_receipt_digest
        || b.installation_content_digest != p.installation_content_digest
        || b.provider_id != p.provider_id
        || b.provider_owner_account_id != p.provider_owner_account_id
        || b.provider_policy_revision != p.provider_policy_revision
        || b.provider_digest != p.provider_digest
        || b.adapter_id != p.logical_adapter_id
        || b.release_version != p.release_version
        || b.adapter_config_revision != p.adapter_config_revision
        || b.adapter_config_digest != p.adapter_config_digest
        || r.adapter_id != p.logical_adapter_id
        || r.release_version != p.release_version
        || r.implementation_digest != p.implementation_digest
        || r.capability_set_digest != p.capability_set_digest
        || r.credential_verifier_digest != p.credential_verifier_digest
        || r.installation_content_digest != p.installation_content_digest
    {
        bail!("historical runtime launch profile V249 roots are not exact");
    }
    Ok(Some(profile.receipt))
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
