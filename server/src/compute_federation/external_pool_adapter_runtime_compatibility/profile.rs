use anyhow::Result;

use crate::compute_federation::{
    external_pool_adapter_release::ComputeExternalPoolAdapterReleaseCapability,
    external_pool_adapter_runtime_launch_profile::server_linux_runtime_launch_policy_catalog,
    external_pool_adapter_supervisor_session_policy_companion::historical_supervisor_session_policy_v1_catalog,
    external_pool_adapter_upstream_transport_target::server_upstream_transport_target_policy_catalog,
};

use super::*;

pub(crate) fn server_runtime_compatibility_profile_catalog(
) -> Result<ExternalPoolAdapterRuntimeCompatibilityProfileEnvelope> {
    let profile = profile_for_validation()?;
    validate_runtime_compatibility_profile(&profile)?;
    let profile_digest = runtime_compatibility_profile_digest(&profile)?;
    Ok(ExternalPoolAdapterRuntimeCompatibilityProfileEnvelope {
        schema: RUNTIME_COMPATIBILITY_PROFILE_ENVELOPE_SCHEMA.into(),
        canonicalization: RUNTIME_COMPATIBILITY_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_COMPATIBILITY_DIGEST_ALGORITHM.into(),
        profile_digest,
        profile,
    })
}

pub(crate) fn build_runtime_compatibility_challenge(
    challenge: ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<ExternalPoolAdapterRuntimeCompatibilityChallenge> {
    validate_runtime_compatibility_challenge_material(&challenge)?;
    let challenge_digest = runtime_compatibility_challenge_digest(&challenge)?;
    Ok(ExternalPoolAdapterRuntimeCompatibilityChallenge {
        schema: RUNTIME_COMPATIBILITY_CHALLENGE_SCHEMA.into(),
        canonicalization: RUNTIME_COMPATIBILITY_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_COMPATIBILITY_DIGEST_ALGORITHM.into(),
        challenge_digest,
        challenge,
    })
}

pub(super) fn profile_for_validation() -> Result<ExternalPoolAdapterRuntimeCompatibilityProfile> {
    let (runtime, runtime_digest) = server_linux_runtime_launch_policy_catalog()?;
    let (transport, transport_digest) = server_upstream_transport_target_policy_catalog()?;
    let (session, session_digest) = historical_supervisor_session_policy_v1_catalog()?;
    let no_effects = no_effects();

    Ok(ExternalPoolAdapterRuntimeCompatibilityProfile {
        schema: RUNTIME_COMPATIBILITY_PROFILE_SCHEMA.into(),
        profile_id: RUNTIME_COMPATIBILITY_PROFILE_ID.into(),
        profile_revision: RUNTIME_COMPATIBILITY_PROFILE_REVISION,
        host_os: "linux".into(),
        host_arch: "x86_64".into(),
        release_capabilities: release_capabilities(),
        runtime_launch_policy: ExternalPoolAdapterCompatibilityPolicyRef {
            policy_id: runtime.policy_id.clone(),
            policy_revision: runtime.policy_revision,
            policy_digest: runtime_digest,
        },
        upstream_transport_policy: ExternalPoolAdapterCompatibilityPolicyRef {
            policy_id: transport.policy_id.clone(),
            policy_revision: transport.policy_revision,
            policy_digest: transport_digest,
        },
        supervisor_session_policy: ExternalPoolAdapterCompatibilityPolicyRef {
            policy_id: session.policy_id.clone(),
            policy_revision: session.policy_revision,
            policy_digest: session_digest,
        },
        elsp: ExternalPoolAdapterCompatibilityElspProtocol {
            protocol_id: session.wire.protocol_id,
            protocol_revision: session.wire.protocol_revision,
            transport: session.wire.transport,
            framing: session.wire.framing,
            frame_magic_ascii: session.wire.frame_magic_ascii,
            frame_header_bytes: session.wire.frame_header_bytes,
            frame_mac_bytes: session.wire.frame_mac_bytes,
            frame_kind_control: session.wire.frame_kind_control,
            frame_kind_config: session.wire.frame_kind_config,
            frame_kind_credential: session.wire.frame_kind_credential,
            control_encoding: session.wire.control_encoding,
            binary_encoding: session.wire.binary_encoding,
            sequence_policy: session.crypto.sequence_policy,
            mac: session.crypto.mac,
            config_delivery_kind: runtime.config_delivery_kind,
            credential_delivery_kind: runtime.credential_delivery_kind,
        },
        elnw: ExternalPoolAdapterCompatibilityElnwProtocol {
            frame_kind: "elsp_control_payload_v1".into(),
            magic_ascii: "ELNW".into(),
            version: 1,
            flags: 0,
            begin_kind: 1,
            request_kind: 2,
            response_kind: 3,
            receipt_kind: 4,
            request_header_bytes: 48,
            response_header_bytes: 44,
            receipt_bytes: 136,
            max_request_bytes: MAX_COMPATIBILITY_REQUEST_BYTES,
            max_response_bytes: MAX_COMPATIBILITY_RESPONSE_BYTES,
            max_probe_timeout_ms: MAX_COMPATIBILITY_PROBE_TIMEOUT_MS,
            root_domain: "elon.external_pool_adapter.no_work_probe.root.v1\0".into(),
            integer_encoding: "unsigned_big_endian_v1".into(),
            completion_policy: "one_shot_child_semantic_validation_then_authenticated_receipt_v1"
                .into(),
        },
        broker: ExternalPoolAdapterCompatibilityBrokerProtocol {
            transport_owner: transport.transport_owner,
            transport_kind: transport.transport_kind,
            tls_version_policy: transport.tls_version_policy,
            tls_server_name_policy: transport.tls_server_name_policy,
            tls_leaf_identity_policy: transport.tls_leaf_identity_policy,
            proxy_policy: transport.proxy_policy,
            redirect_policy: transport.redirect_policy,
            zero_rtt_policy: transport.zero_rtt_policy,
            client_certificate_policy: transport.client_certificate_policy,
            adapter_network_policy: transport.adapter_network_policy,
            application_exchange_policy: "single_bounded_write_exact_length_read_v1".into(),
        },
        required_observations: REQUIRED_RUNTIME_OBSERVATIONS
            .iter()
            .map(
                |id| ExternalPoolAdapterRuntimeCompatibilityObservationRequirement {
                    observation_id: (*id).into(),
                    observation_revision: 1,
                    required_outcome: "passed".into(),
                },
            )
            .collect(),
        candidate_evidence_scope: RUNTIME_COMPATIBILITY_EVIDENCE_SCOPE.into(),
        effects: no_effects.clone(),
    })
}

pub(super) fn no_effects() -> ExternalPoolAdapterRuntimeCompatibilityEffects {
    ExternalPoolAdapterRuntimeCompatibilityEffects {
        conformance_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        credential_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        adapter_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        provider_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        route_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        activation_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        execution_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        usage_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        market_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
        settlement_effect: RUNTIME_COMPATIBILITY_NO_EFFECT.into(),
    }
}

fn release_capabilities() -> Vec<ComputeExternalPoolAdapterReleaseCapability> {
    [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ]
    .into_iter()
    .map(
        |capability_id| ComputeExternalPoolAdapterReleaseCapability {
            capability_id: capability_id.into(),
            capability_revision: 1,
        },
    )
    .collect()
}
