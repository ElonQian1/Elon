use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    store::compute_external_pool_adapter_provider_active_successor::{
        CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority,
        HistoricalExternalPoolAdapterAtomicActivationAuthority,
    },
};

use super::{
    super::{
        read::{revocation_by_run_on, run_by_id_on, run_head_by_release_on},
        roots::canonical_time,
        runtime::ExternalPoolAdapterTaskProtocolConformanceRuntime,
    },
    roots::{
        current_active_roots_for_receipt_on,
        current_active_roots_for_receipt_with_renewed_route_carrier_on,
        require_current_active_receipt_roots_for_renewed_route_carrier_ref_on,
    },
    types::{
        CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority,
        CurrentExternalPoolAdapterTaskProtocolProjectedActiveLeafAuthority,
    },
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
    let Some(stored) = current_projected_active_run_on(
        transaction,
        run_receipt_id,
        expected_run_receipt_digest,
        runtime,
        checked_at,
    )?
    else {
        return Ok(None);
    };
    let receipt = &stored.receipt;
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

pub(in crate::store) fn current_external_pool_adapter_task_protocol_conformance_for_renewed_route_carrier_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    carrier: CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn>,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn>>> {
    let release_id = carrier
        .registry_release()
        .release()
        .registry_release_id
        .clone();
    let Some(head) = run_head_by_release_on(transaction, &release_id)? else {
        return Ok(None);
    };
    let receipt_id = head.receipt.run_receipt_id.clone();
    let receipt_digest = head.receipt.run_receipt_digest.clone();
    drop(head);
    let Some(stored) = current_projected_active_run_on(
        transaction,
        &receipt_id,
        &receipt_digest,
        runtime,
        checked_at,
    )?
    else {
        return Ok(None);
    };
    let roots = current_active_roots_for_receipt_with_renewed_route_carrier_on(
        transaction,
        &stored.receipt,
        carrier,
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

pub(in crate::store) fn current_external_pool_adapter_task_protocol_conformance_leaf_for_renewed_route_carrier_on<
    'authority,
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    carrier: &'authority CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn>,
    run_receipt_id: &str,
    expected_run_receipt_digest: &str,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    checked_at: &str,
) -> Result<
    Option<
        CurrentExternalPoolAdapterTaskProtocolProjectedActiveLeafAuthority<'authority, 'tx, 'conn>,
    >,
> {
    let Some(stored) = current_projected_active_run_on(
        transaction,
        run_receipt_id,
        expected_run_receipt_digest,
        runtime,
        checked_at,
    )?
    else {
        return Ok(None);
    };
    require_current_active_receipt_roots_for_renewed_route_carrier_ref_on(
        transaction,
        &stored.receipt,
        carrier,
        checked_at,
    )?;
    Ok(Some(
        CurrentExternalPoolAdapterTaskProtocolProjectedActiveLeafAuthority::new(
            transaction,
            stored.receipt,
            carrier,
            checked_at.into(),
        ),
    ))
}

fn current_projected_active_run_on(
    transaction: &Transaction<'_>,
    run_receipt_id: &str,
    expected_run_receipt_digest: &str,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    checked_at: &str,
) -> Result<Option<super::super::types::StoredTaskProtocolConformanceRun>> {
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
    Ok(Some(stored))
}
