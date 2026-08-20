//! Same-connection V274 refresh readback, promotion, and pending-plan discard.

use anyhow::{bail, ensure, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::external_pool_adapter_provider_active_successor::PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
    store::compute_external_pool_adapter_runtime_bundle::ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard,
};

use super::{
    readback::{
        require_exact_readback_on, CommittedExternalPoolAdapterProviderActiveSuccessorAppend,
    },
    refresh::PendingExternalPoolAdapterProviderActiveSuccessorRefresh,
};

pub(in crate::store) fn postcommit_external_pool_adapter_provider_active_successor_refresh_on(
    connection: &Connection,
    pending: PendingExternalPoolAdapterProviderActiveSuccessorRefresh<'_>,
) -> Result<CommittedExternalPoolAdapterProviderActiveSuccessorAppend> {
    ensure!(
        connection.is_autocommit(),
        "V274 refresh promotion requires a committed autocommit connection"
    );
    let PendingExternalPoolAdapterProviderActiveSuccessorRefresh {
        mut append,
        plan_guard,
    } = pending;
    require_refresh_guard(&plan_guard, connection)?;
    require_exact_readback_on(connection, &append)?;
    let promoted = append
        .runtime
        .promote_provider_active_successor_process_seal_for_refresh(
            connection,
            &plan_guard,
            PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
            &append.receipt.active_successor_receipt_id,
            &append.receipt_integrity_digest,
        )?;
    if !promoted {
        bail!("V274 refresh lost its exact pending purpose seal after commit");
    }
    append.mark_promoted();
    let receipt = append.receipt.clone();
    plan_guard.discard()?;
    Ok(CommittedExternalPoolAdapterProviderActiveSuccessorAppend::new(receipt))
}

fn require_refresh_guard(
    guard: &ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard,
    connection: &Connection,
) -> Result<()> {
    guard.ensure_same_connection(connection)?;
    guard.ensure_fully_consumed()
}
