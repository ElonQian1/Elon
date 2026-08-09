use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_observation_projection
        BEFORE INSERT ON compute_attempt_start_remote_observations
        WHEN json_extract(NEW.observation_json,'$.schema') IS NOT NEW.observation_schema
          OR json_extract(NEW.observation_json,'$.observation_id') IS NOT NEW.observation_id
          OR json_extract(NEW.observation_json,'$.observation_digest')
                IS NOT NEW.observation_digest
          OR json_extract(NEW.observation_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.observation_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.observation_json,'$.send_attempt_id') IS NOT NEW.send_attempt_id
          OR json_extract(NEW.observation_json,'$.outbox_id') IS NOT NEW.outbox_id
          OR json_extract(NEW.observation_json,'$.outbox_digest') IS NOT NEW.outbox_digest
          OR json_extract(NEW.observation_json,'$.operation_kind') IS NOT NEW.operation_kind
          OR json_extract(NEW.observation_json,'$.observation_kind')
                IS NOT NEW.observation_kind
          OR json_extract(NEW.observation_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.observation_json,'$.command_digest') IS NOT NEW.command_digest
          OR json_extract(NEW.observation_json,'$.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.observation_json,'$.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.observation_json,'$.adapter_binding_digest')
                IS NOT NEW.adapter_binding_digest
          OR json_extract(NEW.observation_json,'$.adapter_observation_id')
                IS NOT NEW.adapter_observation_id
          OR json_extract(NEW.observation_json,'$.response_outcome')
                IS NOT NEW.response_outcome
          OR json_extract(NEW.observation_json,'$.remote_execution_state')
                IS NOT NEW.remote_execution_state
          OR json_extract(NEW.observation_json,'$.terminality') IS NOT NEW.terminality
          OR json_type(NEW.observation_json,'$.remote_execution_ref') IS NULL
          OR json_extract(NEW.observation_json,'$.remote_execution_ref')
                IS NOT NEW.remote_execution_ref
          OR json_extract(NEW.observation_json,'$.remote_sequence') IS NOT NEW.remote_sequence
          OR json_type(NEW.observation_json,'$.no_commit_tombstone_id') IS NULL
          OR json_extract(NEW.observation_json,'$.no_commit_tombstone_id')
                IS NOT NEW.no_commit_tombstone_id
          OR json_type(NEW.observation_json,'$.no_commit_tombstone_digest') IS NULL
          OR json_extract(NEW.observation_json,'$.no_commit_tombstone_digest')
                IS NOT NEW.no_commit_tombstone_digest
          OR json_type(NEW.observation_json,'$.reason_code') IS NULL
          OR json_extract(NEW.observation_json,'$.reason_code') IS NOT NEW.reason_code
          OR json_extract(NEW.observation_json,'$.verification_kind')
                IS NOT NEW.verification_kind
          OR json_extract(NEW.observation_json,'$.verifier_id') IS NOT NEW.verifier_id
          OR json_extract(NEW.observation_json,'$.verification_digest')
                IS NOT NEW.verification_digest
          OR json_extract(NEW.observation_json,'$.authenticated_at')
                IS NOT NEW.authenticated_at
          OR json_extract(NEW.observation_json,'$.observed_at') IS NOT NEW.observed_at
          OR json_extract(NEW.observation_json,'$.received_at') IS NOT NEW.received_at
          OR json_extract(NEW.observation_json,'$.recorded_at') IS NOT NEW.recorded_at
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt remote observation projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_observation_exact_attempt
        BEFORE INSERT ON compute_attempt_start_remote_observations
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_start_send_attempts attempt
              JOIN compute_attempt_start_outbox o ON o.outbox_id=attempt.outbox_id
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=attempt.route_authorization_id
               AND route.route_authorization_digest=attempt.route_authorization_digest
              JOIN compute_route_authorization_capabilities capability
                ON capability.route_authorization_id=route.route_authorization_id
               AND capability.capability_id='authenticated_ack'
             WHERE attempt.send_attempt_id=NEW.send_attempt_id
               AND attempt.outbox_id=NEW.outbox_id
               AND attempt.outbox_digest=NEW.outbox_digest
               AND attempt.operation_kind=NEW.operation_kind
               AND attempt.command_id=NEW.command_id
               AND attempt.command_digest=NEW.command_digest
               AND o.outbox_digest=NEW.outbox_digest
               AND o.provider_id=NEW.provider_id
               AND o.adapter_id=NEW.adapter_id
               AND o.adapter_binding_digest=NEW.adapter_binding_digest
               AND route.provider_id=NEW.provider_id
               AND route.adapter_id=NEW.adapter_id
               AND route.adapter_binding_digest=NEW.adapter_binding_digest
               AND route.verification_kind=NEW.verification_kind
               AND route.verifier_id=NEW.verifier_id
               AND route.verifier_digest=NEW.verification_digest
               AND route.authenticated_at<=NEW.authenticated_at
               AND NEW.authenticated_at<route.cleanup_expires_at
               AND o.state='in_flight_unknown'
               AND attempt.started_at<=NEW.received_at
               AND (
                    (NEW.operation_kind='prepare'
                        AND NEW.observation_kind='prepare_response')
                    OR (NEW.operation_kind='commit'
                        AND NEW.observation_kind='commit_response')
                    OR (NEW.operation_kind='cancel'
                        AND NEW.observation_kind='cancel_response')
                    OR (NEW.operation_kind='reconcile'
                        AND NEW.observation_kind='reconcile_attestation')
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt observation lacks exact send-attempt');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_adapter_ack_requires_observation_v213
        BEFORE INSERT ON compute_attempt_dispatch_acks
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_start_remote_observations observation
              JOIN compute_attempt_start_outbox o ON o.outbox_id=observation.outbox_id
             WHERE o.operation_kind='prepare'
               AND o.command_id=NEW.command_id
               AND o.command_digest=NEW.command_digest
               AND o.adapter_binding_digest=NEW.adapter_binding_digest
               AND observation.operation_kind='prepare'
               AND observation.observation_kind='prepare_response'
               AND observation.command_id=NEW.command_id
               AND observation.command_digest=NEW.command_digest
               AND observation.provider_id=NEW.provider_id
               AND observation.adapter_id=NEW.adapter_id
               AND observation.adapter_binding_digest=NEW.adapter_binding_digest
               AND observation.adapter_observation_id=NEW.adapter_ack_id
               AND observation.remote_execution_ref IS NEW.remote_execution_ref
               AND observation.reason_code IS NEW.reason_code
               AND observation.observed_at=NEW.observed_at
               AND observation.received_at=NEW.received_at
               AND observation.recorded_at<=NEW.created_at
               AND (
                    (NEW.outcome='accepted'
                        AND observation.response_outcome='accepted'
                        AND observation.remote_execution_state='prepared'
                        AND observation.terminality='non_terminal')
                    OR (NEW.outcome='rejected'
                        AND observation.response_outcome='rejected'
                        AND observation.remote_execution_state='rejected'
                        AND observation.terminality='final')
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt ACK requires authenticated prepare observation');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_quarantined_ack_cleanup_v213
        BEFORE INSERT ON compute_attempt_dispatch_acks
        WHEN NEW.disposition='quarantined' AND NOT EXISTS (
            SELECT 1
              FROM compute_attempt_start_outbox prepare
              JOIN compute_attempt_start_outbox cancel
                ON cancel.subject_outbox_id=prepare.outbox_id
              JOIN compute_attempt_start_outbox reconcile
                ON reconcile.subject_outbox_id=cancel.outbox_id
             WHERE prepare.command_id=NEW.command_id
               AND prepare.operation_kind='prepare'
               AND cancel.operation_kind='cancel' AND cancel.operation_generation=1
               AND cancel.command_id=NEW.command_id
               AND cancel.command_digest=NEW.command_digest
               AND cancel.provider_id=NEW.provider_id
               AND cancel.adapter_id=NEW.adapter_id
               AND cancel.adapter_binding_digest=NEW.adapter_binding_digest
               AND cancel.ack_id=NEW.ack_id AND cancel.ack_digest=NEW.ack_digest
               AND cancel.plan_id=prepare.plan_id AND cancel.plan_digest=prepare.plan_digest
               AND cancel.lease_id=prepare.lease_id
               AND cancel.fencing_generation=prepare.fencing_generation
               AND cancel.actor_receipt_id=prepare.actor_receipt_id
               AND cancel.actor_receipt_digest=prepare.actor_receipt_digest
               AND cancel.route_authorization_id=prepare.route_authorization_id
               AND cancel.route_authorization_digest=prepare.route_authorization_digest
               AND cancel.state='pending' AND cancel.state_revision=1
               AND cancel.issued_at=NEW.created_at AND cancel.not_before=NEW.created_at
               AND reconcile.operation_kind='reconcile'
               AND reconcile.operation_generation=1
               AND reconcile.command_id=NEW.command_id
               AND reconcile.command_digest=NEW.command_digest
               AND reconcile.provider_id=NEW.provider_id
               AND reconcile.adapter_id=NEW.adapter_id
               AND reconcile.adapter_binding_digest=NEW.adapter_binding_digest
               AND reconcile.ack_id=NEW.ack_id AND reconcile.ack_digest=NEW.ack_digest
               AND reconcile.plan_id=prepare.plan_id
               AND reconcile.plan_digest=prepare.plan_digest
               AND reconcile.lease_id=prepare.lease_id
               AND reconcile.fencing_generation=prepare.fencing_generation
               AND reconcile.actor_receipt_id=prepare.actor_receipt_id
               AND reconcile.actor_receipt_digest=prepare.actor_receipt_digest
               AND reconcile.route_authorization_id=prepare.route_authorization_id
               AND reconcile.route_authorization_digest=prepare.route_authorization_digest
               AND reconcile.state='blocked' AND reconcile.state_revision=1
               AND reconcile.issued_at=NEW.created_at
               AND reconcile.not_before=NEW.created_at
               AND cancel.not_after<=((SELECT route.cleanup_expires_at
                    FROM compute_route_authorization_receipts route
                    WHERE route.route_authorization_id=prepare.route_authorization_id))
               AND reconcile.not_after<=((SELECT route.cleanup_expires_at
                    FROM compute_route_authorization_receipts route
                    WHERE route.route_authorization_id=prepare.route_authorization_id))
               AND EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=prepare.route_authorization_id
                       AND cap.capability_id='cancel_no_start'
               )
               AND EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=prepare.route_authorization_id
                       AND cap.capability_id='reconcile'
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'quarantined Attempt ACK requires cancel and reconcile intents');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_application_commit_closure_v213
        BEFORE INSERT ON compute_attempt_dispatch_applications
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_commands command
              JOIN compute_attempt_dispatch_acks ack ON ack.command_id=command.command_id
              JOIN compute_attempt_activations activation ON activation.lease_id=NEW.lease_id
              JOIN compute_attempt_dispatch_actor_receipts actor
                ON actor.command_id=command.command_id AND actor.actor_phase='application'
              JOIN compute_attempt_lease_authority_bindings authority
                ON authority.application_id=NEW.application_id
              JOIN compute_attempt_start_outbox commit_intent
                ON commit_intent.application_id=NEW.application_id
              JOIN compute_attempt_start_outbox prepare_intent
                ON prepare_intent.outbox_id=commit_intent.subject_outbox_id
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=commit_intent.route_authorization_id
               AND route.route_authorization_digest=commit_intent.route_authorization_digest
             WHERE command.command_id=NEW.command_id
               AND ack.ack_id=NEW.ack_id AND ack.ack_digest=authority.ack_digest
               AND ack.outcome='accepted' AND ack.disposition='accepted_applied'
               AND actor.actor_receipt_id=authority.application_actor_receipt_id
               AND actor.actor_receipt_digest=authority.application_actor_receipt_digest
               AND actor.ack_id=NEW.ack_id AND actor.ack_digest=ack.ack_digest
               AND actor.application_id=NEW.application_id
               AND actor.application_digest=NEW.application_digest
               AND NEW.created_at<actor.valid_until
               AND actor.provider_id=command.provider_id
               AND actor.provider_owner_account_id=command.activated_by_user_id
               AND actor.route_authorization_id=route.route_authorization_id
               AND actor.route_authorization_digest=route.route_authorization_digest
               AND authority.command_id=NEW.command_id
               AND authority.command_digest=command.command_digest
               AND authority.plan_id=command.execution_plan_id
               AND authority.plan_digest=command.execution_plan_digest
               AND authority.ack_id=NEW.ack_id
               AND authority.application_digest=NEW.application_digest
               AND authority.application_id=NEW.application_id
               AND authority.lease_id=NEW.lease_id
               AND authority.lease_digest=NEW.lease_digest
               AND authority.provider_id=command.provider_id
               AND authority.executor_id=command.executor_id
               AND authority.fencing_generation=command.fencing_generation
               AND authority.non_bearer_authority_ref=command.lease_credential_ref
               AND authority.authority_hint=command.lease_credential_hint
               AND authority.route_authorization_id=route.route_authorization_id
               AND authority.route_authorization_digest=route.route_authorization_digest
               AND authority.authority_kind=(SELECT p.lease_authority_kind
                    FROM compute_attempt_execution_plans p
                    WHERE p.plan_id=command.execution_plan_id)
               AND authority.delivery_mode=(SELECT p.lease_delivery_mode
                    FROM compute_attempt_execution_plans p
                    WHERE p.plan_id=command.execution_plan_id)
               AND authority.audience=(SELECT p.lease_audience
                    FROM compute_attempt_execution_plans p
                    WHERE p.plan_id=command.execution_plan_id)
               AND authority.scopes_json=(SELECT json_extract(p.plan_json,
                    '$.plan.lease_authority.required_scopes')
                    FROM compute_attempt_execution_plans p
                    WHERE p.plan_id=command.execution_plan_id)
               AND authority.expires_at=(SELECT p.lease_authority_valid_until
                    FROM compute_attempt_execution_plans p
                    WHERE p.plan_id=command.execution_plan_id)
               AND ack.received_at<=authority.issued_at
               AND authority.recorded_at<=NEW.created_at
               AND command.hard_deadline_at<=authority.expires_at
               AND activation.lease_digest=NEW.lease_digest
               AND commit_intent.operation_kind='commit'
               AND commit_intent.operation_generation=1
               AND commit_intent.command_id=NEW.command_id
               AND commit_intent.command_digest=command.command_digest
               AND commit_intent.provider_id=command.provider_id
               AND commit_intent.adapter_id=command.adapter_id
               AND commit_intent.adapter_binding_digest=command.adapter_binding_digest
               AND commit_intent.actor_receipt_id=actor.actor_receipt_id
               AND commit_intent.actor_receipt_digest=actor.actor_receipt_digest
               AND commit_intent.plan_id=command.execution_plan_id
               AND commit_intent.plan_digest=command.execution_plan_digest
               AND commit_intent.lease_id=NEW.lease_id
               AND commit_intent.fencing_generation=command.fencing_generation
               AND commit_intent.ack_id=NEW.ack_id
               AND commit_intent.ack_digest=ack.ack_digest
               AND commit_intent.application_digest=NEW.application_digest
               AND commit_intent.lease_authority_id=authority.lease_authority_id
               AND commit_intent.lease_authority_revision=authority.authority_revision
               AND commit_intent.lease_authority_digest=authority.lease_authority_digest
               AND commit_intent.state='pending' AND commit_intent.state_revision=1
               AND commit_intent.next_attempt_at=commit_intent.not_before
               AND commit_intent.not_after<=authority.expires_at
               AND commit_intent.not_after<=route.expires_at
               AND prepare_intent.operation_kind='prepare'
               AND prepare_intent.command_id=NEW.command_id
               AND prepare_intent.route_authorization_id=commit_intent.route_authorization_id
               AND prepare_intent.route_authorization_digest=commit_intent.route_authorization_digest
               AND EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=route.route_authorization_id
                       AND cap.capability_id='idempotent_commit'
               )
        )
        BEGIN
            SELECT RAISE(ABORT,
                'compute attempt application requires authority and commit outbox');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_observations_no_update
        BEFORE UPDATE ON compute_attempt_start_remote_observations
        BEGIN SELECT RAISE(ABORT, 'compute attempt remote observations are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_observations_no_delete
        BEFORE DELETE ON compute_attempt_start_remote_observations
        BEGIN SELECT RAISE(ABORT, 'compute attempt remote observations are append-only'); END;
        "#,
    )?;
    Ok(())
}
