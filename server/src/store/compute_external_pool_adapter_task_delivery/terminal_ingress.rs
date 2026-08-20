//! Sealed V273 receipt handoff into the existing V213/V185/V215 accepted-ACK kernel.

use anyhow::{ensure, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::{
    compute_federation::{
        attempt_gateway::{
            ComputeAttemptAdapterAckEnvelope, ComputeAttemptAdapterBinding,
            VerifiedComputeAttemptAdapterAckView, COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED,
        },
        external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTaskVerifiedObservation,
        external_pool_adapter_task_protocol_production::ExternalPoolAdapterTaskExchangeReceiptEnvelope,
        provider::PROVIDER_KIND_EXTERNAL_POOL,
        start_outbox::{
            canonical_start_outbox_send_attempt_json_and_digest,
            ComputeStartOutboxRemoteObservationEnvelope, ComputeStartOutboxSendAttemptEnvelope,
            VerifiedComputeStartOutboxRemoteObservationView, COMPUTE_OBSERVATION_PREPARE_RESPONSE,
            COMPUTE_OBSERVATION_RECONCILE_ATTESTATION,
        },
    },
    store::compute_attempt_dispatches::{
        ingest_verified_historical_external_pool_adapter_ack_at_on,
        ComputeAttemptAdapterAckIngressTimes, ComputeAttemptDispatchAckCommit,
    },
};

use super::{
    historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    ingress_obligation::PendingTaskIngressObligation,
    receipt_ingress::PendingExternalPoolAdapterTaskReceiptIngress,
    reconcile_ingress::PendingExternalPoolAdapterTaskTerminalIngress,
};

const RECEIPT_VERIFICATION_KIND: &str = "external_pool_adapter_task_receipt.v1";

pub(in crate::store) struct ExternalPoolAdapterTaskTerminalIngressFactory<'a, T> {
    receipt: &'a ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    semantic: &'a T,
    send_attempt: &'a ComputeStartOutboxSendAttemptEnvelope,
    source: TerminalIngressSource,
}

pub(in crate::store) struct SealedExternalPoolAdapterTaskTerminalAck {
    adapter: ComputeAttemptAdapterBinding,
    ack: ComputeAttemptAdapterAckEnvelope,
    observation: SealedTerminalObservation,
}

struct SealedTerminalObservation {
    envelope: ComputeStartOutboxRemoteObservationEnvelope,
    send_attempt: ComputeStartOutboxSendAttemptEnvelope,
}

#[derive(Clone, Copy)]
enum TerminalIngressSource {
    DirectPrepare,
    Reconcile,
}

impl<'a, T: ExternalPoolAdapterBrokerTaskVerifiedObservation>
    ExternalPoolAdapterTaskTerminalIngressFactory<'a, T>
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

    pub(in crate::store) fn seal_accepted(
        self,
        adapter: ComputeAttemptAdapterBinding,
        ack: ComputeAttemptAdapterAckEnvelope,
        observation: ComputeStartOutboxRemoteObservationEnvelope,
    ) -> Result<SealedExternalPoolAdapterTaskTerminalAck> {
        self.semantic
            .validate_terminal_ack(&adapter, &ack, &observation)?;
        let receipt = &self.receipt.receipt;
        let command = &receipt.identity.command;
        let (operation_kind, source_kind, observation_kind) = match self.source {
            TerminalIngressSource::DirectPrepare => (
                "prepare",
                "start_outbox_send_attempt",
                COMPUTE_OBSERVATION_PREPARE_RESPONSE,
            ),
            TerminalIngressSource::Reconcile => (
                "reconcile",
                "reconcile_poll",
                COMPUTE_OBSERVATION_RECONCILE_ATTESTATION,
            ),
        };
        ensure!(
            receipt.identity.operation_kind == operation_kind
                && receipt.identity.source.source_kind == source_kind
                && self.send_attempt.operation_kind == "prepare"
                && adapter.provider_kind == PROVIDER_KIND_EXTERNAL_POOL
                && ack.outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED
                && ack.command_id == command.command_id
                && ack.command_digest == command.command_digest
                && ack.adapter_binding_digest == observation.adapter_binding_digest
                && ack.adapter_ack_id == observation.adapter_observation_id
                && ack.remote_execution_ref == observation.remote_execution_ref
                && ack.observed_at == observation.observed_at
                && ack.received_at == observation.received_at
                && observation.send_attempt_id == command.send_attempt_id
                && observation.outbox_id == command.outbox_id
                && observation.outbox_digest == command.outbox_digest
                && observation.command_id == command.command_id
                && observation.command_digest == command.command_digest
                && observation.operation_kind == self.send_attempt.operation_kind
                && observation.observation_kind == observation_kind
                && observation.provider_id == adapter.provider_id
                && observation.adapter_id == adapter.adapter_id
                && observation.verification_kind == RECEIPT_VERIFICATION_KIND
                && observation.verifier_id == self.receipt.exchange_receipt_id
                && observation.verification_digest == receipt.semantic_observation_sha256
                && observation.authenticated_at == receipt.authenticated_at
                && observation.received_at == receipt.received_at
                && observation.recorded_at == receipt.recorded_at,
            "V278 accepted terminal semantic does not bind the exact receipt/send/ACK"
        );
        Ok(SealedExternalPoolAdapterTaskTerminalAck {
            adapter,
            ack,
            observation: SealedTerminalObservation {
                envelope: observation,
                send_attempt: self.send_attempt.clone(),
            },
        })
    }
}

impl VerifiedComputeStartOutboxRemoteObservationView for SealedTerminalObservation {
    fn envelope(&self) -> &ComputeStartOutboxRemoteObservationEnvelope {
        &self.envelope
    }

    fn send_attempt_envelope(&self) -> &ComputeStartOutboxSendAttemptEnvelope {
        &self.send_attempt
    }
}

impl VerifiedComputeAttemptAdapterAckView for SealedExternalPoolAdapterTaskTerminalAck {
    fn adapter(&self) -> &ComputeAttemptAdapterBinding {
        &self.adapter
    }

    fn ack(&self) -> &ComputeAttemptAdapterAckEnvelope {
        &self.ack
    }

    fn prepare_observation(&self) -> &dyn VerifiedComputeStartOutboxRemoteObservationView {
        &self.observation
    }
}

pub(in crate::store) fn apply_external_pool_adapter_task_terminal_ack_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    transaction: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    pending: PendingExternalPoolAdapterTaskTerminalIngress<'tx, 'conn, T>,
    times: &ComputeAttemptAdapterAckIngressTimes,
    build: impl FnOnce(
        ExternalPoolAdapterTaskTerminalIngressFactory<'_, T>,
    ) -> Result<SealedExternalPoolAdapterTaskTerminalAck>,
) -> Result<ComputeAttemptDispatchAckCommit> {
    let (receipt, semantic, cleanup_expires_at, obligation) = pending.into_parts_on(transaction)?;
    times.ensure_not_after(&cleanup_expires_at)?;
    apply_terminal_parts_on(
        transaction,
        authority,
        receipt,
        semantic,
        TerminalIngressSource::Reconcile,
        times,
        build,
        obligation,
    )
}

pub(in crate::store) fn apply_external_pool_adapter_task_direct_terminal_ack_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    transaction: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    pending: PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T>,
    times: &ComputeAttemptAdapterAckIngressTimes,
    build: impl FnOnce(
        ExternalPoolAdapterTaskTerminalIngressFactory<'_, T>,
    ) -> Result<SealedExternalPoolAdapterTaskTerminalAck>,
) -> Result<ComputeAttemptDispatchAckCommit> {
    let cleanup_expires_at = pending.cleanup_expires_at().to_string();
    let (receipt, semantic, obligation) = pending.into_parts_on(transaction)?;
    times.ensure_not_after(&cleanup_expires_at)?;
    apply_terminal_parts_on(
        transaction,
        authority,
        receipt,
        semantic,
        TerminalIngressSource::DirectPrepare,
        times,
        build,
        obligation,
    )
}

fn apply_terminal_parts_on<T: ExternalPoolAdapterBrokerTaskVerifiedObservation>(
    transaction: &Transaction<'_>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'_, '_>,
    receipt: ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    semantic: T,
    source: TerminalIngressSource,
    times: &ComputeAttemptAdapterAckIngressTimes,
    build: impl FnOnce(
        ExternalPoolAdapterTaskTerminalIngressFactory<'_, T>,
    ) -> Result<SealedExternalPoolAdapterTaskTerminalAck>,
    obligation: PendingTaskIngressObligation,
) -> Result<ComputeAttemptDispatchAckCommit> {
    let send_attempt = exact_send_attempt_on(
        transaction,
        &receipt.receipt.identity.command.send_attempt_id,
        &receipt.receipt.identity.command.send_attempt_digest,
    )?;
    let sealed = build(ExternalPoolAdapterTaskTerminalIngressFactory {
        receipt: &receipt,
        semantic: &semantic,
        send_attempt: &send_attempt,
        source,
    })?;
    let commit = ingest_verified_historical_external_pool_adapter_ack_at_on(
        transaction,
        &sealed,
        times,
        authority,
    )?;
    ensure!(
        matches!(&commit, ComputeAttemptDispatchAckCommit::Activated { .. }),
        "V278 accepted terminal receipt did not produce the exact activation closure"
    );
    obligation.resolve(transaction)?;
    Ok(commit)
}

pub(super) fn exact_send_attempt_on(
    connection: &rusqlite::Connection,
    send_attempt_id: &str,
    expected_digest: &str,
) -> Result<ComputeStartOutboxSendAttemptEnvelope> {
    let row = connection
        .query_row(
            "SELECT send_attempt_json,send_attempt_digest
               FROM compute_attempt_start_send_attempts WHERE send_attempt_id=?1",
            params![send_attempt_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("V278 terminal ingress send-attempt is missing"))?;
    let envelope: ComputeStartOutboxSendAttemptEnvelope = serde_json::from_str(&row.0)?;
    let (canonical, digest) = canonical_start_outbox_send_attempt_json_and_digest(&envelope)?;
    ensure!(
        canonical == row.0
            && digest == row.1
            && digest == expected_digest
            && envelope.send_attempt_id == send_attempt_id,
        "V278 terminal ingress send-attempt failed exact canonical readback"
    );
    Ok(envelope)
}
