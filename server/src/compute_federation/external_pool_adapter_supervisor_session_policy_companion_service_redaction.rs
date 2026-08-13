//! Recursive public-response redaction for V259 supervisor/session policy-companion APIs.

use anyhow::Error as AnyError;
use serde::Serialize;
use serde_json::Value;

use super::external_pool_adapter_supervisor_session_policy_companion_service::SupervisorSessionPolicyCompanionServiceError;

pub(super) fn redacted_json<T: Serialize>(
    value: T,
) -> Result<Value, SupervisorSessionPolicyCompanionServiceError> {
    let mut value = serde_json::to_value(value).map_err(|error| {
        SupervisorSessionPolicyCompanionServiceError::Conflict(AnyError::new(error))
    })?;
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
        "dns_hostname",
        "port",
        "tls_server_name",
        "expected_tls_leaf_spki_sha256",
        "provider_owner_account_id",
        "recorded_by_actor_kind",
        "recorded_by_actor_user_id",
        "revoked_by_actor_kind",
        "revoked_by_actor_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "route_adapter_projection_id",
        "service_actor_id",
        "credential_ref",
        "credential_locator",
        "credential_locator_commitment",
        "credential_bytes",
        "credential_sha256",
        "config_locator",
        "config_bytes",
        "config_sha256",
        "installation_path",
        "installation_root",
        "storage_namespace",
        "entrypoint_path",
        "entrypoint_relative_path",
        "filesystem_path",
        "source_path",
        "archive_path",
        "session_key",
        "directional_key",
        "derived_key",
        "host_nonce",
        "child_nonce",
        "transcript_digest",
        "pid",
        "pidfd",
        "cgroup_path",
        "runtime_locator",
        "receipt_json",
    ]
}
