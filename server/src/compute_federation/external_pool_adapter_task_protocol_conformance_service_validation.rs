//! Strict caller-input validation for V272 task-protocol conformance routes.

use anyhow::{bail, Result};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedTaskProtocolConformancePredecessor {
    pub run_receipt_id: String,
    pub run_receipt_digest: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateTaskProtocolConformanceRunBody {
    pub expected_registry_release_digest: String,
    pub provider_binding_id: String,
    pub expected_provider_binding_digest: String,
    pub expected_installation_receipt_id: String,
    pub expected_installation_receipt_digest: String,
    pub sandbox_reattestation_receipt_id: String,
    pub expected_sandbox_reattestation_receipt_digest: String,
    pub runtime_compatibility_verification_receipt_id: String,
    pub expected_runtime_compatibility_verification_receipt_digest: String,
    pub expected_task_protocol_profile_digest: String,
    pub expected_fixture_catalog_digest: String,
    pub expected_predecessor: Option<ExpectedTaskProtocolConformancePredecessor>,
    pub idempotency_key: String,
    pub confirm_task_protocol_conformance_run: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeTaskProtocolConformanceRunBody {
    pub expected_run_receipt_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}

pub(super) fn validate_create(
    admin_user_id: &str,
    registry_release_id: &str,
    body: &CreateTaskProtocolConformanceRunBody,
) -> Result<()> {
    if !body.confirm_task_protocol_conformance_run {
        bail!("task-protocol conformance run requires explicit confirmation")
    }
    validate_common(admin_user_id, registry_release_id, &body.idempotency_key)?;
    for (value, label) in [
        (&body.provider_binding_id, "Provider binding ID"),
        (
            &body.expected_installation_receipt_id,
            "installation receipt ID",
        ),
        (
            &body.sandbox_reattestation_receipt_id,
            "sandbox reattestation receipt ID",
        ),
        (
            &body.runtime_compatibility_verification_receipt_id,
            "runtime compatibility verification receipt ID",
        ),
    ] {
        validate_identifier(value, label)?;
    }
    for (value, label) in [
        (
            &body.expected_registry_release_digest,
            "registry release digest",
        ),
        (
            &body.expected_provider_binding_digest,
            "Provider binding digest",
        ),
        (
            &body.expected_installation_receipt_digest,
            "installation receipt digest",
        ),
        (
            &body.expected_sandbox_reattestation_receipt_digest,
            "sandbox reattestation receipt digest",
        ),
        (
            &body.expected_runtime_compatibility_verification_receipt_digest,
            "runtime compatibility verification receipt digest",
        ),
        (
            &body.expected_task_protocol_profile_digest,
            "task-protocol conformance profile digest",
        ),
        (
            &body.expected_fixture_catalog_digest,
            "task-protocol conformance fixture catalog digest",
        ),
    ] {
        validate_digest(value, label)?;
    }
    if let Some(predecessor) = &body.expected_predecessor {
        validate_identifier(&predecessor.run_receipt_id, "predecessor run receipt ID")?;
        validate_digest(
            &predecessor.run_receipt_digest,
            "predecessor run receipt digest",
        )?;
    }
    Ok(())
}

pub(super) fn validate_currentness(admin_user_id: &str, registry_release_id: &str) -> Result<()> {
    validate_identifier(admin_user_id, "administrator user ID")?;
    validate_identifier(registry_release_id, "registry release ID")
}

pub(super) fn validate_revoke(
    admin_user_id: &str,
    registry_release_id: &str,
    run_receipt_id: &str,
    body: &RevokeTaskProtocolConformanceRunBody,
) -> Result<()> {
    if !body.confirm_revocation {
        bail!("task-protocol conformance revocation requires explicit confirmation")
    }
    validate_common(admin_user_id, registry_release_id, &body.idempotency_key)?;
    validate_identifier(run_receipt_id, "task-protocol conformance run receipt ID")?;
    validate_digest(
        &body.expected_run_receipt_digest,
        "task-protocol conformance run receipt digest",
    )?;
    if body.reason.trim() != body.reason
        || !(12..=500).contains(&body.reason.chars().count())
        || body.reason.chars().any(char::is_control)
    {
        bail!("task-protocol conformance revocation reason is invalid")
    }
    Ok(())
}

pub(super) fn idempotency_scope(operation: &str, admin_user_id: &str) -> String {
    format!("v272:task-protocol-conformance:{operation}:{admin_user_id}")
}

fn validate_common(admin_user_id: &str, registry_release_id: &str, key: &str) -> Result<()> {
    validate_identifier(admin_user_id, "administrator user ID")?;
    validate_identifier(registry_release_id, "registry release ID")?;
    validate_identifier(key, "idempotency key")
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

fn validate_digest(value: &str, label: &'static str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} is invalid")
    }
    Ok(())
}
