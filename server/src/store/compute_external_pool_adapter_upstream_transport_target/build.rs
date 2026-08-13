use anyhow::Result;

use crate::{
    compute_federation::external_pool_adapter_upstream_transport_target::*,
    store::{
        compute_external_pool_adapter_runtime_launch_profile::CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
        new_id,
    },
};

use super::{
    policy::upstream_transport_target_policy_catalog,
    types::{
        ExternalPoolAdapterUpstreamTransportTargetDraft,
        RevokeExternalPoolAdapterUpstreamTransportTarget, StoredUpstreamTransportTarget,
    },
};

#[allow(clippy::too_many_arguments)]
pub(super) fn build_target(
    authority: &CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
    predecessor: Option<&StoredUpstreamTransportTarget>,
    draft: ExternalPoolAdapterUpstreamTransportTargetDraft,
    sequence: u64,
    now: &str,
    recorded_by_actor_kind: &str,
    recorded_by_actor_user_id: &str,
    idempotency_scope: &str,
    idempotency_key: &str,
    confirmation: &str,
) -> Result<ExternalPoolAdapterUpstreamTransportTargetReceipt> {
    let profile_receipt = authority.profile();
    let p = &profile_receipt.profile;
    let policy = upstream_transport_target_policy_catalog()?;
    let material = ExternalPoolAdapterUpstreamTransportTargetMaterial {
        profile_id: profile_receipt.profile_id.clone(),
        profile_digest: profile_receipt.profile_digest.clone(),
        candidate_id: p.candidate_id.clone(),
        candidate_digest: p.candidate_digest.clone(),
        delegation_id: p.delegation_id.clone(),
        delegation_digest: p.delegation_digest.clone(),
        provider_binding_id: p.provider_binding_id.clone(),
        provider_binding_digest: p.provider_binding_digest.clone(),
        registry_release_id: p.registry_release_id.clone(),
        registry_release_digest: p.registry_release_digest.clone(),
        installation_receipt_id: p.installation_receipt_id.clone(),
        installation_receipt_digest: p.installation_receipt_digest.clone(),
        installation_content_digest: p.installation_content_digest.clone(),
        route_adapter_projection_id: p.route_adapter_projection_id.clone(),
        provider_id: p.provider_id.clone(),
        provider_owner_account_id: p.provider_owner_account_id.clone(),
        provider_policy_revision: p.provider_policy_revision,
        provider_digest: p.provider_digest.clone(),
        provider_status: p.provider_status.clone(),
        logical_adapter_id: p.logical_adapter_id.clone(),
        release_version: p.release_version.clone(),
        adapter_config_revision: p.adapter_config_revision,
        adapter_config_digest: p.adapter_config_digest.clone(),
        implementation_digest: p.implementation_digest.clone(),
        capability_set_digest: p.capability_set_digest.clone(),
        credential_verifier_digest: p.credential_verifier_digest.clone(),
        launch_policy_digest: p.launch_policy_digest.clone(),
        network_egress_policy_id: p.launch_policy.network_egress_policy_id.clone(),
        network_egress_policy_revision: p.launch_policy.network_egress_policy_revision,
        network_egress_policy_digest: p.launch_policy.network_egress_policy_digest.clone(),
        service_actor_id: p.service_actor_id.clone(),
        target_policy_digest: policy.digest,
        target_policy: policy.policy,
        tls_server_name: draft.dns_hostname.clone(),
        dns_hostname: draft.dns_hostname,
        port: draft.port,
        expected_tls_leaf_spki_sha256: draft.expected_tls_leaf_spki_sha256,
        sequence,
        predecessor_target_id: predecessor.map(|value| value.receipt.target_id.clone()),
        predecessor_target_digest: predecessor.map(|value| value.receipt.target_digest.clone()),
        recorded_by_actor_kind: recorded_by_actor_kind.into(),
        recorded_by_actor_user_id: recorded_by_actor_user_id.into(),
        recorded_at: now.into(),
        idempotency_scope: idempotency_scope.into(),
        idempotency_key: idempotency_key.into(),
        confirmation: confirmation.into(),
        target_status: UPSTREAM_TRANSPORT_TARGET_STATUS.into(),
        target_effect: UPSTREAM_TRANSPORT_TARGET_EFFECT.into(),
        adapter_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        runtime_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        provider_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        credential_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        route_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        execution_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        usage_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        market_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        settlement_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        broker_connect_ready: false,
        upstream_probe_observed: false,
        runtime_launch_ready: false,
        activation_ready: false,
    };
    seal_target(material)
}

pub(super) fn build_revocation(
    input: &RevokeExternalPoolAdapterUpstreamTransportTarget,
    target: &StoredUpstreamTransportTarget,
    now: &str,
) -> Result<ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt> {
    let t = &target.receipt.target;
    let material = ExternalPoolAdapterUpstreamTransportTargetRevocationMaterial {
        target_id: target.receipt.target_id.clone(),
        target_digest: target.receipt.target_digest.clone(),
        profile_id: t.profile_id.clone(),
        profile_digest: t.profile_digest.clone(),
        provider_binding_id: t.provider_binding_id.clone(),
        provider_binding_digest: t.provider_binding_digest.clone(),
        provider_id: t.provider_id.clone(),
        revoked_by_actor_kind: input.revoked_by_actor_kind.clone(),
        revoked_by_actor_user_id: input.revoked_by_actor_user_id.clone(),
        reason: input.reason.clone(),
        revoked_at: now.into(),
        recorded_at: now.into(),
        idempotency_scope: input.idempotency_scope.clone(),
        idempotency_key: input.idempotency_key.clone(),
        confirmation: input.confirmation.clone(),
        revocation_effect: UPSTREAM_TRANSPORT_TARGET_REVOCATION_EFFECT.into(),
        adapter_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        runtime_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        provider_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        credential_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        route_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        execution_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        usage_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        market_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        settlement_effect: UPSTREAM_TRANSPORT_TARGET_NO_EFFECT.into(),
        broker_connect_ready: false,
        upstream_probe_observed: false,
        runtime_launch_ready: false,
        activation_ready: false,
    };
    seal_revocation(material)
}

fn seal_target(
    material: ExternalPoolAdapterUpstreamTransportTargetMaterial,
) -> Result<ExternalPoolAdapterUpstreamTransportTargetReceipt> {
    let mut receipt = ExternalPoolAdapterUpstreamTransportTargetReceipt {
        schema: UPSTREAM_TRANSPORT_TARGET_SCHEMA.into(),
        target_id: new_id("external_pool_adapter_upstream_transport_target"),
        target_digest: String::new(),
        target_material_digest: upstream_transport_target_material_digest(&material)?,
        canonicalization: UPSTREAM_TRANSPORT_TARGET_CANONICALIZATION.into(),
        digest_algorithm: UPSTREAM_TRANSPORT_TARGET_DIGEST_ALGORITHM.into(),
        target: material,
    };
    receipt.target_digest = canonical_upstream_transport_target_json_and_digest(&receipt)?.1;
    validate_upstream_transport_target_receipt(&receipt)?;
    Ok(receipt)
}

fn seal_revocation(
    material: ExternalPoolAdapterUpstreamTransportTargetRevocationMaterial,
) -> Result<ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt> {
    let mut receipt = ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt {
        schema: UPSTREAM_TRANSPORT_TARGET_REVOCATION_SCHEMA.into(),
        revocation_id: new_id("external_pool_adapter_upstream_transport_target_revocation"),
        revocation_digest: String::new(),
        revocation_material_digest: upstream_transport_target_revocation_material_digest(
            &material,
        )?,
        canonicalization: UPSTREAM_TRANSPORT_TARGET_CANONICALIZATION.into(),
        digest_algorithm: UPSTREAM_TRANSPORT_TARGET_DIGEST_ALGORITHM.into(),
        revocation: material,
    };
    receipt.revocation_digest =
        canonical_upstream_transport_target_revocation_json_and_digest(&receipt)?.1;
    validate_upstream_transport_target_revocation_receipt(&receipt)?;
    Ok(receipt)
}
