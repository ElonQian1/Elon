use anyhow::{bail, Result};

use crate::{
    compute_federation::{execution::ComputeJob, execution::ComputeReservation},
    store::{
        ComputeAttemptActivationReceipt, ComputeAttemptConsumerReviewReceipt,
        ComputeAttemptPlatformObservationReceipt, ComputeAttemptTerminalCandidateReceipt,
        ComputeAttemptUsageDeclarationReceipt, ComputeAttemptVerificationDecisionReceipt,
    },
};

use super::{
    build_execution_receipt, execution_receipt_request_digest, normalize_execution_receipt_request,
    StoredExecutionReceipt,
};
use crate::store::compute_attempt_execution_receipts::{
    ComputeAttemptExecutionReceiptEnvelope, IssueComputeAttemptExecutionReceiptRequest,
};

impl StoredExecutionReceipt {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::store::compute_attempt_execution_receipts) fn into_envelope(
        self,
        verification: &ComputeAttemptVerificationDecisionReceipt,
        candidate: &ComputeAttemptTerminalCandidateReceipt,
        consumer_review: &ComputeAttemptConsumerReviewReceipt,
        platform_observation: &ComputeAttemptPlatformObservationReceipt,
        provider_usage: &ComputeAttemptUsageDeclarationReceipt,
        activation: &ComputeAttemptActivationReceipt,
        job: &ComputeJob,
        reservation: &ComputeReservation,
        replayed: bool,
    ) -> Result<ComputeAttemptExecutionReceiptEnvelope> {
        let request =
            normalize_execution_receipt_request(&IssueComputeAttemptExecutionReceiptRequest {
                lease_id: self.lease_id.clone(),
                expected_verification_decision_id: self.verification_decision_id.clone(),
                expected_verification_event_digest: self.verification_event_digest.clone(),
                idempotency_key: self.idempotency_key.clone(),
                issued_by_user_id: self.issued_by_user_id.clone(),
            })?;
        let expected_request_digest = execution_receipt_request_digest(&request)?;
        let expected_receipt = build_execution_receipt(
            &self.execution_receipt_id,
            verification,
            candidate,
            consumer_review,
            platform_observation,
            provider_usage,
            activation,
            job,
            reservation,
            &self.issued_at,
        )?;
        if self.verification_decision_id != verification.verification_decision_id
            || self.verification_event_digest != verification.event_digest
            || self.receipt.receipt_id != self.execution_receipt_id
            || self.receipt.receipt_digest != self.receipt_digest
            || self.receipt != expected_receipt
            || self.request_digest != expected_request_digest
            || self.idempotency_scope
                != format!(
                    "compute_attempt_execution_receipt:{}",
                    self.issued_by_user_id
                )
            || self.created_at != self.issued_at
        {
            bail!("Execution Receipt 审计内容、摘要或幂等字段不一致");
        }
        Ok(ComputeAttemptExecutionReceiptEnvelope {
            receipt: self.receipt,
            verification_decision_id: self.verification_decision_id,
            verification_event_digest: self.verification_event_digest,
            request_digest: self.request_digest,
            issued_by_user_id: self.issued_by_user_id,
            issued_at: self.issued_at,
            execution_effect: "execution_receipt_recorded",
            lease_effect: "unchanged",
            job_effect: "unchanged",
            capacity_effect: "unchanged",
            reservation_effect: "unchanged",
            money_effect: "preauthorization_unchanged",
            replayed,
        })
    }
}
