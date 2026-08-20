use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_task_protocol_conformance::{
            server_task_protocol_conformance_fixture_catalog,
            server_task_protocol_conformance_profile_catalog,
            ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
            ExternalPoolAdapterTaskProtocolConformanceRunRoots,
        },
    },
    store::{
        compute_external_pool_adapter_provider_active_successor::{
            current_external_pool_adapter_projected_active_historical_carrier_on,
            historical_external_pool_adapter_atomic_activation_for_binding_on,
            CurrentExternalPoolAdapterProjectedActiveHistoricalCarrierAuthority,
            HistoricalExternalPoolAdapterAtomicActivationAuthority,
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::super::{
    roots::into_execution_input_from_observation, run::TaskProtocolConformanceExecutionInput,
};
use super::super::{
    roots::{domain_roots_from_parts, require_current_admin, task_protocol_conformance_expires_at},
    types::CreateExternalPoolAdapterTaskProtocolConformanceRun,
};

pub(super) struct CurrentProjectedActiveTaskProtocolRoots<'tx, 'conn> {
    pub(super) carrier:
        CurrentExternalPoolAdapterProjectedActiveHistoricalCarrierAuthority<'tx, 'conn>,
    pub(super) vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    pub(super) sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
}

pub(super) fn current_active_roots_for_create_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    input: &CreateExternalPoolAdapterTaskProtocolConformanceRun,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<CurrentProjectedActiveTaskProtocolRoots<'tx, 'conn>> {
    require_current_admin(transaction, &input.recorded_by_admin_user_id)?;
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    if input.expected_task_protocol_profile_digest != profile.profile_digest
        || input.expected_fixture_catalog_digest != fixture.catalog_digest
    {
        bail!("projected-active V272 profile or fixture catalog CAS is stale");
    }
    let historical = historical_external_pool_adapter_atomic_activation_for_binding_on(
        transaction,
        &input.provider_binding_id,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("projected-active V272 lacks durable V277 history"))?;
    let roots = current_active_roots_on(
        transaction,
        historical,
        prepared,
        &input.sandbox_reattestation_receipt_id,
        &input.expected_sandbox_reattestation_receipt_digest,
        checked_at,
    )?;
    let root = &roots
        .carrier
        .historical_activation()
        .activation_root()
        .activation_root;
    let runtime = roots.carrier.runtime_compatibility().verification();
    if root.provider_binding_digest != input.expected_provider_binding_digest
        || root.registry_release_id != input.registry_release_id
        || root.registry_release_digest != input.expected_registry_release_digest
        || root.installation_receipt_id != input.expected_installation_receipt_id
        || root.installation_receipt_digest != input.expected_installation_receipt_digest
        || runtime.verification_receipt_id != input.runtime_compatibility_verification_receipt_id
        || runtime.verification_receipt_digest
            != input.expected_runtime_compatibility_verification_receipt_digest
    {
        bail!("projected-active V272 input roots are not exact");
    }
    Ok(roots)
}

pub(super) fn current_active_roots_for_receipt_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    historical: HistoricalExternalPoolAdapterAtomicActivationAuthority,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<CurrentProjectedActiveTaskProtocolRoots<'tx, 'conn>> {
    let expected = &receipt.run;
    let roots = current_active_roots_on(
        transaction,
        historical,
        prepared,
        &expected.sandbox_reattestation.reattestation_receipt_id,
        &expected.sandbox_reattestation.reattestation_receipt_digest,
        checked_at,
    )?;
    let projected = projected_domain_roots(&roots)?;
    if projected.registry_release != expected.registry_release
        || projected.vulnerability_reattestation != expected.vulnerability_reattestation
        || projected.sandbox_reattestation != expected.sandbox_reattestation
        || projected.sandbox_verifier_key != expected.sandbox_verifier_key
        || projected.runtime_compatibility != expected.runtime_compatibility
    {
        bail!("projected-active V272 receipt roots are not current and exact");
    }
    Ok(roots)
}

fn current_active_roots_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    historical: HistoricalExternalPoolAdapterAtomicActivationAuthority,
    prepared: PreparedExternalPoolAdapterInstallation,
    sandbox_receipt_id: &str,
    sandbox_receipt_digest: &str,
    checked_at: &str,
) -> Result<CurrentProjectedActiveTaskProtocolRoots<'tx, 'conn>> {
    let carrier = current_external_pool_adapter_projected_active_historical_carrier_on(
        transaction,
        historical,
        prepared,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("projected-active V272 carrier reproof failed"))?;
    let release = carrier.registry_release().release();
    let sandbox = current_external_pool_adapter_sandbox_reattestation_authority_on(
        transaction,
        &release.registry_release_id,
        sandbox_receipt_id,
        sandbox_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("projected-active V272 lacks exact current V252"))?;
    let sandbox_binding = &sandbox.receipt().reattestation.binding;
    let vulnerability = current_external_pool_adapter_vulnerability_reattestation_authority_on(
        transaction,
        &release.registry_release_id,
        &sandbox_binding.vulnerability_reattestation_receipt_id,
        &sandbox_binding.vulnerability_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("projected-active V272 lacks exact current V250"))?;
    if carrier.checked_at() != checked_at
        || sandbox.checked_at() != checked_at
        || vulnerability.checked_at() != checked_at
    {
        bail!("projected-active V272 roots use different checked_at anchors");
    }
    Ok(CurrentProjectedActiveTaskProtocolRoots {
        carrier,
        vulnerability,
        sandbox,
    })
}

pub(super) fn projected_domain_roots(
    roots: &CurrentProjectedActiveTaskProtocolRoots<'_, '_>,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceRunRoots> {
    domain_roots_from_parts(
        roots.carrier.registry_release().release(),
        roots.vulnerability.receipt(),
        roots.sandbox.receipt(),
        roots.carrier.runtime_compatibility(),
    )
}

pub(super) fn projected_expiry(
    checked_at: &str,
    roots: &CurrentProjectedActiveTaskProtocolRoots<'_, '_>,
) -> Result<String> {
    task_protocol_conformance_expires_at(checked_at, &projected_domain_roots(roots)?)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) fn into_projected_execution_input(
    roots: CurrentProjectedActiveTaskProtocolRoots<'_, '_>,
) -> Result<TaskProtocolConformanceExecutionInput> {
    let projected = projected_domain_roots(&roots)?;
    let observation = roots
        .carrier
        .runtime_compatibility()
        .run_observation()
        .clone();
    let prepared = roots.carrier.into_prepared();
    into_execution_input_from_observation(projected, &observation, prepared)
}
