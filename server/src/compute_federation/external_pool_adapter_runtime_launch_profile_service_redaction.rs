//! Recursive public-response redaction for V255 runtime launch-profile APIs.

use anyhow::Error as AnyError;
use serde::Serialize;
use serde_json::Value;

use super::external_pool_adapter_runtime_launch_profile_service::RuntimeLaunchProfileServiceError;

pub(super) fn redacted_json<T: Serialize>(
    value: T,
) -> Result<Value, RuntimeLaunchProfileServiceError> {
    let mut value = serde_json::to_value(value)
        .map_err(|error| RuntimeLaunchProfileServiceError::Conflict(AnyError::new(error)))?;
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
        "provider_owner_account_id",
        "service_actor_id",
        "route_adapter_projection_id",
        "recorded_by_actor_kind",
        "recorded_by_actor_user_id",
        "revoked_by_actor_kind",
        "revoked_by_actor_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "credential_ref",
        "non_bearer_credential_ref",
        "credential_locator",
        "credential_locator_commitment",
        "config_locator",
        "resolver_backend_policy_digest",
        "resolver_backend_root",
        "installation_path",
        "installation_root",
        "storage_namespace",
        "entrypoint_path",
        "entrypoint_relative_path",
        "filesystem_path",
        "source_path",
        "archive_path",
        "receipt_json",
    ]
}
