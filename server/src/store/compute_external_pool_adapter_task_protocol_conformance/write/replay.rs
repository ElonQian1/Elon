use anyhow::{bail, Result};

use crate::compute_federation::external_pool_adapter_task_protocol_conformance::ExternalPoolAdapterTaskProtocolConformanceRunReceipt;

use super::super::{
    error::ExternalPoolAdapterTaskProtocolConformanceStoreError as StoreError,
    runtime::ExternalPoolAdapterTaskProtocolConformanceRuntime,
    types::{
        CreateExternalPoolAdapterTaskProtocolConformanceRun,
        ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt,
        StoredTaskProtocolConformanceRun,
    },
};

pub(super) fn ensure_create_replay(
    input: &CreateExternalPoolAdapterTaskProtocolConformanceRun,
    stored: &StoredTaskProtocolConformanceRun,
) -> Result<()> {
    let r = &stored.receipt.run;
    if r.registry_release.registry_release_id != input.registry_release_id
        || r.registry_release.registry_release_digest != input.expected_registry_release_digest
        || r.sandbox_reattestation.reattestation_receipt_id
            != input.sandbox_reattestation_receipt_id
        || r.sandbox_reattestation.reattestation_receipt_digest
            != input.expected_sandbox_reattestation_receipt_digest
        || r.runtime_compatibility.verification_receipt_id
            != input.runtime_compatibility_verification_receipt_id
        || r.runtime_compatibility.verification_receipt_digest
            != input.expected_runtime_compatibility_verification_receipt_digest
        || r.task_protocol_profile_digest != input.expected_task_protocol_profile_digest
        || r.fixture_catalog_digest != input.expected_fixture_catalog_digest
        || r.predecessor_run_receipt_id != input.predecessor_run_receipt_id
        || r.predecessor_run_receipt_digest != input.expected_predecessor_run_receipt_digest
        || stored.recorded_by_admin_user_id != input.recorded_by_admin_user_id
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
        || stored.confirmation != input.confirmation
    {
        bail!("task-protocol conformance replay conflicts with immutable neutral input")
    }
    Ok(())
}

pub(super) fn ensure_fresh_readback(
    input: &CreateExternalPoolAdapterTaskProtocolConformanceRun,
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    stored: &StoredTaskProtocolConformanceRun,
    custody_epoch_digest: &str,
    process_hmac_seal: &str,
    receipt_integrity_digest: &str,
) -> Result<()> {
    ensure_create_replay(input, stored)?;
    if stored.receipt != *receipt
        || stored.runtime_custody_epoch_digest != custody_epoch_digest
        || stored.process_hmac_seal != process_hmac_seal
        || stored.receipt_integrity_digest != receipt_integrity_digest
    {
        bail!("task-protocol conformance fresh durable readback is not exact")
    }
    Ok(())
}

pub(super) fn replay_output(
    stored: &StoredTaskProtocolConformanceRun,
) -> ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt {
    ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt {
        run: stored.receipt.clone(),
        replayed: true,
    }
}

/// A replay may complete an already remembered exact pending tuple. This operation never mints a
/// seal or inserts registry state; false means the durable receipt remains historical.
pub(super) fn promote_exact_pending_replay(
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    receipt_id: &str,
    receipt_integrity_digest: &str,
) -> std::result::Result<(), StoreError> {
    let _ = runtime
        .process_custody()
        .promote_task_protocol_conformance_seal(receipt_id, receipt_integrity_digest)
        .map_err(StoreError::storage)?;
    Ok(())
}
