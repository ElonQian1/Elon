//! Store-private V273 ledger audit and poll-claim recovery.
//!
//! This module exposes no Offer/Job/plan/start intent producer and no public transport authority.

mod candidate;
mod columns;
mod event_ingress;
mod first_event;
mod first_reconcile;
mod first_reconcile_cancel;
mod historical_cleanup;
mod ingress_obligation;
mod mapping;
mod no_start_ingress;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod outbound;
mod poll_exchange;
mod poll_plan;
mod polls;
mod reachability_pending_plan;
mod read;
mod receipt_ingress;
mod reconcile_completion;
mod reconcile_ingress;
mod reconcile_replay;
mod reconcile_source;
mod recovery;
mod retry_polls;
mod sealed;
mod terminal_ingress;
mod types;
mod write;

pub(in crate::store) use event_ingress::{
    insert_external_pool_adapter_task_event_ingress_on,
    read_external_pool_adapter_task_event_ingress_replay_on,
    ExternalPoolAdapterTaskEventIngressFactory,
};
pub(in crate::store) use first_event::{
    insert_first_external_pool_adapter_task_event_poll_on,
    ExternalPoolAdapterTaskFirstEventPollFactory,
};
pub(in crate::store) use first_reconcile::{
    insert_first_external_pool_adapter_task_reconcile_poll_on,
    ExternalPoolAdapterTaskFirstReconcilePollRequest,
};
pub(in crate::store) use first_reconcile_cancel::insert_first_external_pool_adapter_task_cancel_reconcile_poll_on;
pub(in crate::store) use historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority;
pub(in crate::store) use no_start_ingress::{
    apply_external_pool_adapter_task_no_start_on, ExternalPoolAdapterTaskNoStartIngressFactory,
    ExternalPoolAdapterTaskNoStartIngressReceipt, SealedExternalPoolAdapterTaskNoStartObservation,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use outbound::{
    record_external_pool_adapter_task_outbound_on,
    record_historical_external_pool_adapter_task_cleanup_outbound_on,
};
pub(in crate::store) use poll_exchange::{
    record_external_pool_adapter_task_event_exchange_attempt_on,
    record_external_pool_adapter_task_reconcile_exchange_attempt_on,
    ExternalPoolAdapterTaskPollExchangeAttemptFactory,
};
pub(in crate::store) use polls::{try_claim_event_poll_at_on, try_claim_reconcile_poll_at_on};
pub(crate) use reachability_pending_plan::register_external_pool_adapter_task_reachability_pending_plan_function;
pub(in crate::store) use reachability_pending_plan::{
    install_external_pool_adapter_task_reachability_pending_plan_on,
    ExternalPoolAdapterTaskReachabilityPendingPlan,
    ExternalPoolAdapterTaskReachabilityPendingPlanGuard,
    ExternalPoolAdapterTaskReachabilityPendingWrite,
    ExternalPoolAdapterTaskReachabilityPendingWriteKind,
};
pub(in crate::store) use receipt_ingress::{
    insert_external_pool_adapter_task_receipt_ingress_on,
    PendingExternalPoolAdapterTaskReceiptIngress,
};
pub(in crate::store) use reconcile_ingress::{
    close_external_pool_adapter_task_reconcile_ingress_on,
    ExternalPoolAdapterTaskReconcileIngressFactory, ExternalPoolAdapterTaskReconcileIngressOutcome,
    PendingExternalPoolAdapterTaskNoStartIngress, PendingExternalPoolAdapterTaskTerminalIngress,
};
pub(in crate::store) use reconcile_replay::read_external_pool_adapter_task_reconcile_ingress_replay_on;
pub(in crate::store) use retry_polls::{
    insert_external_pool_adapter_task_event_retry_after_unknown_on,
    insert_external_pool_adapter_task_reconcile_retry_after_unknown_on,
    ExternalPoolAdapterTaskRetryPollRequest,
};
pub(in crate::store) use sealed::ExternalPoolAdapterTaskExchangeAttemptFactory;
pub(in crate::store) use terminal_ingress::{
    apply_external_pool_adapter_task_direct_terminal_ack_on,
    apply_external_pool_adapter_task_terminal_ack_on,
    ExternalPoolAdapterTaskTerminalIngressFactory,
};
pub(in crate::store) use types::CommittedExternalPoolAdapterTaskOutbound;
pub(in crate::store) use types::CommittedExternalPoolAdapterTaskPollExchange;
pub(in crate::store) use types::ExternalPoolAdapterTaskLedgerWriteDisposition;
pub(in crate::store) use write::{
    insert_external_pool_adapter_task_event_batch_on, insert_external_pool_adapter_task_event_on,
    insert_external_pool_adapter_task_event_poll_on,
    insert_external_pool_adapter_task_exchange_attempt_on,
    insert_external_pool_adapter_task_exchange_receipt_on,
    insert_external_pool_adapter_task_reconcile_poll_on,
};

use anyhow::Result;
use rusqlite::TransactionBehavior;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::path::Path;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::compute_external_pool_adapter_runtime_bundle::ExternalPoolAdapterProviderRuntimeReadinessRuntime;
use super::Store;

impl Store {
    /// Audits and recovers only existing poll claim projections. Eligibility remains zero.
    pub(crate) fn recover_external_pool_adapter_task_delivery(&self) -> Result<usize> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let report = recovery::recover_on(&transaction)?;
        let eligible_rows = report.eligible_rows;
        transaction.commit()?;
        Ok(eligible_rows)
    }

    /// Runs the transaction-only source stage after S2 active preparation. It recovers bounded
    /// historical poll custody and reports only currently pending external_pool V213 rows. The
    /// V254 #13-18 producer gap means a normal deployment still reports zero.
    pub(crate) fn run_external_pool_adapter_task_delivery_source_cycle(
        &self,
        checked_at: &str,
    ) -> Result<Option<String>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let _recovery = recovery::recover_on(&transaction)?;
        let _observed_rows = candidate::eligible_external_pool_rows_on(&transaction, checked_at)?;
        let observed_provider =
            candidate::next_unadmitted_external_pool_source_provider_on(&transaction, checked_at)?;
        transaction.commit()?;
        Ok(observed_provider)
    }

    /// Reaches the S2 final callback without treating unadmitted fixture rows as eligible work.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(crate) fn reprove_external_pool_adapter_task_delivery_source(
        &self,
        provider_id: &str,
        data_dir: &Path,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    ) -> Result<()> {
        self.with_reproved_external_pool_adapter_route_and_active_successor(
            provider_id,
            data_dir,
            runtime,
            |transaction, authority| {
                let candidate =
                    candidate::next_eligible_external_pool_provider_on(transaction, authority)?;
                anyhow::ensure!(
                    candidate.is_none(),
                    "V278 external_pool source has no admitted Offer/Job/plan/start producer"
                );
                Ok(())
            },
        )?;
        Ok(())
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use candidate::next_eligible_external_pool_provider_on;
