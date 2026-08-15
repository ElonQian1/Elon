use anyhow::Result;

use crate::compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseCapability;

use super::*;

pub(crate) const TASK_PROTOCOL_CONFORMANCE_SESSION_ROOTS_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.session.roots.v1\0";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_SESSION_KDF_SALT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.session.kdf_salt.v1\0";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_REQUEST_DIGEST_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol.request.v1\0";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_EXCHANGE_DIGEST_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol.exchange.v1\0";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_COMMIT_UNCERTAINTY_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.commit_uncertainty.v1\0";

pub(crate) const TASK_PROTOCOL_CONFORMANCE_SESSION_ROOT_NAMES: [&str;
    TASK_PROTOCOL_CONFORMANCE_SESSION_ROOT_COUNT] = [
    "supervisor_session_policy_digest",
    "task_protocol_profile_digest",
    "run_nonce_digest",
    "fixture_catalog_digest",
    "registry_release_digest",
    "installation_content_digest",
    "capability_set_digest",
    "sandbox_reattestation_receipt_digest",
    "runtime_compatibility_verification_receipt_digest",
    "source_capsule_sha256",
    "launch_image_sha256",
    "public_fixture_delivery_root",
    "synthetic_fixture_lane_digest",
    "synthetic_fixture_executor_digest",
];

pub(crate) const TASK_PROTOCOL_CONFORMANCE_CAPABILITY_IDS: [&str;
    TASK_PROTOCOL_CONFORMANCE_CAPABILITY_COUNT] = [
    "authenticated_ack",
    "authenticated_events",
    "cancel_no_start",
    "idempotent_commit",
    "prepare",
    "reconcile",
];

pub(crate) fn server_task_protocol_conformance_profile_catalog(
) -> Result<ExternalPoolAdapterTaskProtocolConformanceProfileEnvelope> {
    let profile = task_protocol_conformance_profile_for_validation();
    let envelope = ExternalPoolAdapterTaskProtocolConformanceProfileEnvelope {
        schema: TASK_PROTOCOL_CONFORMANCE_PROFILE_ENVELOPE_SCHEMA.into(),
        canonicalization: TASK_PROTOCOL_CONFORMANCE_CANONICALIZATION.into(),
        digest_algorithm: TASK_PROTOCOL_CONFORMANCE_DIGEST_ALGORITHM.into(),
        profile_digest: task_protocol_conformance_profile_digest(&profile)?,
        profile,
    };
    validate_task_protocol_conformance_profile_envelope(&envelope)?;
    Ok(envelope)
}

pub(crate) fn server_task_protocol_conformance_fixture_catalog(
) -> Result<ExternalPoolAdapterTaskProtocolConformanceFixtureCatalogEnvelope> {
    let catalog = task_protocol_conformance_fixture_catalog_for_validation();
    let envelope = ExternalPoolAdapterTaskProtocolConformanceFixtureCatalogEnvelope {
        schema: TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_ENVELOPE_SCHEMA.into(),
        canonicalization: TASK_PROTOCOL_CONFORMANCE_CANONICALIZATION.into(),
        digest_algorithm: TASK_PROTOCOL_CONFORMANCE_DIGEST_ALGORITHM.into(),
        catalog_digest: task_protocol_conformance_fixture_catalog_digest(&catalog)?,
        catalog,
    };
    validate_task_protocol_conformance_fixture_catalog_envelope(&envelope)?;
    Ok(envelope)
}

pub(super) fn task_protocol_conformance_profile_for_validation(
) -> ExternalPoolAdapterTaskProtocolConformanceProfile {
    ExternalPoolAdapterTaskProtocolConformanceProfile {
        schema: TASK_PROTOCOL_CONFORMANCE_PROFILE_SCHEMA.into(),
        profile_id: TASK_PROTOCOL_CONFORMANCE_PROFILE_ID.into(),
        profile_revision: TASK_PROTOCOL_CONFORMANCE_PROFILE_REVISION,
        host_os: "linux".into(),
        host_arch: "x86_64".into(),
        wire_prefix: "ELTP|version=1|kind|flags=0".into(),
        wire_version: 1,
        control_kinds: [
            ("begin", 1),
            ("request", 2),
            ("response", 3),
            ("receipt", 4),
        ]
        .into_iter()
        .map(code_point)
        .collect(),
        operations: [
            ("prepare", 1),
            ("idempotent_commit", 2),
            ("cancel_no_start", 3),
            ("reconcile", 4),
            ("authenticated_events", 5),
        ]
        .into_iter()
        .map(code_point)
        .collect(),
        required_capabilities: required_capabilities(),
        session_root_names: TASK_PROTOCOL_CONFORMANCE_SESSION_ROOT_NAMES
            .into_iter()
            .map(str::to_owned)
            .collect(),
        session_roots_domain: String::from_utf8_lossy(
            TASK_PROTOCOL_CONFORMANCE_SESSION_ROOTS_DOMAIN,
        )
        .into_owned(),
        session_kdf_salt_domain: String::from_utf8_lossy(
            TASK_PROTOCOL_CONFORMANCE_SESSION_KDF_SALT_DOMAIN,
        )
        .into_owned(),
        request_digest_domain: String::from_utf8_lossy(
            TASK_PROTOCOL_CONFORMANCE_REQUEST_DIGEST_DOMAIN,
        )
        .into_owned(),
        exchange_digest_domain: String::from_utf8_lossy(
            TASK_PROTOCOL_CONFORMANCE_EXCHANGE_DIGEST_DOMAIN,
        )
        .into_owned(),
        first_ordinal: 1,
        max_ordinal: TASK_PROTOCOL_CONFORMANCE_MAX_ORDINAL,
        exchange_timeout_ms: TASK_PROTOCOL_CONFORMANCE_EXCHANGE_TIMEOUT_MS,
        max_request_bytes: TASK_PROTOCOL_CONFORMANCE_MAX_REQUEST_BYTES,
        max_upstream_request_bytes: TASK_PROTOCOL_CONFORMANCE_MAX_UPSTREAM_REQUEST_BYTES,
        max_response_bytes: TASK_PROTOCOL_CONFORMANCE_MAX_RESPONSE_BYTES,
        max_observation_bytes: TASK_PROTOCOL_CONFORMANCE_MAX_OBSERVATION_BYTES,
        framing_policy: "exact_length_no_delimiter_eof_chunked_or_streaming_v1".into(),
        reserved_policy: "all_reserved_bits_and_bytes_zero_v1".into(),
        authenticated_ack_policy: "receipt_for_every_exchange_aggregated_capability_v1".into(),
        cleanup_policy:
            "authenticated_shutdown_pidfd_reap_cgroup_leaf_and_scratch_cleanup_required_v1".into(),
        authority_status: TASK_PROTOCOL_CONFORMANCE_NON_PRODUCTION_AUTHORITY.into(),
        effects: task_protocol_conformance_no_effects(),
        readiness: task_protocol_conformance_no_readiness(),
    }
}

pub(super) fn task_protocol_conformance_fixture_catalog_for_validation(
) -> ExternalPoolAdapterTaskProtocolConformanceFixtureCatalog {
    ExternalPoolAdapterTaskProtocolConformanceFixtureCatalog {
        schema: TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_SCHEMA.into(),
        catalog_id: TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_ID.into(),
        catalog_revision: TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_REVISION,
        scenario_ids: vec!["synthetic_command_a".into(), "synthetic_command_b".into()],
        exchanges: vec![
            exchange(
                1,
                "synthetic_command_a",
                "prepare",
                "prepare",
                "fresh",
                &["absent"],
                &["prepared"],
                "nonterminal",
                true,
                Some(1),
                false,
                &[],
                0,
                0,
            ),
            exchange(
                2,
                "synthetic_command_a",
                "idempotent_commit",
                "idempotent_commit",
                "fresh",
                &["prepared"],
                &["committed"],
                "nonterminal",
                true,
                Some(2),
                false,
                &[],
                1,
                0,
            ),
            exchange(
                3,
                "synthetic_command_a",
                "idempotent_commit",
                "idempotent_commit",
                "same_idempotency_exact_replay",
                &["committed"],
                &["committed"],
                "nonterminal",
                true,
                Some(2),
                false,
                &[],
                1,
                0,
            ),
            exchange(
                4,
                "synthetic_command_a",
                "reconcile",
                "reconcile",
                "fresh",
                &["committed"],
                &["running"],
                "nonterminal",
                true,
                Some(2),
                false,
                &[],
                1,
                0,
            ),
            exchange(
                5,
                "synthetic_command_a",
                "authenticated_events",
                "authenticated_events",
                "fresh",
                &["running"],
                &["terminal_after_run"],
                "final",
                true,
                Some(2),
                false,
                &["started", "terminal"],
                1,
                2,
            ),
            exchange(
                6,
                "synthetic_command_b",
                "prepare",
                "prepare",
                "fresh",
                &["absent"],
                &["prepared"],
                "nonterminal",
                true,
                Some(1),
                false,
                &[],
                0,
                0,
            ),
            exchange(
                7,
                "synthetic_command_b",
                "cancel_no_start",
                "cancel_no_start",
                "fresh",
                &["prepared"],
                &["prepared"],
                "nonterminal",
                true,
                Some(1),
                false,
                &[],
                0,
                0,
            ),
            exchange(
                8,
                "synthetic_command_b",
                "reconcile",
                "reconcile",
                "fresh",
                &["prepared"],
                &["terminal_no_start"],
                "final",
                true,
                Some(2),
                true,
                &[],
                0,
                0,
            ),
        ],
        capability_order: TASK_PROTOCOL_CONFORMANCE_CAPABILITY_IDS
            .into_iter()
            .map(str::to_owned)
            .collect(),
        capability_exchange_ordinals: vec![
            (1..=8).collect(),
            vec![5],
            vec![7, 8],
            vec![2, 3],
            vec![1, 6],
            vec![4, 8],
        ],
        capability_evidence_policy: "server_run_observation_required".into(),
        response_policy: "expected_accepted_response".into(),
        authority_status: TASK_PROTOCOL_CONFORMANCE_NON_PRODUCTION_AUTHORITY.into(),
    }
}

pub(crate) fn task_protocol_conformance_no_effects(
) -> ExternalPoolAdapterTaskProtocolConformanceEffects {
    ExternalPoolAdapterTaskProtocolConformanceEffects {
        credential_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
        adapter_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
        provider_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
        route_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
        activation_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
        execution_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
        usage_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
        market_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
        settlement_effect: TASK_PROTOCOL_CONFORMANCE_NO_EFFECT.into(),
    }
}

pub(crate) fn task_protocol_conformance_no_readiness(
) -> ExternalPoolAdapterTaskProtocolConformanceReadiness {
    ExternalPoolAdapterTaskProtocolConformanceReadiness {
        process_spawn_ready: false,
        ipc_session_ready: false,
        secret_delivery_ready: false,
        broker_connect_ready: false,
        upstream_probe_ready: false,
        runtime_launch_ready: false,
        route_ready: false,
        execution_ready: false,
        activation_ready: false,
    }
}

fn required_capabilities() -> Vec<ComputeExternalPoolAdapterReleaseCapability> {
    TASK_PROTOCOL_CONFORMANCE_CAPABILITY_IDS
        .into_iter()
        .map(
            |capability_id| ComputeExternalPoolAdapterReleaseCapability {
                capability_id: capability_id.into(),
                capability_revision: 1,
            },
        )
        .collect()
}

fn code_point((name, code): (&str, u64)) -> ExternalPoolAdapterTaskProtocolConformanceCodePoint {
    ExternalPoolAdapterTaskProtocolConformanceCodePoint {
        name: name.into(),
        code,
    }
}

#[allow(clippy::too_many_arguments)]
fn exchange(
    exchange_ordinal: u64,
    scenario_id: &str,
    operation_kind: &str,
    capability_id: &str,
    replay_kind: &str,
    allowed_state_before: &[&str],
    allowed_state_after: &[&str],
    terminality: &str,
    reference_required: bool,
    remote_sequence: Option<u64>,
    tombstone_required: bool,
    event_kinds: &[&str],
    expected_start_count: u64,
    expected_event_count: u64,
) -> ExternalPoolAdapterTaskProtocolConformanceFixtureExchange {
    let (uncertainty_before, uncertainty_after, uncertainty_marker_required) =
        match exchange_ordinal {
            1 | 2 => ("clear", "clear", false),
            3 => ("clear", "unknown_after_remote_acceptance", true),
            4 => (
                "unknown_after_remote_acceptance",
                "resolved_by_reconcile",
                true,
            ),
            5 => ("resolved_by_reconcile", "resolved_by_reconcile", false),
            6..=8 => ("not_applicable", "not_applicable", false),
            _ => ("not_applicable", "not_applicable", false),
        };
    let (event_replay_classification, expected_event_replay_batch_count, replay_root_required) =
        if exchange_ordinal == 5 {
            (Some("exact_duplicate_batch_replay".into()), 1, true)
        } else {
            (None, 0, false)
        };
    ExternalPoolAdapterTaskProtocolConformanceFixtureExchange {
        exchange_ordinal,
        scenario_id: scenario_id.into(),
        operation_kind: operation_kind.into(),
        capability_id: capability_id.into(),
        replay_kind: replay_kind.into(),
        allowed_state_before: allowed_state_before
            .iter()
            .map(|value| (*value).into())
            .collect(),
        allowed_state_after: allowed_state_after
            .iter()
            .map(|value| (*value).into())
            .collect(),
        terminality: terminality.into(),
        reference_required,
        remote_sequence,
        tombstone_required,
        event_kinds: event_kinds.iter().map(|value| (*value).into()).collect(),
        commit_uncertainty_state_before: uncertainty_before.into(),
        commit_uncertainty_state_after: uncertainty_after.into(),
        commit_uncertainty_marker_required: uncertainty_marker_required,
        event_replay_classification,
        expected_event_replay_batch_count,
        event_replay_root_required: replay_root_required,
        expected_start_count,
        expected_event_count,
    }
}
