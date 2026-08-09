use anyhow::{bail, Result};
use rusqlite::Connection;

pub(super) fn ensure_no_unsafe_backfill(conn: &Connection) -> Result<()> {
    let unsafe_cleanup =
        conn.query_row(UNKNOWN_CLEANUP_BACKFILL, [], |row| row.get::<_, bool>(0))?;
    let unsafe_proof = conn.query_row(REMOTE_PROOF_BACKFILL, [], |row| row.get::<_, bool>(0))?;
    if unsafe_cleanup || unsafe_proof {
        bail!("COMPUTE_START_RECOVERY_BACKFILL_REQUIRED");
    }
    Ok(())
}

const UNKNOWN_CLEANUP_BACKFILL: &str = r#"
    SELECT EXISTS(
        SELECT 1 FROM compute_attempt_start_outbox cleanup
         WHERE cleanup.operation_kind IN ('cancel','reconcile')
           AND cleanup.ack_id IS NULL
           AND NOT EXISTS (
                SELECT 1
                  FROM compute_attempt_start_outbox prepare
                  JOIN compute_attempt_dispatch_commands command
                    ON command.command_id=prepare.command_id
                   AND command.command_digest=prepare.command_digest
                  JOIN compute_attempt_start_send_attempts prepare_send
                    ON prepare_send.outbox_id=prepare.outbox_id
                   AND prepare_send.outbox_digest=prepare.outbox_digest
                   AND prepare_send.operation_kind='prepare'
                   AND prepare_send.command_id=prepare.command_id
                   AND prepare_send.command_digest=prepare.command_digest
                   AND prepare_send.route_authorization_id=prepare.route_authorization_id
                   AND prepare_send.route_authorization_digest=prepare.route_authorization_digest
                   AND prepare_send.attempt_no=prepare.attempt_count
                   AND prepare_send.claim_generation=prepare.claim_generation
                   AND prepare_send.claim_token_digest=prepare.claim_token_digest
                  JOIN compute_route_authorization_receipts route
                    ON route.route_authorization_id=prepare.route_authorization_id
                   AND route.route_authorization_digest=prepare.route_authorization_digest
                 WHERE cleanup.operation_generation=1
                   AND prepare.operation_kind='prepare'
                   AND prepare.operation_generation=1
                   AND prepare.state='in_flight_unknown'
                   AND prepare.claim_expires_at<=cleanup.issued_at
                   AND cleanup.issued_at=cleanup.created_at
                   AND cleanup.issued_at=cleanup.not_before
                   AND cleanup.issued_at<route.cleanup_expires_at
                   AND cleanup.not_after<=route.cleanup_expires_at
                   AND cleanup.command_id=prepare.command_id
                   AND cleanup.command_digest=prepare.command_digest
                   AND cleanup.provider_id=prepare.provider_id
                   AND cleanup.adapter_id=prepare.adapter_id
                   AND cleanup.adapter_binding_digest=prepare.adapter_binding_digest
                   AND cleanup.route_authorization_id=prepare.route_authorization_id
                   AND cleanup.route_authorization_digest=prepare.route_authorization_digest
                   AND cleanup.actor_receipt_id=prepare.actor_receipt_id
                   AND cleanup.actor_receipt_digest=prepare.actor_receipt_digest
                   AND cleanup.plan_id=prepare.plan_id
                   AND cleanup.plan_digest=prepare.plan_digest
                   AND cleanup.lease_id=prepare.lease_id
                   AND cleanup.fencing_generation=prepare.fencing_generation
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_dispatch_acks ack
                         WHERE ack.command_id=prepare.command_id
                   )
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_start_remote_observations response
                         WHERE response.outbox_id=prepare.outbox_id
                           AND response.operation_kind='prepare'
                           AND response.observation_kind='prepare_response'
                   )
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_activations activation
                         WHERE activation.lease_id=prepare.lease_id
                            OR activation.reservation_id=command.reservation_id
                   )
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_dispatch_applications application
                         WHERE application.command_id=prepare.command_id
                            OR application.lease_id=prepare.lease_id
                   )
                   AND NOT EXISTS (
                        SELECT 1
                          FROM compute_attempt_start_outbox commit_intent
                          JOIN compute_attempt_start_send_attempts commit_send
                            ON commit_send.outbox_id=commit_intent.outbox_id
                           AND commit_send.outbox_digest=commit_intent.outbox_digest
                           AND commit_send.operation_kind='commit'
                         WHERE commit_intent.command_id=prepare.command_id
                           AND commit_intent.operation_kind='commit'
                   )
                   AND ((cleanup.operation_kind='cancel'
                            AND cleanup.subject_outbox_id=prepare.outbox_id)
                        OR (cleanup.operation_kind='reconcile' AND EXISTS (
                            SELECT 1 FROM compute_attempt_start_outbox cancel
                             WHERE cancel.outbox_id=cleanup.subject_outbox_id
                               AND cancel.operation_kind='cancel'
                               AND cancel.operation_generation=1
                               AND cancel.subject_outbox_id=prepare.outbox_id
                               AND cancel.ack_id IS NULL AND cancel.ack_digest IS NULL
                               AND cancel.command_id=cleanup.command_id
                               AND cancel.command_digest=cleanup.command_digest
                               AND cancel.provider_id=cleanup.provider_id
                               AND cancel.adapter_id=cleanup.adapter_id
                               AND cancel.adapter_binding_digest=
                                    cleanup.adapter_binding_digest
                               AND cancel.route_authorization_id=
                                    cleanup.route_authorization_id
                               AND cancel.route_authorization_digest=
                                    cleanup.route_authorization_digest
                               AND cancel.actor_receipt_id=cleanup.actor_receipt_id
                               AND cancel.actor_receipt_digest=cleanup.actor_receipt_digest
                               AND cancel.plan_id=cleanup.plan_id
                               AND cancel.plan_digest=cleanup.plan_digest
                               AND cancel.lease_id=cleanup.lease_id
                               AND cancel.fencing_generation=cleanup.fencing_generation
                               AND cancel.issued_at=cleanup.issued_at
                               AND cancel.not_before=cleanup.not_before
                               AND cancel.not_after=cleanup.not_after
                        )))
           )
    )
"#;

const REMOTE_PROOF_BACKFILL: &str = r#"
    SELECT EXISTS(
        SELECT 1 FROM compute_attempt_no_start_proofs proof
         WHERE proof.proof_kind='remote_never_committed'
           AND NOT EXISTS (
                SELECT 1
                  FROM compute_attempt_start_outbox prepare
                  JOIN compute_attempt_dispatch_commands command
                    ON command.command_id=prepare.command_id
                   AND command.command_digest=prepare.command_digest
                  JOIN compute_attempt_start_send_attempts prepare_send
                    ON prepare_send.outbox_id=prepare.outbox_id
                   AND prepare_send.outbox_digest=prepare.outbox_digest
                   AND prepare_send.operation_kind='prepare'
                   AND prepare_send.command_id=prepare.command_id
                   AND prepare_send.command_digest=prepare.command_digest
                   AND prepare_send.route_authorization_id=prepare.route_authorization_id
                   AND prepare_send.route_authorization_digest=prepare.route_authorization_digest
                   AND prepare_send.attempt_no=prepare.attempt_count
                   AND prepare_send.claim_generation=prepare.claim_generation
                  JOIN compute_attempt_start_outbox cancel
                    ON cancel.subject_outbox_id=prepare.outbox_id
                   AND cancel.operation_kind='cancel'
                  JOIN compute_attempt_start_send_attempts cancel_send
                    ON cancel_send.outbox_id=cancel.outbox_id
                   AND cancel_send.outbox_digest=cancel.outbox_digest
                   AND cancel_send.operation_kind='cancel'
                   AND cancel_send.command_id=cancel.command_id
                   AND cancel_send.command_digest=cancel.command_digest
                   AND cancel_send.route_authorization_id=cancel.route_authorization_id
                   AND cancel_send.route_authorization_digest=cancel.route_authorization_digest
                   AND cancel_send.attempt_no=cancel.attempt_count
                   AND cancel_send.claim_generation=cancel.claim_generation
                  JOIN compute_attempt_start_remote_observations cancel_response
                    ON cancel_response.send_attempt_id=cancel_send.send_attempt_id
                   AND cancel_response.outbox_id=cancel.outbox_id
                   AND cancel_response.outbox_digest=cancel.outbox_digest
                  JOIN compute_attempt_start_outbox reconcile
                    ON reconcile.subject_outbox_id=cancel.outbox_id
                   AND reconcile.operation_kind='reconcile'
                  JOIN compute_attempt_start_send_attempts reconcile_send
                    ON reconcile_send.outbox_id=reconcile.outbox_id
                   AND reconcile_send.outbox_digest=reconcile.outbox_digest
                   AND reconcile_send.operation_kind='reconcile'
                   AND reconcile_send.command_id=reconcile.command_id
                   AND reconcile_send.command_digest=reconcile.command_digest
                   AND reconcile_send.route_authorization_id=
                        reconcile.route_authorization_id
                   AND reconcile_send.route_authorization_digest=
                        reconcile.route_authorization_digest
                   AND reconcile_send.attempt_no=reconcile.attempt_count
                   AND reconcile_send.claim_generation=reconcile.claim_generation
                  JOIN compute_attempt_start_remote_observations observation
                    ON observation.send_attempt_id=reconcile_send.send_attempt_id
                   AND observation.outbox_id=reconcile.outbox_id
                   AND observation.outbox_digest=reconcile.outbox_digest
                  JOIN compute_attempt_dispatch_acks ack
                    ON ack.ack_id=cancel.ack_id AND ack.ack_digest=cancel.ack_digest
                  JOIN compute_route_authorization_receipts route
                    ON route.route_authorization_id=prepare.route_authorization_id
                   AND route.route_authorization_digest=prepare.route_authorization_digest
                 WHERE prepare.outbox_id=proof.outbox_id
                   AND prepare.outbox_digest=proof.outbox_digest
                   AND prepare.operation_kind='prepare'
                   AND prepare.operation_generation=1
                   AND prepare.command_id=proof.command_id
                   AND prepare.command_digest=proof.command_digest
                   AND prepare.provider_id=proof.provider_id
                   AND prepare.adapter_id=proof.adapter_id
                   AND prepare.adapter_binding_digest=proof.adapter_binding_digest
                   AND prepare.route_authorization_id=proof.route_authorization_id
                   AND prepare.route_authorization_digest=proof.route_authorization_digest
                   AND prepare.plan_id=proof.plan_id AND prepare.plan_digest=proof.plan_digest
                   AND prepare.lease_id=proof.lease_id
                   AND prepare.fencing_generation=proof.fencing_generation
                   AND cancel.operation_generation=1
                   AND cancel.command_id=prepare.command_id
                   AND cancel.command_digest=prepare.command_digest
                   AND cancel.provider_id=prepare.provider_id
                   AND cancel.adapter_id=prepare.adapter_id
                   AND cancel.adapter_binding_digest=prepare.adapter_binding_digest
                   AND cancel.route_authorization_id=prepare.route_authorization_id
                   AND cancel.route_authorization_digest=prepare.route_authorization_digest
                   AND cancel.actor_receipt_id=prepare.actor_receipt_id
                   AND cancel.actor_receipt_digest=prepare.actor_receipt_digest
                   AND cancel.plan_id=prepare.plan_id
                   AND cancel.plan_digest=prepare.plan_digest
                   AND cancel.lease_id=prepare.lease_id
                   AND cancel.fencing_generation=prepare.fencing_generation
                   AND cancel.state='delivery_observed'
                   AND reconcile.operation_generation=1
                   AND reconcile.command_id=prepare.command_id
                   AND reconcile.command_digest=prepare.command_digest
                   AND reconcile.provider_id=prepare.provider_id
                   AND reconcile.adapter_id=prepare.adapter_id
                   AND reconcile.adapter_binding_digest=prepare.adapter_binding_digest
                   AND reconcile.route_authorization_id=prepare.route_authorization_id
                   AND reconcile.route_authorization_digest=prepare.route_authorization_digest
                   AND reconcile.actor_receipt_id=prepare.actor_receipt_id
                   AND reconcile.actor_receipt_digest=prepare.actor_receipt_digest
                   AND reconcile.plan_id=prepare.plan_id
                   AND reconcile.plan_digest=prepare.plan_digest
                   AND reconcile.lease_id=prepare.lease_id
                   AND reconcile.fencing_generation=prepare.fencing_generation
                   AND reconcile.state='delivery_observed'
                   AND reconcile.issued_at=cancel.issued_at
                   AND reconcile.not_before=cancel.not_before
                   AND reconcile.not_after=cancel.not_after
                   AND cancel.issued_at=cancel.created_at
                   AND reconcile.issued_at=reconcile.created_at
                   AND cancel.issued_at<route.cleanup_expires_at
                   AND cancel.not_after<=route.cleanup_expires_at
                   AND cancel.ack_id=reconcile.ack_id
                   AND cancel.ack_digest=reconcile.ack_digest
                   AND ack.command_id=prepare.command_id
                   AND ack.command_digest=prepare.command_digest
                   AND ack.provider_id=prepare.provider_id
                   AND ack.adapter_id=prepare.adapter_id
                   AND ack.adapter_binding_digest=prepare.adapter_binding_digest
                   AND ack.disposition='quarantined'
                   AND cancel_response.operation_kind='cancel'
                   AND cancel_response.observation_kind='cancel_response'
                   AND cancel_response.command_id=prepare.command_id
                   AND cancel_response.command_digest=prepare.command_digest
                   AND cancel_response.provider_id=prepare.provider_id
                   AND cancel_response.adapter_id=prepare.adapter_id
                   AND cancel_response.adapter_binding_digest=prepare.adapter_binding_digest
                   AND cancel_response.verification_kind=route.verification_kind
                   AND cancel_response.verifier_id=route.verifier_id
                   AND cancel_response.verification_digest=route.verifier_digest
                   AND cancel_response.recorded_at<=proof.proven_at
                   AND observation.observation_id=proof.observation_id
                   AND observation.observation_digest=proof.observation_digest
                   AND observation.operation_kind='reconcile'
                   AND observation.observation_kind='reconcile_attestation'
                   AND observation.command_id=prepare.command_id
                   AND observation.command_digest=prepare.command_digest
                   AND observation.provider_id=prepare.provider_id
                   AND observation.adapter_id=prepare.adapter_id
                   AND observation.adapter_binding_digest=prepare.adapter_binding_digest
                   AND observation.response_outcome='observed'
                   AND observation.remote_execution_state='terminal_no_start'
                   AND observation.terminality='final'
                   AND length(trim(observation.no_commit_tombstone_id)) BETWEEN 1 AND 160
                   AND length(observation.no_commit_tombstone_digest)=64
                   AND observation.no_commit_tombstone_id=proof.no_commit_tombstone_id
                   AND observation.no_commit_tombstone_digest=
                        proof.no_commit_tombstone_digest
                   AND observation.verification_kind=route.verification_kind
                   AND observation.verifier_id=route.verifier_id
                   AND observation.verification_digest=route.verifier_digest
                   AND observation.recorded_at<=proof.proven_at
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_activations activation
                         WHERE activation.lease_id=prepare.lease_id
                            OR activation.reservation_id=command.reservation_id
                   )
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_dispatch_applications application
                         WHERE application.command_id=prepare.command_id
                            OR application.lease_id=prepare.lease_id
                   )
                   AND NOT EXISTS (
                        SELECT 1
                          FROM compute_attempt_start_outbox commit_intent
                          JOIN compute_attempt_start_send_attempts commit_send
                            ON commit_send.outbox_id=commit_intent.outbox_id
                           AND commit_send.outbox_digest=commit_intent.outbox_digest
                           AND commit_send.operation_kind='commit'
                         WHERE commit_intent.command_id=prepare.command_id
                           AND commit_intent.operation_kind='commit'
                   )
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_start_remote_observations contradiction
                         WHERE contradiction.command_id=prepare.command_id
                           AND contradiction.remote_execution_state IN (
                                'committed','running','terminal_after_run'
                           )
                   )
           )
    )
"#;
