use anyhow::{bail, ensure, Result};
use rusqlite::{params, types::Type, OptionalExtension, Transaction};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_supervisor_session_policy_companion::{
            canonical_supervisor_session_companion_json_and_digest,
            validate_supervisor_session_companion_receipt,
            ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        },
    },
    store::{
        compute_external_pool_adapter_artifact_package::current_artifact_package_authority_on,
        compute_external_pool_adapter_artifact_source::external_pool_adapter_artifact_source_authority_on,
        compute_external_pool_adapter_credential_reattestation::current_external_pool_adapter_projected_active_credential_reattestation_authority_on,
        compute_external_pool_adapter_installation::{
            external_pool_adapter_installation_is_revoked_on,
            external_pool_adapter_installation_receipt_authority_on,
        },
        compute_external_pool_adapter_registry::{
            current_external_pool_adapter_registry_release_authority_on,
            historical_external_pool_adapter_registry_provider_binding_authority_on,
            historical_external_pool_adapter_registry_release_authority_on,
        },
        compute_external_pool_adapter_release_lifecycle::current_external_pool_adapter_release_admission_authority_on,
        compute_external_pool_adapter_runtime_compatibility_verification::current_external_pool_adapter_runtime_compatibility_verification_authority_on,
        compute_external_pool_adapter_runtime_launch_profile::historical_external_pool_adapter_runtime_launch_profile_authority_on,
        compute_external_pool_adapter_upstream_transport_target::historical_external_pool_adapter_upstream_transport_target_authority_on,
        compute_external_pool_provider_activation_candidate::historical_external_pool_provider_activation_candidate_authority_on,
    },
};

use super::types::{
    CurrentExternalPoolAdapterProjectedActiveHistoricalCarrierAuthority,
    HistoricalExternalPoolAdapterAtomicActivationAuthority,
};

pub(in crate::store) fn current_external_pool_adapter_projected_active_historical_carrier_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    historical_activation: HistoricalExternalPoolAdapterAtomicActivationAuthority,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterProjectedActiveHistoricalCarrierAuthority<'tx, 'conn>>>
{
    let root = &historical_activation.activation_root().activation_root;
    let receipt = historical_activation.receipt();
    ensure!(
        checked_at >= receipt.activation.evidence_checked_at.as_str(),
        "active historical carrier predates its V277 evidence"
    );

    let registry_binding = historical_external_pool_adapter_registry_provider_binding_authority_on(
        transaction,
        &root.provider_binding_id,
        &root.provider_binding_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier lost V249 binding"))?;
    let registry_release = historical_external_pool_adapter_registry_release_authority_on(
        transaction,
        &root.registry_release_id,
        &root.registry_release_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier lost V249 release"))?;
    let candidate = historical_external_pool_provider_activation_candidate_authority_on(
        transaction,
        &root.candidate_id,
        &root.candidate_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier lost V254 candidate"))?;
    let profile = historical_external_pool_adapter_runtime_launch_profile_authority_on(
        transaction,
        &root.profile_id,
        &root.profile_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier lost V255 profile"))?;
    let target = historical_external_pool_adapter_upstream_transport_target_authority_on(
        transaction,
        &root.target_id,
        &root.target_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier lost V258 target"))?;
    let companion =
        historical_companion_on(transaction, &root.companion_id, &root.companion_digest)?
            .ok_or_else(|| anyhow::anyhow!("active historical carrier lost V259 companion"))?;

    audit_historical_roots(
        root,
        registry_binding.binding(),
        registry_release.release(),
        candidate.candidate(),
        &profile,
        &target,
        &companion,
    )?;

    let (credential_receipt_id, credential_receipt_digest) =
        credential_reattestation_head_on(transaction, &root.provider_binding_id)?.ok_or_else(
            || anyhow::anyhow!("active historical carrier lacks a projected-active V253 head"),
        )?;
    let credential =
        current_external_pool_adapter_projected_active_credential_reattestation_authority_on(
            transaction,
            &root.provider_binding_id,
            &credential_receipt_id,
            &credential_receipt_digest,
            checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("active historical carrier lacks projected-active V253"))?;
    let observed = &credential.receipt().reattestation.binding;
    let active = historical_activation.active_provider();
    if credential.checked_at() != checked_at
        || observed.provider_binding_id != root.provider_binding_id
        || observed.provider_binding_digest != root.provider_binding_digest
        || observed.provider_id != active.provider_id
        || observed.observed_provider_policy_revision != active.policy_revision
        || observed.observed_provider_status != active.status
    {
        bail!("active historical carrier V253 roots are not exact");
    }

    let (verification_id, verification_digest) = transaction
        .query_row(
            "SELECT verification_receipt_id,verification_receipt_digest
               FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts
              WHERE registry_release_id=?1 ORDER BY sequence DESC LIMIT 1",
            params![root.registry_release_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("active historical carrier lacks a V268 head"))?;
    let runtime_compatibility =
        current_external_pool_adapter_runtime_compatibility_verification_authority_on(
            transaction,
            &verification_id,
            &verification_digest,
            checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("active historical carrier lacks current V268"))?;
    if runtime_compatibility.checked_at() != checked_at
        || runtime_compatibility.release().registry_release_id != root.registry_release_id
        || runtime_compatibility.release().registry_release_digest != root.registry_release_digest
    {
        bail!("active historical carrier V268 roots are not exact");
    }

    let release = &registry_release.release().release;
    let current_release = current_external_pool_adapter_registry_release_authority_on(
        transaction,
        &root.registry_release_id,
        &root.registry_release_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier release is no longer current"))?;
    let release_admission = current_external_pool_adapter_release_admission_authority_on(
        transaction,
        &release.admission_id,
        &release.admission_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier admission is no longer current"))?;
    let package_authority = current_artifact_package_authority_on(
        transaction,
        &release.admission_id,
        &release.package_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier package is no longer current"))?;
    let source = external_pool_adapter_artifact_source_authority_on(
        transaction,
        &release.admission_id,
        &release.admission_digest,
        &release.source_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier source is no longer exact"))?;
    let installation = external_pool_adapter_installation_receipt_authority_on(
        transaction,
        &root.installation_receipt_id,
        &root.installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active historical carrier lost installation history"))?;
    if current_release.checked_at() != checked_at
        || current_release.release() != registry_release.release()
        || release_admission.admission_id() != release.admission_id
        || release_admission.admission_digest() != release.admission_digest
        || package_authority.receipt().package_receipt_digest != release.package_receipt_digest
        || source.source_receipt_digest() != release.source_receipt_digest
        || prepared.binding() != &installation.receipt().installation.binding
        || prepared.installation_content_digest() != root.installation_content_digest
        || external_pool_adapter_installation_is_revoked_on(
            transaction,
            &root.installation_receipt_id,
        )?
    {
        bail!("active historical carrier release/content roots are not current and exact");
    }
    let package = package_authority.receipt().clone();

    Ok(Some(
        CurrentExternalPoolAdapterProjectedActiveHistoricalCarrierAuthority::new(
            transaction,
            historical_activation,
            registry_binding,
            registry_release,
            current_release,
            candidate,
            profile,
            target,
            companion,
            credential,
            runtime_compatibility,
            release_admission,
            package,
            source,
            prepared,
            checked_at.to_owned(),
        ),
    ))
}

fn credential_reattestation_head_on(
    transaction: &Transaction<'_>,
    provider_binding_id: &str,
) -> Result<Option<(String, String)>> {
    let (count, receipt_id, receipt_digest): (i64, Option<String>, Option<String>) = transaction
        .query_row(
            "SELECT COUNT(*),MIN(candidate.reattestation_receipt_id),
                    MIN(candidate.reattestation_receipt_digest)
               FROM compute_external_pool_adapter_credential_reattestation_receipts candidate
              WHERE candidate.provider_binding_id=?1 AND NOT EXISTS(
                    SELECT 1
                      FROM compute_external_pool_adapter_credential_reattestation_receipts successor
                     WHERE successor.predecessor_receipt_id=candidate.reattestation_receipt_id)",
            params![provider_binding_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    ensure!(
        count <= 1,
        "active historical carrier found multiple V253 heads"
    );
    Ok(receipt_id.zip(receipt_digest))
}

fn historical_companion_on(
    transaction: &Transaction<'_>,
    companion_id: &str,
    expected_digest: &str,
) -> Result<Option<ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt>> {
    let stored = transaction
        .query_row(
            "SELECT companion_json
               FROM compute_external_pool_adapter_supervisor_session_policy_companions
              WHERE companion_id=?1",
            params![companion_id],
            |row| {
                let json: String = row.get(0)?;
                let receipt = serde_json::from_str(&json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                })?;
                Ok((receipt, json))
            },
        )
        .optional()?;
    let Some((receipt, json)) = stored else {
        return Ok(None);
    };
    validate_supervisor_session_companion_receipt(&receipt)?;
    let (canonical, digest) = canonical_supervisor_session_companion_json_and_digest(&receipt)?;
    if canonical != json || digest != expected_digest || receipt.companion_digest != expected_digest
    {
        bail!("historical V259 companion failed canonical audit");
    }
    Ok(Some(receipt))
}

#[allow(clippy::too_many_arguments)]
fn audit_historical_roots(
    root: &crate::compute_federation::external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope,
    binding: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryProviderBindingReceipt,
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
    candidate: &crate::compute_federation::external_pool_provider_activation_candidate::ExternalPoolProviderActivationCandidateReceipt,
    profile: &crate::compute_federation::external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    target: &crate::compute_federation::external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
    companion: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
) -> Result<()> {
    let b = &binding.binding;
    let c = &candidate.candidate;
    let p = &profile.profile;
    let t = &target.target;
    let s = &companion.companion;
    ensure!(
        binding.provider_binding_id == root.provider_binding_id
            && binding.provider_binding_digest == root.provider_binding_digest
            && b.registry_release_id == root.registry_release_id
            && b.registry_release_digest == root.registry_release_digest
            && release.registry_release_id == root.registry_release_id
            && release.registry_release_digest == root.registry_release_digest
            && release.registry_release_material_digest == root.registry_release_material_digest
            && c.provider_binding_id == root.provider_binding_id
            && candidate.candidate_id == root.candidate_id
            && candidate.candidate_digest == root.candidate_digest
            && c.delegation_id == root.delegation_id
            && c.delegation_digest == root.delegation_digest
            && profile.profile_id == root.profile_id
            && profile.profile_digest == root.profile_digest
            && p.provider_binding_id == root.provider_binding_id
            && target.target_id == root.target_id
            && target.target_digest == root.target_digest
            && t.profile_id == root.profile_id
            && companion.companion_id == root.companion_id
            && companion.companion_digest == root.companion_digest
            && s.target_id == root.target_id
            && s.target_digest == root.target_digest,
        "active historical carrier immutable roots diverged"
    );
    Ok(())
}
