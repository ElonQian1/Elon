use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    store::compute_external_pool_adapter_provider_active_successor::HistoricalExternalPoolAdapterAtomicActivationAuthority,
};

use super::{
    super::{
        read::{revocation_by_run_on, run_by_id_on, run_head_by_release_on},
        roots::canonical_time,
        runtime::ExternalPoolAdapterTaskProtocolConformanceRuntime,
    },
    roots::current_active_roots_for_receipt_on,
    types::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority,
};

#[allow(clippy::too_many_arguments)]
pub(in crate::store) fn current_external_pool_adapter_task_protocol_conformance_projected_active_authority_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    run_receipt_id: &str,
    expected_run_receipt_digest: &str,
    historical_activation: HistoricalExternalPoolAdapterAtomicActivationAuthority,
    prepared: PreparedExternalPoolAdapterInstallation,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn>>> {
    let Some(stored) = run_by_id_on(transaction, run_receipt_id)? else {
        return Ok(None);
    };
    let receipt = &stored.receipt;
    let run = &receipt.run;
    let head = run_head_by_release_on(transaction, &run.registry_release.registry_release_id)?
        .ok_or_else(|| anyhow::anyhow!("projected-active V272 head disappeared"))?;
    if receipt.run_receipt_digest != expected_run_receipt_digest
        || head.receipt.run_receipt_id != receipt.run_receipt_id
        || head.receipt.run_receipt_digest != receipt.run_receipt_digest
        || revocation_by_run_on(transaction, run_receipt_id)?.is_some()
        || canonical_time(&run.post_cleanup_checked_at)? > canonical_time(checked_at)?
        || canonical_time(&run.expires_at)? <= canonical_time(checked_at)?
        || !runtime
            .process_custody()
            .attests_task_protocol_conformance_seal(
                &receipt.run_receipt_id,
                &stored.receipt_integrity_digest,
                &receipt.run_receipt_digest,
                &stored.runtime_custody_epoch_digest,
                &stored.process_hmac_seal,
                &run.expires_at,
            )?
    {
        bail!("projected-active V272 receipt is not the exact current process head");
    }
    let roots = current_active_roots_for_receipt_on(
        transaction,
        receipt,
        historical_activation,
        prepared,
        checked_at,
    )?;
    Ok(Some(
        CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority::new(
            transaction,
            stored.receipt,
            roots.carrier,
            checked_at.into(),
        )?,
    ))
}
