use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;

use crate::{
    compute_federation::external_pool_provider_activation_candidate::{
        ACTIVATION_CANDIDATE_CURRENTNESS_SCHEMA, ACTIVATION_CANDIDATE_STATUS,
        ACTIVATION_CLOSURE_NOT_IMPLEMENTED, ACTIVATION_INPUTS_CURRENT, ACTIVATION_PREFLIGHT_SCHEMA,
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::current_external_pool_adapter_credential_reattestation_authority_on,
        compute_external_pool_adapter_registry::current_external_pool_adapter_registry_provider_binding_authority_on,
        compute_external_pool_adapter_sandbox_reattestation::current_external_pool_adapter_sandbox_reattestation_authority_on,
        compute_external_pool_adapter_vulnerability_reattestation::current_external_pool_adapter_vulnerability_reattestation_authority_on,
        Store,
    },
};

use super::{read::*, roots::audit_static_roots, types::*};

impl Store {
    pub(crate) fn external_pool_provider_activation_candidate_currentness(
        &self,
        candidate_id: &str,
        prepared: crate::compute_federation::external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    ) -> Result<Option<ExternalPoolProviderActivationCandidateCurrentness>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let checked_at = now();
        let Some(candidate) = candidate_by_id_on(&tx, candidate_id)? else {
            return Ok(None);
        };
        let delegation = current_delegation_for_candidate(&tx, &candidate)?;
        let authority = current_external_pool_adapter_registry_provider_binding_authority_on(
            &tx,
            &candidate.receipt.candidate.provider_binding_id,
            prepared,
            &checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("current exact V249 Provider binding was not found"))?;
        audit_static_roots(&authority, &delegation.receipt, &candidate.receipt)?;
        let receipt = ExternalPoolProviderActivationCandidateCurrentness {
            schema: ACTIVATION_CANDIDATE_CURRENTNESS_SCHEMA,
            delegation: delegation.summary(),
            candidate: candidate.summary(),
            current_status: ACTIVATION_CANDIDATE_STATUS.into(),
            provider_status: "registering".into(),
            file_inventory_status: "prepared_exact".into(),
            delegation_status: "current_unrevoked".into(),
            route_projection_status: "reserved_absent".into(),
            activation_closure_status: ACTIVATION_CLOSURE_NOT_IMPLEMENTED.into(),
            activation_ready: false,
            checked_at,
        };
        tx.commit()?;
        Ok(Some(receipt))
    }

    pub(crate) fn current_external_pool_provider_activation_preflight(
        &self,
        input: GetCurrentExternalPoolProviderActivationPreflight,
    ) -> Result<Option<ExternalPoolProviderActivationPreflightReceipt>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let checked_at = now();
        let Some(authority) = current_external_pool_provider_activation_preflight_authority_on(
            &tx,
            input,
            &checked_at,
        )?
        else {
            return Ok(None);
        };
        let receipt = ExternalPoolProviderActivationPreflightReceipt {
            schema: ACTIVATION_PREFLIGHT_SCHEMA,
            delegation: delegation_summary(authority.delegation()),
            candidate: candidate_summary(authority.candidate()),
            checked_at: authority.checked_at().to_string(),
            inputs_status: ACTIVATION_INPUTS_CURRENT.into(),
            activation_closure_status: ACTIVATION_CLOSURE_NOT_IMPLEMENTED.into(),
            activation_ready: false,
        };
        tx.commit()?;
        Ok(Some(receipt))
    }
}

pub(in crate::store) fn current_external_pool_provider_activation_preflight_authority_on(
    conn: &Connection,
    input: GetCurrentExternalPoolProviderActivationPreflight,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolProviderActivationPreflightAuthority>> {
    let GetCurrentExternalPoolProviderActivationPreflight {
        prepared,
        candidate_id,
        expected_candidate_digest,
        vulnerability_reattestation_receipt_id,
        expected_vulnerability_reattestation_receipt_digest,
        sandbox_reattestation_receipt_id,
        expected_sandbox_reattestation_receipt_digest,
        credential_reattestation_receipt_id,
        expected_credential_reattestation_receipt_digest,
    } = input;
    let Some(candidate) = candidate_by_id_on(conn, &candidate_id)? else {
        return Ok(None);
    };
    if candidate.receipt.candidate_digest != expected_candidate_digest {
        bail!("activation preflight candidate digest is not exact");
    }
    let delegation = current_delegation_for_candidate(conn, &candidate)?;
    let registry = current_external_pool_adapter_registry_provider_binding_authority_on(
        conn,
        &candidate.receipt.candidate.provider_binding_id,
        prepared,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current exact V249 Provider binding was not found"))?;
    audit_static_roots(&registry, &delegation.receipt, &candidate.receipt)?;
    let release_id = &candidate.receipt.candidate.registry_release_id;
    let vulnerability = current_external_pool_adapter_vulnerability_reattestation_authority_on(
        conn,
        release_id,
        &vulnerability_reattestation_receipt_id,
        &expected_vulnerability_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current exact V250 authority was not found"))?;
    let sandbox = current_external_pool_adapter_sandbox_reattestation_authority_on(
        conn,
        release_id,
        &sandbox_reattestation_receipt_id,
        &expected_sandbox_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current exact V252 authority was not found"))?;
    let credential = current_external_pool_adapter_credential_reattestation_authority_on(
        conn,
        &candidate.receipt.candidate.provider_binding_id,
        &credential_reattestation_receipt_id,
        &expected_credential_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current exact V253 authority was not found"))?;
    let authority = CurrentExternalPoolProviderActivationPreflightAuthority {
        registry,
        vulnerability,
        sandbox,
        credential,
        delegation: delegation.receipt,
        candidate: candidate.receipt,
        checked_at: checked_at.to_string(),
    };
    audit_dynamic_roots(&authority)?;
    Ok(Some(authority))
}

pub(in crate::store) fn current_external_pool_provider_activation_candidate_static_authority_on(
    conn: &Connection,
    prepared: crate::compute_federation::external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    candidate_id: &str,
    expected_candidate_digest: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolProviderActivationCandidateStaticAuthority>> {
    let Some(candidate) = candidate_by_id_on(conn, candidate_id)? else {
        return Ok(None);
    };
    if candidate.receipt.candidate_digest != expected_candidate_digest {
        bail!("activation candidate static authority digest is not exact");
    }
    let delegation = current_delegation_for_candidate(conn, &candidate)?;
    let registry = current_external_pool_adapter_registry_provider_binding_authority_on(
        conn,
        &candidate.receipt.candidate.provider_binding_id,
        prepared,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current exact V249 Provider binding was not found"))?;
    audit_static_roots(&registry, &delegation.receipt, &candidate.receipt)?;
    Ok(Some(
        CurrentExternalPoolProviderActivationCandidateStaticAuthority::new(
            registry,
            delegation.receipt,
            candidate.receipt,
            checked_at.to_string(),
        ),
    ))
}

pub(in crate::store) fn historical_external_pool_provider_activation_candidate_authority_on(
    conn: &Connection,
    candidate_id: &str,
    expected_candidate_digest: &str,
) -> Result<Option<HistoricalExternalPoolProviderActivationCandidateAuthority>> {
    let Some(candidate) = candidate_by_id_on(conn, candidate_id)? else {
        return Ok(None);
    };
    if candidate.receipt.candidate_digest != expected_candidate_digest {
        bail!("historical activation candidate digest is not exact");
    }
    let delegation = delegation_by_id_on(conn, &candidate.receipt.candidate.delegation_id)?
        .ok_or_else(|| anyhow::anyhow!("historical activation candidate lost delegation"))?;
    if delegation.receipt.delegation_digest != candidate.receipt.candidate.delegation_digest {
        bail!("historical activation candidate delegation root is not exact");
    }
    Ok(Some(
        HistoricalExternalPoolProviderActivationCandidateAuthority::new(
            delegation.receipt,
            candidate.receipt,
        ),
    ))
}

fn current_delegation_for_candidate(
    conn: &rusqlite::Connection,
    candidate: &StoredCandidate,
) -> Result<StoredDelegation> {
    let c = &candidate.receipt.candidate;
    let delegation = delegation_by_id_on(conn, &c.delegation_id)?
        .ok_or_else(|| anyhow::anyhow!("activation candidate lost delegation"))?;
    let head = delegation_head_by_binding_on(conn, &c.provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("activation candidate lost lineage head"))?;
    let candidate_head = candidate_head_by_binding_on(conn, &c.provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("activation candidate lost candidate head"))?;
    if delegation.receipt.delegation_id != head.receipt.delegation_id
        || candidate.receipt.candidate_id != candidate_head.receipt.candidate_id
        || revocation_by_delegation_on(conn, &c.delegation_id)?.is_some()
    {
        bail!("activation candidate is historical, superseded, or revoked");
    }
    Ok(delegation)
}

fn audit_dynamic_roots(
    authority: &CurrentExternalPoolProviderActivationPreflightAuthority,
) -> Result<()> {
    let candidate = &authority.candidate.candidate;
    let registry_binding = authority.registry.binding();
    let vulnerability = &authority.vulnerability.receipt().reattestation.binding;
    let sandbox = &authority.sandbox.receipt().reattestation.binding;
    let credential = &authority.credential.receipt().reattestation.binding;
    if authority.registry.checked_at() != authority.checked_at
        || authority.vulnerability.checked_at() != authority.checked_at
        || authority.sandbox.checked_at() != authority.checked_at
        || authority.credential.checked_at() != authority.checked_at
        || vulnerability.registry_release_id != candidate.registry_release_id
        || vulnerability.registry_release_digest != candidate.registry_release_digest
        || vulnerability.implementation_digest != candidate.implementation_digest
        || vulnerability.installation_content_digest != candidate.installation_content_digest
        || sandbox.registry_release_id != candidate.registry_release_id
        || sandbox.registry_release_digest != candidate.registry_release_digest
        || sandbox.implementation_digest != candidate.implementation_digest
        || sandbox.capability_set_digest != candidate.capability_set_digest
        || sandbox.credential_verifier_digest != candidate.credential_verifier_digest
        || sandbox.vulnerability_reattestation_receipt_id
            != authority.vulnerability.receipt().reattestation_receipt_id
        || sandbox.vulnerability_reattestation_receipt_digest
            != authority
                .vulnerability
                .receipt()
                .reattestation_receipt_digest
        || credential.provider_binding_id != candidate.provider_binding_id
        || credential.provider_binding_digest != candidate.provider_binding_digest
        || credential.registry_release_id != candidate.registry_release_id
        || credential.registry_release_digest != candidate.registry_release_digest
        || credential.route_adapter_projection_id != candidate.route_adapter_projection_id
        || credential.installation_receipt_id != candidate.installation_receipt_id
        || credential.installation_receipt_digest != candidate.installation_receipt_digest
        || credential.installation_content_digest != candidate.installation_content_digest
        || credential.provider_id != candidate.provider_id
        || credential.provider_owner_account_id != candidate.provider_owner_account_id
        || credential.observed_provider_policy_revision != candidate.provider_policy_revision
        || credential.observed_provider_digest != candidate.provider_digest
        || credential.observed_provider_status != candidate.provider_status
        || credential.adapter_id != candidate.logical_adapter_id
        || credential.release_version != candidate.release_version
        || credential.adapter_config_revision != candidate.adapter_config_revision
        || credential.adapter_config_digest != candidate.adapter_config_digest
        || credential.credential_verifier_digest != candidate.credential_verifier_digest
        || registry_binding.provider_binding_id != candidate.provider_binding_id
    {
        bail!("activation preflight dynamic current authorities do not close over one identity");
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
