use anyhow::Result;

use crate::compute_federation::external_pool_adapter_artifact_package::ARTIFACT_PACKAGE_RESOURCE_ROLE;

use super::*;

pub(crate) const RUNTIME_COMPATIBILITY_RUNNER_POLICY_ID: &str =
    "external_pool_adapter_server_owned_runtime_compatibility_runner_v1";
pub(crate) const RUNTIME_COMPATIBILITY_RUNNER_POLICY_REVISION: u64 = 1;
pub(crate) const RUNTIME_COMPATIBILITY_PUBLIC_FIXTURE_CATALOG_ID: &str =
    "external_pool_adapter_runtime_compatibility_public_fixture_catalog_v1";
pub(crate) const RUNTIME_COMPATIBILITY_PUBLIC_FIXTURE_CATALOG_REVISION: u64 = 1;
pub(crate) const RUNTIME_COMPATIBILITY_SOURCE_CAPSULE_POLICY_ID: &str =
    "external_pool_adapter_entrypoint_capsule_policy_v1";
pub(crate) const RUNTIME_COMPATIBILITY_SOURCE_CAPSULE_POLICY_REVISION: u64 = 1;
pub(crate) const RUNTIME_COMPATIBILITY_SOURCE_CAPSULE_POLICY_DIGEST: &str =
    "710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f";

pub(crate) const RUNTIME_COMPATIBILITY_CONFIG_FIXTURE_PATH: &str = "compatibility/v2/config.bin";
pub(crate) const RUNTIME_COMPATIBILITY_CREDENTIAL_FIXTURE_PATH: &str =
    "compatibility/v2/credential.bin";
pub(crate) const RUNTIME_COMPATIBILITY_NO_WORK_REQUEST_FIXTURE_PATH: &str =
    "compatibility/v2/no-work-request.bin";
pub(crate) const RUNTIME_COMPATIBILITY_NO_WORK_RESPONSE_FIXTURE_PATH: &str =
    "compatibility/v2/no-work-response.bin";
pub(crate) const RUNTIME_COMPATIBILITY_MAX_CONFIG_BYTES: u64 = 1_048_576;
pub(crate) const RUNTIME_COMPATIBILITY_MAX_CREDENTIAL_BYTES: u64 = 65_536;
pub(crate) const RUNTIME_COMPATIBILITY_MAX_REQUEST_BYTES: u64 = 16_384;
pub(crate) const RUNTIME_COMPATIBILITY_MAX_RESPONSE_BYTES: u64 = 65_536;
pub(crate) const RUNTIME_COMPATIBILITY_MAX_RUN_SECONDS: u64 = 30;
pub(crate) const RUNTIME_COMPATIBILITY_MAX_PROBE_TIMEOUT_MS: u64 = 15_000;

pub(crate) fn server_runtime_compatibility_runner_policy_catalog(
) -> Result<(ExternalPoolAdapterRuntimeCompatibilityRunnerPolicy, String)> {
    let policy = runtime_compatibility_runner_policy_for_validation();
    validate_runtime_compatibility_runner_policy(&policy)?;
    let digest = runtime_compatibility_runner_policy_digest(&policy)?;
    Ok((policy, digest))
}

pub(super) fn runtime_compatibility_runner_policy_for_validation(
) -> ExternalPoolAdapterRuntimeCompatibilityRunnerPolicy {
    ExternalPoolAdapterRuntimeCompatibilityRunnerPolicy {
        policy_id: RUNTIME_COMPATIBILITY_RUNNER_POLICY_ID.into(),
        policy_revision: RUNTIME_COMPATIBILITY_RUNNER_POLICY_REVISION,
        host_os: "linux".into(),
        host_arch: "x86_64".into(),
        runner_owner: "store_private_server_owned_runner_v1".into(),
        retained_file_policy: "installation_audit_no_follow_retained_handles_exact_rehash_v1".into(),
        launch_image_derivation_policy:
            "v267_validated_source_to_relocated_et_exec_rx_post_exec_stub_v1".into(),
        post_exec_dumpable_policy:
            "v267_stub_pr_set_dumpable_zero_then_pr_get_dumpable_zero_before_seed_protocol_v1"
                .into(),
        exec_transition_ptrace_guard:
            "v267_yama_scope_2_or_3_nofollow_exact_exec_transition_guard_v1".into(),
        seqpacket_ancillary_policy:
            "v267_seqpacket_reject_msg_trunc_msg_ctrunc_or_nonzero_control_v1".into(),
        no_work_protocol_policy: "elnw_v1_exact_one_shot_authenticated_receipt_v1".into(),
        fixture_delivery_policy: "authenticated_child_public_fixture_only_v1".into(),
        request_match_policy: "byte_exact_public_fixture_match_v1".into(),
        response_policy: "byte_exact_public_no_work_fixture_response_v1".into(),
        network_policy: "no_network_no_dns_no_socket_v1".into(),
        upstream_policy: "no_upstream_target_or_connection_v1".into(),
        cleanup_policy:
            "v267_authenticated_shutdown_bounded_pidfd_reap_cgroup_and_scratch_cleanup_all_errors_visible_v1"
                .into(),
        observation_commit_policy:
            "after_authenticated_shutdown_bounded_reap_and_cgroup_cleanup_v1".into(),
        challenge_validity_seconds: 300,
        verification_receipt_validity_seconds: 86_400,
        max_run_seconds: RUNTIME_COMPATIBILITY_MAX_RUN_SECONDS,
        max_probe_timeout_ms: RUNTIME_COMPATIBILITY_MAX_PROBE_TIMEOUT_MS,
        caller_supplied_material_allowed: false,
    }
}

pub(crate) fn server_runtime_compatibility_public_fixture_catalog() -> Result<(
    ExternalPoolAdapterRuntimeCompatibilityPublicFixtureCatalog,
    String,
)> {
    let catalog = runtime_compatibility_public_fixture_catalog_for_validation();
    validate_runtime_compatibility_public_fixture_catalog(&catalog)?;
    let digest = runtime_compatibility_fixture_catalog_digest(&catalog)?;
    Ok((catalog, digest))
}

pub(super) fn runtime_compatibility_public_fixture_catalog_for_validation(
) -> ExternalPoolAdapterRuntimeCompatibilityPublicFixtureCatalog {
    ExternalPoolAdapterRuntimeCompatibilityPublicFixtureCatalog {
        catalog_id: RUNTIME_COMPATIBILITY_PUBLIC_FIXTURE_CATALOG_ID.into(),
        catalog_revision: RUNTIME_COMPATIBILITY_PUBLIC_FIXTURE_CATALOG_REVISION,
        inventory_policy: "exact_four_package_manifest_resources_public_fixture_only_v1".into(),
        resources: vec![
            fixture_requirement(
                "config",
                RUNTIME_COMPATIBILITY_CONFIG_FIXTURE_PATH,
                RUNTIME_COMPATIBILITY_MAX_CONFIG_BYTES,
            ),
            fixture_requirement(
                "credential",
                RUNTIME_COMPATIBILITY_CREDENTIAL_FIXTURE_PATH,
                RUNTIME_COMPATIBILITY_MAX_CREDENTIAL_BYTES,
            ),
            fixture_requirement(
                "no_work_request",
                RUNTIME_COMPATIBILITY_NO_WORK_REQUEST_FIXTURE_PATH,
                RUNTIME_COMPATIBILITY_MAX_REQUEST_BYTES,
            ),
            fixture_requirement(
                "no_work_response",
                RUNTIME_COMPATIBILITY_NO_WORK_RESPONSE_FIXTURE_PATH,
                RUNTIME_COMPATIBILITY_MAX_RESPONSE_BYTES,
            ),
        ],
    }
}

pub(crate) fn runtime_compatibility_source_capsule_policy_ref(
) -> ExternalPoolAdapterRuntimeCompatibilityPolicyRef {
    // V257 keeps this catalog Store-private. V268 freezes the exact V257 root and the Store-runner
    // must compare it with the materialized capsule root before an observation can be recorded.
    ExternalPoolAdapterRuntimeCompatibilityPolicyRef {
        policy_id: RUNTIME_COMPATIBILITY_SOURCE_CAPSULE_POLICY_ID.into(),
        policy_revision: RUNTIME_COMPATIBILITY_SOURCE_CAPSULE_POLICY_REVISION,
        policy_digest: RUNTIME_COMPATIBILITY_SOURCE_CAPSULE_POLICY_DIGEST.into(),
    }
}

pub(crate) fn runtime_compatibility_no_effects() -> ExternalPoolAdapterRuntimeCompatibilityEffects {
    ExternalPoolAdapterRuntimeCompatibilityEffects {
        credential_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
        adapter_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
        provider_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
        route_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
        activation_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
        execution_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
        usage_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
        market_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
        settlement_effect: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT.into(),
    }
}

pub(crate) fn runtime_compatibility_no_readiness(
) -> ExternalPoolAdapterRuntimeCompatibilityReadiness {
    ExternalPoolAdapterRuntimeCompatibilityReadiness {
        process_ready: false,
        session_ready: false,
        secret_delivery_ready: false,
        broker_connect_ready: false,
        upstream_probe_ready: false,
        runtime_launch_ready: false,
        activation_ready: false,
    }
}

fn fixture_requirement(
    purpose: &str,
    path: &str,
    max_size_bytes: u64,
) -> ExternalPoolAdapterRuntimeCompatibilityFixtureResourceRequirement {
    ExternalPoolAdapterRuntimeCompatibilityFixtureResourceRequirement {
        purpose: purpose.into(),
        path: path.into(),
        role: ARTIFACT_PACKAGE_RESOURCE_ROLE.into(),
        min_size_bytes: 1,
        max_size_bytes,
        public_fixture_only: true,
    }
}
