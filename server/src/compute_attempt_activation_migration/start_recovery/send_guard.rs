use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS trg_compute_attempt_start_send_attempt_claim;
        CREATE TRIGGER trg_compute_attempt_start_send_attempt_claim
        BEFORE INSERT ON compute_attempt_start_send_attempts
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_start_outbox o
              JOIN compute_attempt_dispatch_commands command
                ON command.command_id=o.command_id AND command.command_digest=o.command_digest
              JOIN compute_attempt_dispatch_actor_receipts actor
                ON actor.actor_receipt_id=o.actor_receipt_id
               AND actor.actor_receipt_digest=o.actor_receipt_digest
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=o.route_authorization_id
               AND route.route_authorization_digest=o.route_authorization_digest
              JOIN compute_route_authorization_capabilities capability
                ON capability.route_authorization_id=route.route_authorization_id
               AND capability.capability_id=CASE o.operation_kind
                    WHEN 'prepare' THEN 'prepare'
                    WHEN 'commit' THEN 'idempotent_commit'
                    WHEN 'cancel' THEN 'cancel_no_start'
                    WHEN 'reconcile' THEN 'reconcile'
               END
              JOIN compute_route_authorization_seals seal
                ON seal.route_authorization_id=route.route_authorization_id
               AND seal.route_authorization_digest=route.route_authorization_digest
             WHERE o.outbox_id=NEW.outbox_id AND o.outbox_digest=NEW.outbox_digest
               AND o.operation_kind=NEW.operation_kind
               AND o.command_id=NEW.command_id AND o.command_digest=NEW.command_digest
               AND o.route_authorization_id=NEW.route_authorization_id
               AND o.route_authorization_digest=NEW.route_authorization_digest
               AND o.state='claimed' AND o.claim_generation=NEW.claim_generation
               AND o.claim_token_digest=NEW.claim_token_digest
               AND o.attempt_count+1=NEW.attempt_no
               AND NEW.started_at<o.claim_expires_at AND NEW.started_at<o.not_after
               AND command.provider_id=o.provider_id AND command.adapter_id=o.adapter_id
               AND command.adapter_binding_digest=o.adapter_binding_digest
               AND command.execution_plan_id=o.plan_id
               AND command.execution_plan_digest=o.plan_digest
               AND command.lease_id=o.lease_id
               AND command.fencing_generation=o.fencing_generation
               AND actor.command_id=o.command_id AND actor.command_digest=o.command_digest
               AND actor.provider_id=o.provider_id
               AND actor.route_authorization_id=o.route_authorization_id
               AND actor.route_authorization_digest=o.route_authorization_digest
               AND actor.actor_phase=CASE o.operation_kind
                    WHEN 'commit' THEN 'application' ELSE 'dispatch' END
               AND (o.operation_kind IN ('cancel','reconcile')
                    OR NEW.started_at<actor.valid_until)
               AND route.provider_id=o.provider_id AND route.adapter_id=o.adapter_id
               AND route.adapter_binding_digest=o.adapter_binding_digest
               AND route.executor_id=command.executor_id
               AND route.recorded_at<=NEW.started_at
               AND ((o.operation_kind IN ('prepare','commit')
                        AND NEW.started_at<route.expires_at)
                    OR (o.operation_kind IN ('cancel','reconcile')
                        AND NEW.started_at<route.cleanup_expires_at))
               AND seal.route_authorization_revision=route.route_authorization_revision
               AND seal.adapter_id=route.adapter_id AND seal.adapter_revision=route.adapter_revision
               AND seal.adapter_registry_digest=route.adapter_registry_digest
               AND seal.credential_id=route.credential_id
               AND seal.credential_revision=route.credential_revision
               AND seal.credential_digest=route.credential_digest
               AND seal.capability_count=route.capability_count
               AND seal.capability_set_digest=route.capability_set_digest
               AND NOT EXISTS (
                    SELECT 1 FROM compute_route_credential_revocations revoked
                     WHERE revoked.credential_id=route.credential_id
                       AND revoked.credential_revision=route.credential_revision
                       AND revoked.revoked_at<=NEW.started_at
                       AND o.operation_kind IN ('prepare','commit')
               )
               AND (o.operation_kind!='commit' OR EXISTS (
                    SELECT 1 FROM compute_attempt_lease_authority_bindings authority
                     WHERE authority.lease_authority_id=o.lease_authority_id
                       AND authority.authority_revision=o.lease_authority_revision
                       AND authority.lease_authority_digest=o.lease_authority_digest
                       AND authority.application_id=o.application_id
                       AND authority.application_digest=o.application_digest
                       AND authority.command_id=o.command_id
                       AND authority.route_authorization_id=o.route_authorization_id
                       AND authority.route_authorization_digest=o.route_authorization_digest
                       AND NEW.started_at<authority.expires_at
               ))
               AND (o.operation_kind NOT IN ('cancel','reconcile')
                    OR EXISTS (
                        SELECT 1 FROM compute_attempt_dispatch_acks ack
                         WHERE ack.ack_id=o.ack_id AND ack.ack_digest=o.ack_digest
                           AND ack.command_id=o.command_id
                           AND ack.command_digest=o.command_digest
                           AND ack.provider_id=o.provider_id
                           AND ack.adapter_id=o.adapter_id
                           AND ack.adapter_binding_digest=o.adapter_binding_digest
                           AND ack.disposition='quarantined'
                    )
                    OR (o.ack_id IS NULL AND o.ack_digest IS NULL AND EXISTS (
                        SELECT 1
                          FROM compute_attempt_start_outbox prepare
                          JOIN compute_attempt_start_send_attempts prepare_send
                            ON prepare_send.outbox_id=prepare.outbox_id
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
                         WHERE prepare.operation_kind='prepare'
                           AND prepare.operation_generation=1
                           AND o.operation_generation=1
                           AND prepare.command_id=o.command_id
                           AND prepare.command_digest=o.command_digest
                           AND prepare.provider_id=o.provider_id
                           AND prepare.adapter_id=o.adapter_id
                           AND prepare.adapter_binding_digest=o.adapter_binding_digest
                           AND prepare.route_authorization_id=o.route_authorization_id
                           AND prepare.route_authorization_digest=o.route_authorization_digest
                           AND prepare.actor_receipt_id=o.actor_receipt_id
                           AND prepare.actor_receipt_digest=o.actor_receipt_digest
                           AND prepare.plan_id=o.plan_id AND prepare.plan_digest=o.plan_digest
                           AND prepare.lease_id=o.lease_id
                           AND prepare.fencing_generation=o.fencing_generation
                           AND o.issued_at=o.created_at
                           AND o.issued_at=o.not_before
                           AND o.issued_at<route.cleanup_expires_at
                           AND o.not_after<=route.cleanup_expires_at
                           AND (
                                (prepare.state='in_flight_unknown'
                                    AND prepare.claim_expires_at<=o.issued_at
                                    AND prepare_send.claim_token_digest=
                                        prepare.claim_token_digest
                                    AND NOT EXISTS (
                                        SELECT 1 FROM compute_attempt_dispatch_acks ack
                                         WHERE ack.command_id=prepare.command_id
                                    )
                                    AND NOT EXISTS (
                                        SELECT 1
                                          FROM compute_attempt_start_remote_observations response
                                         WHERE response.outbox_id=prepare.outbox_id
                                           AND response.operation_kind='prepare'
                                           AND response.observation_kind='prepare_response'
                                    ))
                                OR (prepare.state='delivery_observed' AND EXISTS (
                                    SELECT 1
                                      FROM compute_attempt_dispatch_acks late_ack
                                      JOIN compute_attempt_start_remote_observations response
                                        ON response.command_id=late_ack.command_id
                                       AND response.adapter_observation_id=
                                            late_ack.adapter_ack_id
                                     WHERE late_ack.command_id=prepare.command_id
                                       AND late_ack.command_digest=prepare.command_digest
                                       AND late_ack.provider_id=prepare.provider_id
                                       AND late_ack.adapter_id=prepare.adapter_id
                                       AND late_ack.adapter_binding_digest=
                                            prepare.adapter_binding_digest
                                       AND late_ack.disposition='quarantined'
                                       AND late_ack.created_at>=o.issued_at
                                       AND response.send_attempt_id=
                                            prepare_send.send_attempt_id
                                       AND response.outbox_id=prepare.outbox_id
                                       AND response.outbox_digest=prepare.outbox_digest
                                       AND response.operation_kind='prepare'
                                       AND response.observation_kind='prepare_response'
                                       AND response.command_id=prepare.command_id
                                       AND response.command_digest=prepare.command_digest
                                       AND response.provider_id=prepare.provider_id
                                       AND response.adapter_id=prepare.adapter_id
                                       AND response.adapter_binding_digest=
                                            prepare.adapter_binding_digest
                                       AND response.response_outcome='accepted'
                                       AND response.remote_execution_state='prepared'
                                       AND response.terminality='non_terminal'
                                       AND response.verification_kind=route.verification_kind
                                       AND response.verifier_id=route.verifier_id
                                       AND response.verification_digest=route.verifier_digest
                                )))
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
                           AND ((o.operation_kind='cancel'
                                AND o.subject_outbox_id=prepare.outbox_id)
                            OR (o.operation_kind='reconcile' AND EXISTS (
                                SELECT 1
                                  FROM compute_attempt_start_outbox cancel
                                  JOIN compute_attempt_start_send_attempts cancel_send
                                    ON cancel_send.outbox_id=cancel.outbox_id
                                   AND cancel_send.outbox_digest=cancel.outbox_digest
                                   AND cancel_send.operation_kind='cancel'
                                   AND cancel_send.command_id=cancel.command_id
                                   AND cancel_send.command_digest=cancel.command_digest
                                   AND cancel_send.route_authorization_id=
                                        cancel.route_authorization_id
                                   AND cancel_send.route_authorization_digest=
                                        cancel.route_authorization_digest
                                   AND cancel_send.attempt_no=cancel.attempt_count
                                   AND cancel_send.claim_generation=cancel.claim_generation
                                  JOIN compute_attempt_start_remote_observations cancel_response
                                    ON cancel_response.send_attempt_id=
                                        cancel_send.send_attempt_id
                                   AND cancel_response.outbox_id=cancel.outbox_id
                                   AND cancel_response.outbox_digest=cancel.outbox_digest
                                 WHERE cancel.outbox_id=o.subject_outbox_id
                                   AND cancel.operation_kind='cancel'
                                   AND cancel.operation_generation=1
                                   AND cancel.subject_outbox_id=prepare.outbox_id
                                   AND cancel.ack_id IS NULL AND cancel.ack_digest IS NULL
                                   AND cancel.state='delivery_observed'
                                   AND cancel.command_id=o.command_id
                                   AND cancel.command_digest=o.command_digest
                                   AND cancel.provider_id=o.provider_id
                                   AND cancel.adapter_id=o.adapter_id
                                   AND cancel.adapter_binding_digest=o.adapter_binding_digest
                                   AND cancel.route_authorization_id=o.route_authorization_id
                                   AND cancel.route_authorization_digest=
                                        o.route_authorization_digest
                                   AND cancel.actor_receipt_id=o.actor_receipt_id
                                   AND cancel.actor_receipt_digest=o.actor_receipt_digest
                                   AND cancel.plan_id=o.plan_id
                                   AND cancel.plan_digest=o.plan_digest
                                   AND cancel.lease_id=o.lease_id
                                   AND cancel.fencing_generation=o.fencing_generation
                                   AND cancel.issued_at=o.issued_at
                                   AND cancel.not_before=o.not_before
                                   AND cancel.not_after=o.not_after
                                   AND cancel.issued_at=cancel.created_at
                                   AND cancel_response.operation_kind='cancel'
                                   AND cancel_response.observation_kind='cancel_response'
                                   AND cancel_response.command_id=o.command_id
                                   AND cancel_response.command_digest=o.command_digest
                                   AND cancel_response.provider_id=o.provider_id
                                   AND cancel_response.adapter_id=o.adapter_id
                                   AND cancel_response.adapter_binding_digest=
                                        o.adapter_binding_digest
                                   AND cancel_response.verification_kind=
                                        route.verification_kind
                                   AND cancel_response.verifier_id=route.verifier_id
                                   AND cancel_response.verification_digest=
                                        route.verifier_digest
                                   AND cancel_response.recorded_at<=NEW.started_at
                            )))
                    )))
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt send-attempt lacks an exact live claim');
        END;
        "#,
    )?;
    Ok(())
}
