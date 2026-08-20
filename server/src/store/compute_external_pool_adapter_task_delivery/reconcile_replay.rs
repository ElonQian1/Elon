//! Zero-write replay of one already completed reconcile receipt disposition.

use anyhow::{ensure, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::compute_federation::external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTaskVerifiedObservation;
use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    ExternalPoolAdapterTaskEventPollEnvelope, ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    ExternalPoolAdapterTaskReconcilePollEnvelope,
};

use super::{
    read::{read_event_poll_on, read_reconcile_poll_on},
    receipt_ingress::PendingExternalPoolAdapterTaskReceiptIngress,
    reconcile_ingress::{
        ExternalPoolAdapterTaskReconcileIngressFactory,
        ExternalPoolAdapterTaskReconcileIngressOutcome,
        PendingExternalPoolAdapterTaskNoStartIngress,
        PendingExternalPoolAdapterTaskTerminalIngress, SealedReconcileClosure,
    },
    reconcile_source::reconcile_source_operation_on,
    types::{ExternalPoolAdapterTaskLedgerWriteDisposition, CLAIM_STATUS_DELIVERY_OBSERVED},
};

pub(in crate::store) fn read_external_pool_adapter_task_reconcile_ingress_replay_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    connection: &'tx Transaction<'conn>,
    pending_receipt: PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T>,
    classify: impl FnOnce(
        ExternalPoolAdapterTaskReconcileIngressFactory<'_, T>,
    ) -> Result<SealedReconcileClosure>,
) -> Result<ExternalPoolAdapterTaskReconcileIngressOutcome<'tx, 'conn, T>> {
    ensure!(
        pending_receipt.disposition() == ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay,
        "V278 reconcile replay requires an exact durable receipt"
    );
    let cleanup_expires_at = pending_receipt.cleanup_expires_at().to_string();
    let (receipt, semantic, obligation) = pending_receipt.into_parts_on(connection)?;
    let source = &receipt.receipt.identity.source;
    ensure!(
        receipt.receipt.identity.operation_kind == "reconcile"
            && source.source_kind == "reconcile_poll",
        "V278 reconcile replay receipt has the wrong durable source"
    );
    let predecessor = read_reconcile_poll_on(connection, &source.source_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 replayed reconcile poll disappeared"))?;
    ensure!(
        predecessor.envelope.reconcile_poll_digest == source.source_digest
            && predecessor.claim.status == CLAIM_STATUS_DELIVERY_OBSERVED
            && predecessor.claim.owner_id.is_none()
            && predecessor.claim.token_digest.is_none()
            && predecessor.claim.expires_at.is_none(),
        "V278 replayed reconcile closure lacks exact completed poll readback"
    );
    let (source_operation, accepted_ack_exists) =
        reconcile_source_operation_on(connection, &receipt)?;
    let closure = classify(ExternalPoolAdapterTaskReconcileIngressFactory::new(
        &receipt,
        &semantic,
        &predecessor.envelope,
        &cleanup_expires_at,
        source_operation == "commit" && accepted_ack_exists,
        source_operation == "cancel" && !accepted_ack_exists,
        source_operation == "prepare" && !accepted_ack_exists,
    ))?;
    let durable_successor = durable_reconcile_successor_on(connection, &predecessor.envelope)?;
    let durable_event_poll = durable_reconcile_event_poll_on(connection, &receipt)?;
    let terminal_ack = has_accepted_terminal_ack_on(connection, &receipt)?;
    let terminal_no_start = has_terminal_no_start_on(connection, &receipt)?;
    let successor_branch = durable_successor.is_some()
        && durable_event_poll.is_none()
        && !terminal_ack
        && !terminal_no_start;
    let event_branch = durable_successor.is_none()
        && durable_event_poll.is_some()
        && !terminal_no_start
        && source_operation == "commit"
        && accepted_ack_exists
        && !terminal_ack;
    let terminal_branch = durable_successor.is_none()
        && durable_event_poll.is_none()
        && terminal_ack
        && !terminal_no_start;
    let no_start_branch = durable_successor.is_none()
        && durable_event_poll.is_none()
        && !terminal_ack
        && terminal_no_start;
    ensure!(
        usize::from(successor_branch)
            + usize::from(event_branch)
            + usize::from(terminal_branch)
            + usize::from(no_start_branch)
            == 1,
        "V278 replayed reconcile closure has no unique durable disposition"
    );
    match closure {
        SealedReconcileClosure::Successor(successor) => {
            ensure!(
                successor_branch && durable_successor.as_ref() == Some(&successor),
                "V278 replayed reconcile successor differs from durable disposition"
            );
            obligation.resolve(connection)?;
            Ok(ExternalPoolAdapterTaskReconcileIngressOutcome::Successor(
                successor,
            ))
        }
        SealedReconcileClosure::EventPoll(poll) => {
            ensure!(
                event_branch && durable_event_poll.as_ref() == Some(&poll),
                "V278 replayed reconcile event poll differs from durable disposition"
            );
            obligation.resolve(connection)?;
            Ok(ExternalPoolAdapterTaskReconcileIngressOutcome::EventPoll(
                poll,
            ))
        }
        SealedReconcileClosure::NoStart => {
            ensure!(
                no_start_branch,
                "V278 replayed no-start conflicts with durable reconcile disposition"
            );
            Ok(ExternalPoolAdapterTaskReconcileIngressOutcome::NoStart(
                PendingExternalPoolAdapterTaskNoStartIngress::new(
                    PendingExternalPoolAdapterTaskTerminalIngress::new(
                        connection,
                        receipt,
                        semantic,
                        cleanup_expires_at,
                        obligation,
                    ),
                ),
            ))
        }
        SealedReconcileClosure::Terminal => {
            ensure!(
                terminal_branch,
                "V278 replayed reconcile terminal conflicts with durable successor"
            );
            Ok(ExternalPoolAdapterTaskReconcileIngressOutcome::Terminal(
                PendingExternalPoolAdapterTaskTerminalIngress::new(
                    connection,
                    receipt,
                    semantic,
                    cleanup_expires_at,
                    obligation,
                ),
            ))
        }
    }
}

fn has_terminal_no_start_on(
    connection: &rusqlite::Connection,
    receipt: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<bool> {
    let count = connection.query_row(
        "SELECT count(*)
           FROM compute_attempt_start_remote_observations observation
           JOIN compute_attempt_no_start_proofs proof
             ON proof.command_id=observation.command_id
            AND proof.observation_id=observation.observation_id
            AND proof.observation_digest=observation.observation_digest
            AND proof.no_commit_tombstone_id=observation.no_commit_tombstone_id
            AND proof.no_commit_tombstone_digest=observation.no_commit_tombstone_digest
          WHERE observation.verification_kind='external_pool_adapter_task_receipt.v1'
            AND observation.verifier_id=?1 AND observation.verification_digest=?2
            AND observation.command_id=?3 AND observation.command_digest=?4
            AND observation.observation_kind='reconcile_attestation'
            AND observation.response_outcome='observed'
            AND observation.remote_execution_state='terminal_no_start'
            AND observation.terminality='final'
            AND proof.proof_kind='remote_never_committed'",
        params![
            receipt.exchange_receipt_id,
            receipt.receipt.semantic_observation_sha256,
            receipt.receipt.identity.command.command_id,
            receipt.receipt.identity.command.command_digest,
        ],
        |row| row.get::<_, u64>(0),
    )?;
    ensure!(
        count <= 1,
        "V278 reconcile no-start disposition is not unique"
    );
    Ok(count == 1)
}

fn durable_reconcile_event_poll_on(
    connection: &rusqlite::Connection,
    receipt: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<Option<ExternalPoolAdapterTaskEventPollEnvelope>> {
    let mut statement = connection.prepare(
        "SELECT event_poll_id FROM compute_external_pool_adapter_task_event_polls
          WHERE source_exchange_receipt_id=?1 AND source_exchange_receipt_digest=?2
            AND predecessor_event_poll_id IS NULL AND poll_ordinal=1
          ORDER BY event_poll_id LIMIT 2",
    )?;
    let ids = statement
        .query_map(
            params![receipt.exchange_receipt_id, receipt.exchange_receipt_digest],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        ids.len() <= 1,
        "V278 reconcile receipt has multiple first event polls"
    );
    ids.first()
        .map(|id| {
            read_event_poll_on(connection, id)?
                .map(|stored| stored.envelope)
                .ok_or_else(|| anyhow::anyhow!("V278 reconcile event poll disappeared"))
        })
        .transpose()
}

fn durable_reconcile_successor_on(
    connection: &rusqlite::Connection,
    predecessor: &ExternalPoolAdapterTaskReconcilePollEnvelope,
) -> Result<Option<ExternalPoolAdapterTaskReconcilePollEnvelope>> {
    let successor_id = connection
        .query_row(
            "SELECT reconcile_poll_id
               FROM compute_external_pool_adapter_task_reconcile_polls
              WHERE predecessor_reconcile_poll_id=?1
                AND predecessor_reconcile_poll_digest=?2",
            params![
                predecessor.reconcile_poll_id,
                predecessor.reconcile_poll_digest
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    successor_id
        .map(|id| {
            read_reconcile_poll_on(connection, &id)?
                .map(|stored| stored.envelope)
                .ok_or_else(|| {
                    anyhow::anyhow!("V278 reconcile successor disappeared during readback")
                })
        })
        .transpose()
}

fn has_accepted_terminal_ack_on(
    connection: &rusqlite::Connection,
    receipt: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<bool> {
    let count = connection.query_row(
        "SELECT count(*)
           FROM compute_attempt_dispatch_acks ack
           JOIN compute_attempt_start_remote_observations observation
             ON observation.command_id=ack.command_id
            AND observation.provider_id=ack.provider_id
            AND observation.adapter_id=ack.adapter_id
            AND observation.adapter_observation_id=ack.adapter_ack_id
          WHERE ack.command_id=?1 AND ack.outcome='accepted'
            AND ack.disposition='accepted_applied'
            AND observation.observation_kind='reconcile_attestation'
            AND observation.verification_kind='external_pool_adapter_task_receipt.v1'
            AND observation.verifier_id=?2 AND observation.verification_digest=?3",
        params![
            receipt.receipt.identity.command.command_id,
            receipt.exchange_receipt_id,
            receipt.receipt.semantic_observation_sha256
        ],
        |row| row.get::<_, u64>(0),
    )?;
    ensure!(count <= 1, "V278 reconcile terminal ACK is not unique");
    Ok(count == 1)
}
