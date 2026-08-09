use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_start_outbox_subject_v214
            ON compute_attempt_start_outbox(subject_outbox_id, operation_kind, outbox_id);
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_start_send_command_v214
            ON compute_attempt_start_send_attempts(command_id, operation_kind, outbox_id);
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_start_observation_proof_v214
            ON compute_attempt_start_remote_observations(
                command_id, observation_kind, response_outcome,
                remote_execution_state, observation_id
            );
        CREATE INDEX IF NOT EXISTS idx_compute_attempt_start_observation_outbox_v214
            ON compute_attempt_start_remote_observations(
                outbox_id, observation_kind, send_attempt_id
            );

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_unknown_cleanup_source_v214
        BEFORE INSERT ON compute_attempt_start_outbox
        WHEN NEW.operation_kind IN ('cancel','reconcile')
          AND NEW.ack_id IS NULL
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
             WHERE NEW.operation_generation=1
               AND prepare.operation_kind='prepare'
               AND prepare.operation_generation=1
               AND prepare.state='in_flight_unknown'
               AND prepare.claim_expires_at<=NEW.issued_at
               AND NEW.issued_at=NEW.created_at
               AND NEW.issued_at=NEW.not_before
               AND NEW.issued_at<route.cleanup_expires_at
               AND NEW.not_after<=route.cleanup_expires_at
               AND NEW.command_id=prepare.command_id
               AND NEW.command_digest=prepare.command_digest
               AND NEW.provider_id=prepare.provider_id
               AND NEW.adapter_id=prepare.adapter_id
               AND NEW.adapter_binding_digest=prepare.adapter_binding_digest
               AND NEW.route_authorization_id=prepare.route_authorization_id
               AND NEW.route_authorization_digest=prepare.route_authorization_digest
               AND NEW.actor_receipt_id=prepare.actor_receipt_id
               AND NEW.actor_receipt_digest=prepare.actor_receipt_digest
               AND NEW.plan_id=prepare.plan_id AND NEW.plan_digest=prepare.plan_digest
               AND NEW.lease_id=prepare.lease_id
               AND NEW.fencing_generation=prepare.fencing_generation
               AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_dispatch_acks ack
                     WHERE ack.command_id=prepare.command_id
               )
               AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_remote_observations observation
                     WHERE observation.command_id=prepare.command_id
                       AND observation.operation_kind='prepare'
                       AND observation.observation_kind='prepare_response'
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
                       AND commit_send.command_id=commit_intent.command_id
                       AND commit_send.command_digest=commit_intent.command_digest
                     WHERE commit_intent.command_id=prepare.command_id
                       AND commit_intent.operation_kind='commit'
               )
               AND EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities capability
                     WHERE capability.route_authorization_id=route.route_authorization_id
                       AND capability.capability_id='cancel_no_start'
               )
               AND EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities capability
                     WHERE capability.route_authorization_id=route.route_authorization_id
                       AND capability.capability_id='reconcile'
               )
               AND (
                    (NEW.operation_kind='cancel'
                        AND NEW.subject_outbox_id=prepare.outbox_id)
                    OR (NEW.operation_kind='reconcile' AND EXISTS (
                        SELECT 1 FROM compute_attempt_start_outbox cancel
                         WHERE cancel.outbox_id=NEW.subject_outbox_id
                           AND cancel.operation_kind='cancel'
                           AND cancel.operation_generation=1
                           AND cancel.subject_outbox_id=prepare.outbox_id
                           AND cancel.ack_id IS NULL AND cancel.ack_digest IS NULL
                           AND cancel.command_id=NEW.command_id
                           AND cancel.command_digest=NEW.command_digest
                           AND cancel.provider_id=NEW.provider_id
                           AND cancel.adapter_id=NEW.adapter_id
                           AND cancel.adapter_binding_digest=NEW.adapter_binding_digest
                           AND cancel.route_authorization_id=NEW.route_authorization_id
                           AND cancel.route_authorization_digest=NEW.route_authorization_digest
                           AND cancel.actor_receipt_id=NEW.actor_receipt_id
                           AND cancel.actor_receipt_digest=NEW.actor_receipt_digest
                           AND cancel.plan_id=NEW.plan_id
                           AND cancel.plan_digest=NEW.plan_digest
                           AND cancel.lease_id=NEW.lease_id
                           AND cancel.fencing_generation=NEW.fencing_generation
                           AND cancel.issued_at=NEW.issued_at
                           AND cancel.not_before=NEW.not_before
                           AND cancel.not_after=NEW.not_after
                    ))
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'unknown delivery cleanup lacks exact prepare source');
        END;
        "#,
    )?;
    Ok(())
}
