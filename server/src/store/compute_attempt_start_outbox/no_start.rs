use anyhow::{anyhow, bail, ensure, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::start_outbox::{
        canonical_start_no_start_proof_json_and_digest, ComputeStartNoStartProofEnvelope,
        COMPUTE_NO_START_PROOF_LOCAL_NEVER_SENT, COMPUTE_NO_START_PROOF_PREPARE_REJECTED,
        COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED, COMPUTE_START_NO_START_PROOF_SCHEMA,
        COMPUTE_START_OUTBOX_CANONICALIZATION, COMPUTE_START_OUTBOX_DIGEST_ALGORITHM,
    },
    store::new_id,
};

use super::types::{
    BrokerFinishStartResolutionBinding, NoStartProofSource, StartNoStartDerivation,
    StartOutboxNoStartProofReceipt, StartResolutionProofReceipt,
};

pub(super) fn derive_and_record_no_start_on(
    connection: &Connection,
    derivation: StartNoStartDerivation<'_>,
) -> Result<StartOutboxNoStartProofReceipt> {
    if let Some(stored) = proof_by_command_on(connection, derivation.command_id())? {
        ensure_derivation_matches(&stored, derivation)?;
        return Ok(proof_receipt(&stored, true));
    }
    let source = super::read::no_start_source_on(connection, derivation.command_id())?
        .ok_or_else(|| anyhow!("no-start derivation lacks an exact command and prepare closure"))?;
    let now = now_nanos();
    ensure!(
        derivation.proven_at() <= now.as_str(),
        "no-start proof cannot be recorded before it is proven"
    );
    let observation = match derivation {
        StartNoStartDerivation::LocalNeverSent { proven_at, .. } => {
            abandon_local_never_sent_on(connection, &source, proven_at)?;
            None
        }
        StartNoStartDerivation::PrepareRejected { observation_id, .. } => Some(
            exact_observation_on(connection, &source, observation_id, "prepare_rejected")?,
        ),
        StartNoStartDerivation::RemoteNeverCommitted { observation_id, .. } => {
            Some(exact_observation_on(
                connection,
                &source,
                observation_id,
                "remote_never_committed",
            )?)
        }
    };
    let (proof_kind, observation_id, observation_digest, tombstone_id, tombstone_digest) =
        match (derivation, observation) {
            (StartNoStartDerivation::LocalNeverSent { .. }, None) => (
                COMPUTE_NO_START_PROOF_LOCAL_NEVER_SENT,
                None,
                None,
                None,
                None,
            ),
            (StartNoStartDerivation::PrepareRejected { .. }, Some(value)) => (
                COMPUTE_NO_START_PROOF_PREPARE_REJECTED,
                Some(value.0),
                Some(value.1),
                None,
                None,
            ),
            (StartNoStartDerivation::RemoteNeverCommitted { .. }, Some(value)) => (
                COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED,
                Some(value.0),
                Some(value.1),
                value.2,
                value.3,
            ),
            _ => bail!("no-start proof derivation shape is inconsistent"),
        };
    let mut envelope = ComputeStartNoStartProofEnvelope {
        schema: COMPUTE_START_NO_START_PROOF_SCHEMA.to_string(),
        proof_id: new_id("start_no_start"),
        proof_digest: String::new(),
        canonicalization: COMPUTE_START_OUTBOX_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_START_OUTBOX_DIGEST_ALGORITHM.to_string(),
        proof_kind: proof_kind.to_string(),
        outbox_id: source.outbox_id,
        outbox_digest: source.outbox_digest,
        command_id: source.command_id,
        command_digest: source.command_digest,
        plan_id: source.plan_id,
        plan_digest: source.plan_digest,
        provider_id: source.provider_id,
        reservation_id: source.reservation_id,
        reservation_revision: source.reservation_revision,
        reservation_digest: source.reservation_digest,
        job_id: source.job_id,
        job_revision: source.job_revision,
        job_digest: source.job_digest,
        capacity_claim_id: source.capacity_claim_id,
        capacity_claim_revision: source.capacity_claim_revision,
        capacity_claim_digest: source.capacity_claim_digest,
        budget_reservation_id: source.budget_reservation_id,
        budget_reserved_fen: source.budget_reserved_fen,
        broker_request_digest: source.broker_request_digest,
        lease_id: source.lease_id,
        lease_digest: None,
        fencing_generation: source.fencing_generation,
        adapter_id: source.adapter_id,
        adapter_revision: source.adapter_revision,
        adapter_registry_digest: source.adapter_registry_digest,
        adapter_binding_digest: source.adapter_binding_digest,
        route_authorization_id: source.route_authorization_id,
        route_authorization_digest: source.route_authorization_digest,
        observation_id,
        observation_digest,
        no_commit_tombstone_id: tombstone_id,
        no_commit_tombstone_digest: tombstone_digest,
        proven_at: derivation.proven_at().to_string(),
        recorded_at: now,
    };
    let (_, digest) = canonical_start_no_start_proof_json_and_digest(&envelope)?;
    envelope.proof_digest = digest;
    persist_proof_on(connection, &envelope)?;
    let stored = proof_by_command_on(connection, &envelope.command_id)?
        .ok_or_else(|| anyhow!("no-start proof is not visible after insert"))?;
    ensure!(
        stored == envelope,
        "no-start proof failed exact durable readback"
    );
    Ok(proof_receipt(&stored, false))
}

pub(super) fn ensure_start_resolved_for_broker_finish_on(
    connection: &Connection,
    binding: BrokerFinishStartResolutionBinding<'_>,
) -> Result<Option<StartResolutionProofReceipt>> {
    let command_exists = connection
        .query_row(
            "SELECT 1 FROM compute_attempt_dispatch_commands
              WHERE reservation_id=?1 LIMIT 1",
            params![binding.reservation_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !command_exists {
        return Ok(None);
    }
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
              WHERE proof.reservation_id=?1 AND proof.reservation_revision=?2
                AND proof.reservation_digest=?3 AND proof.job_id=?4
                AND proof.job_revision=?5 AND proof.job_digest=?6
                AND proof.capacity_claim_id=?7 AND proof.capacity_claim_revision=?8
                AND proof.capacity_claim_digest=?9 AND proof.budget_reservation_id=?10
                AND proof.budget_reserved_fen=?11
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
            && super::read::no_start_semantics_exact_on(connection, &proof)?,
        "Broker finish no-start proof failed canonical exact audit"
    );
    Ok(Some(StartResolutionProofReceipt {
        proof_id: proof.proof_id,
        proof_digest: proof.proof_digest,
    }))
}

fn abandon_local_never_sent_on(
    connection: &Connection,
    source: &NoStartProofSource,
    proven_at: &str,
) -> Result<()> {
    ensure!(
        source.prepare_not_after.as_str() <= proven_at,
        "local-never-sent cannot be proven before the delivery window closes"
    );
    if source.prepare_state == "abandoned_no_send" {
        return Ok(());
    }
    ensure!(
        source.prepare_state == "pending"
            || (source.prepare_state == "claimed"
                && source
                    .prepare_claim_expires_at
                    .as_deref()
                    .is_some_and(|expiry| expiry <= proven_at)),
        "local-never-sent source is not safely abandonable"
    );
    let changed = connection.execute(
        "UPDATE compute_attempt_start_outbox
            SET state='abandoned_no_send', state_revision=state_revision+1,
                claim_owner_id=NULL, claim_token_digest=NULL, claim_expires_at=NULL,
                last_failure_code='DELIVERY_WINDOW_CLOSED_BEFORE_SEND', updated_at=?1
          WHERE outbox_id=?2 AND state=?3 AND state_revision=?4
            AND attempt_count=?5 AND claim_generation=?6
            AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_start_send_attempts send
                 WHERE send.outbox_id=compute_attempt_start_outbox.outbox_id
            )",
        params![
            proven_at,
            source.outbox_id,
            source.prepare_state,
            source.prepare_state_revision,
            source.prepare_attempt_count,
            source.prepare_claim_generation,
        ],
    )?;
    ensure!(
        changed == 1,
        "local-never-sent abandonment lost its exact CAS"
    );
    Ok(())
}

fn exact_observation_on(
    connection: &Connection,
    source: &NoStartProofSource,
    observation_id: &str,
    proof_kind: &str,
) -> Result<(String, String, Option<String>, Option<String>)> {
    let expected = match proof_kind {
        "prepare_rejected" => ("prepare_response", "rejected", "rejected", "final"),
        "remote_never_committed" => (
            "reconcile_attestation",
            "observed",
            "terminal_no_start",
            "final",
        ),
        _ => bail!("unsupported observation-backed no-start proof"),
    };
    connection
        .query_row(
            "SELECT observation_id, observation_digest,
                    no_commit_tombstone_id, no_commit_tombstone_digest
               FROM compute_attempt_start_remote_observations
              WHERE observation_id=?1 AND command_id=?2
                AND observation_kind=?3 AND response_outcome=?4
                AND remote_execution_state=?5 AND terminality=?6",
            params![
                observation_id,
                source.command_id,
                expected.0,
                expected.1,
                expected.2,
                expected.3
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("no-start derivation lacks exact authenticated observation"))
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
            && super::read::no_start_semantics_exact_on(connection, &proof)?,
        "stored no-start proof failed canonical audit"
    );
    Ok(Some(proof))
}

fn ensure_derivation_matches(
    stored: &ComputeStartNoStartProofEnvelope,
    derivation: StartNoStartDerivation<'_>,
) -> Result<()> {
    let kind = match derivation {
        StartNoStartDerivation::LocalNeverSent { .. } => COMPUTE_NO_START_PROOF_LOCAL_NEVER_SENT,
        StartNoStartDerivation::PrepareRejected { .. } => COMPUTE_NO_START_PROOF_PREPARE_REJECTED,
        StartNoStartDerivation::RemoteNeverCommitted { .. } => {
            COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED
        }
    };
    ensure!(
        stored.command_id == derivation.command_id()
            && stored.proof_kind == kind
            && stored.observation_id.as_deref() == derivation.observation_id()
            && stored.proven_at == derivation.proven_at(),
        "no-start proof replay conflicts with the requested derivation"
    );
    Ok(())
}

fn proof_receipt(
    proof: &ComputeStartNoStartProofEnvelope,
    replayed: bool,
) -> StartOutboxNoStartProofReceipt {
    StartOutboxNoStartProofReceipt {
        proof_id: proof.proof_id.clone(),
        proof_digest: proof.proof_digest.clone(),
        command_id: proof.command_id.clone(),
        replayed,
    }
}

fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
