use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_outbox_projection
        BEFORE INSERT ON compute_attempt_start_outbox
        WHEN json_extract(NEW.outbox_json,'$.schema') IS NOT NEW.outbox_schema
          OR json_extract(NEW.outbox_json,'$.outbox_id') IS NOT NEW.outbox_id
          OR json_extract(NEW.outbox_json,'$.outbox_digest') IS NOT NEW.outbox_digest
          OR json_extract(NEW.outbox_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.outbox_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.outbox_json,'$.operation_kind') IS NOT NEW.operation_kind
          OR json_extract(NEW.outbox_json,'$.operation_generation')
                IS NOT NEW.operation_generation
          OR json_type(NEW.outbox_json,'$.subject_outbox_id') IS NULL
          OR json_extract(NEW.outbox_json,'$.subject_outbox_id') IS NOT NEW.subject_outbox_id
          OR json_extract(NEW.outbox_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.outbox_json,'$.command_digest') IS NOT NEW.command_digest
          OR json_extract(NEW.outbox_json,'$.adapter_binding_digest')
                IS NOT NEW.adapter_binding_digest
          OR json_extract(NEW.outbox_json,'$.route_authorization_id')
                IS NOT NEW.route_authorization_id
          OR json_extract(NEW.outbox_json,'$.route_authorization_digest')
                IS NOT NEW.route_authorization_digest
          OR json_extract(NEW.outbox_json,'$.actor_receipt_id') IS NOT NEW.actor_receipt_id
          OR json_extract(NEW.outbox_json,'$.actor_receipt_digest')
                IS NOT NEW.actor_receipt_digest
          OR json_extract(NEW.outbox_json,'$.plan_id') IS NOT NEW.plan_id
          OR json_extract(NEW.outbox_json,'$.plan_digest') IS NOT NEW.plan_digest
          OR json_extract(NEW.outbox_json,'$.lease_id') IS NOT NEW.lease_id
          OR json_extract(NEW.outbox_json,'$.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_type(NEW.outbox_json,'$.ack_id') IS NULL
          OR json_extract(NEW.outbox_json,'$.ack_id') IS NOT NEW.ack_id
          OR json_type(NEW.outbox_json,'$.ack_digest') IS NULL
          OR json_extract(NEW.outbox_json,'$.ack_digest') IS NOT NEW.ack_digest
          OR json_type(NEW.outbox_json,'$.application_id') IS NULL
          OR json_extract(NEW.outbox_json,'$.application_id') IS NOT NEW.application_id
          OR json_type(NEW.outbox_json,'$.application_digest') IS NULL
          OR json_extract(NEW.outbox_json,'$.application_digest')
                IS NOT NEW.application_digest
          OR json_type(NEW.outbox_json,'$.lease_authority_id') IS NULL
          OR json_extract(NEW.outbox_json,'$.lease_authority_id')
                IS NOT NEW.lease_authority_id
          OR json_type(NEW.outbox_json,'$.lease_authority_revision') IS NULL
          OR json_extract(NEW.outbox_json,'$.lease_authority_revision')
                IS NOT NEW.lease_authority_revision
          OR json_type(NEW.outbox_json,'$.lease_authority_digest') IS NULL
          OR json_extract(NEW.outbox_json,'$.lease_authority_digest')
                IS NOT NEW.lease_authority_digest
          OR json_extract(NEW.outbox_json,'$.issued_at') IS NOT NEW.issued_at
          OR json_extract(NEW.outbox_json,'$.not_before') IS NOT NEW.not_before
          OR json_extract(NEW.outbox_json,'$.not_after') IS NOT NEW.not_after
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt start outbox projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_send_attempt_projection
        BEFORE INSERT ON compute_attempt_start_send_attempts
        WHEN json_extract(NEW.send_attempt_json,'$.schema') IS NOT NEW.send_attempt_schema
          OR json_extract(NEW.send_attempt_json,'$.send_attempt_id')
                IS NOT NEW.send_attempt_id
          OR json_extract(NEW.send_attempt_json,'$.send_attempt_digest')
                IS NOT NEW.send_attempt_digest
          OR json_extract(NEW.send_attempt_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.send_attempt_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.send_attempt_json,'$.outbox_id') IS NOT NEW.outbox_id
          OR json_extract(NEW.send_attempt_json,'$.outbox_digest') IS NOT NEW.outbox_digest
          OR json_extract(NEW.send_attempt_json,'$.attempt_no') IS NOT NEW.attempt_no
          OR json_extract(NEW.send_attempt_json,'$.operation_kind') IS NOT NEW.operation_kind
          OR json_extract(NEW.send_attempt_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.send_attempt_json,'$.command_digest') IS NOT NEW.command_digest
          OR json_extract(NEW.send_attempt_json,'$.route_authorization_id')
                IS NOT NEW.route_authorization_id
          OR json_extract(NEW.send_attempt_json,'$.route_authorization_digest')
                IS NOT NEW.route_authorization_digest
          OR json_extract(NEW.send_attempt_json,'$.claim_generation')
                IS NOT NEW.claim_generation
          OR json_extract(NEW.send_attempt_json,'$.claim_token_digest')
                IS NOT NEW.claim_token_digest
          OR json_extract(NEW.send_attempt_json,'$.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.send_attempt_json,'$.started_at') IS NOT NEW.started_at
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt start send-attempt projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_outbox_initial_shape
        BEFORE INSERT ON compute_attempt_start_outbox
        WHEN NEW.state_revision!=1 OR NEW.attempt_count!=0 OR NEW.claim_generation!=0
          OR NEW.claim_owner_id IS NOT NULL OR NEW.claim_token_digest IS NOT NULL
          OR NEW.claim_expires_at IS NOT NULL OR NEW.last_failure_code IS NOT NULL
          OR NEW.next_attempt_at!=NEW.not_before
          OR NOT (
                (NEW.operation_kind IN ('prepare','commit','cancel') AND NEW.state='pending')
                OR (NEW.operation_kind='reconcile' AND NEW.state='blocked')
          )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt start outbox initial shape is invalid');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_requires_prepare_outbox_v213
        AFTER INSERT ON compute_attempt_dispatch_commands
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_start_outbox o
              JOIN compute_attempt_dispatch_actor_receipts actor
                ON actor.actor_receipt_id=o.actor_receipt_id
               AND actor.actor_receipt_digest=o.actor_receipt_digest
              JOIN compute_service_actor_authorizations actor_authority
                ON actor_authority.actor_authorization_id=actor.actor_authorization_id
               AND actor_authority.actor_authorization_digest=actor.actor_authorization_digest
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=o.route_authorization_id
               AND route.route_authorization_digest=o.route_authorization_digest
              JOIN compute_route_authorization_seals seal
                ON seal.route_authorization_id=route.route_authorization_id
               AND seal.route_authorization_digest=route.route_authorization_digest
              JOIN compute_attempt_execution_plans plan
                ON plan.plan_id=o.plan_id AND plan.plan_digest=o.plan_digest
             WHERE o.command_id=NEW.command_id AND o.command_digest=NEW.command_digest
               AND o.operation_kind='prepare' AND o.operation_generation=1
               AND o.subject_outbox_id IS NULL AND o.state='pending'
               AND o.state_revision=1 AND o.attempt_count=0 AND o.claim_generation=0
               AND o.next_attempt_at=o.not_before
               AND o.provider_id=NEW.provider_id AND o.adapter_id=NEW.adapter_id
               AND o.adapter_binding_digest=NEW.adapter_binding_digest
               AND o.plan_id=NEW.execution_plan_id
               AND o.plan_digest=NEW.execution_plan_digest
               AND o.lease_id=NEW.lease_id
               AND o.fencing_generation=NEW.fencing_generation
               AND o.issued_at=NEW.issued_at AND o.not_after=NEW.not_after
               AND actor.actor_phase='dispatch' AND actor.command_id=NEW.command_id
               AND actor.command_digest=NEW.command_digest
               AND NEW.created_at<actor.valid_until
               AND actor.provider_id=NEW.provider_id
               AND actor.provider_owner_account_id=NEW.activated_by_user_id
               AND actor.route_authorization_id=route.route_authorization_id
               AND actor.route_authorization_digest=route.route_authorization_digest
               AND actor_authority.provider_id=NEW.provider_id
               AND actor_authority.provider_owner_account_id=NEW.activated_by_user_id
               AND actor_authority.service_actor_id=actor.service_actor_id
               AND actor_authority.issued_at<=actor.issued_at
               AND actor.recorded_at<actor_authority.valid_until
               AND EXISTS (
                    SELECT 1 FROM json_each(actor_authority.allowed_actor_phases_json) phase
                     WHERE phase.type='text' AND phase.value='dispatch'
               )
               AND route.provider_id=NEW.provider_id
               AND route.provider_kind=NEW.provider_kind
               AND route.provider_owner_account_id=NEW.activated_by_user_id
               AND route.executor_id=NEW.executor_id
               AND route.route_kind=NEW.route_kind
               AND route.endpoint_id IS NEW.endpoint_id
               AND route.endpoint_transport IS NEW.endpoint_transport
               AND route.route_binding_digest=plan.route_binding_digest
               AND route.adapter_binding_digest=NEW.adapter_binding_digest
               AND route.adapter_config_revision=NEW.adapter_config_revision
               AND route.adapter_config_digest=NEW.adapter_config_digest
               AND route.adapter_id=NEW.adapter_id
               AND route.adapter_release_version=NEW.adapter_version
               AND route.recorded_at<=NEW.created_at AND NEW.created_at<route.expires_at
               AND seal.route_authorization_revision=route.route_authorization_revision
               AND seal.adapter_id=route.adapter_id
               AND seal.adapter_revision=route.adapter_revision
               AND seal.adapter_registry_digest=route.adapter_registry_digest
               AND seal.credential_id=route.credential_id
               AND seal.credential_revision=route.credential_revision
               AND seal.credential_digest=route.credential_digest
               AND seal.capability_count=route.capability_count
               AND seal.capability_set_digest=route.capability_set_digest
               AND EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=route.route_authorization_id
                       AND cap.capability_id='prepare'
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt command requires exact prepare outbox');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_send_attempt_claim
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
              JOIN compute_route_authorization_capabilities cap
                ON cap.route_authorization_id=route.route_authorization_id
               AND cap.capability_id=CASE o.operation_kind
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
               AND command.provider_id=o.provider_id
               AND command.adapter_id=o.adapter_id
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
               AND NEW.started_at<actor.valid_until
               AND route.provider_id=o.provider_id
               AND route.adapter_id=o.adapter_id
               AND route.adapter_binding_digest=o.adapter_binding_digest
               AND route.executor_id=command.executor_id
               AND route.recorded_at<=NEW.started_at
               AND (
                    (o.operation_kind IN ('prepare','commit')
                        AND NEW.started_at<route.expires_at)
                    OR (o.operation_kind IN ('cancel','reconcile')
                        AND NEW.started_at<route.cleanup_expires_at)
               )
               AND seal.route_authorization_revision=route.route_authorization_revision
               AND seal.adapter_id=route.adapter_id
               AND seal.adapter_revision=route.adapter_revision
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
               AND (o.operation_kind NOT IN ('cancel','reconcile') OR EXISTS (
                    SELECT 1 FROM compute_attempt_dispatch_acks ack
                     WHERE ack.ack_id=o.ack_id AND ack.ack_digest=o.ack_digest
                       AND ack.command_id=o.command_id AND ack.disposition='quarantined'
               ))
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt send-attempt lacks an exact live claim');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_outbox_payload_immutable
        BEFORE UPDATE ON compute_attempt_start_outbox
        WHEN OLD.outbox_id IS NOT NEW.outbox_id OR OLD.outbox_schema IS NOT NEW.outbox_schema
          OR OLD.outbox_digest IS NOT NEW.outbox_digest OR OLD.outbox_json IS NOT NEW.outbox_json
          OR OLD.canonicalization IS NOT NEW.canonicalization
          OR OLD.digest_algorithm IS NOT NEW.digest_algorithm
          OR OLD.operation_kind IS NOT NEW.operation_kind
          OR OLD.operation_generation IS NOT NEW.operation_generation
          OR OLD.subject_outbox_id IS NOT NEW.subject_outbox_id
          OR OLD.command_id IS NOT NEW.command_id OR OLD.command_digest IS NOT NEW.command_digest
          OR OLD.provider_id IS NOT NEW.provider_id OR OLD.adapter_id IS NOT NEW.adapter_id
          OR OLD.adapter_binding_digest IS NOT NEW.adapter_binding_digest
          OR OLD.route_authorization_id IS NOT NEW.route_authorization_id
          OR OLD.route_authorization_digest IS NOT NEW.route_authorization_digest
          OR OLD.actor_receipt_id IS NOT NEW.actor_receipt_id
          OR OLD.actor_receipt_digest IS NOT NEW.actor_receipt_digest
          OR OLD.plan_id IS NOT NEW.plan_id OR OLD.plan_digest IS NOT NEW.plan_digest
          OR OLD.lease_id IS NOT NEW.lease_id
          OR OLD.fencing_generation IS NOT NEW.fencing_generation
          OR OLD.ack_id IS NOT NEW.ack_id OR OLD.ack_digest IS NOT NEW.ack_digest
          OR OLD.application_id IS NOT NEW.application_id
          OR OLD.application_digest IS NOT NEW.application_digest
          OR OLD.lease_authority_id IS NOT NEW.lease_authority_id
          OR OLD.lease_authority_revision IS NOT NEW.lease_authority_revision
          OR OLD.lease_authority_digest IS NOT NEW.lease_authority_digest
          OR OLD.issued_at IS NOT NEW.issued_at OR OLD.not_before IS NOT NEW.not_before
          OR OLD.not_after IS NOT NEW.not_after OR OLD.created_at IS NOT NEW.created_at
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt start outbox payload is immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_outbox_transition
        BEFORE UPDATE ON compute_attempt_start_outbox
        WHEN NEW.state_revision!=OLD.state_revision+1
          OR NEW.attempt_count<OLD.attempt_count
          OR NEW.claim_generation<OLD.claim_generation
          OR NEW.updated_at<=OLD.updated_at
          OR NOT (
                (OLD.state='blocked' AND NEW.state IN ('pending','quarantined'))
             OR (OLD.state='pending' AND NEW.state IN ('claimed','abandoned_no_send','quarantined'))
             OR (OLD.state='claimed' AND NEW.state IN (
                    'pending','in_flight_unknown','abandoned_no_send','quarantined'))
             OR (OLD.state='in_flight_unknown' AND NEW.state IN (
                    'delivery_observed','quarantined'))
             OR (OLD.state='delivery_observed' AND NEW.state='quarantined')
          )
          OR (NEW.state='claimed' AND (
                NEW.claim_generation!=OLD.claim_generation+1
                OR NEW.attempt_count!=OLD.attempt_count
                OR NEW.next_attempt_at IS NOT OLD.next_attempt_at
                OR NEW.updated_at<OLD.next_attempt_at
                OR NEW.updated_at<OLD.not_before OR NEW.updated_at>=OLD.not_after
                OR NEW.claim_expires_at<=NEW.updated_at
          ))
          OR (NEW.state!='claimed' AND NEW.claim_generation!=OLD.claim_generation)
          OR (OLD.state='claimed' AND NEW.state='in_flight_unknown' AND (
                NEW.attempt_count!=OLD.attempt_count+1
                OR NEW.claim_owner_id IS NOT OLD.claim_owner_id
                OR NEW.claim_token_digest IS NOT OLD.claim_token_digest
                OR NEW.claim_expires_at IS NOT OLD.claim_expires_at
                OR NEW.next_attempt_at IS NOT OLD.next_attempt_at
          ))
          OR (NOT (OLD.state='claimed' AND NEW.state='in_flight_unknown')
                AND NEW.attempt_count!=OLD.attempt_count)
          OR (OLD.state='claimed' AND NEW.state='in_flight_unknown' AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_start_send_attempts attempt
                 WHERE attempt.outbox_id=OLD.outbox_id
                   AND attempt.attempt_no=NEW.attempt_count
                   AND attempt.claim_generation=OLD.claim_generation
                   AND attempt.claim_token_digest=OLD.claim_token_digest
          ))
          OR (OLD.state='claimed' AND NEW.state='pending' AND (
                NEW.updated_at<OLD.claim_expires_at OR EXISTS (
                    SELECT 1 FROM compute_attempt_start_send_attempts attempt
                     WHERE attempt.outbox_id=OLD.outbox_id
                )
          ))
          OR (NEW.state IN ('abandoned_no_send','quarantined')
                AND OLD.state IN ('pending','claimed') AND EXISTS (
                SELECT 1 FROM compute_attempt_start_send_attempts attempt
                 WHERE attempt.outbox_id=OLD.outbox_id
          ))
          OR (NEW.state='abandoned_no_send' AND (
                NEW.updated_at<OLD.not_after
                OR (OLD.state='claimed' AND OLD.claim_expires_at>NEW.updated_at)
          ))
          OR (NEW.state='delivery_observed' AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_start_remote_observations observation
                 WHERE observation.outbox_id=OLD.outbox_id
                   AND observation.outbox_digest=OLD.outbox_digest
          ))
          OR (OLD.state='blocked' AND NEW.state='pending' AND NOT EXISTS (
                SELECT 1
                  FROM compute_attempt_start_outbox cancel
                  JOIN compute_attempt_start_remote_observations observation
                    ON observation.outbox_id=cancel.outbox_id
                 WHERE OLD.operation_kind='reconcile'
                   AND cancel.outbox_id=OLD.subject_outbox_id
                   AND cancel.operation_kind='cancel'
                   AND cancel.state='delivery_observed'
                   AND observation.observation_kind='cancel_response'
                   AND observation.operation_kind='cancel'
          ))
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt start outbox transition is invalid');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_outbox_no_delete
        BEFORE DELETE ON compute_attempt_start_outbox
        BEGIN SELECT RAISE(ABORT, 'compute attempt start outbox cannot be deleted'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_send_attempts_no_update
        BEFORE UPDATE ON compute_attempt_start_send_attempts
        BEGIN SELECT RAISE(ABORT, 'compute attempt send-attempts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_send_attempts_no_delete
        BEFORE DELETE ON compute_attempt_start_send_attempts
        BEGIN SELECT RAISE(ABORT, 'compute attempt send-attempts are append-only'); END;
        "#,
    )?;
    Ok(())
}
