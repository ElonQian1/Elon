use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::start_outbox::{
    COMPUTE_NO_START_PROOF_PREPARE_REJECTED, COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED,
};

use super::super::super::types::NoStartProofSource;
use super::DurableObservationEvidence;

pub(super) fn exact_observation_on(
    connection: &Connection,
    source: &NoStartProofSource,
    observation_id: &str,
    proof_kind: &str,
) -> Result<DurableObservationEvidence> {
    let row = match proof_kind {
        COMPUTE_NO_START_PROOF_PREPARE_REJECTED => connection
            .query_row(
                "SELECT observation.observation_id, observation.observation_digest,
                        observation.no_commit_tombstone_id,
                        observation.no_commit_tombstone_digest, ack.created_at
                   FROM compute_attempt_start_remote_observations observation
                   JOIN compute_attempt_dispatch_acks ack ON ack.command_id=observation.command_id
                  WHERE observation.observation_id=?1 AND observation.command_id=?2
                    AND observation.outbox_id=?3
                    AND observation.observation_kind='prepare_response'
                    AND observation.response_outcome='rejected'
                    AND observation.remote_execution_state='rejected'
                    AND observation.terminality='final'
                    AND ack.outcome='rejected' AND ack.disposition='rejected'
                    AND ack.adapter_ack_id=observation.adapter_observation_id",
                params![observation_id, source.command_id, source.outbox_id],
                evidence_row,
            )
            .optional()?,
        COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED => connection
            .query_row(
                "SELECT observation.observation_id, observation.observation_digest,
                        observation.no_commit_tombstone_id,
                        observation.no_commit_tombstone_digest, observation.recorded_at
                   FROM compute_attempt_start_remote_observations observation
                  WHERE observation.observation_id=?1 AND observation.command_id=?2
                    AND observation.observation_kind='reconcile_attestation'
                    AND observation.response_outcome='observed'
                    AND observation.remote_execution_state='terminal_no_start'
                    AND observation.terminality='final'
                    AND observation.no_commit_tombstone_id IS NOT NULL
                    AND observation.no_commit_tombstone_digest IS NOT NULL
                    AND observation.provider_id=?4 AND observation.adapter_id=?5
                    AND observation.adapter_binding_digest=?6
                    AND (EXISTS (
                         SELECT 1 FROM compute_attempt_start_outbox reconcile
                         JOIN compute_attempt_start_outbox cancel
                           ON cancel.outbox_id=reconcile.subject_outbox_id
                        WHERE reconcile.outbox_id=observation.outbox_id
                          AND reconcile.operation_kind='reconcile'
                          AND reconcile.state='delivery_observed'
                          AND reconcile.outbox_digest=observation.outbox_digest
                          AND reconcile.command_digest=observation.command_digest
                          AND cancel.operation_kind='cancel'
                          AND cancel.state='delivery_observed'
                          AND cancel.subject_outbox_id=?3
                          AND cancel.ack_id IS reconcile.ack_id
                          AND cancel.ack_digest IS reconcile.ack_digest)
                     OR (observation.verification_kind='external_pool_adapter_task_receipt.v1'
                         AND EXISTS (
                         SELECT 1
                           FROM compute_external_pool_adapter_task_exchange_receipts receipt
                           JOIN compute_external_pool_adapter_task_reconcile_polls poll
                             ON poll.reconcile_poll_id=receipt.source_id
                            AND poll.reconcile_poll_digest=receipt.source_digest
                           JOIN compute_attempt_start_send_attempts send
                             ON send.send_attempt_id=poll.send_attempt_id
                            AND send.send_attempt_digest=poll.send_attempt_digest
                           JOIN compute_external_pool_adapter_task_exchange_receipts cancel_receipt
                             ON cancel_receipt.exchange_attempt_id
                                =poll.uncertain_exchange_attempt_id
                            AND cancel_receipt.exchange_attempt_digest
                                =poll.uncertain_exchange_attempt_digest
                            AND cancel_receipt.operation_kind='cancel_no_start'
                            AND cancel_receipt.source_kind='start_outbox_send_attempt'
                            AND cancel_receipt.source_id=send.send_attempt_id
                            AND cancel_receipt.source_digest=send.send_attempt_digest
                           JOIN compute_attempt_start_outbox source_outbox
                             ON source_outbox.outbox_id=send.outbox_id
                            AND source_outbox.outbox_digest=send.outbox_digest
                          WHERE receipt.exchange_receipt_id=observation.verifier_id
                            AND receipt.semantic_observation_sha256
                                =observation.verification_digest
                            AND receipt.operation_kind='reconcile'
                            AND receipt.source_kind='reconcile_poll'
                            AND receipt.command_id=observation.command_id
                            AND receipt.command_digest=observation.command_digest
                            AND receipt.outbox_id=observation.outbox_id
                            AND receipt.outbox_digest=observation.outbox_digest
                            AND receipt.send_attempt_id=observation.send_attempt_id
                            AND receipt.authenticated_at=observation.authenticated_at
                            AND receipt.received_at=observation.received_at
                            AND receipt.recorded_at=observation.recorded_at
                            AND poll.claim_status='delivery_observed'
                            AND poll.command_id=receipt.command_id
                            AND poll.command_digest=receipt.command_digest
                            AND poll.outbox_id=receipt.outbox_id
                            AND poll.outbox_digest=receipt.outbox_digest
                            AND poll.authenticated_subject_sha256
                                =cancel_receipt.semantic_observation_sha256
                            AND source_outbox.state='delivery_observed'
                            AND send.operation_kind='cancel'
                            AND observation.operation_kind=send.operation_kind
                            AND source_outbox.subject_outbox_id=?3)))",
                params![
                    observation_id,
                    source.command_id,
                    source.outbox_id,
                    source.provider_id,
                    source.adapter_id,
                    source.adapter_binding_digest
                ],
                evidence_row,
            )
            .optional()?,
        _ => bail!("unsupported observation-backed no-start proof"),
    };
    row.ok_or_else(|| anyhow!("no-start derivation lacks exact authenticated observation"))
}

fn evidence_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DurableObservationEvidence> {
    Ok(DurableObservationEvidence {
        observation_id: row.get(0)?,
        observation_digest: row.get(1)?,
        no_commit_tombstone_id: row.get(2)?,
        no_commit_tombstone_digest: row.get(3)?,
        proven_at: row.get(4)?,
    })
}

pub(super) fn rejected_observation_id_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT observation.observation_id
               FROM compute_attempt_start_remote_observations observation
               JOIN compute_attempt_dispatch_acks ack ON ack.command_id=observation.command_id
              WHERE observation.command_id=?1
                AND observation.observation_kind='prepare_response'
                AND observation.response_outcome='rejected'
                AND observation.remote_execution_state='rejected'
                AND observation.terminality='final'
                AND ack.outcome='rejected' AND ack.disposition='rejected'
                AND ack.adapter_ack_id=observation.adapter_observation_id",
            params![command_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

pub(super) fn final_reconcile_observation_id_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT observation.observation_id
               FROM compute_attempt_start_remote_observations observation
              WHERE observation.command_id=?1
                AND observation.observation_kind='reconcile_attestation'
                AND observation.response_outcome='observed'
                AND observation.remote_execution_state='terminal_no_start'
                AND observation.terminality='final'
                AND observation.no_commit_tombstone_id IS NOT NULL
                AND observation.no_commit_tombstone_digest IS NOT NULL",
            params![command_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}
