//! Receipt-bound terminal-no-start handoff into the existing V214 proof kernel.

use anyhow::{ensure, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::{
    compute_federation::{
        external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTaskVerifiedObservation,
        external_pool_adapter_task_protocol_production::ExternalPoolAdapterTaskExchangeReceiptEnvelope,
        start_outbox::{
            ComputeStartOutboxRemoteObservationEnvelope, ComputeStartOutboxSendAttemptEnvelope,
            VerifiedComputeStartOutboxRemoteObservationView,
            COMPUTE_OBSERVATION_RECONCILE_ATTESTATION, COMPUTE_REMOTE_EXECUTION_TERMINAL_NO_START,
            COMPUTE_REMOTE_TERMINALITY_FINAL,
        },
    },
    store::compute_attempt_start_outbox::{
        record_verified_observation_at_on, StartOutboxObservationReceipt,
    },
};

use super::{
    reconcile_ingress::PendingExternalPoolAdapterTaskNoStartIngress,
    terminal_ingress::exact_send_attempt_on,
};

const RECEIPT_VERIFICATION_KIND: &str = "external_pool_adapter_task_receipt.v1";

pub(in crate::store) struct ExternalPoolAdapterTaskNoStartIngressFactory<'a, T> {
    receipt: &'a ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    semantic: &'a T,
    send_attempt: &'a ComputeStartOutboxSendAttemptEnvelope,
}

pub(in crate::store) struct SealedExternalPoolAdapterTaskNoStartObservation {
    envelope: ComputeStartOutboxRemoteObservationEnvelope,
    send_attempt: ComputeStartOutboxSendAttemptEnvelope,
}

pub(in crate::store) struct ExternalPoolAdapterTaskNoStartIngressReceipt {
    pub observation: StartOutboxObservationReceipt,
    pub proof_id: String,
    pub proof_digest: String,
    pub replayed: bool,
}

impl<'a, T: ExternalPoolAdapterBrokerTaskVerifiedObservation>
    ExternalPoolAdapterTaskNoStartIngressFactory<'a, T>
{
    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterTaskExchangeReceiptEnvelope {
        self.receipt
    }

    pub(in crate::store) fn semantic(&self) -> &T {
        self.semantic
    }

    pub(in crate::store) fn send_attempt(&self) -> &ComputeStartOutboxSendAttemptEnvelope {
        self.send_attempt
    }

    pub(in crate::store) fn seal_terminal_no_start(
        self,
        observation: ComputeStartOutboxRemoteObservationEnvelope,
    ) -> Result<SealedExternalPoolAdapterTaskNoStartObservation> {
        self.semantic.validate_terminal_no_start(&observation)?;
        let receipt = &self.receipt.receipt;
        let command = &receipt.identity.command;
        ensure!(
            receipt.identity.operation_kind == "reconcile"
                && receipt.identity.source.source_kind == "reconcile_poll"
                && self.send_attempt.operation_kind == "cancel"
                && observation.send_attempt_id == command.send_attempt_id
                && observation.outbox_id == command.outbox_id
                && observation.outbox_digest == command.outbox_digest
                && observation.command_id == command.command_id
                && observation.command_digest == command.command_digest
                && observation.operation_kind == self.send_attempt.operation_kind
                && observation.observation_kind == COMPUTE_OBSERVATION_RECONCILE_ATTESTATION
                && observation.provider_id == receipt.identity.adapter.provider_id
                && observation.adapter_id == receipt.identity.adapter.adapter_id
                && observation.response_outcome == "observed"
                && observation.remote_execution_state == COMPUTE_REMOTE_EXECUTION_TERMINAL_NO_START
                && observation.terminality == COMPUTE_REMOTE_TERMINALITY_FINAL
                && observation.remote_execution_ref.is_none()
                && observation.no_commit_tombstone_id.is_some()
                && observation.no_commit_tombstone_digest.is_some()
                && observation.verification_kind == RECEIPT_VERIFICATION_KIND
                && observation.verifier_id == self.receipt.exchange_receipt_id
                && observation.verification_digest == receipt.semantic_observation_sha256
                && observation.authenticated_at == receipt.authenticated_at
                && observation.received_at == receipt.received_at
                && observation.recorded_at == receipt.recorded_at,
            "V278 terminal no-start observation does not bind the exact reconcile receipt"
        );
        Ok(SealedExternalPoolAdapterTaskNoStartObservation {
            envelope: observation,
            send_attempt: self.send_attempt.clone(),
        })
    }
}

impl VerifiedComputeStartOutboxRemoteObservationView
    for SealedExternalPoolAdapterTaskNoStartObservation
{
    fn envelope(&self) -> &ComputeStartOutboxRemoteObservationEnvelope {
        &self.envelope
    }

    fn send_attempt_envelope(&self) -> &ComputeStartOutboxSendAttemptEnvelope {
        &self.send_attempt
    }
}

pub(in crate::store) fn apply_external_pool_adapter_task_no_start_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    transaction: &'tx Transaction<'conn>,
    pending: PendingExternalPoolAdapterTaskNoStartIngress<'tx, 'conn, T>,
    transitioned_at: &str,
    build: impl FnOnce(
        ExternalPoolAdapterTaskNoStartIngressFactory<'_, T>,
    ) -> Result<SealedExternalPoolAdapterTaskNoStartObservation>,
) -> Result<ExternalPoolAdapterTaskNoStartIngressReceipt> {
    let (receipt, semantic, cleanup_expires_at, obligation) =
        pending.into_inner().into_parts_on(transaction)?;
    ensure!(
        transitioned_at < cleanup_expires_at.as_str(),
        "V278 no-start transition exceeded historical cleanup custody"
    );
    let command = &receipt.receipt.identity.command;
    let send_attempt = exact_send_attempt_on(
        transaction,
        &command.send_attempt_id,
        &command.send_attempt_digest,
    )?;
    let sealed = build(ExternalPoolAdapterTaskNoStartIngressFactory {
        receipt: &receipt,
        semantic: &semantic,
        send_attempt: &send_attempt,
    })?;
    let observation = record_verified_observation_at_on(transaction, &sealed, transitioned_at)?;
    let proof = exact_no_start_proof_on(transaction, &sealed.envelope)?;
    ensure!(
        observation.replayed || proof.2 == transitioned_at,
        "V278 fresh terminal no-start proof used a different transition time"
    );
    let ingress = ExternalPoolAdapterTaskNoStartIngressReceipt {
        replayed: observation.replayed,
        observation,
        proof_id: proof.0,
        proof_digest: proof.1,
    };
    obligation.resolve(transaction)?;
    Ok(ingress)
}

fn exact_no_start_proof_on(
    connection: &rusqlite::Connection,
    observation: &ComputeStartOutboxRemoteObservationEnvelope,
) -> Result<(String, String, String)> {
    connection
        .query_row(
            "SELECT proof_id,proof_digest,recorded_at
               FROM compute_attempt_no_start_proofs
              WHERE proof_kind='remote_never_committed' AND command_id=?1
                AND observation_id=?2 AND observation_digest=?3
                AND no_commit_tombstone_id=?4 AND no_commit_tombstone_digest=?5",
            params![
                observation.command_id,
                observation.observation_id,
                observation.observation_digest,
                observation.no_commit_tombstone_id,
                observation.no_commit_tombstone_digest,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("V278 terminal no-start proof is not durable"))
}
