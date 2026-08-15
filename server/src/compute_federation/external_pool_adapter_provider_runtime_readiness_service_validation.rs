//! HTTP-path and request guards for V270 Provider runtime-readiness services.

use anyhow::{bail, Result};

use super::external_pool_adapter_provider_runtime_readiness::{
    validate_create_provider_runtime_readiness_receipt_body,
    validate_revoke_provider_runtime_readiness_receipt_body,
    CreateProviderRuntimeReadinessReceiptBody, RevokeProviderRuntimeReadinessReceiptBody,
};

pub(super) fn validate_create(
    path: [&str; 5],
    body: &CreateProviderRuntimeReadinessReceiptBody,
) -> Result<()> {
    validate_path(path, None)?;
    validate_create_provider_runtime_readiness_receipt_body(body)
}

pub(super) fn validate_currentness(path: [&str; 5], readiness_receipt_id: &str) -> Result<()> {
    validate_path(path, Some(readiness_receipt_id))
}

pub(super) fn validate_revoke(
    path: [&str; 5],
    readiness_receipt_id: &str,
    body: &RevokeProviderRuntimeReadinessReceiptBody,
) -> Result<()> {
    validate_path(path, Some(readiness_receipt_id))?;
    validate_revoke_provider_runtime_readiness_receipt_body(body)
}

pub(super) fn validate_path(path: [&str; 5], readiness_receipt_id: Option<&str>) -> Result<()> {
    for (value, label) in path.into_iter().zip([
        "Provider binding ID",
        "activation candidate ID",
        "runtime launch-profile ID",
        "upstream transport-target ID",
        "supervisor/session policy-companion ID",
    ]) {
        validate_identifier(value, label)?;
    }
    if let Some(value) = readiness_receipt_id {
        validate_identifier(value, "Provider runtime-readiness receipt ID")?;
    }
    Ok(())
}

pub(super) fn require_exact(actual: &str, expected: &str, authority: &str) -> Result<()> {
    if actual != expected {
        bail!("{authority} does not belong to the requested Provider runtime-readiness path")
    }
    Ok(())
}

pub(super) fn idempotency_scope(operation: &str, actor_kind: &str, actor_user_id: &str) -> String {
    if operation == "create" {
        format!("v270:provider-runtime-readiness:create:{actor_user_id}")
    } else {
        format!("v270:provider-runtime-readiness:{operation}:{actor_kind}:{actor_user_id}")
    }
}

fn validate_identifier(value: &str, label: &'static str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid")
    }
    Ok(())
}
