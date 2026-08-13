use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, TransactionBehavior};

use super::{
    current::current_external_pool_adapter_runtime_bundle_authority_on,
    entrypoint_capsule::{
        external_pool_adapter_entrypoint_capsule_policy_root,
        with_external_pool_adapter_entrypoint_capsule, ExternalPoolAdapterEntrypointSource,
        PreparedExternalPoolAdapterEntrypointCapsule,
    },
    types::{
        CurrentExternalPoolAdapterProbePreparationAuthority,
        CurrentExternalPoolAdapterRuntimeBundleAuthority, ExternalPoolAdapterRuntimeBundleRoot,
    },
};
use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_runtime_launch_profile::{
            runtime_launch_entrypoint_path_digest, ExternalPoolAdapterRuntimeLaunchProfileMaterial,
        },
        external_pool_provider_activation_candidate::ExternalPoolProviderActivationCandidateMaterial,
    },
    store::{
        compute_external_pool_adapter_sandbox_reattestation::{
            current_external_pool_adapter_sandbox_reattestation_head_authority_on,
            CurrentExternalPoolAdapterSandboxReattestationAuthority,
        },
        compute_external_pool_adapter_vulnerability_reattestation::{
            current_external_pool_adapter_vulnerability_reattestation_head_authority_on,
            CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
        },
        Store,
    },
};

impl Store {
    pub(in crate::store) fn with_current_external_pool_adapter_probe_preparation_authority(
        &self,
        profile_id: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
        bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
        consume: impl FnOnce(
            &CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>,
        ) -> Result<()>,
    ) -> Result<bool> {
        let mut conn = self.conn()?;
        let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let Some(bundle) = current_external_pool_adapter_runtime_bundle_authority_on(
            &transaction,
            profile_id,
            prepared,
            bundle_root,
            &checked_at,
        )?
        else {
            drop(transaction);
            return Ok(false);
        };
        let selected =
            select_current_probe_preparation_roots_on(&transaction, &bundle, &checked_at)?;
        materialize_probe_preparation(&bundle, &selected, consume)?;
        drop(selected);
        drop(bundle);
        transaction.commit()?;
        Ok(true)
    }
}

struct RetainedEntrypointSource<'a> {
    prepared: &'a PreparedExternalPoolAdapterInstallation,
}

impl ExternalPoolAdapterEntrypointSource for RetainedEntrypointSource<'_> {
    fn retained_entrypoint(&self) -> Result<(&std::fs::File, &str, u64)> {
        self.prepared.retained_entrypoint()
    }
}

fn materialize_probe_preparation(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    selected: &CurrentExternalPoolAdapterProbePreparationRoots,
    consume: impl FnOnce(&CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>) -> Result<()>,
) -> Result<()> {
    let source = RetainedEntrypointSource {
        prepared: prepared_entrypoint(bundle),
    };
    bundle.revalidate()?;
    with_external_pool_adapter_entrypoint_capsule(&source, |capsule| {
        let policy = external_pool_adapter_entrypoint_capsule_policy_root()?;
        audit_capsule(bundle, selected, capsule, &policy)?;
        bundle.revalidate()?;
        let authority = CurrentExternalPoolAdapterProbePreparationAuthority::new(
            capsule,
            &selected.vulnerability,
            &selected.sandbox,
            bundle,
            policy.policy_id,
            policy.policy_revision,
            &policy.policy_digest,
        );
        recheck_callback_freshness(bundle, selected)?;
        consume(&authority)?;
        bundle.revalidate()?;
        Ok(())
    })
}

fn recheck_callback_freshness(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    selected: &CurrentExternalPoolAdapterProbePreparationRoots,
) -> Result<()> {
    let now = Utc::now();
    let checked = DateTime::parse_from_rfc3339(bundle.checked_at())?.with_timezone(&Utc);
    if checked > now + Duration::minutes(5)
        || now.signed_duration_since(checked) > Duration::minutes(5)
    {
        bail!("probe preparation checked_at is no longer a near-now observation");
    }
    let vulnerability_expires = DateTime::parse_from_rfc3339(
        &selected
            .vulnerability
            .receipt()
            .reattestation
            .binding
            .intelligence
            .expires_at,
    )?;
    let sandbox_expires = DateTime::parse_from_rfc3339(
        &selected
            .sandbox
            .receipt()
            .reattestation
            .binding
            .report_expires_at,
    )?;
    let credential_expires = DateTime::parse_from_rfc3339(
        &bundle
            .credential()
            .receipt()
            .reattestation
            .binding
            .report_expires_at,
    )?;
    if now >= vulnerability_expires || now >= sandbox_expires || now >= credential_expires {
        bail!("probe preparation current evidence expired before callback");
    }
    Ok(())
}

fn audit_capsule(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    _selected: &CurrentExternalPoolAdapterProbePreparationRoots,
    capsule: &PreparedExternalPoolAdapterEntrypointCapsule,
    policy: &super::entrypoint_capsule::ExternalPoolAdapterEntrypointCapsulePolicyRoot,
) -> Result<()> {
    let profile = &bundle.launch_profile().profile().profile;
    let installed = prepared_entrypoint(bundle).binding();
    let retained = prepared_entrypoint(bundle).retained_entrypoint()?;
    if policy.policy_id != "external_pool_adapter_entrypoint_capsule_policy_v1"
        || policy.policy_revision != 1
        || policy.policy_digest != capsule.policy_digest()
        || capsule.entrypoint_sha256() != profile.entrypoint_sha256
        || capsule.entrypoint_sha256() != installed.entrypoint_sha256
        || capsule.entrypoint_sha256() != retained.1
        || capsule.entrypoint_size_bytes() != profile.entrypoint_size_bytes
        || capsule.entrypoint_size_bytes() != installed.entrypoint_size_bytes
        || capsule.entrypoint_size_bytes() != retained.2
    {
        bail!("probe preparation capsule roots drifted");
    }
    Ok(())
}

pub(super) struct CurrentExternalPoolAdapterProbePreparationRoots {
    pub(super) vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    pub(super) sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
}

pub(super) fn select_current_probe_preparation_roots_on(
    conn: &Connection,
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    checked_at: &str,
) -> Result<CurrentExternalPoolAdapterProbePreparationRoots> {
    if bundle.checked_at() != checked_at {
        bail!("probe preparation reused a different checked_at anchor");
    }
    let profile = &bundle.launch_profile().profile().profile;
    let vulnerability =
        current_external_pool_adapter_vulnerability_reattestation_head_authority_on(
            conn,
            &profile.registry_release_id,
            checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("probe preparation lacks a current V250 head"))?;
    let sandbox = current_external_pool_adapter_sandbox_reattestation_head_authority_on(
        conn,
        &profile.registry_release_id,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("probe preparation lacks a current V252 head"))?;
    audit_probe_preparation_roots(bundle, &vulnerability, &sandbox, checked_at)?;
    Ok(CurrentExternalPoolAdapterProbePreparationRoots {
        vulnerability,
        sandbox,
    })
}

pub(super) fn prepared_entrypoint<'a>(
    bundle: &'a CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
) -> &'a PreparedExternalPoolAdapterInstallation {
    bundle.launch_profile().candidate().registry().prepared()
}

fn audit_probe_preparation_roots(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    vulnerability: &CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    sandbox: &CurrentExternalPoolAdapterSandboxReattestationAuthority,
    checked_at: &str,
) -> Result<()> {
    let profile_authority = bundle.launch_profile();
    let profile = &profile_authority.profile().profile;
    let candidate = &profile_authority.candidate().candidate().candidate;
    let registry = profile_authority.candidate().registry();
    let release_receipt = registry.release();
    let release = &release_receipt.release;
    let binding_receipt = registry.binding();
    let prepared = registry.prepared();
    let installed = prepared.binding();
    let credential = bundle.credential();
    let credential_binding = &credential.receipt().reattestation.binding;
    let vulnerability_receipt = vulnerability.receipt();
    let vulnerability_binding = &vulnerability_receipt.reattestation.binding;
    let sandbox_receipt = sandbox.receipt();
    let sandbox_binding = &sandbox_receipt.reattestation.binding;

    audit_same_checked_at(
        bundle,
        [
            profile_authority.checked_at(),
            profile_authority.candidate().checked_at(),
            registry.checked_at(),
            vulnerability.checked_at(),
            sandbox.checked_at(),
            credential.checked_at(),
        ],
        checked_at,
    )?;
    audit_launch_policy(profile, installed)?;
    audit_static_roots(
        profile,
        candidate,
        release_receipt,
        binding_receipt,
        installed,
    )?;
    audit_vulnerability_roots(profile, release, vulnerability_binding)?;
    audit_sandbox_roots(profile, release, vulnerability_receipt, sandbox_binding)?;
    audit_credential_roots(profile, credential_binding)?;
    Ok(())
}

fn audit_launch_policy(
    profile: &ExternalPoolAdapterRuntimeLaunchProfileMaterial,
    installed: &crate::compute_federation::external_pool_adapter_installation::ExternalPoolAdapterInstallationBinding,
) -> Result<()> {
    let policy = &profile.launch_policy;
    if policy.runtime_kind != "server_sidecar_v1"
        || policy.host_os != "linux"
        || policy.host_arch != "x86_64"
        || policy.host_environment != "linux_native_process_v1"
        || policy.executable_kind != "native_process_image_v1"
        || policy.binary_format != "elf_native_v1"
        || policy.executable_verification_status != "deferred_to_runtime_supervisor"
        || policy.materialization_kind
            != "copy_from_retained_handle_create_new_private_executable_v1"
        || policy.shell_allowed
        || policy.argv_policy != "empty_no_shell_v1"
        || policy.environment_policy != "empty_allowlisted_runtime_v1"
        || policy.working_directory_policy != "isolated_private_runtime_directory_v1"
        || profile.entrypoint_relative_path != installed.entrypoint_path
        || profile.entrypoint_path_digest
            != runtime_launch_entrypoint_path_digest(&installed.entrypoint_path)
        || profile.entrypoint_sha256 != installed.entrypoint_sha256
        || profile.entrypoint_size_bytes != installed.entrypoint_size_bytes
    {
        bail!("probe preparation launch and entrypoint policy is not exact");
    }
    Ok(())
}

fn audit_static_roots(
    profile: &ExternalPoolAdapterRuntimeLaunchProfileMaterial,
    candidate: &ExternalPoolProviderActivationCandidateMaterial,
    release_receipt: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
    binding_receipt: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryProviderBindingReceipt,
    installed: &crate::compute_federation::external_pool_adapter_installation::ExternalPoolAdapterInstallationBinding,
) -> Result<()> {
    let release = &release_receipt.release;
    let binding = &binding_receipt.binding;
    if profile.registry_release_id != release_receipt.registry_release_id
        || profile.registry_release_digest != release_receipt.registry_release_digest
        || profile.provider_binding_id != binding_receipt.provider_binding_id
        || profile.provider_binding_digest != binding_receipt.provider_binding_digest
        || profile.installation_receipt_id != binding.installation_receipt_id
        || profile.installation_receipt_digest != binding.installation_receipt_digest
        || profile.installation_content_digest != installed.installation_content_digest
        || profile.implementation_digest != release.implementation_digest
        || profile.implementation_digest != candidate.implementation_digest
        || profile.capability_set_digest != release.capability_set_digest
        || profile.capability_set_digest != candidate.capability_set_digest
        || profile.credential_verifier_digest != release.credential_verifier_digest
        || profile.credential_verifier_digest != candidate.credential_verifier_digest
        || profile.adapter_config_digest != binding.adapter_config_digest
        || profile.adapter_config_revision != binding.adapter_config_revision
        || profile.provider_id != binding.provider_id
        || profile.provider_owner_account_id != binding.provider_owner_account_id
    {
        bail!("probe preparation static V249/V254/V255 roots drifted");
    }
    Ok(())
}

fn audit_vulnerability_roots(
    profile: &ExternalPoolAdapterRuntimeLaunchProfileMaterial,
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseMaterial,
    vulnerability: &crate::compute_federation::external_pool_adapter_vulnerability_reattestation::ExternalPoolAdapterVulnerabilityReattestationBinding,
) -> Result<()> {
    if vulnerability.registry_release_id != profile.registry_release_id
        || vulnerability.registry_release_digest != profile.registry_release_digest
        || vulnerability.implementation_digest != profile.implementation_digest
        || vulnerability.installation_content_digest != profile.installation_content_digest
        || vulnerability.admission_id != release.admission_id
        || vulnerability.admission_digest != release.admission_digest
        || vulnerability.package_receipt_id != release.package_receipt_id
        || vulnerability.package_receipt_digest != release.package_receipt_digest
    {
        bail!("probe preparation V250 roots drifted");
    }
    Ok(())
}

fn audit_sandbox_roots(
    profile: &ExternalPoolAdapterRuntimeLaunchProfileMaterial,
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseMaterial,
    vulnerability: &crate::compute_federation::external_pool_adapter_vulnerability_reattestation::ExternalPoolAdapterVulnerabilityReattestationReceipt,
    sandbox: &crate::compute_federation::external_pool_adapter_sandbox_reattestation::ExternalPoolAdapterSandboxReattestationBinding,
) -> Result<()> {
    if sandbox.registry_release_id != profile.registry_release_id
        || sandbox.registry_release_digest != profile.registry_release_digest
        || sandbox.implementation_digest != profile.implementation_digest
        || sandbox.installation_content_digest != profile.installation_content_digest
        || sandbox.capability_set_digest != profile.capability_set_digest
        || sandbox.credential_verifier_digest != profile.credential_verifier_digest
        || sandbox.admission_id != release.admission_id
        || sandbox.admission_digest != release.admission_digest
        || sandbox.package_receipt_id != release.package_receipt_id
        || sandbox.package_receipt_digest != release.package_receipt_digest
        || sandbox.vulnerability_reattestation_receipt_id != vulnerability.reattestation_receipt_id
        || sandbox.vulnerability_reattestation_receipt_digest
            != vulnerability.reattestation_receipt_digest
        || sandbox.vulnerability_reattestation_material_digest
            != vulnerability.reattestation_material_digest
        || sandbox.vulnerability_reattestation_sequence
            != vulnerability.reattestation.binding.sequence
        || sandbox.vulnerability_intelligence_snapshot_digest
            != vulnerability
                .reattestation
                .binding
                .intelligence
                .snapshot_digest
        || sandbox.vulnerability_intelligence_expires_at
            != vulnerability.reattestation.binding.intelligence.expires_at
    {
        bail!("probe preparation V252/V250 exact roots drifted");
    }
    Ok(())
}

fn audit_credential_roots(
    profile: &ExternalPoolAdapterRuntimeLaunchProfileMaterial,
    credential: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationBinding,
) -> Result<()> {
    if credential.provider_binding_id != profile.provider_binding_id
        || credential.provider_binding_digest != profile.provider_binding_digest
        || credential.registry_release_id != profile.registry_release_id
        || credential.registry_release_digest != profile.registry_release_digest
        || credential.installation_receipt_id != profile.installation_receipt_id
        || credential.installation_receipt_digest != profile.installation_receipt_digest
        || credential.installation_content_digest != profile.installation_content_digest
        || credential.provider_id != profile.provider_id
        || credential.provider_owner_account_id != profile.provider_owner_account_id
        || credential.observed_provider_policy_revision != profile.provider_policy_revision
        || credential.observed_provider_digest != profile.provider_digest
        || credential.observed_provider_status != profile.provider_status
        || credential.adapter_id != profile.logical_adapter_id
        || credential.release_version != profile.release_version
        || credential.adapter_config_revision != profile.adapter_config_revision
        || credential.adapter_config_digest != profile.adapter_config_digest
        || credential.credential_verifier_digest != profile.credential_verifier_digest
    {
        bail!("probe preparation V253 roots drifted");
    }
    Ok(())
}

fn audit_same_checked_at(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    observed: [&str; 6],
    checked_at: &str,
) -> Result<()> {
    if bundle.checked_at() != checked_at || observed.into_iter().any(|value| value != checked_at) {
        bail!("probe preparation authorities were not checked at one instant");
    }
    Ok(())
}
