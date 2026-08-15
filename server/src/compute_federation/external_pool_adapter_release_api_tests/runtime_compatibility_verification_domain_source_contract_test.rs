use sha2::{Digest, Sha256};

const V1_TYPES: &str = include_str!("../external_pool_adapter_runtime_compatibility/types.rs");
const V1_PROFILE_JSON: &str = include_str!(
    "../../../../docs/distributed-compute/external-pool-adapter-runtime-compatibility-profile-v1.json"
);
const V2_TYPES: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification/types.rs");
const V2_PROFILE: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification/profile.rs");
const V2_POLICY: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification/policy.rs");
const V2_CANONICAL: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification/canonical.rs");
const V2_EVIDENCE_VALIDATION: &str = include_str!(
    "../external_pool_adapter_runtime_compatibility_verification/evidence_validation.rs"
);

#[test]
fn runtime_compatibility_domain_source_preserves_v1_and_splits_v2_identity() {
    for required in [
        "compute_federation.external_pool_adapter_runtime_compatibility_profile.v1",
        "compute_federation.external_pool_adapter_runtime_compatibility_profile_envelope.v1",
        "external_pool_adapter_linux_runtime_compatibility_v1",
        "RUNTIME_COMPATIBILITY_PROFILE_REVISION: u64 = 1",
    ] {
        assert!(
            V1_TYPES.contains(required),
            "V1 identity drifted: {required}"
        );
    }
    assert_eq!(
        hex::encode(Sha256::digest(V1_PROFILE_JSON.as_bytes())),
        "2aabd86ea0cc840f4ef3df251727f7b3e26dd0ea32ca664c0b240dcc570d808f"
    );
    assert!(V1_PROFILE_JSON.contains(
        "\"profile_digest\": \"a63d30b6f2f75c78c156ddb9ea609312f8b9b6726f403fedb960ed8a754fa047\""
    ));
    for required in [
        "compute_federation.external_pool_adapter_runtime_compatibility_profile.v2",
        "compute_federation.external_pool_adapter_runtime_compatibility_profile_envelope.v2",
        "external_pool_adapter_linux_runtime_compatibility_v2",
        "RUNTIME_COMPATIBILITY_V2_PROFILE_REVISION: u64 = 2",
    ] {
        assert!(
            V2_TYPES.contains(required),
            "missing V2 identity {required}"
        );
    }
    for required in [
        "server_linux_runtime_launch_policy_catalog",
        "server_supervisor_session_policy_catalog",
        "server_upstream_transport_target_policy_catalog",
        "runtime_compatibility_source_capsule_policy_ref",
        "server_runtime_compatibility_runner_policy_catalog",
        "server_runtime_compatibility_public_fixture_catalog",
    ] {
        assert!(
            V2_PROFILE.contains(required),
            "missing V2 profile root {required}"
        );
    }
}

#[test]
fn runtime_compatibility_domain_source_freezes_four_release_declared_public_fixture_paths() {
    for path in [
        "compatibility/v2/config.bin",
        "compatibility/v2/credential.bin",
        "compatibility/v2/no-work-request.bin",
        "compatibility/v2/no-work-response.bin",
    ] {
        assert_eq!(
            V2_POLICY.matches(path).count(),
            1,
            "fixture path drifted: {path}"
        );
    }
    for required in [
        "exact_four_package_manifest_resources_public_fixture_only_v1",
        "role: ARTIFACT_PACKAGE_RESOURCE_ROLE.into()",
        "public_fixture_only: true",
        "caller_supplied_material_allowed: false",
        "v267_validated_source_to_relocated_et_exec_rx_post_exec_stub_v1",
        "v267_stub_pr_set_dumpable_zero_then_pr_get_dumpable_zero_before_seed_protocol_v1",
        "v267_yama_scope_2_or_3_nofollow_exact_exec_transition_guard_v1",
        "v267_seqpacket_reject_msg_trunc_msg_ctrunc_or_nonzero_control_v1",
        "elnw_v1_exact_one_shot_authenticated_receipt_v1",
        "v267_authenticated_shutdown_bounded_pidfd_reap_cgroup_and_scratch_cleanup_all_errors_visible_v1",
        "no_network_no_dns_no_socket_v1",
        "no_upstream_target_or_connection_v1",
        "after_authenticated_shutdown_bounded_reap_and_cgroup_cleanup_v1",
    ] {
        assert!(
            V2_POLICY.contains(required),
            "fixture policy drifted: {required}"
        );
    }
}

#[test]
fn runtime_compatibility_domain_source_freezes_new_v237_signature_domain() {
    for required in [
        "rsa-pkcs1v15-sha256",
        "v237_signature_over_server_owned_controlled_public_fixture_runtime_observation_v1",
    ] {
        assert!(
            V2_TYPES.contains(required),
            "missing V237 binding {required}"
        );
    }
    for required in [
        "ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-SIGNATURE-MESSAGE-V1",
        "runtime_compatibility_signature_challenge(",
        "runtime_compatibility_signature_message(",
        "message.extend_from_slice(SIGNATURE_MESSAGE_DOMAIN)",
        "challenge.challenge_digest.as_str()",
        "observation.run_observation_digest.as_str()",
    ] {
        assert!(
            V2_CANONICAL.contains(required),
            "missing signature domain {required}"
        );
    }
    for required in [
        "verify_runtime_compatibility_signature(",
        "VerifyingKey::<Sha256>::new(public)",
        "value.signature_message_digest != expected_message.signature_message_digest",
    ] {
        assert!(
            V2_EVIDENCE_VALIDATION.contains(required),
            "missing signature verification {required}"
        );
    }
}

#[test]
fn runtime_compatibility_observation_has_no_durable_signature_message_cycle() {
    let observation = struct_block(
        V2_TYPES,
        "ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial",
    );
    for forbidden in ["signature_message_base64", "signature_message_digest"] {
        assert!(
            !observation.contains(forbidden),
            "durable observation leaks {forbidden}"
        );
    }
    let transient = struct_block(
        V2_TYPES,
        "ExternalPoolAdapterRuntimeCompatibilitySignatureChallenge",
    );
    assert!(transient.contains("signature_message_base64"));
    assert!(transient.contains("signature_message_digest"));
    assert!(V2_TYPES
        .contains("Store-runner handoff. It is deliberately non-Clone/non-Debug/non-Serde."));
    for required in [
        "source_capsule_sha256",
        "launch_image_sha256",
        "value.source_capsule_sha256 == value.launch_image_sha256",
    ] {
        let source = format!("{V2_TYPES}{V2_EVIDENCE_VALIDATION}");
        assert!(
            source.contains(required),
            "missing dual capsule root {required}"
        );
    }
}

#[test]
fn runtime_compatibility_domain_source_freezes_lineage_none_effects_and_false_readiness() {
    for required in [
        "sequence: u64",
        "predecessor_verification_receipt_id: Option<String>",
        "predecessor_verification_receipt_digest: Option<String>",
        "current_signed_verifier_assertion",
        "historical_signed_verifier_assertion",
    ] {
        assert!(
            V2_TYPES.contains(required),
            "missing release lineage {required}"
        );
    }
    for effect in [
        "credential_effect",
        "adapter_effect",
        "provider_effect",
        "route_effect",
        "activation_effect",
        "execution_effect",
        "usage_effect",
        "market_effect",
        "settlement_effect",
    ] {
        assert!(
            V2_POLICY.contains(&format!(
                "{effect}: RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT"
            )),
            "effect is not frozen none: {effect}"
        );
    }
    for readiness in [
        "process_ready",
        "session_ready",
        "secret_delivery_ready",
        "broker_connect_ready",
        "upstream_probe_ready",
        "runtime_launch_ready",
        "activation_ready",
    ] {
        assert!(
            V2_POLICY.contains(&format!("{readiness}: false")),
            "readiness is not frozen false: {readiness}"
        );
    }
}

fn struct_block<'a>(source: &'a str, name: &str) -> &'a str {
    source
        .split_once(&format!("struct {name} {{"))
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0
}
