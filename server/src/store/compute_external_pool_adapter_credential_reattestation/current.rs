use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        external_pool_adapter_credential_reattestation::CREDENTIAL_REATTESTATION_CURRENTNESS_SCHEMA,
        provider::{
            PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING,
        },
    },
    store::{
        compute_external_pool_adapter_adoption::external_pool_adapter_adoption_is_revoked_on,
        compute_external_pool_adapter_credential_verifier::credential_verifier_is_current_exact_on,
        compute_external_pool_adapter_credential_verifier_key::credential_verifier_key_is_current_exact_on,
        compute_external_pool_adapter_installation::external_pool_adapter_installation_is_revoked_on,
        compute_external_pool_adapter_registry::external_pool_adapter_registry_release_is_current_exact_on,
        compute_external_pool_onboarding::historical_external_pool_onboarding_application_authority_on,
        compute_provider_registry::current_registered_provider_on, Store,
    },
};

use super::{read::*, types::*};

pub(super) fn currentness_on(
    conn: &Connection,
    provider_binding_id: &str,
    checked_at: &str,
) -> Result<Option<ExternalPoolAdapterCredentialReattestationCurrentness>> {
    let checked = canonical_checked_at(checked_at)?;
    let Some(head) = head_by_provider_binding_on(conn, provider_binding_id)? else {
        return Ok(None);
    };
    let receipt = &head.receipt;
    let b = &receipt.reattestation.binding;
    let revocation = revocation_by_receipt_on(conn, &receipt.reattestation_receipt_id)?;
    let release_current = release_is_current(conn, b, checked_at)?;
    let (subject_exact, revision_status) = provider_statuses(conn, b)?;
    let key_current = verifier_key_is_current(conn, b)?;
    let upstream_terminal =
        external_pool_adapter_installation_is_revoked_on(conn, &b.installation_receipt_id)?
            || external_pool_adapter_adoption_is_revoked_on(conn, &b.adoption_receipt_id)?;
    let verified = DateTime::parse_from_rfc3339(&receipt.reattestation.verified_at)?;
    let expires = DateTime::parse_from_rfc3339(&b.report_expires_at)?;
    let report_current = checked >= verified && checked < expires;
    let provider_current = subject_exact
        && matches!(
            revision_status,
            "exact_registering" | "adjacent_active" | "exact_active"
        );
    let verified_current = release_current
        && !upstream_terminal
        && provider_current
        && key_current
        && report_current
        && revocation.is_none();

    Ok(Some(
        ExternalPoolAdapterCredentialReattestationCurrentness {
            schema: CREDENTIAL_REATTESTATION_CURRENTNESS_SCHEMA,
            reattestation: head.summary(),
            revocation: revocation.as_ref().map(|item| item.summary()),
            current_status: status(verified_current, "verified_current", "historical_only"),
            head_status: "head".into(),
            provider_binding_status: "binding_exact".into(),
            registry_release_status: status(release_current, "release_current", "historical_only"),
            provider_subject_status: status(subject_exact, "subject_exact", "drifted"),
            provider_revision_status: revision_status.into(),
            credential_verifier_key_status: status(key_current, "active", "revoked"),
            report_validity_status: status(report_current, "current", "expired"),
            revocation_status: status(revocation.is_none(), "none", "revoked"),
        },
    ))
}

pub(in crate::store) fn current_external_pool_adapter_credential_reattestation_authority_on(
    conn: &Connection,
    provider_binding_id: &str,
    expected_receipt_id: &str,
    expected_receipt_digest: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterCredentialReattestationAuthority>> {
    let Some(currentness) = currentness_on(conn, provider_binding_id, checked_at)? else {
        return Ok(None);
    };
    if currentness.current_status != "verified_current"
        || currentness.reattestation.reattestation_receipt_id != expected_receipt_id
        || currentness.reattestation.reattestation_receipt_digest != expected_receipt_digest
    {
        bail!("credential re-attestation authority is not current and exact");
    }
    let stored = receipt_by_id_on(conn, expected_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("credential re-attestation disappeared"))?;
    Ok(Some(
        CurrentExternalPoolAdapterCredentialReattestationAuthority::new(
            stored.receipt,
            checked_at.to_string(),
        ),
    ))
}

/// Selects the current V253 head inside Store instead of trusting caller-supplied receipt roots.
pub(in crate::store) fn current_external_pool_adapter_credential_reattestation_head_authority_on(
    conn: &Connection,
    provider_binding_id: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterCredentialReattestationAuthority>> {
    let Some(currentness) = currentness_on(conn, provider_binding_id, checked_at)? else {
        return Ok(None);
    };
    if currentness.current_status != "verified_current" {
        bail!("credential re-attestation head authority is not current");
    }
    current_external_pool_adapter_credential_reattestation_authority_on(
        conn,
        provider_binding_id,
        &currentness.reattestation.reattestation_receipt_id,
        &currentness.reattestation.reattestation_receipt_digest,
        checked_at,
    )
}

fn release_is_current(
    conn: &Connection,
    b: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationBinding,
    checked_at: &str,
) -> Result<bool> {
    external_pool_adapter_registry_release_is_current_exact_on(
        conn,
        &b.registry_release_id,
        &b.registry_release_digest,
        checked_at,
    )
}

fn provider_statuses(
    conn: &Connection,
    b: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<(bool, &'static str)> {
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        conn,
        &b.application_id,
        &b.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("credential re-attestation lost V221 subject root"))?;
    let Some(current) = current_registered_provider_on(conn, &b.provider_id)? else {
        return Ok((false, "historical_only"));
    };
    let provider = &current.provider;
    let adapter = provider.adapter.as_ref();
    let subject_exact = provider.provider_id == b.provider_id
        && provider.provider_kind == PROVIDER_KIND_EXTERNAL_POOL
        && provider.owner_account_id == b.provider_owner_account_id
        && provider.created_at == onboarding.provider().created_at
        && adapter.map(|item| item.adapter_id.as_str()) == Some(b.adapter_id.as_str())
        && adapter.map(|item| item.adapter_version.as_str()) == Some(b.release_version.as_str())
        && adapter.map(|item| item.config_revision) == Some(b.adapter_config_revision)
        && adapter.map(|item| item.config_digest.as_str())
            == Some(b.adapter_config_digest.as_str());
    if !subject_exact {
        return Ok((false, "historical_only"));
    }
    let revision = if b.observed_provider_status == PROVIDER_STATUS_REGISTERING
        && provider.status == PROVIDER_STATUS_REGISTERING
        && provider.policy_revision == b.observed_provider_policy_revision
        && current.provider_digest == b.observed_provider_digest
    {
        "exact_registering"
    } else if b.observed_provider_status == PROVIDER_STATUS_REGISTERING
        && provider.status == PROVIDER_STATUS_ACTIVE
        && b.observed_provider_policy_revision.checked_add(1) == Some(provider.policy_revision)
    {
        "adjacent_active"
    } else if b.observed_provider_status == PROVIDER_STATUS_ACTIVE
        && provider.status == PROVIDER_STATUS_ACTIVE
        && provider.policy_revision == b.observed_provider_policy_revision
        && current.provider_digest == b.observed_provider_digest
    {
        "exact_active"
    } else {
        "historical_only"
    };
    Ok((true, revision))
}

fn verifier_key_is_current(
    conn: &Connection,
    b: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<bool> {
    let key_current = credential_verifier_key_is_current_exact_on(
        conn,
        &b.credential_verifier_key_record_id,
        &b.credential_verifier_key_record_digest,
        &b.credential_verifier_key_id,
    )?;
    let verifier_current = credential_verifier_is_current_exact_on(
        conn,
        &b.credential_verifier_record_id,
        &b.credential_verifier_record_digest,
        &b.expected_credential_verifier.verification_kind,
        &b.expected_credential_verifier.verifier_id,
        b.expected_credential_verifier.verifier_revision,
        &b.expected_credential_verifier.verifier_digest,
    )?;
    Ok(key_current && verifier_current)
}

fn canonical_checked_at(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
        || parsed > Utc::now() + chrono::Duration::minutes(5)
    {
        bail!("credential re-attestation checked_at is not canonical UTC nanoseconds");
    }
    Ok(parsed)
}

fn status(condition: bool, yes: &str, no: &str) -> String {
    if condition { yes } else { no }.into()
}

impl Store {
    pub(crate) fn external_pool_adapter_credential_reattestation_currentness(
        &self,
        provider_binding_id: &str,
    ) -> Result<Option<ExternalPoolAdapterCredentialReattestationCurrentness>> {
        let conn = self.conn()?;
        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        currentness_on(&conn, provider_binding_id, &checked_at)
    }

    #[cfg(test)]
    pub(crate) fn external_pool_adapter_credential_reattestation_currentness_at(
        &self,
        provider_binding_id: &str,
        checked_at: &str,
    ) -> Result<Option<ExternalPoolAdapterCredentialReattestationCurrentness>> {
        let conn = self.conn()?;
        currentness_on(&conn, provider_binding_id, checked_at)
    }
}
