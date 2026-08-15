use anyhow::{bail, Result};

use crate::{
    compute_federation::external_pool_adapter_task_protocol_conformance::*,
    store::compute_external_pool_adapter_registry::CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
};

use super::{domain_roots, CarrierExpectation, CurrentTaskProtocolConformanceRoots};

pub(super) fn audit_carrier(
    authority: &CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    expected: &CarrierExpectation<'_>,
    registry_release_id: &str,
    checked_at: &str,
) -> Result<()> {
    let binding = authority.binding();
    let b = &binding.binding;
    if authority.checked_at() != checked_at
        || authority.release().registry_release_id != registry_release_id
        || b.registry_release_id != registry_release_id
        || expected
            .expected_provider_binding_digest
            .is_some_and(|value| binding.provider_binding_digest.as_str() != value)
        || expected
            .expected_installation_receipt_id
            .is_some_and(|value| b.installation_receipt_id.as_str() != value)
        || expected
            .expected_installation_receipt_digest
            .is_some_and(|value| b.installation_receipt_digest.as_str() != value)
    {
        bail!("task-protocol conformance execution carrier is not exact")
    }
    Ok(())
}

pub(super) fn audit_current_roots(
    roots: &CurrentTaskProtocolConformanceRoots<'_, '_>,
    checked_at: &str,
) -> Result<()> {
    let release = roots.carrier.release();
    let sandbox = roots.sandbox.receipt();
    let s = &sandbox.reattestation.binding;
    let vulnerability = roots.vulnerability.receipt();
    let v = &vulnerability.reattestation.binding;
    let verification = roots.runtime_compatibility.verification();
    let compatibility = &verification.verification;
    let observation = roots.runtime_compatibility.run_observation();
    let o = &observation.observation;
    let runtime_verifier = roots.runtime_compatibility.verifier_key();
    if roots.carrier.checked_at() != checked_at
        || roots.vulnerability.checked_at() != checked_at
        || roots.sandbox.checked_at() != checked_at
        || roots.runtime_compatibility.checked_at() != checked_at
        || release != roots.runtime_compatibility.release()
        || s.registry_release_id != release.registry_release_id
        || s.registry_release_digest != release.registry_release_digest
        || s.registry_release_material_digest != release.registry_release_material_digest
        || v.registry_release_id != release.registry_release_id
        || v.registry_release_digest != release.registry_release_digest
        || v.registry_release_material_digest != release.registry_release_material_digest
        || s.vulnerability_reattestation_receipt_id != vulnerability.reattestation_receipt_id
        || s.vulnerability_reattestation_receipt_digest
            != vulnerability.reattestation_receipt_digest
        || s.vulnerability_reattestation_material_digest
            != vulnerability.reattestation_material_digest
        || &compatibility.registry_release != release
        || &o.registry_release != release
        || compatibility.run_observation_id != observation.run_observation_id
        || compatibility.run_observation_digest != observation.run_observation_digest
        || compatibility.run_observation_material_digest
            != observation.run_observation_material_digest
        || compatibility.runner_execution_id != o.runner_execution_id
        || compatibility.public_fixture_delivery_root != o.public_fixture_delivery_root
        || s.sandbox_verifier_key_record_id != runtime_verifier.key_record_id()
        || s.sandbox_verifier_key_record_digest != runtime_verifier.key_record_digest()
        || s.sandbox_verifier_key_id != runtime_verifier.key_id()
        || s.sandbox_verifier_operator != runtime_verifier.verifier_operator()
        || s.sandbox_verifier_product != runtime_verifier.verifier_product()
    {
        bail!("task-protocol conformance V249/V250/V252/V268 roots are not exact")
    }
    Ok(())
}

pub(super) fn audit_domain_roots(
    roots: &CurrentTaskProtocolConformanceRoots<'_, '_>,
    expected: &ExternalPoolAdapterTaskProtocolConformanceRunMaterial,
    checked_at: &str,
) -> Result<()> {
    audit_current_roots(roots, checked_at)?;
    let projected = domain_roots(roots)?;
    if projected.registry_release != expected.registry_release
        || projected.vulnerability_reattestation != expected.vulnerability_reattestation
        || projected.sandbox_reattestation != expected.sandbox_reattestation
        || projected.sandbox_verifier_key != expected.sandbox_verifier_key
        || projected.runtime_compatibility != expected.runtime_compatibility
    {
        bail!("task-protocol conformance canonical roots are no longer current and exact")
    }
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    if expected.task_protocol_profile_id != profile.profile.profile_id
        || expected.task_protocol_profile_revision != profile.profile.profile_revision
        || expected.task_protocol_profile_digest != profile.profile_digest
        || expected.fixture_catalog_id != fixture.catalog.catalog_id
        || expected.fixture_catalog_revision != fixture.catalog.catalog_revision
        || expected.fixture_catalog_digest != fixture.catalog_digest
    {
        bail!("task-protocol conformance canonical catalog roots are stale")
    }
    Ok(())
}
