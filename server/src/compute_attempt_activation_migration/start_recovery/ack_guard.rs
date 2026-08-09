use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS trg_compute_attempt_quarantined_ack_cleanup_v213;
        CREATE TRIGGER trg_compute_attempt_quarantined_ack_cleanup_v213
        BEFORE INSERT ON compute_attempt_dispatch_acks
        WHEN NEW.disposition='quarantined' AND NOT EXISTS (
            SELECT 1
              FROM compute_attempt_start_outbox prepare
              JOIN compute_attempt_dispatch_commands command
                ON command.command_id=prepare.command_id
               AND command.command_digest=prepare.command_digest
              JOIN compute_attempt_start_outbox cancel
                ON cancel.subject_outbox_id=prepare.outbox_id
               AND cancel.operation_kind='cancel'
              JOIN compute_attempt_start_outbox reconcile
                ON reconcile.subject_outbox_id=cancel.outbox_id
               AND reconcile.operation_kind='reconcile'
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=prepare.route_authorization_id
               AND route.route_authorization_digest=prepare.route_authorization_digest
             WHERE prepare.command_id=NEW.command_id
               AND prepare.command_digest=NEW.command_digest
               AND prepare.operation_kind='prepare'
               AND prepare.operation_generation=1
               AND prepare.provider_id=NEW.provider_id
               AND prepare.adapter_id=NEW.adapter_id
               AND prepare.adapter_binding_digest=NEW.adapter_binding_digest
               AND cancel.operation_generation=1
               AND cancel.command_id=prepare.command_id
               AND cancel.command_digest=prepare.command_digest
               AND cancel.provider_id=prepare.provider_id
               AND cancel.adapter_id=prepare.adapter_id
               AND cancel.adapter_binding_digest=prepare.adapter_binding_digest
               AND cancel.plan_id=prepare.plan_id
               AND cancel.plan_digest=prepare.plan_digest
               AND cancel.lease_id=prepare.lease_id
               AND cancel.fencing_generation=prepare.fencing_generation
               AND cancel.actor_receipt_id=prepare.actor_receipt_id
               AND cancel.actor_receipt_digest=prepare.actor_receipt_digest
               AND cancel.route_authorization_id=prepare.route_authorization_id
               AND cancel.route_authorization_digest=prepare.route_authorization_digest
               AND reconcile.operation_generation=1
               AND reconcile.command_id=prepare.command_id
               AND reconcile.command_digest=prepare.command_digest
               AND reconcile.provider_id=prepare.provider_id
               AND reconcile.adapter_id=prepare.adapter_id
               AND reconcile.adapter_binding_digest=prepare.adapter_binding_digest
               AND reconcile.plan_id=prepare.plan_id
               AND reconcile.plan_digest=prepare.plan_digest
               AND reconcile.lease_id=prepare.lease_id
               AND reconcile.fencing_generation=prepare.fencing_generation
               AND reconcile.actor_receipt_id=prepare.actor_receipt_id
               AND reconcile.actor_receipt_digest=prepare.actor_receipt_digest
               AND reconcile.route_authorization_id=prepare.route_authorization_id
               AND reconcile.route_authorization_digest=prepare.route_authorization_digest
               AND reconcile.issued_at=cancel.issued_at
               AND reconcile.not_before=cancel.not_before
               AND reconcile.not_after=cancel.not_after
               AND cancel.issued_at=cancel.created_at
               AND reconcile.issued_at=reconcile.created_at
               AND cancel.issued_at<=NEW.created_at
               AND cancel.issued_at<route.cleanup_expires_at
               AND cancel.not_after<=route.cleanup_expires_at
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
               AND (
                    (cancel.ack_id=NEW.ack_id AND cancel.ack_digest=NEW.ack_digest
                        AND reconcile.ack_id=NEW.ack_id
                        AND reconcile.ack_digest=NEW.ack_digest
                        AND cancel.state='pending' AND cancel.state_revision=1
                        AND cancel.attempt_count=0 AND cancel.claim_generation=0
                        AND reconcile.state='blocked' AND reconcile.state_revision=1
                        AND reconcile.attempt_count=0 AND reconcile.claim_generation=0
                        AND cancel.issued_at=NEW.created_at
                        AND cancel.not_before=NEW.created_at)
                    OR (cancel.ack_id IS NULL AND cancel.ack_digest IS NULL
                        AND reconcile.ack_id IS NULL AND reconcile.ack_digest IS NULL
                        AND prepare.state='delivery_observed'
                        AND EXISTS (
                            SELECT 1 FROM compute_attempt_start_send_attempts prepare_send
                             WHERE prepare_send.outbox_id=prepare.outbox_id
                               AND prepare_send.outbox_digest=prepare.outbox_digest
                               AND prepare_send.operation_kind='prepare'
                               AND prepare_send.command_id=prepare.command_id
                               AND prepare_send.command_digest=prepare.command_digest
                               AND prepare_send.route_authorization_id=
                                    prepare.route_authorization_id
                               AND prepare_send.route_authorization_digest=
                                    prepare.route_authorization_digest
                               AND prepare_send.attempt_no=prepare.attempt_count
                               AND prepare_send.claim_generation=prepare.claim_generation
                        )
                        AND EXISTS (
                            SELECT 1
                              FROM compute_attempt_start_remote_observations observation
                              JOIN compute_attempt_start_send_attempts observed_send
                                ON observed_send.send_attempt_id=observation.send_attempt_id
                               AND observed_send.outbox_id=prepare.outbox_id
                               AND observed_send.outbox_digest=prepare.outbox_digest
                               AND observed_send.operation_kind='prepare'
                               AND observed_send.command_id=prepare.command_id
                               AND observed_send.command_digest=prepare.command_digest
                               AND observed_send.attempt_no=prepare.attempt_count
                               AND observed_send.claim_generation=prepare.claim_generation
                             WHERE observation.outbox_id=prepare.outbox_id
                               AND observation.outbox_digest=prepare.outbox_digest
                               AND observation.operation_kind='prepare'
                               AND observation.observation_kind='prepare_response'
                               AND observation.command_id=NEW.command_id
                               AND observation.command_digest=NEW.command_digest
                               AND observation.provider_id=NEW.provider_id
                               AND observation.adapter_id=NEW.adapter_id
                               AND observation.adapter_binding_digest=
                                    NEW.adapter_binding_digest
                               AND observation.adapter_observation_id=NEW.adapter_ack_id
                               AND observation.response_outcome='accepted'
                               AND observation.remote_execution_state='prepared'
                               AND observation.terminality='non_terminal'
                               AND observation.verification_kind=route.verification_kind
                               AND observation.verifier_id=route.verifier_id
                               AND observation.verification_digest=route.verifier_digest
                               AND observation.recorded_at<=NEW.created_at
                        )
                        )
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'quarantined Attempt ACK requires exact cleanup intents');
        END;
        "#,
    )?;
    Ok(())
}
