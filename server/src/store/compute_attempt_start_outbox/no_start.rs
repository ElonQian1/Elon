use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::start_outbox::{
    canonical_start_no_start_proof_json_and_digest, ComputeStartNoStartProofEnvelope,
};

use super::types::StartOutboxNoStartProofReceipt;

mod derive;
mod finish_gate;

pub(in crate::store) use derive::record_prepare_rejected_no_start_on;
pub(super) use derive::{record_remote_never_committed_no_start_on, recover_no_start_on};
pub(in crate::store) use finish_gate::ensure_start_resolved_for_broker_finish_on;

fn proof_by_command_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<ComputeStartNoStartProofEnvelope>> {
    let json = connection
        .query_row(
            "SELECT proof_json FROM compute_attempt_no_start_proofs WHERE command_id=?1",
            params![command_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(json) = json else { return Ok(None) };
    let proof: ComputeStartNoStartProofEnvelope = serde_json::from_str(&json)?;
    let (canonical, digest) = canonical_start_no_start_proof_json_and_digest(&proof)?;
    ensure!(
        canonical == json
            && digest == proof.proof_digest
            && proof.lease_digest.is_none()
            && proof.proven_at <= proof.recorded_at
            && super::read::no_start_semantics_exact_on(connection, &proof)?,
        "stored no-start proof failed canonical audit"
    );
    Ok(Some(proof))
}

fn persist_proof_on(
    connection: &Connection,
    proof: &ComputeStartNoStartProofEnvelope,
) -> Result<()> {
    let (json, digest) = canonical_start_no_start_proof_json_and_digest(proof)?;
    ensure!(
        digest == proof.proof_digest,
        "no-start proof digest mismatch"
    );
    connection.execute(
        "INSERT INTO compute_attempt_no_start_proofs (
            proof_id, proof_schema, proof_digest, proof_json, canonicalization,
            digest_algorithm, proof_kind, outbox_id, outbox_digest, command_id,
            command_digest, plan_id, plan_digest, provider_id, reservation_id,
            reservation_revision, reservation_digest, job_id, job_revision, job_digest,
            capacity_claim_id, capacity_claim_revision, capacity_claim_digest,
            budget_reservation_id, budget_reserved_fen, broker_request_digest,
            lease_id, lease_digest, fencing_generation, adapter_id, adapter_revision,
            adapter_registry_digest, adapter_binding_digest, route_authorization_id,
            route_authorization_digest, observation_id, observation_digest,
            no_commit_tombstone_id, no_commit_tombstone_digest, proven_at, recorded_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21,
            ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31,
            ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41
         )",
        params![
            proof.proof_id,
            proof.schema,
            proof.proof_digest,
            json,
            proof.canonicalization,
            proof.digest_algorithm,
            proof.proof_kind,
            proof.outbox_id,
            proof.outbox_digest,
            proof.command_id,
            proof.command_digest,
            proof.plan_id,
            proof.plan_digest,
            proof.provider_id,
            proof.reservation_id,
            proof.reservation_revision,
            proof.reservation_digest,
            proof.job_id,
            proof.job_revision,
            proof.job_digest,
            proof.capacity_claim_id,
            proof.capacity_claim_revision,
            proof.capacity_claim_digest,
            proof.budget_reservation_id,
            proof.budget_reserved_fen,
            proof.broker_request_digest,
            proof.lease_id,
            proof.lease_digest,
            proof.fencing_generation,
            proof.adapter_id,
            proof.adapter_revision,
            proof.adapter_registry_digest,
            proof.adapter_binding_digest,
            proof.route_authorization_id,
            proof.route_authorization_digest,
            proof.observation_id,
            proof.observation_digest,
            proof.no_commit_tombstone_id,
            proof.no_commit_tombstone_digest,
            proof.proven_at,
            proof.recorded_at,
        ],
    )?;
    Ok(())
}

fn proof_receipt(
    proof: &ComputeStartNoStartProofEnvelope,
    replayed: bool,
) -> StartOutboxNoStartProofReceipt {
    StartOutboxNoStartProofReceipt {
        proof_id: proof.proof_id.clone(),
        proof_digest: proof.proof_digest.clone(),
        proof_kind: proof.proof_kind.clone(),
        command_id: proof.command_id.clone(),
        replayed,
    }
}
