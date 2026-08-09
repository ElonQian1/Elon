use anyhow::Result;
use rusqlite::TransactionBehavior;

use crate::compute_federation::attempt_gateway::{
    ValidatedComputeAttemptStartDispatch, VerifiedComputeAttemptAdapterAck,
};

use super::Store;

mod read;
mod replay;
mod source;
mod types;
mod validation;
mod write;

pub(crate) use types::{ComputeAttemptDispatchAckCommit, ComputeAttemptDispatchCommandReceipt};

impl Store {
    pub(crate) fn prepare_compute_attempt_start_dispatch(
        &self,
        plan: &ValidatedComputeAttemptStartDispatch,
    ) -> Result<ComputeAttemptDispatchCommandReceipt> {
        let prepared = validation::prepare_start_dispatch(plan)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = write::prepare_start_dispatch_on(&transaction, plan, &prepared)?;
        transaction.commit()?;
        Ok(receipt)
    }

    pub(crate) fn ingest_verified_compute_attempt_adapter_ack(
        &self,
        verified: &VerifiedComputeAttemptAdapterAck,
    ) -> Result<ComputeAttemptDispatchAckCommit> {
        let prepared = validation::prepare_verified_ack(verified)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = write::ingest_verified_ack_on(&transaction, verified, &prepared)?;
        transaction.commit()?;
        Ok(receipt)
    }
}
