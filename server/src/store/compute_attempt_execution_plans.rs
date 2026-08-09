use anyhow::Result;
use rusqlite::TransactionBehavior;

use crate::compute_federation::execution_plan::ValidatedComputeAttemptExecutionPlanInputs;

use super::Store;

mod read;
mod replay;
mod replay_validation;
mod source;
mod types;
mod validation;
mod write;

pub(super) use read::ensure_current_plan_for_dispatch_on;
pub(in crate::store) use read::{plan_by_id_on as audited_plan_by_id_on, StoredPlan};
pub(crate) use types::ComputeAttemptExecutionPlanReceipt;

impl Store {
    pub(crate) fn produce_compute_attempt_execution_plan(
        &self,
        input: &ValidatedComputeAttemptExecutionPlanInputs,
    ) -> Result<ComputeAttemptExecutionPlanReceipt> {
        let prepared = validation::prepare_inputs(input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = write::produce_on(&transaction, input.plan(), &prepared)?;
        transaction.commit()?;
        Ok(receipt)
    }
}
