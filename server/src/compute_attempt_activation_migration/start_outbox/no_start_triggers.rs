use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        ALTER TABLE compute_broker_finish_receipts
            ADD COLUMN start_resolution_proof_id TEXT
                REFERENCES compute_attempt_no_start_proofs(proof_id) ON DELETE RESTRICT;
        ALTER TABLE compute_broker_finish_receipts
            ADD COLUMN start_resolution_proof_digest TEXT;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_proof_projection
        BEFORE INSERT ON compute_attempt_no_start_proofs
        WHEN json_extract(NEW.proof_json,'$.schema') IS NOT NEW.proof_schema
          OR json_extract(NEW.proof_json,'$.proof_id') IS NOT NEW.proof_id
          OR json_extract(NEW.proof_json,'$.proof_digest') IS NOT NEW.proof_digest
          OR json_extract(NEW.proof_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.proof_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.proof_json,'$.proof_kind') IS NOT NEW.proof_kind
          OR json_extract(NEW.proof_json,'$.outbox_id') IS NOT NEW.outbox_id
          OR json_extract(NEW.proof_json,'$.outbox_digest') IS NOT NEW.outbox_digest
          OR json_extract(NEW.proof_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.proof_json,'$.command_digest') IS NOT NEW.command_digest
          OR json_extract(NEW.proof_json,'$.plan_id') IS NOT NEW.plan_id
          OR json_extract(NEW.proof_json,'$.plan_digest') IS NOT NEW.plan_digest
          OR json_extract(NEW.proof_json,'$.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.proof_json,'$.reservation_id') IS NOT NEW.reservation_id
          OR json_extract(NEW.proof_json,'$.reservation_revision')
                IS NOT NEW.reservation_revision
          OR json_extract(NEW.proof_json,'$.reservation_digest') IS NOT NEW.reservation_digest
          OR json_extract(NEW.proof_json,'$.job_id') IS NOT NEW.job_id
          OR json_extract(NEW.proof_json,'$.job_revision') IS NOT NEW.job_revision
          OR json_extract(NEW.proof_json,'$.job_digest') IS NOT NEW.job_digest
          OR json_extract(NEW.proof_json,'$.capacity_claim_id')
                IS NOT NEW.capacity_claim_id
          OR json_extract(NEW.proof_json,'$.capacity_claim_revision')
                IS NOT NEW.capacity_claim_revision
          OR json_extract(NEW.proof_json,'$.capacity_claim_digest')
                IS NOT NEW.capacity_claim_digest
          OR json_extract(NEW.proof_json,'$.budget_reservation_id')
                IS NOT NEW.budget_reservation_id
          OR json_extract(NEW.proof_json,'$.budget_reserved_fen')
                IS NOT NEW.budget_reserved_fen
          OR json_extract(NEW.proof_json,'$.broker_request_digest')
                IS NOT NEW.broker_request_digest
          OR json_extract(NEW.proof_json,'$.lease_id') IS NOT NEW.lease_id
          OR json_type(NEW.proof_json,'$.lease_digest') IS NOT 'null'
          OR NEW.lease_digest IS NOT NULL
          OR json_extract(NEW.proof_json,'$.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.proof_json,'$.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.proof_json,'$.adapter_revision') IS NOT NEW.adapter_revision
          OR json_extract(NEW.proof_json,'$.adapter_registry_digest')
                IS NOT NEW.adapter_registry_digest
          OR json_extract(NEW.proof_json,'$.adapter_binding_digest')
                IS NOT NEW.adapter_binding_digest
          OR json_extract(NEW.proof_json,'$.route_authorization_id')
                IS NOT NEW.route_authorization_id
          OR json_extract(NEW.proof_json,'$.route_authorization_digest')
                IS NOT NEW.route_authorization_digest
          OR json_type(NEW.proof_json,'$.observation_id') IS NULL
          OR json_extract(NEW.proof_json,'$.observation_id') IS NOT NEW.observation_id
          OR json_type(NEW.proof_json,'$.observation_digest') IS NULL
          OR json_extract(NEW.proof_json,'$.observation_digest')
                IS NOT NEW.observation_digest
          OR json_type(NEW.proof_json,'$.no_commit_tombstone_id') IS NULL
          OR json_extract(NEW.proof_json,'$.no_commit_tombstone_id')
                IS NOT NEW.no_commit_tombstone_id
          OR json_type(NEW.proof_json,'$.no_commit_tombstone_digest') IS NULL
          OR json_extract(NEW.proof_json,'$.no_commit_tombstone_digest')
                IS NOT NEW.no_commit_tombstone_digest
          OR json_extract(NEW.proof_json,'$.proven_at') IS NOT NEW.proven_at
          OR json_extract(NEW.proof_json,'$.recorded_at') IS NOT NEW.recorded_at
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt no-start proof projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_proof_exact
        BEFORE INSERT ON compute_attempt_no_start_proofs
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_commands command
              JOIN compute_attempt_start_outbox prepare
                ON prepare.command_id=command.command_id
               AND prepare.operation_kind='prepare'
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=prepare.route_authorization_id
               AND route.route_authorization_digest=prepare.route_authorization_digest
             WHERE command.command_id=NEW.command_id
               AND command.command_digest=NEW.command_digest
               AND command.execution_plan_id=NEW.plan_id
               AND command.execution_plan_digest=NEW.plan_digest
               AND command.reservation_id=NEW.reservation_id
               AND command.reservation_revision=NEW.reservation_revision
               AND command.reservation_digest=NEW.reservation_digest
               AND command.job_id=NEW.job_id
               AND command.job_revision=NEW.job_revision
               AND command.job_digest=NEW.job_digest
               AND command.capacity_claim_id=NEW.capacity_claim_id
               AND command.claim_revision=NEW.capacity_claim_revision
               AND command.claim_digest=NEW.capacity_claim_digest
               AND command.budget_reservation_id=NEW.budget_reservation_id
               AND command.budget_reserved_fen=NEW.budget_reserved_fen
               AND command.broker_request_digest=NEW.broker_request_digest
               AND command.lease_id=NEW.lease_id
               AND command.fencing_generation=NEW.fencing_generation
               AND command.provider_id=NEW.provider_id
               AND command.adapter_id=NEW.adapter_id
               AND command.adapter_binding_digest=NEW.adapter_binding_digest
               AND prepare.outbox_id=NEW.outbox_id
               AND prepare.outbox_digest=NEW.outbox_digest
               AND prepare.plan_id=NEW.plan_id AND prepare.plan_digest=NEW.plan_digest
               AND prepare.provider_id=NEW.provider_id
               AND prepare.adapter_id=NEW.adapter_id
               AND prepare.adapter_binding_digest=NEW.adapter_binding_digest
               AND route.adapter_revision=NEW.adapter_revision
               AND route.adapter_registry_digest=NEW.adapter_registry_digest
               AND route.route_authorization_id=NEW.route_authorization_id
               AND route.route_authorization_digest=NEW.route_authorization_digest
               AND NEW.proven_at<=NEW.recorded_at
               AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_activations activation
                     WHERE activation.lease_id=NEW.lease_id
                        OR activation.reservation_id=NEW.reservation_id
               )
               AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_dispatch_applications application
                     WHERE application.command_id=NEW.command_id
                        OR application.lease_id=NEW.lease_id
               )
               AND NOT EXISTS (
                    SELECT 1
                      FROM compute_attempt_start_outbox commit_intent
                      JOIN compute_attempt_start_send_attempts attempt
                        ON attempt.outbox_id=commit_intent.outbox_id
                     WHERE commit_intent.command_id=NEW.command_id
                       AND commit_intent.operation_kind='commit'
               )
               AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_remote_observations contradiction
                     WHERE contradiction.command_id=NEW.command_id
                       AND contradiction.remote_execution_state IN (
                            'committed','running','terminal_after_run'
                       )
               )
               AND (
                    (NEW.proof_kind='local_never_sent'
                        AND prepare.state='abandoned_no_send'
                        AND prepare.not_after<=NEW.proven_at
                        AND NOT EXISTS (
                            SELECT 1 FROM compute_attempt_start_send_attempts attempt
                             WHERE attempt.outbox_id=prepare.outbox_id
                        )
                        AND NOT EXISTS (
                            SELECT 1 FROM compute_attempt_start_remote_observations observation
                             WHERE observation.command_id=NEW.command_id
                        )
                        AND NOT EXISTS (
                            SELECT 1 FROM compute_attempt_dispatch_acks ack
                             WHERE ack.command_id=NEW.command_id
                        ))
                    OR (NEW.proof_kind='prepare_rejected' AND EXISTS (
                        SELECT 1
                          FROM compute_attempt_start_remote_observations observation
                          JOIN compute_attempt_dispatch_acks ack
                            ON ack.command_id=observation.command_id
                         WHERE observation.observation_id=NEW.observation_id
                           AND observation.observation_digest=NEW.observation_digest
                           AND observation.outbox_id=NEW.outbox_id
                           AND observation.command_id=NEW.command_id
                           AND observation.observation_kind='prepare_response'
                           AND observation.response_outcome='rejected'
                           AND observation.remote_execution_state='rejected'
                           AND observation.terminality='final'
                           AND observation.recorded_at<=NEW.proven_at
                           AND ack.outcome='rejected' AND ack.disposition='rejected'
                           AND ack.adapter_ack_id=observation.adapter_observation_id
                    ))
                    OR (NEW.proof_kind='remote_never_committed' AND EXISTS (
                        SELECT 1
                          FROM compute_attempt_start_remote_observations observation
                          JOIN compute_attempt_start_outbox reconcile
                            ON reconcile.outbox_id=observation.outbox_id
                          JOIN compute_attempt_start_outbox cancel
                            ON cancel.outbox_id=reconcile.subject_outbox_id
                         WHERE observation.observation_id=NEW.observation_id
                           AND observation.observation_digest=NEW.observation_digest
                           AND observation.command_id=NEW.command_id
                           AND observation.observation_kind='reconcile_attestation'
                           AND observation.remote_execution_state='terminal_no_start'
                           AND observation.terminality='final'
                           AND observation.no_commit_tombstone_id=NEW.no_commit_tombstone_id
                           AND observation.no_commit_tombstone_digest=NEW.no_commit_tombstone_digest
                           AND observation.recorded_at<=NEW.proven_at
                           AND reconcile.operation_kind='reconcile'
                           AND cancel.operation_kind='cancel'
                           AND cancel.subject_outbox_id=prepare.outbox_id
                    ))
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt no-start proof is not authoritative');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_blocks_send
        BEFORE INSERT ON compute_attempt_start_send_attempts
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_no_start_proofs proof
             WHERE proof.command_id=NEW.command_id
        )
        BEGIN
            SELECT RAISE(ABORT, 'no-start proof forbids later Attempt sends');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_blocks_acceptance
        BEFORE INSERT ON compute_attempt_dispatch_acks
        WHEN NEW.outcome='accepted' AND EXISTS (
            SELECT 1 FROM compute_attempt_no_start_proofs proof
             WHERE proof.command_id=NEW.command_id
        )
        BEGIN
            SELECT RAISE(ABORT, 'no-start proof forbids later accepted ACK');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_blocks_activation
        BEFORE INSERT ON compute_attempt_activations
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_no_start_proofs proof
             WHERE proof.lease_id=NEW.lease_id OR proof.reservation_id=NEW.reservation_id
        )
        BEGIN
            SELECT RAISE(ABORT, 'no-start proof forbids later activation');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_blocks_application
        BEFORE INSERT ON compute_attempt_dispatch_applications
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_no_start_proofs proof
             WHERE proof.command_id=NEW.command_id OR proof.lease_id=NEW.lease_id
        )
        BEGIN
            SELECT RAISE(ABORT, 'no-start proof forbids later dispatch application');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_blocks_contradiction
        BEFORE INSERT ON compute_attempt_start_remote_observations
        WHEN NEW.remote_execution_state IN ('prepared','committed','running','terminal_after_run')
          AND EXISTS (
                SELECT 1 FROM compute_attempt_no_start_proofs proof
                 WHERE proof.command_id=NEW.command_id
          )
        BEGIN
            SELECT RAISE(ABORT, 'remote observation contradicts immutable no-start proof');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_broker_finish_start_resolution_v213
        BEFORE INSERT ON compute_broker_finish_receipts
        WHEN (
            NOT EXISTS (
                SELECT 1 FROM compute_attempt_dispatch_commands command
                 WHERE command.reservation_id=NEW.reservation_id
            )
            AND (NEW.start_resolution_proof_id IS NOT NULL
                OR NEW.start_resolution_proof_digest IS NOT NULL)
        ) OR (
            EXISTS (
                SELECT 1 FROM compute_attempt_dispatch_commands command
                 WHERE command.reservation_id=NEW.reservation_id
            )
            AND NOT EXISTS (
                SELECT 1
                  FROM compute_attempt_no_start_proofs proof
                  JOIN compute_attempt_dispatch_commands command
                    ON command.command_id=proof.command_id
                 WHERE proof.proof_id=NEW.start_resolution_proof_id
                   AND proof.proof_digest=NEW.start_resolution_proof_digest
                   AND proof.reservation_id=NEW.reservation_id
                   AND proof.reservation_revision=NEW.source_reservation_revision
                   AND proof.reservation_digest=NEW.source_reservation_digest
                   AND proof.job_id=NEW.job_id
                   AND proof.job_revision=NEW.source_job_revision
                   AND proof.job_digest=NEW.source_job_digest
                   AND proof.capacity_claim_id=NEW.source_claim_id
                   AND proof.capacity_claim_revision=NEW.source_claim_revision
                   AND proof.capacity_claim_digest=NEW.source_claim_digest
                   AND proof.budget_reservation_id=NEW.budget_reservation_id
                   AND proof.budget_reserved_fen=NEW.budget_refunded_fen
                   AND command.command_digest=proof.command_digest
                   AND command.reservation_id=NEW.reservation_id
                   AND command.job_id=NEW.job_id
                   AND command.capacity_claim_id=NEW.source_claim_id
                   AND command.budget_reservation_id=NEW.budget_reservation_id
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_activations activation
                         WHERE activation.lease_id=proof.lease_id
                            OR activation.reservation_id=NEW.reservation_id
                   )
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_dispatch_applications application
                         WHERE application.command_id=proof.command_id
                   )
            )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute broker finish requires exact no-start proof');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_proofs_no_update
        BEFORE UPDATE ON compute_attempt_no_start_proofs
        BEGIN SELECT RAISE(ABORT, 'compute attempt no-start proofs are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_proofs_no_delete
        BEFORE DELETE ON compute_attempt_no_start_proofs
        BEGIN SELECT RAISE(ABORT, 'compute attempt no-start proofs are append-only'); END;
        "#,
    )?;
    Ok(())
}
