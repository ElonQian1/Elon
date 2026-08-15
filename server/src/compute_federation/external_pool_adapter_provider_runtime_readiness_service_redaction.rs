//! Recursive public-response redaction for V270 Provider runtime-readiness APIs.

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

pub(super) fn redacted_json<T: Serialize>(value: T) -> Result<Value> {
    let mut value = serde_json::to_value(value)?;
    redact(&mut value);
    Ok(value)
}

fn redact(value: &mut Value) {
    match value {
        Value::Object(map) => {
            redacted_keys().iter().for_each(|key| {
                map.remove(*key);
            });
            map.values_mut().for_each(redact);
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        _ => {}
    }
}

fn redacted_keys() -> &'static [&'static str] {
    &[
        "sealed_bindings",
        "runtime_custody_epoch",
        "runtime_custody_epoch_digest",
        "runtime_bundle_identity_commitment",
        "post_cleanup_observation_commitment",
        "probe_execution_id",
        "request_digest",
        "response_digest",
        "request_bytes",
        "response_bytes",
        "selected_address",
        "selected_ip_address",
        "dns_hostname",
        "port",
        "tls_server_name",
        "expected_tls_leaf_spki_sha256",
        "tls_leaf_spki_sha256",
        "credential_ref",
        "credential_locator",
        "credential_locator_commitment",
        "credential_bytes",
        "credential_sha256",
        "config_locator",
        "config_bytes",
        "config_sha256",
        "bundle_generation",
        "runtime_bundle_generation",
        "runtime_bundle_identity",
        "installation_path",
        "installation_root",
        "bundle_root",
        "storage_namespace",
        "entrypoint_path",
        "entrypoint_relative_path",
        "filesystem_path",
        "source_path",
        "archive_path",
        "session_key",
        "directional_key",
        "derived_key",
        "hmac_key",
        "host_nonce",
        "child_nonce",
        "transcript_digest",
        "pid",
        "pidfd",
        "fd",
        "cgroup_path",
        "cgroup_parent",
        "scratch_path",
        "runtime_locator",
        "provider_owner_account_id",
        "recorded_by_actor_kind",
        "recorded_by_actor_user_id",
        "revoked_by_actor_kind",
        "revoked_by_actor_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "receipt_json",
    ]
}
