use anyhow::Result;

use super::*;

pub(crate) const PROVIDER_RUNTIME_READINESS_POLICY_ID: &str =
    "external_pool_adapter_provider_runtime_readiness_policy_v1";
pub(crate) const PROVIDER_RUNTIME_READINESS_POLICY_REVISION: u64 = 1;

pub(crate) fn server_provider_runtime_readiness_policy_catalog(
) -> Result<ExternalPoolAdapterProviderRuntimeReadinessPolicyEnvelope> {
    let policy = provider_runtime_readiness_policy_for_validation();
    validate_provider_runtime_readiness_policy(&policy)?;
    let envelope = ExternalPoolAdapterProviderRuntimeReadinessPolicyEnvelope {
        schema: PROVIDER_RUNTIME_READINESS_POLICY_ENVELOPE_SCHEMA.into(),
        canonicalization: PROVIDER_RUNTIME_READINESS_CANONICALIZATION.into(),
        digest_algorithm: PROVIDER_RUNTIME_READINESS_DIGEST_ALGORITHM.into(),
        policy_digest: provider_runtime_readiness_policy_digest(&policy)?,
        policy,
    };
    validate_provider_runtime_readiness_policy_envelope(&envelope)?;
    Ok(envelope)
}

pub(super) fn provider_runtime_readiness_policy_for_validation(
) -> ExternalPoolAdapterProviderRuntimeReadinessPolicy {
    ExternalPoolAdapterProviderRuntimeReadinessPolicy {
        schema: PROVIDER_RUNTIME_READINESS_POLICY_SCHEMA.into(),
        policy_id: PROVIDER_RUNTIME_READINESS_POLICY_ID.into(),
        policy_revision: PROVIDER_RUNTIME_READINESS_POLICY_REVISION,
        host_os: "linux".into(),
        host_arch: "x86_64".into(),
        runtime_kind: "server_sidecar_v1".into(),
        trigger_policy: "synchronous_platform_admin_only_default_off_v1".into(),
        startup_custody_policy: "independent_three_environment_fail_closed_startup_custody_v1"
            .into(),
        bundle_root_policy:
            "startup_retained_nofollow_content_addressed_bundle_root_directory_authority_v1".into(),
        cgroup_parent_policy:
            "startup_retained_nofollow_delegated_cgroup_v2_parent_directory_authority_v1".into(),
        hmac_algorithm: PROVIDER_RUNTIME_READINESS_HMAC_ALGORITHM.into(),
        hmac_key_policy: "locked_zeroize_on_drop_process_ephemeral_256_bit_v1".into(),
        custody_epoch_policy: "random_process_epoch_domain_separated_digest_restart_historical_v1"
            .into(),
        runtime_bundle_identity_commitment_policy:
            "hmac_over_exact_transient_v256_identity_and_relevant_provider_roots_v1".into(),
        post_cleanup_observation_commitment_policy:
            "hmac_over_bundle_v263_v265_target_shutdown_reap_cgroup_scratch_cleanup_v1".into(),
        evidence_policy: "exact_current_v249_v250_v252_v253_v254_v255_v258_v259_and_v268_v1".into(),
        late_binding_policy: "six_independent_installation_prepared_reopen_and_exact_rehash_v1"
            .into(),
        probe_contract: "provider_specific_authenticated_no_work_v1".into(),
        max_probe_timeout_ms: PROVIDER_RUNTIME_READINESS_MAX_PROBE_TIMEOUT_MS,
        max_request_bytes: PROVIDER_RUNTIME_READINESS_MAX_REQUEST_BYTES,
        max_response_bytes: PROVIDER_RUNTIME_READINESS_MAX_RESPONSE_BYTES,
        cleanup_policy: "authenticated_shutdown_pidfd_bounded_reap_cgroup_and_scratch_cleanup_v1"
            .into(),
        observation_commit_policy:
            "post_cleanup_begin_immediate_same_checked_at_exact_root_reproof_v1".into(),
        expiry_policy:
            "minimum_checked_at_plus_probe_timeout_v250_v252_v253_and_exact_v268_expiry_v1".into(),
        lineage_policy: "one_linear_head_per_provider_binding_v1".into(),
        currentness_policy:
            "same_process_epoch_fresh_bundle_commitment_and_all_exact_roots_current_v1".into(),
        revocation_policy:
            "structural_head_only_owner_or_platform_admin_without_runtime_custody_v1".into(),
        endpoint_disclosure_policy:
            "no_endpoint_selected_address_secret_or_unkeyed_secret_derived_digest_v1".into(),
        caller_supplied_runtime_material_allowed: false,
        activation_authority: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        effects: provider_runtime_readiness_no_effects(),
        observed_readiness: provider_runtime_readiness_observed_readiness(),
    }
}

pub(crate) fn provider_runtime_readiness_no_effects(
) -> ExternalPoolAdapterProviderRuntimeReadinessEffects {
    ExternalPoolAdapterProviderRuntimeReadinessEffects {
        credential_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        adapter_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        provider_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        route_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        activation_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        execution_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        usage_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        market_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
        settlement_effect: PROVIDER_RUNTIME_READINESS_NO_EFFECT.into(),
    }
}

pub(crate) fn provider_runtime_readiness_observed_readiness(
) -> ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness {
    ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness {
        process_spawn_ready: true,
        ipc_session_ready: true,
        secret_delivery_ready: true,
        broker_connect_ready: true,
        upstream_probe_observed: true,
        runtime_launch_ready: true,
        activation_ready: false,
    }
}

pub(crate) fn provider_runtime_readiness_no_readiness(
) -> ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness {
    ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness {
        process_spawn_ready: false,
        ipc_session_ready: false,
        secret_delivery_ready: false,
        broker_connect_ready: false,
        upstream_probe_observed: false,
        runtime_launch_ready: false,
        activation_ready: false,
    }
}
