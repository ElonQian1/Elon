use anyhow::{anyhow, bail, Result};
use rusqlite::{Connection, TransactionBehavior};

use crate::compute_federation::attempt_gateway::{
    ValidatedComputeAttemptStartDispatch, VerifiedComputeAttemptAdapterAck,
};

use super::Store;

mod ack_write;
mod read;
mod replay;
mod source;
mod types;
mod validation;
mod write;

pub(crate) use types::{ComputeAttemptDispatchAckCommit, ComputeAttemptDispatchCommandReceipt};
pub(in crate::store) use validation::PreparedApplication;

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
        let receipt = ack_write::ingest_verified_ack_on(&transaction, verified, &prepared)?;
        transaction.commit()?;
        Ok(receipt)
    }
}

/// Reuses the v211 command/source/budget kernel before a durable prepare send-attempt. The
/// outbox layer must not maintain a weaker copy of Provider route or Reservation currentness.
pub(super) fn ensure_start_outbox_prepare_current_on(
    connection: &Connection,
    command_id: &str,
    command_digest: &str,
    checked_at: &str,
) -> Result<()> {
    let command = read::command_by_id_on(connection, command_id)?
        .ok_or_else(|| anyhow!("Start outbox references an unknown dispatch command"))?;
    if command.command.command_digest != command_digest {
        bail!("Start outbox command digest is stale");
    }
    super::compute_attempt_execution_plans::ensure_current_plan_for_dispatch_on(
        connection,
        &command.command,
        &command.adapter,
        &command.activated_by_user_id,
    )?;
    if let Some(reason) = source::current_source_blocker_on(
        connection,
        &command.command,
        &command.adapter,
        &command.activated_by_user_id,
        &command.activation_idempotency_key,
        true,
    )? {
        bail!("Start outbox prepare source is not current: {reason}");
    }
    if let Some(reason) = source::current_budget_blocker_on(connection, &command, checked_at)? {
        bail!("Start outbox prepare budget is not current: {reason}");
    }
    Ok(())
}
