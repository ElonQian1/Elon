use anyhow::{bail, Result};
use rusqlite::{params, Transaction};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_task_protocol_conformance::*,
    },
    store::{
        compute_external_pool_adapter_registry::{
            current_external_pool_adapter_registry_provider_binding_authority_on,
            CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
        },
        compute_external_pool_adapter_runtime_compatibility_verification::{
            current_external_pool_adapter_runtime_compatibility_verification_authority_on,
            CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
        },
        compute_external_pool_adapter_sandbox_reattestation::{
            current_external_pool_adapter_sandbox_reattestation_authority_on,
            CurrentExternalPoolAdapterSandboxReattestationAuthority,
        },
        compute_external_pool_adapter_vulnerability_reattestation::{
            current_external_pool_adapter_vulnerability_reattestation_authority_on,
            CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
        },
    },
};

use super::types::CreateExternalPoolAdapterTaskProtocolConformanceRun;

mod projection;
mod reproof;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) use projection::into_execution_input;
pub(super) use projection::{canonical_time, domain_roots, task_protocol_conformance_expires_at};
use reproof::{audit_carrier, audit_current_roots, audit_domain_roots};

pub(super) struct CurrentTaskProtocolConformanceRoots<'tx, 'conn> {
    pub(super) carrier: CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    pub(super) vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    pub(super) sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
    pub(super) runtime_compatibility:
        CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn>,
}

pub(super) struct CarrierExpectation<'a> {
    pub(super) provider_binding_id: &'a str,
    pub(super) expected_provider_binding_digest: Option<&'a str>,
    pub(super) expected_installation_receipt_id: Option<&'a str>,
    pub(super) expected_installation_receipt_digest: Option<&'a str>,
}

pub(super) fn current_roots_for_create_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    input: &CreateExternalPoolAdapterTaskProtocolConformanceRun,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<CurrentTaskProtocolConformanceRoots<'tx, 'conn>> {
    require_current_admin(transaction, &input.recorded_by_admin_user_id)?;
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    if input.expected_task_protocol_profile_digest != profile.profile_digest
        || input.expected_fixture_catalog_digest != fixture.catalog_digest
    {
        bail!("task-protocol conformance profile or fixture catalog CAS is stale")
    }
    let roots = current_roots_on(
        transaction,
        &input.registry_release_id,
        &input.sandbox_reattestation_receipt_id,
        &input.expected_sandbox_reattestation_receipt_digest,
        &input.runtime_compatibility_verification_receipt_id,
        &input.expected_runtime_compatibility_verification_receipt_digest,
        CarrierExpectation {
            provider_binding_id: &input.provider_binding_id,
            expected_provider_binding_digest: Some(&input.expected_provider_binding_digest),
            expected_installation_receipt_id: Some(&input.expected_installation_receipt_id),
            expected_installation_receipt_digest: Some(&input.expected_installation_receipt_digest),
        },
        prepared,
        checked_at,
    )?;
    let release = roots.carrier.release();
    if release.registry_release_digest != input.expected_registry_release_digest {
        bail!("task-protocol conformance expected V249 release digest is not exact")
    }
    Ok(roots)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn current_roots_for_receipt_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    provider_binding_id: &str,
    expected_provider_binding_digest: &str,
    expected_installation_receipt_id: &str,
    expected_installation_receipt_digest: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<CurrentTaskProtocolConformanceRoots<'tx, 'conn>> {
    let r = &receipt.run;
    let roots = current_roots_on(
        transaction,
        &r.registry_release.registry_release_id,
        &r.sandbox_reattestation.reattestation_receipt_id,
        &r.sandbox_reattestation.reattestation_receipt_digest,
        &r.runtime_compatibility.verification_receipt_id,
        &r.runtime_compatibility.verification_receipt_digest,
        CarrierExpectation {
            provider_binding_id,
            expected_provider_binding_digest: Some(expected_provider_binding_digest),
            expected_installation_receipt_id: Some(expected_installation_receipt_id),
            expected_installation_receipt_digest: Some(expected_installation_receipt_digest),
        },
        prepared,
        checked_at,
    )?;
    audit_domain_roots(&roots, r, checked_at)?;
    Ok(roots)
}

#[allow(clippy::too_many_arguments)]
fn current_roots_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    registry_release_id: &str,
    sandbox_receipt_id: &str,
    sandbox_receipt_digest: &str,
    verification_receipt_id: &str,
    verification_receipt_digest: &str,
    carrier: CarrierExpectation<'_>,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<CurrentTaskProtocolConformanceRoots<'tx, 'conn>> {
    let carrier_authority = current_external_pool_adapter_registry_provider_binding_authority_on(
        transaction,
        carrier.provider_binding_id,
        prepared,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("task-protocol conformance carrier was not found"))?;
    audit_carrier(
        &carrier_authority,
        &carrier,
        registry_release_id,
        checked_at,
    )?;
    let sandbox = current_external_pool_adapter_sandbox_reattestation_authority_on(
        transaction,
        registry_release_id,
        sandbox_receipt_id,
        sandbox_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("task-protocol conformance lost exact current V252"))?;
    let sandbox_binding = &sandbox.receipt().reattestation.binding;
    let vulnerability = current_external_pool_adapter_vulnerability_reattestation_authority_on(
        transaction,
        registry_release_id,
        &sandbox_binding.vulnerability_reattestation_receipt_id,
        &sandbox_binding.vulnerability_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("task-protocol conformance lost exact current V250"))?;
    let runtime_compatibility =
        current_external_pool_adapter_runtime_compatibility_verification_authority_on(
            transaction,
            verification_receipt_id,
            verification_receipt_digest,
            checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("task-protocol conformance lost exact current V268"))?;
    let roots = CurrentTaskProtocolConformanceRoots {
        carrier: carrier_authority,
        vulnerability,
        sandbox,
        runtime_compatibility,
    };
    audit_current_roots(&roots, checked_at)?;
    Ok(roots)
}

pub(super) fn require_current_admin(
    conn: &rusqlite::Connection,
    admin_user_id: &str,
) -> Result<()> {
    let current: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users
          WHERE id=?1 AND status='active' AND role IN ('admin','owner'))",
        params![admin_user_id],
        |row| row.get(0),
    )?;
    if !current {
        bail!("task-protocol conformance actor is not a current platform administrator")
    }
    Ok(())
}
