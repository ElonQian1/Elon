use anyhow::{anyhow, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::start_outbox::{
    canonical_start_no_start_proof_json_and_digest, ComputeStartNoStartProofEnvelope,
};

use super::{
    super::types::{BrokerFinishStartResolutionBinding, StartResolutionProofReceipt},
    derive::derive_local_never_sent_if_due_on,
};

pub(in crate::store) fn ensure_start_resolved_for_broker_finish_on(
    connection: &Connection,
    binding: BrokerFinishStartResolutionBinding<'_>,
) -> Result<Option<StartResolutionProofReceipt>> {
    let command_id = connection
        .query_row(
            "SELECT command_id FROM compute_attempt_dispatch_commands
              WHERE reservation_id=?1",
            params![binding.reservation_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(command_id) = command_id else {
        return Ok(None);
    };
    let _ = derive_local_never_sent_if_due_on(connection, &command_id)?;
    let json = connection
        .query_row(
            "SELECT proof.proof_json
               FROM compute_attempt_no_start_proofs proof
               JOIN compute_attempt_dispatch_commands command
                 ON command.command_id=proof.command_id
               JOIN compute_attempt_start_outbox prepare
                 ON prepare.outbox_id=proof.outbox_id
                AND prepare.outbox_digest=proof.outbox_digest
               JOIN compute_route_authorization_receipts route
                 ON route.route_authorization_id=proof.route_authorization_id
                AND route.route_authorization_digest=proof.route_authorization_digest
              WHERE proof.command_id=?1
                AND proof.reservation_id=?2 AND proof.reservation_revision=?3
                AND proof.reservation_digest=?4 AND proof.job_id=?5
                AND proof.job_revision=?6 AND proof.job_digest=?7
                AND proof.capacity_claim_id=?8 AND proof.capacity_claim_revision=?9
                AND proof.capacity_claim_digest=?10 AND proof.budget_reservation_id=?11
                AND proof.budget_reserved_fen=?12
                AND command.reservation_id=proof.reservation_id
                AND command.command_digest=proof.command_digest
                AND command.job_id=proof.job_id
                AND command.capacity_claim_id=proof.capacity_claim_id
                AND command.budget_reservation_id=proof.budget_reservation_id
                AND prepare.command_id=proof.command_id
                AND prepare.plan_id=proof.plan_id AND prepare.plan_digest=proof.plan_digest
                AND prepare.provider_id=proof.provider_id
                AND prepare.adapter_id=proof.adapter_id
                AND prepare.adapter_binding_digest=proof.adapter_binding_digest
                AND route.adapter_revision=proof.adapter_revision
                AND route.adapter_registry_digest=proof.adapter_registry_digest
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_activations activation
                     WHERE activation.lease_id=proof.lease_id
                        OR activation.reservation_id=proof.reservation_id
                )
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_dispatch_applications application
                     WHERE application.command_id=proof.command_id
                        OR application.lease_id=proof.lease_id
                )
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_outbox commit_intent
                    JOIN compute_attempt_start_send_attempts send
                      ON send.outbox_id=commit_intent.outbox_id
                    WHERE commit_intent.command_id=proof.command_id
                      AND commit_intent.operation_kind='commit'
                )
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_remote_observations observation
                     WHERE observation.command_id=proof.command_id
                       AND observation.remote_execution_state IN (
                            'committed','running','terminal_after_run'
                       )
                )",
            params![
                command_id,
                binding.reservation_id,
                binding.reservation_revision,
                binding.reservation_digest,
                binding.job_id,
                binding.job_revision,
                binding.job_digest,
                binding.capacity_claim_id,
                binding.capacity_claim_revision,
                binding.capacity_claim_digest,
                binding.budget_reservation_id,
                binding.budget_refunded_fen,
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Broker finish requires an exact authoritative no-start proof"))?;
    let proof: ComputeStartNoStartProofEnvelope = serde_json::from_str(&json)?;
    let (canonical, digest) = canonical_start_no_start_proof_json_and_digest(&proof)?;
    ensure!(
        canonical == json
            && digest == proof.proof_digest
            && proof.lease_digest.is_none()
            && proof.reservation_id == binding.reservation_id
            && proof.reservation_revision == binding.reservation_revision
            && proof.reservation_digest == binding.reservation_digest
            && proof.job_id == binding.job_id
            && proof.job_revision == binding.job_revision
            && proof.job_digest == binding.job_digest
            && proof.capacity_claim_id == binding.capacity_claim_id
            && proof.capacity_claim_revision == binding.capacity_claim_revision
            && proof.capacity_claim_digest == binding.capacity_claim_digest
            && proof.budget_reservation_id == binding.budget_reservation_id
            && proof.budget_reserved_fen == binding.budget_refunded_fen
            && super::super::read::no_start_semantics_exact_on(connection, &proof)?,
        "Broker finish no-start proof failed canonical exact audit"
    );
    Ok(Some(StartResolutionProofReceipt {
        proof_id: proof.proof_id,
        proof_digest: proof.proof_digest,
    }))
}
