use anyhow::Result;
use rusqlite::Connection;

use super::install_projection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    install_projection(
        conn,
        "v273_task_exchange_attempt_projection",
        "compute_external_pool_adapter_task_exchange_attempts",
        "exchange_attempt_json",
        &attempt_fields(),
    )?;
    install_projection(
        conn,
        "v273_task_exchange_receipt_projection",
        "compute_external_pool_adapter_task_exchange_receipts",
        "exchange_receipt_json",
        &receipt_fields(),
    )
}

fn attempt_fields() -> Vec<(&'static str, &'static str)> {
    let mut fields = vec![
        ("exchange_attempt_schema", "$.schema"),
        ("exchange_attempt_id", "$.exchange_attempt_id"),
        ("exchange_attempt_digest", "$.exchange_attempt_digest"),
        ("canonicalization", "$.canonicalization"),
        ("digest_algorithm", "$.digest_algorithm"),
    ];
    fields.extend(identity_fields("$.attempt.identity"));
    fields.extend([
        ("started_at", "$.attempt.started_at"),
        ("authority_status", "$.attempt.boundary.authority_status"),
        ("effects_json", "$.attempt.boundary.effects"),
        ("readiness_json", "$.attempt.boundary.readiness"),
    ]);
    fields
}

fn receipt_fields() -> Vec<(&'static str, &'static str)> {
    let mut fields = vec![
        ("exchange_receipt_schema", "$.schema"),
        ("exchange_receipt_id", "$.exchange_receipt_id"),
        ("exchange_receipt_digest", "$.exchange_receipt_digest"),
        ("canonicalization", "$.canonicalization"),
        ("digest_algorithm", "$.digest_algorithm"),
        ("exchange_attempt_id", "$.receipt.exchange_attempt_id"),
        (
            "exchange_attempt_digest",
            "$.receipt.exchange_attempt_digest",
        ),
    ];
    fields.extend(identity_fields("$.receipt.identity"));
    fields.extend([
        ("exchange_ordinal", "$.receipt.exchange_ordinal"),
        ("exchange_nonce_digest", "$.receipt.exchange_nonce_digest"),
        ("upstream_request_bytes", "$.receipt.upstream_request_bytes"),
        (
            "upstream_request_sha256",
            "$.receipt.upstream_request_sha256",
        ),
        (
            "upstream_response_bytes",
            "$.receipt.upstream_response_bytes",
        ),
        (
            "upstream_response_sha256",
            "$.receipt.upstream_response_sha256",
        ),
        (
            "semantic_observation_bytes",
            "$.receipt.semantic_observation_bytes",
        ),
        (
            "semantic_observation_sha256",
            "$.receipt.semantic_observation_sha256",
        ),
        (
            "session_transcript_digest",
            "$.receipt.session_transcript_digest",
        ),
        ("exchange_root", "$.receipt.exchange_root"),
        ("authenticated_at", "$.receipt.authenticated_at"),
        ("received_at", "$.receipt.received_at"),
        ("recorded_at", "$.receipt.recorded_at"),
        ("authority_status", "$.receipt.boundary.authority_status"),
        ("effects_json", "$.receipt.boundary.effects"),
        ("readiness_json", "$.receipt.boundary.readiness"),
    ]);
    fields
}

fn identity_fields(prefix: &'static str) -> Vec<(&'static str, &'static str)> {
    // Prefixes are two frozen literals; keep paths explicit so source review can audit the ABI.
    if prefix == "$.attempt.identity" {
        attempt_identity_fields()
    } else {
        receipt_identity_fields()
    }
}

macro_rules! identity_fields {
    ($prefix:literal) => {
        vec![
            ("operation_kind", concat!($prefix, ".operation_kind")),
            ("source_kind", concat!($prefix, ".source.source_kind")),
            ("source_id", concat!($prefix, ".source.source_id")),
            ("source_digest", concat!($prefix, ".source.source_digest")),
            ("provider_id", concat!($prefix, ".adapter.provider_id")),
            ("adapter_id", concat!($prefix, ".adapter.adapter_id")),
            (
                "adapter_revision",
                concat!($prefix, ".adapter.adapter_revision"),
            ),
            (
                "adapter_registry_digest",
                concat!($prefix, ".adapter.adapter_registry_digest"),
            ),
            (
                "adapter_implementation_digest",
                concat!($prefix, ".adapter.adapter_implementation_digest"),
            ),
            ("command_id", concat!($prefix, ".command.command_id")),
            (
                "command_digest",
                concat!($prefix, ".command.command_digest"),
            ),
            ("outbox_id", concat!($prefix, ".command.outbox_id")),
            ("outbox_digest", concat!($prefix, ".command.outbox_digest")),
            (
                "send_attempt_id",
                concat!($prefix, ".command.send_attempt_id"),
            ),
            (
                "send_attempt_digest",
                concat!($prefix, ".command.send_attempt_digest"),
            ),
            (
                "route_authorization_id",
                concat!($prefix, ".route.route_authorization_id"),
            ),
            (
                "route_authorization_digest",
                concat!($prefix, ".route.route_authorization_digest"),
            ),
            (
                "route_credential_id",
                concat!($prefix, ".route.route_credential_id"),
            ),
            (
                "route_credential_revision",
                concat!($prefix, ".route.route_credential_revision"),
            ),
            (
                "route_credential_digest",
                concat!($prefix, ".route.route_credential_digest"),
            ),
            (
                "credential_verification_receipt_id",
                concat!($prefix, ".route.credential_verification_receipt_id"),
            ),
            (
                "credential_verification_receipt_digest",
                concat!($prefix, ".route.credential_verification_receipt_digest"),
            ),
            (
                "credential_verifier_id",
                concat!($prefix, ".route.credential_verifier_id"),
            ),
            (
                "credential_verifier_revision",
                concat!($prefix, ".route.credential_verifier_revision"),
            ),
            (
                "credential_verifier_digest",
                concat!($prefix, ".route.credential_verifier_digest"),
            ),
            (
                "executor_binding_digest",
                concat!($prefix, ".executor_binding_digest"),
            ),
            (
                "fencing_generation",
                concat!($prefix, ".fencing_generation"),
            ),
            ("fence_digest", concat!($prefix, ".fence_digest")),
            (
                "supervisor_session_policy_digest",
                concat!($prefix, ".session.roots.supervisor_session_policy_digest"),
            ),
            (
                "runtime_launch_profile_digest",
                concat!($prefix, ".session.roots.runtime_launch_profile_digest"),
            ),
            (
                "task_protocol_profile_digest",
                concat!($prefix, ".session.roots.task_protocol_profile_digest"),
            ),
            (
                "upstream_transport_target_digest",
                concat!($prefix, ".session.roots.upstream_transport_target_digest"),
            ),
            (
                "supervisor_session_policy_companion_digest",
                concat!(
                    $prefix,
                    ".session.roots.supervisor_session_policy_companion_digest"
                ),
            ),
            (
                "launch_image_sha256",
                concat!($prefix, ".session.roots.launch_image_sha256"),
            ),
            (
                "ephemeral_task_secret_delivery_root",
                concat!(
                    $prefix,
                    ".session.roots.ephemeral_task_secret_delivery_root"
                ),
            ),
            (
                "task_protocol_conformance_run_receipt_digest",
                concat!(
                    $prefix,
                    ".session.roots.task_protocol_conformance_run_receipt_digest"
                ),
            ),
            (
                "session_roots_digest",
                concat!($prefix, ".session.session_roots_digest"),
            ),
            (
                "session_transcript_digest",
                concat!($prefix, ".session.session_transcript_digest"),
            ),
            (
                "upstream_transport_target_id",
                concat!($prefix, ".session.upstream_transport_target_id"),
            ),
            (
                "task_protocol_conformance_run_receipt_id",
                concat!($prefix, ".session.task_protocol_conformance_run_receipt_id"),
            ),
            ("request_digest", concat!($prefix, ".request_digest")),
            (
                "delivery_attempt_digest",
                concat!($prefix, ".delivery_attempt_digest"),
            ),
        ]
    };
}

fn attempt_identity_fields() -> Vec<(&'static str, &'static str)> {
    identity_fields!("$.attempt.identity")
}

fn receipt_identity_fields() -> Vec<(&'static str, &'static str)> {
    identity_fields!("$.receipt.identity")
}
