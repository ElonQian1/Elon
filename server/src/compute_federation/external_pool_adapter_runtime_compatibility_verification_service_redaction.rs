//! Recursive public projection redaction for V268 runtime compatibility evidence.

use anyhow::Error as AnyError;
use serde::Serialize;
use serde_json::Value;

use super::external_pool_adapter_runtime_compatibility_verification_service::RuntimeCompatibilityVerificationServiceError;

pub(super) fn redacted_json<T: Serialize>(
    value: T,
) -> Result<Value, RuntimeCompatibilityVerificationServiceError> {
    let mut value = serde_json::to_value(value).map_err(|error| {
        RuntimeCompatibilityVerificationServiceError::Internal(AnyError::new(error))
    })?;
    redact(&mut value);
    Ok(value)
}

fn redact(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|key, _| !redacted_key(key));
            map.values_mut().for_each(redact);
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        _ => {}
    }
}

fn redacted_key(key: &str) -> bool {
    const EXACT: &[&str] = &[
        // Challenge and signed-verifier material is never a public response projection.
        "challenge_nonce_base64",
        "challenge_nonce_digest",
        "nonce_base64",
        "nonce_digest",
        "signature_message",
        "signature_message_base64",
        "signature_message_digest",
        "signature_base64",
        "signature_digest",
        "public_key_pem",
        // Preserve run_observation_id/digest, but never the full durable observation.
        "observation",
        "observations",
        "server_run_observation",
        "run_observation",
        "observation_receipt",
        "run_observation_json",
        "observation_material",
        "no_work",
        "registry_release",
        "fixture_resources",
        // Source/derived-launch identities and process internals stay Store-private.
        "source_capsule_sha256",
        "source_capsule_digest",
        "source_capsule_size_bytes",
        "entrypoint_path",
        "entrypoint_relative_path",
        "entrypoint_sha256",
        "entrypoint_size_bytes",
        "launch_image_sha256",
        "launch_image_digest",
        "launch_image_size_bytes",
        "derived_launch_sha256",
        "derived_launch_digest",
        "runner_internal",
        "runner_state",
        "runner_receipt",
        "runner_execution_id",
        "sandbox_verifier_operator",
        "sandbox_verifier_product",
        "public_fixture_delivery_root",
        "stdout",
        "stderr",
        "exit_status",
        "process_id",
        "pid",
        "pidfd",
        "child_pid",
        "child_fd",
        "cgroup_path",
        "scratch_path",
        "cleanup_receipt",
        // Authentication, replay, confirmation, and raw persistence material.
        "actor",
        "actor_kind",
        "actor_user_id",
        "admin_user_id",
        "created_by_admin_user_id",
        "verified_by_admin_user_id",
        "revoked_by_admin_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "confirm_challenge",
        "confirm_verification",
        "confirm_revocation",
        "receipt_json",
        "challenge_json",
        "observation_json",
        "verification_json",
        "revocation_json",
        "material_json",
        "raw_json",
        // Production Secret and endpoint roots are outside this release-neutral authority.
        "config_bytes",
        "config_digest",
        "config_locator",
        "config_root",
        "credential_bytes",
        "credential_digest",
        "credential_material_digest",
        "credential_ref",
        "credential_locator",
        "credential_locator_commitment",
        "credential_root",
        "secret_bytes",
        "secret_digest",
        "secret_locator",
        "secret_root",
        "secret_delivery_policy_digest",
        "resolver_backend_policy_digest",
        "delivery_root",
        "session_root",
        "session_key",
        "endpoint",
        "endpoint_root",
        "target_id",
        "target_digest",
        "target_root",
        "dns_hostname",
        "hostname",
        "port",
        "tls_server_name",
        "expected_tls_leaf_spki_sha256",
        "spki_sha256",
        "socket_address",
        "selected_address",
        "resolved_addresses",
        "upstream_address",
        "upstream_endpoint",
        "upstream_target",
        "installation_path",
        "installation_root",
        "install_root",
        "installed_path",
        "filesystem_path",
        "source_path",
        "archive_path",
    ];

    EXACT.contains(&key)
        || key.ends_with("_json")
        || key.ends_with("_admin_user_id")
        || key.ends_with("_actor_user_id")
        || key.ends_with("_actor_kind")
        || key.starts_with("confirm_")
        || key.contains("nonce")
        || key.starts_with("raw_")
}
