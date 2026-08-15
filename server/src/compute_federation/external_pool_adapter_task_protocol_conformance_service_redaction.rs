//! Recursive public-response redaction for V272 task-protocol conformance APIs.

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
            for key in redacted_keys() {
                map.remove(*key);
            }
            map.values_mut().for_each(redact);
        }
        Value::Array(values) => values.iter_mut().for_each(redact),
        _ => {}
    }
}

fn redacted_keys() -> &'static [&'static str] {
    &[
        "runtime_custody_epoch",
        "runtime_custody_epoch_digest",
        "process_hmac_seal",
        "receipt_integrity_digest",
        "committed_seal",
        "provider_binding_id",
        "provider_binding_digest",
        "installation_receipt_id",
        "installation_receipt_digest",
        "recorded_by_admin_user_id",
        "revoked_by_admin_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "receipt_json",
        "raw_transcript",
        "raw_request",
        "raw_response",
        "raw_observation",
        "config_bytes",
        "credential_bytes",
        "secret_bytes",
        "session_key",
        "directional_key",
        "derived_key",
        "hmac_key",
        "host_nonce",
        "child_nonce",
        "selected_address",
        "selected_ip_address",
        "dns_hostname",
        "tls_server_name",
        "tls_leaf_spki_sha256",
        "credential_locator",
        "installation_path",
        "installation_root",
        "filesystem_path",
        "source_path",
        "archive_path",
        "pid",
        "pidfd",
        "fd",
        "cgroup_path",
        "cgroup_parent",
        "scratch_path",
    ]
}
