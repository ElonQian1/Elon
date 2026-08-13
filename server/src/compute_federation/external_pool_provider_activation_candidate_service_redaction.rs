//! Recursive public-response redaction for V254 candidate APIs.

use anyhow::Error as AnyError;
use serde::Serialize;
use serde_json::Value;

use super::external_pool_provider_activation_candidate_service::ActivationCandidateServiceError;

pub(super) fn redacted_json<T: Serialize>(
    value: T,
) -> Result<Value, ActivationCandidateServiceError> {
    let mut value = serde_json::to_value(value)
        .map_err(|error| ActivationCandidateServiceError::Conflict(AnyError::new(error)))?;
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
        "service_actor_id",
        "route_adapter_projection_id",
        "provider_owner_account_id",
        "issued_by_owner_user_id",
        "revoked_by_owner_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "credential_ref",
        "non_bearer_credential_ref",
        "credential_locator_commitment",
        "installation_path",
        "installation_root",
        "entrypoint_path",
        "filesystem_path",
        "source_path",
        "archive_path",
        "receipt_json",
    ]
}
