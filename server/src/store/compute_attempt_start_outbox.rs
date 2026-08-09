use anyhow::{ensure, Result};
use rusqlite::TransactionBehavior;

use crate::compute_federation::start_outbox::{
    VerifiedComputeStartOutboxRemoteObservation, COMPUTE_OBSERVATION_PREPARE_RESPONSE,
    COMPUTE_START_OPERATION_PREPARE,
};

use super::Store;

mod claim;
mod cleanup;
mod currentness;
mod enqueue;
mod no_start;
mod observations;
mod read;
mod replay;
mod send;
mod types;

pub(super) use cleanup::enqueue_quarantined_cleanup_on;
pub(super) use enqueue::enqueue_prepare_on;
pub(super) use no_start::{
    ensure_start_resolved_for_broker_finish_on, record_prepare_rejected_no_start_on,
};
pub(super) use observations::record_verified_observation_on;
pub(super) use types::{
    BrokerFinishStartResolutionBinding, StartOutboxEnqueueReceipt, StartOutboxObservationReceipt,
    StartResolutionProofReceipt,
};
pub(crate) use types::{
    CommittedStartSendAuthority, PreparedStartSendRequest, StartNoStartRecoveryReceipt,
    StartOutboxClaimHandle, StartOutboxCleanupReceipt, StartOutboxNoStartProofReceipt,
};

impl Store {
    /// Claims one durable operation for local work. This grants no network-send authority.
    pub(crate) fn try_claim_compute_attempt_start_outbox(
        &self,
        claim_owner_id: &str,
        claimed_at: &str,
        claim_expires_at: &str,
    ) -> Result<Option<types::StartOutboxClaimHandle>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let claimed =
            claim::try_claim_on(&transaction, claim_owner_id, claimed_at, claim_expires_at)?;
        transaction.commit()?;
        Ok(claimed)
    }

    /// Records send-start and commits the unknown-delivery state before returning authority.
    /// `PreparedStartSendRequest` intentionally has no constructor in this batch.
    pub(crate) fn commit_compute_attempt_start_send(
        &self,
        claim: StartOutboxClaimHandle,
        request: PreparedStartSendRequest,
    ) -> Result<CommittedStartSendAuthority> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let attempt = send::record_send_started_on(&transaction, &claim, &request)?;
        transaction.commit()?;
        Ok(CommittedStartSendAuthority {
            attempt,
            claim,
            request,
        })
    }

    /// Standalone observation entry point. ACK ingestion should call the `_on` helper in its
    /// existing outer transaction so observation and ACK visibility remain atomic.
    pub(crate) fn record_verified_compute_start_observation(
        &self,
        observation: &VerifiedComputeStartOutboxRemoteObservation,
    ) -> Result<StartOutboxObservationReceipt> {
        let envelope = observation.envelope();
        ensure!(
            envelope.operation_kind != COMPUTE_START_OPERATION_PREPARE
                && envelope.observation_kind != COMPUTE_OBSERVATION_PREPARE_RESPONSE,
            "COMPUTE_ATTEMPT_PREPARE_OBSERVATION_REQUIRES_ACK_TRANSACTION"
        );
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = observations::record_verified_observation_on(&transaction, observation)?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Store-owned recovery seam. Its receipt grants neither transport nor no-start authority.
    pub(crate) fn recover_compute_attempt_start_no_start(
        &self,
        command_id: &str,
    ) -> Result<StartNoStartRecoveryReceipt> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = no_start::recover_no_start_on(&transaction, command_id)?;
        transaction.commit()?;
        Ok(receipt)
    }
}
