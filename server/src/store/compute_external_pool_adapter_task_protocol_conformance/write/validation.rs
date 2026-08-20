use anyhow::{bail, Result};

use crate::compute_federation::{
    external_pool_adapter_installation::ExternalPoolAdapterInstallationFsError,
    external_pool_adapter_task_protocol_conformance::TASK_PROTOCOL_CONFORMANCE_CONFIRMATION,
};

use super::super::{
    error::ExternalPoolAdapterTaskProtocolConformanceStoreError as StoreError,
    read::identifier,
    types::{
        CreateExternalPoolAdapterTaskProtocolConformanceRun, StoredTaskProtocolConformanceRun,
    },
};

pub(in super::super) fn validate_create_input(
    input: &CreateExternalPoolAdapterTaskProtocolConformanceRun,
) -> Result<()> {
    for value in [
        &input.registry_release_id,
        &input.sandbox_reattestation_receipt_id,
        &input.runtime_compatibility_verification_receipt_id,
        &input.provider_binding_id,
        &input.expected_installation_receipt_id,
        &input.recorded_by_admin_user_id,
        &input.idempotency_key,
    ] {
        identifier(value)?;
    }
    for value in [
        &input.expected_registry_release_digest,
        &input.expected_sandbox_reattestation_receipt_digest,
        &input.expected_runtime_compatibility_verification_receipt_digest,
        &input.expected_task_protocol_profile_digest,
        &input.expected_fixture_catalog_digest,
        &input.expected_provider_binding_digest,
        &input.expected_installation_receipt_digest,
    ] {
        exact_digest(value)?;
    }
    if input.predecessor_run_receipt_id.is_some()
        != input.expected_predecessor_run_receipt_digest.is_some()
    {
        bail!("task-protocol conformance predecessor pair is incomplete")
    }
    if let Some(value) = &input.predecessor_run_receipt_id {
        identifier(value)?;
    }
    if let Some(value) = &input.expected_predecessor_run_receipt_digest {
        exact_digest(value)?;
    }
    let expected_scope = format!(
        "v272:task-protocol-conformance:create:{}",
        input.recorded_by_admin_user_id
    );
    if input.idempotency_scope != expected_scope
        || input.confirmation != TASK_PROTOCOL_CONFORMANCE_CONFIRMATION
    {
        bail!("task-protocol conformance actor-bound request metadata is invalid")
    }
    Ok(())
}

pub(in super::super) fn ensure_predecessor(
    input: &CreateExternalPoolAdapterTaskProtocolConformanceRun,
    head: Option<&StoredTaskProtocolConformanceRun>,
) -> Result<()> {
    match (
        head,
        input.predecessor_run_receipt_id.as_deref(),
        input.expected_predecessor_run_receipt_digest.as_deref(),
    ) {
        (None, None, None) => Ok(()),
        (Some(head), Some(id), Some(digest))
            if head.receipt.run_receipt_id == id && head.receipt.run_receipt_digest == digest =>
        {
            Ok(())
        }
        _ => bail!("task-protocol conformance predecessor is missing, stale, or unexpected"),
    }
}

pub(in super::super) fn classify_installation_error(
    error: ExternalPoolAdapterInstallationFsError,
) -> StoreError {
    if matches!(&error, ExternalPoolAdapterInstallationFsError::Storage(_)) {
        StoreError::storage(anyhow::Error::new(error))
    } else {
        StoreError::conflict(anyhow::Error::new(error))
    }
}

pub(in super::super) fn classify_execution_error(error: anyhow::Error) -> StoreError {
    if error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<ExternalPoolAdapterInstallationFsError>(),
            Some(ExternalPoolAdapterInstallationFsError::Storage(_))
        )
    }) || error
        .chain()
        .any(|cause| cause.downcast_ref::<std::io::Error>().is_some())
    {
        StoreError::storage(error)
    } else {
        StoreError::conflict(error)
    }
}

fn exact_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("task-protocol conformance expected digest is invalid")
    }
    Ok(())
}
