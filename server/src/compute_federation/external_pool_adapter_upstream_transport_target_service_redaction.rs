//! Recursive public-response redaction for V258 upstream transport-target APIs.

use anyhow::Error as AnyError;
use serde::Serialize;
use serde_json::Value;

use super::external_pool_adapter_upstream_transport_target_service::UpstreamTransportTargetServiceError;

pub(super) fn redacted_json<T: Serialize>(
    value: T,
) -> Result<Value, UpstreamTransportTargetServiceError> {
    let mut value = serde_json::to_value(value)
        .map_err(|error| UpstreamTransportTargetServiceError::Conflict(AnyError::new(error)))?;
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
        "config_locator",
        "installation_path",
        "installation_root",
        "entrypoint_path",
        "entrypoint_relative_path",
        "filesystem_path",
        "receipt_json",
    ]
}
