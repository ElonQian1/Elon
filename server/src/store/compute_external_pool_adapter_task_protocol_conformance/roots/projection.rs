use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};

use crate::compute_federation::{
    external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
    external_pool_adapter_runtime_compatibility_verification::{
        server_runtime_compatibility_v2_profile_catalog,
        ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
    },
    external_pool_adapter_sandbox_reattestation::ExternalPoolAdapterSandboxReattestationReceipt,
    external_pool_adapter_task_protocol_conformance::*,
    external_pool_adapter_vulnerability_reattestation::ExternalPoolAdapterVulnerabilityReattestationReceipt,
};
use crate::store::compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::super::run::{
    TaskProtocolConformanceExecutionInput, TaskProtocolConformanceFixtureResourceIdentity,
};
use super::CurrentTaskProtocolConformanceRoots;

pub(in super::super) fn domain_roots(
    roots: &CurrentTaskProtocolConformanceRoots<'_, '_>,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceRunRoots> {
    domain_roots_from_parts(
        roots.carrier.release(),
        roots.vulnerability.receipt(),
        roots.sandbox.receipt(),
        &roots.runtime_compatibility,
    )
}

pub(in super::super) fn domain_roots_from_parts(
    release_receipt: &ExternalPoolAdapterRegistryReleaseReceipt,
    vulnerability_receipt: &ExternalPoolAdapterVulnerabilityReattestationReceipt,
    sandbox_receipt: &ExternalPoolAdapterSandboxReattestationReceipt,
    runtime_compatibility: &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<
        '_,
        '_,
    >,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceRunRoots> {
    let release = &release_receipt.release;
    let entrypoint = release
        .manifest
        .files
        .iter()
        .find(|file| file.path == release.manifest.runtime.entrypoint)
        .ok_or_else(|| anyhow::anyhow!("task-protocol conformance V249 entrypoint disappeared"))?;
    let vulnerability = &vulnerability_receipt.reattestation.binding;
    let sandbox = &sandbox_receipt.reattestation.binding;
    let verification_receipt = runtime_compatibility.verification();
    let verification = &verification_receipt.verification;
    let observation_receipt = runtime_compatibility.run_observation();
    let observation = &observation_receipt.observation;
    let runtime_profile = server_runtime_compatibility_v2_profile_catalog()?;
    Ok(ExternalPoolAdapterTaskProtocolConformanceRunRoots {
        registry_release: ExternalPoolAdapterTaskProtocolConformanceRegistryReleaseRoots {
            registry_release_id: release_receipt.registry_release_id.clone(),
            registry_release_digest: release_receipt.registry_release_digest.clone(),
            registry_release_material_digest: release_receipt
                .registry_release_material_digest
                .clone(),
            admission_id: release.admission_id.clone(),
            admission_digest: release.admission_digest.clone(),
            package_receipt_id: release.package_receipt_id.clone(),
            package_receipt_digest: release.package_receipt_digest.clone(),
            package_material_digest: release.package_material_digest.clone(),
            source_receipt_id: release.source_receipt_id.clone(),
            source_receipt_digest: release.source_receipt_digest.clone(),
            adapter_id: release.adapter_id.clone(),
            release_version: release.release_version.clone(),
            route_kind: release.route_kind.clone(),
            implementation_digest: release.implementation_digest.clone(),
            declared_implementation_sha256: release.declared_implementation_sha256.clone(),
            entrypoint_path: entrypoint.path.clone(),
            entrypoint_sha256: entrypoint.sha256.clone(),
            entrypoint_size_bytes: entrypoint.size_bytes,
            installation_content_digest: release.installation_content_digest.clone(),
            supported_capabilities: release.supported_capabilities.clone(),
            capability_set_digest: release.capability_set_digest.clone(),
        },
        vulnerability_reattestation: ExternalPoolAdapterTaskProtocolConformanceVulnerabilityRoots {
            reattestation_receipt_id: vulnerability_receipt.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: vulnerability_receipt
                .reattestation_receipt_digest
                .clone(),
            reattestation_material_digest: vulnerability_receipt
                .reattestation_material_digest
                .clone(),
            intelligence_snapshot_digest: vulnerability.intelligence.snapshot_digest.clone(),
            intelligence_expires_at: vulnerability.intelligence.expires_at.clone(),
            blocking_finding_count: vulnerability.blocking_finding_count,
        },
        sandbox_reattestation: ExternalPoolAdapterTaskProtocolConformanceSandboxRoots {
            reattestation_receipt_id: sandbox_receipt.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: sandbox_receipt.reattestation_receipt_digest.clone(),
            reattestation_material_digest: sandbox_receipt.reattestation_material_digest.clone(),
            sandbox_policy_id: sandbox.sandbox_policy_id.clone(),
            test_plan_digest: sandbox.test_plan_digest.clone(),
            observation_inventory_digest: sandbox.observation_inventory_digest.clone(),
            report_expires_at: sandbox.report_expires_at.clone(),
            passed_capability_count: sandbox.passed_capability_count,
            policy_violation_count: sandbox.policy_violation_count,
        },
        sandbox_verifier_key: ExternalPoolAdapterTaskProtocolConformanceSandboxVerifierKeyRoots {
            key_record_id: sandbox.sandbox_verifier_key_record_id.clone(),
            key_record_digest: sandbox.sandbox_verifier_key_record_digest.clone(),
            key_id: sandbox.sandbox_verifier_key_id.clone(),
            verifier_operator: sandbox.sandbox_verifier_operator.clone(),
            verifier_product: sandbox.sandbox_verifier_product.clone(),
        },
        runtime_compatibility:
            ExternalPoolAdapterTaskProtocolConformanceRuntimeCompatibilityRoots {
                verification_receipt_id: verification_receipt.verification_receipt_id.clone(),
                verification_receipt_digest: verification_receipt
                    .verification_receipt_digest
                    .clone(),
                verification_material_digest: verification_receipt
                    .verification_material_digest
                    .clone(),
                run_observation_id: observation_receipt.run_observation_id.clone(),
                run_observation_digest: observation_receipt.run_observation_digest.clone(),
                run_observation_material_digest: observation_receipt
                    .run_observation_material_digest
                    .clone(),
                runner_execution_id: verification.runner_execution_id.clone(),
                profile_id: verification.profile_id.clone(),
                profile_revision: verification.profile_revision,
                profile_digest: verification.profile_digest.clone(),
                runner_policy_digest: verification.runner_policy_digest.clone(),
                fixture_catalog_digest: verification.fixture_catalog_digest.clone(),
                supervisor_session_policy_digest: runtime_profile
                    .profile
                    .supervisor_session_policy
                    .policy_digest,
                source_capsule_sha256: observation.source_capsule_sha256.clone(),
                source_capsule_size_bytes: observation.source_capsule_size_bytes,
                launch_image_sha256: observation.launch_image_sha256.clone(),
                launch_image_size_bytes: observation.launch_image_size_bytes,
                public_fixture_delivery_root: verification.public_fixture_delivery_root.clone(),
                expires_at: verification.expires_at.clone(),
            },
    })
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in super::super) fn into_execution_input(
    roots: CurrentTaskProtocolConformanceRoots<'_, '_>,
) -> Result<TaskProtocolConformanceExecutionInput> {
    let projected = domain_roots(&roots)?;
    into_execution_input_from_parts(
        projected,
        &roots.runtime_compatibility,
        roots.carrier.into_prepared(),
    )
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in super::super) fn into_execution_input_from_parts(
    projected: ExternalPoolAdapterTaskProtocolConformanceRunRoots,
    runtime_compatibility: &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<
        '_,
        '_,
    >,
    prepared_installation: PreparedExternalPoolAdapterInstallation,
) -> Result<TaskProtocolConformanceExecutionInput> {
    into_execution_input_from_observation(
        projected,
        runtime_compatibility.run_observation(),
        prepared_installation,
    )
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in super::super) fn into_execution_input_from_observation(
    projected: ExternalPoolAdapterTaskProtocolConformanceRunRoots,
    compatibility: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
    prepared_installation: PreparedExternalPoolAdapterInstallation,
) -> Result<TaskProtocolConformanceExecutionInput> {
    let fixture_resources = compatibility
        .observation
        .fixture_resources
        .iter()
        .map(|resource| TaskProtocolConformanceFixtureResourceIdentity {
            purpose: resource.purpose.clone(),
            path: resource.path.clone(),
            sha256: resource.sha256.clone(),
            size_bytes: resource.size_bytes,
        })
        .collect();
    let runtime = &projected.runtime_compatibility;
    Ok(TaskProtocolConformanceExecutionInput {
        registry_release: projected.registry_release,
        supervisor_session_policy_digest: runtime.supervisor_session_policy_digest.clone(),
        sandbox_reattestation_receipt_digest: projected
            .sandbox_reattestation
            .reattestation_receipt_digest,
        runtime_compatibility_verification_receipt_digest: runtime
            .verification_receipt_digest
            .clone(),
        source_capsule_sha256: runtime.source_capsule_sha256.clone(),
        source_capsule_size_bytes: runtime.source_capsule_size_bytes,
        launch_image_sha256: runtime.launch_image_sha256.clone(),
        launch_image_size_bytes: runtime.launch_image_size_bytes,
        fixture_resources,
        prepared_installation,
    })
}

pub(in super::super) fn task_protocol_conformance_expires_at(
    checked_at: &str,
    roots: &ExternalPoolAdapterTaskProtocolConformanceRunRoots,
) -> Result<String> {
    let checked = canonical_time(checked_at)?;
    let local_expiry = checked
        .checked_add_signed(Duration::seconds(TASK_PROTOCOL_CONFORMANCE_EXPIRY_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("task-protocol conformance local expiry overflow"))?;
    let expires = [
        local_expiry,
        canonical_time(&roots.vulnerability_reattestation.intelligence_expires_at)?,
        canonical_time(&roots.sandbox_reattestation.report_expires_at)?,
        canonical_time(&roots.runtime_compatibility.expires_at)?,
    ]
    .into_iter()
    .min()
    .ok_or_else(|| anyhow::anyhow!("task-protocol conformance expiry set is empty"))?;
    Ok(expires.to_rfc3339_opts(SecondsFormat::Nanos, true))
}

pub(in super::super) fn canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("task-protocol conformance timestamp is not canonical UTC nanoseconds")
    }
    Ok(parsed.with_timezone(&Utc))
}
