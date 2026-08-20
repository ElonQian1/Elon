use anyhow::{bail, Result};

use crate::{
    compute_federation::{
        external_pool_adapter_provider_active_successor::{
            canonical_external_pool_adapter_provider_active_successor_receipt_json_and_digest,
            canonical_external_pool_adapter_provider_active_successor_revocation_json_and_digest,
            provider_active_successor_private_integrity_digest,
            validate_external_pool_adapter_provider_active_successor_receipt,
            validate_external_pool_adapter_provider_active_successor_revocation,
            ExternalPoolAdapterProviderActiveSuccessorStructuralInput,
            PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
            PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_PROCESS_KIND,
        },
        external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        external_pool_adapter_supervisor_session_policy_companion::ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        external_pool_adapter_task_protocol_production::{
            derive_external_pool_adapter_task_production_lane_subject,
            validate_task_production_carrier_policy_digest,
            ExternalPoolAdapterTaskProductionLaneSubjectInput,
            TASK_PRODUCTION_CARRIER_POLICY_DIGEST,
        },
        external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
        external_pool_provider_activation_candidate::{
            external_pool_activation_service_actor_id, logical_projection_compatibility_digest,
            ExternalPoolProviderActivationCandidateReceipt,
            ExternalPoolProviderActivationDelegationReceipt,
        },
        provider::PROVIDER_STATUS_REGISTERING,
    },
    store::{
        compute_external_pool_adapter_registry::CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
        compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
    },
};

use super::types::{
    StoredExternalPoolAdapterProviderActiveSuccessor,
    StoredExternalPoolAdapterProviderActiveSuccessorRevocation,
};

pub(super) fn audit_receipt(
    conn: &rusqlite::Connection,
    stored: StoredExternalPoolAdapterProviderActiveSuccessor,
) -> Result<StoredExternalPoolAdapterProviderActiveSuccessor> {
    validate_external_pool_adapter_provider_active_successor_receipt(&stored.receipt)?;
    let (json, digest) =
        canonical_external_pool_adapter_provider_active_successor_receipt_json_and_digest(
            &stored.receipt,
        )?;
    let integrity = provider_active_successor_private_integrity_digest(
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
        &stored.receipt.receipt_digest,
        &stored.process_custody,
    )?;
    let successor = &stored.receipt.successor;
    let activation = &successor.activation.activation_root;
    let binding = crate::store::compute_external_pool_adapter_registry::historical_external_pool_adapter_registry_provider_binding_authority_on(
        conn,
        &activation.provider_binding_id,
        &activation.provider_binding_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("provider active-successor lost historical V249 binding"))?;
    if json != stored.receipt_json
        || digest != stored.receipt.receipt_digest
        || integrity != stored.receipt_integrity_digest
        || binding.binding().binding.provider_id != activation.provider_id
        || binding.binding().binding.route_adapter_projection_id
            != activation.route_adapter_projection_id
    {
        bail!("provider active-successor durable receipt failed exact audit");
    }
    Ok(stored)
}

pub(super) fn audit_revocation(
    conn: &rusqlite::Connection,
    stored: StoredExternalPoolAdapterProviderActiveSuccessorRevocation,
) -> Result<StoredExternalPoolAdapterProviderActiveSuccessorRevocation> {
    validate_external_pool_adapter_provider_active_successor_revocation(&stored.receipt)?;
    let (json, digest) =
        canonical_external_pool_adapter_provider_active_successor_revocation_json_and_digest(
            &stored.receipt,
        )?;
    let integrity = provider_active_successor_private_integrity_digest(
        PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_PROCESS_KIND,
        &stored.receipt.revocation_digest,
        &stored.process_custody,
    )?;
    let target = &stored.receipt.revocation;
    let target_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_adapter_provider_active_successor_receipts
          WHERE active_successor_receipt_id=?1 AND receipt_digest=?2
            AND provider_binding_id=?3 AND activation_root_digest=?4)",
        rusqlite::params![
            target.target_active_successor_receipt_id,
            target.target_active_successor_receipt_digest,
            target.provider_binding_id,
            target.activation_root_digest,
        ],
        |row| row.get(0),
    )?;
    if json != stored.revocation_json
        || digest != stored.receipt.revocation_digest
        || integrity != stored.receipt_integrity_digest
        || !target_exists
    {
        bail!("provider active-successor durable revocation failed exact audit");
    }
    Ok(stored)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn audited_structural_input(
    registry: &CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    delegation: &ExternalPoolProviderActivationDelegationReceipt,
    candidate: &ExternalPoolProviderActivationCandidateReceipt,
    profile: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    target: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
    companion: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
    compatibility: &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'_, '_>,
    task_protocol_profile_digest: &str,
    checked_at: &str,
) -> Result<ExternalPoolAdapterProviderActiveSuccessorStructuralInput> {
    validate_task_production_carrier_policy_digest(TASK_PRODUCTION_CARRIER_POLICY_DIGEST)?;
    let binding = registry.binding();
    let release = registry.release();
    let b = &binding.binding;
    let c = &candidate.candidate;
    let p = &profile.profile;
    let t = &target.target;
    let s = &companion.companion;
    let verification = &compatibility.verification().verification;
    let observation = &compatibility.run_observation().observation;
    if compatibility.checked_at() != checked_at
        || binding.provider_binding_id != c.provider_binding_id
        || binding.provider_binding_digest != c.provider_binding_digest
        || delegation.delegation_id != c.delegation_id
        || delegation.delegation_digest != c.delegation_digest
        || candidate.candidate_id != p.candidate_id
        || candidate.candidate_digest != p.candidate_digest
        || profile.profile_id != t.profile_id
        || profile.profile_digest != t.profile_digest
        || profile.profile_id != s.profile_id
        || profile.profile_digest != s.profile_digest
        || target.target_id != s.target_id
        || target.target_digest != s.target_digest
        || c.provider_status != PROVIDER_STATUS_REGISTERING
        || p.provider_status != PROVIDER_STATUS_REGISTERING
        || t.provider_status != PROVIDER_STATUS_REGISTERING
        || s.provider_status != PROVIDER_STATUS_REGISTERING
        || !common_binding_roots_are_exact(b, c, p, t, s)
        || !common_provider_origins_are_exact(b, c, p, t, s)
        || release.registry_release_id != b.registry_release_id
        || release.registry_release_digest != b.registry_release_digest
        || verification.registry_release.registry_release_id != b.registry_release_id
        || verification.registry_release.registry_release_digest != b.registry_release_digest
        || compatibility.release().registry_release_id != b.registry_release_id
        || compatibility.release().registry_release_digest != b.registry_release_digest
        || observation.launch_image_sha256.len() != 64
    {
        bail!("provider active-successor structural roots are not exact");
    }
    let expected_compatibility = logical_projection_compatibility_digest(
        &binding.provider_binding_id,
        &binding.provider_binding_digest,
        &b.registry_release_id,
        &b.registry_release_digest,
        &c.logical_adapter_binding_digest,
        &b.route_adapter_projection_id,
    )?;
    let expected_actor = external_pool_activation_service_actor_id(
        &b.provider_id,
        &binding.provider_binding_id,
        &binding.provider_binding_digest,
        &b.route_adapter_projection_id,
    )?;
    if expected_compatibility != c.logical_projection_compatibility_digest
        || expected_actor != c.service_actor_id
        || c.service_actor_id != p.service_actor_id
        || p.service_actor_id != t.service_actor_id
        || t.service_actor_id != s.service_actor_id
    {
        bail!("provider active-successor projection bridge is not exact");
    }
    let lane = derive_external_pool_adapter_task_production_lane_subject(
        ExternalPoolAdapterTaskProductionLaneSubjectInput {
            provider_id: b.provider_id.clone(),
            provider_owner_account_id: b.provider_owner_account_id.clone(),
            provider_binding_id: binding.provider_binding_id.clone(),
            provider_binding_digest: binding.provider_binding_digest.clone(),
            registry_release_id: b.registry_release_id.clone(),
            registry_release_digest: b.registry_release_digest.clone(),
            route_adapter_projection_id: b.route_adapter_projection_id.clone(),
            logical_adapter_binding_digest: c.logical_adapter_binding_digest.clone(),
            logical_projection_compatibility_digest: c
                .logical_projection_compatibility_digest
                .clone(),
        },
    )?;
    Ok(ExternalPoolAdapterProviderActiveSuccessorStructuralInput {
        provider_id: b.provider_id.clone(),
        provider_owner_account_id: b.provider_owner_account_id.clone(),
        provider_binding_id: binding.provider_binding_id.clone(),
        provider_binding_digest: binding.provider_binding_digest.clone(),
        registry_release_id: b.registry_release_id.clone(),
        registry_release_digest: b.registry_release_digest.clone(),
        registry_release_material_digest: release.registry_release_material_digest.clone(),
        installation_receipt_id: b.installation_receipt_id.clone(),
        installation_receipt_digest: b.installation_receipt_digest.clone(),
        installation_content_digest: b.installation_content_digest.clone(),
        candidate_id: candidate.candidate_id.clone(),
        candidate_digest: candidate.candidate_digest.clone(),
        delegation_id: delegation.delegation_id.clone(),
        delegation_digest: delegation.delegation_digest.clone(),
        service_actor_id: c.service_actor_id.clone(),
        logical_adapter_id: b.adapter_id.clone(),
        logical_adapter_binding_digest: c.logical_adapter_binding_digest.clone(),
        logical_projection_compatibility_digest: c.logical_projection_compatibility_digest.clone(),
        route_adapter_projection_id: b.route_adapter_projection_id.clone(),
        profile_id: profile.profile_id.clone(),
        profile_digest: profile.profile_digest.clone(),
        launch_policy_digest: p.launch_policy_digest.clone(),
        target_id: target.target_id.clone(),
        target_digest: target.target_digest.clone(),
        target_policy_digest: t.target_policy_digest.clone(),
        companion_id: companion.companion_id.clone(),
        companion_digest: companion.companion_digest.clone(),
        supervisor_session_policy_digest: s.supervisor_session_policy_digest.clone(),
        entrypoint_capsule_policy_digest: s.entrypoint_capsule_policy_digest.clone(),
        launch_image_sha256: observation.launch_image_sha256.clone(),
        task_protocol_profile_digest: task_protocol_profile_digest.into(),
        lane_subject_digest: lane.lane_subject_digest,
        task_production_carrier_policy_digest: TASK_PRODUCTION_CARRIER_POLICY_DIGEST.into(),
    })
}

fn common_binding_roots_are_exact(
    b: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryProviderBindingMaterial,
    c: &crate::compute_federation::external_pool_provider_activation_candidate::ExternalPoolProviderActivationCandidateMaterial,
    p: &crate::compute_federation::external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileMaterial,
    t: &crate::compute_federation::external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetMaterial,
    s: &crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::ExternalPoolAdapterSupervisorSessionPolicyCompanionMaterial,
) -> bool {
    let expected = (
        b.provider_id.as_str(),
        b.provider_owner_account_id.as_str(),
        b.registry_release_id.as_str(),
        b.registry_release_digest.as_str(),
        b.installation_receipt_id.as_str(),
        b.installation_receipt_digest.as_str(),
        b.installation_content_digest.as_str(),
        b.route_adapter_projection_id.as_str(),
        b.adapter_id.as_str(),
        b.release_version.as_str(),
        b.adapter_config_revision,
        b.adapter_config_digest.as_str(),
    );
    let candidate = (
        c.provider_id.as_str(),
        c.provider_owner_account_id.as_str(),
        c.registry_release_id.as_str(),
        c.registry_release_digest.as_str(),
        c.installation_receipt_id.as_str(),
        c.installation_receipt_digest.as_str(),
        c.installation_content_digest.as_str(),
        c.route_adapter_projection_id.as_str(),
        c.logical_adapter_id.as_str(),
        c.release_version.as_str(),
        c.adapter_config_revision,
        c.adapter_config_digest.as_str(),
    );
    let profile = (
        p.provider_id.as_str(),
        p.provider_owner_account_id.as_str(),
        p.registry_release_id.as_str(),
        p.registry_release_digest.as_str(),
        p.installation_receipt_id.as_str(),
        p.installation_receipt_digest.as_str(),
        p.installation_content_digest.as_str(),
        p.route_adapter_projection_id.as_str(),
        p.logical_adapter_id.as_str(),
        p.release_version.as_str(),
        p.adapter_config_revision,
        p.adapter_config_digest.as_str(),
    );
    let target = (
        t.provider_id.as_str(),
        t.provider_owner_account_id.as_str(),
        t.registry_release_id.as_str(),
        t.registry_release_digest.as_str(),
        t.installation_receipt_id.as_str(),
        t.installation_receipt_digest.as_str(),
        t.installation_content_digest.as_str(),
        t.route_adapter_projection_id.as_str(),
        t.logical_adapter_id.as_str(),
        t.release_version.as_str(),
        t.adapter_config_revision,
        t.adapter_config_digest.as_str(),
    );
    let companion = (
        s.provider_id.as_str(),
        s.provider_owner_account_id.as_str(),
        s.registry_release_id.as_str(),
        s.registry_release_digest.as_str(),
        s.installation_receipt_id.as_str(),
        s.installation_receipt_digest.as_str(),
        s.installation_content_digest.as_str(),
        s.route_adapter_projection_id.as_str(),
        s.logical_adapter_id.as_str(),
        s.release_version.as_str(),
        s.adapter_config_revision,
        s.adapter_config_digest.as_str(),
    );
    expected == candidate && expected == profile && expected == target && expected == companion
}

fn common_provider_origins_are_exact(
    b: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryProviderBindingMaterial,
    c: &crate::compute_federation::external_pool_provider_activation_candidate::ExternalPoolProviderActivationCandidateMaterial,
    p: &crate::compute_federation::external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileMaterial,
    t: &crate::compute_federation::external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetMaterial,
    s: &crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::ExternalPoolAdapterSupervisorSessionPolicyCompanionMaterial,
) -> bool {
    c.provider_policy_revision == b.provider_policy_revision
        && p.provider_policy_revision == b.provider_policy_revision
        && t.provider_policy_revision == b.provider_policy_revision
        && s.provider_policy_revision == b.provider_policy_revision
        && c.provider_digest == b.provider_digest
        && p.provider_digest == b.provider_digest
        && t.provider_digest == b.provider_digest
        && s.provider_digest == b.provider_digest
}
