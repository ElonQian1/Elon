//! Store-private V273 ledger audit and poll-claim recovery.
//!
//! This module exposes no producer for the four immutable ledgers and no transport authority.

mod columns;
mod mapping;
mod polls;
mod read;
mod recovery;
mod types;

use anyhow::Result;
use rusqlite::TransactionBehavior;

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
}
