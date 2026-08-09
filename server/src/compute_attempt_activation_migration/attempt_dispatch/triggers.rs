use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_commands_projection
        BEFORE INSERT ON compute_attempt_dispatch_commands
        WHEN json_extract(NEW.command_json,'$.schema') IS NOT NEW.command_schema
          OR json_extract(NEW.command_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.command_json,'$.command_digest') IS NOT NEW.command_digest
          OR json_extract(NEW.command_json,'$.issued_at') IS NOT NEW.issued_at
          OR json_extract(NEW.command_json,'$.not_after') IS NOT NEW.not_after
          OR json_extract(NEW.command_json,'$.command.command_type') IS NOT NEW.command_type
          OR json_extract(NEW.command_json,'$.command.identity.job_id') IS NOT NEW.job_id
          OR json_extract(NEW.command_json,'$.command.identity.reservation_id')
                IS NOT NEW.reservation_id
          OR json_extract(NEW.command_json,'$.command.identity.attempt_lease_id') IS NOT NEW.lease_id
          OR json_extract(NEW.command_json,'$.command.identity.attempt_no') IS NOT NEW.attempt_no
          OR json_type(NEW.command_json,'$.command.identity.shard_id') IS NULL
          OR json_extract(NEW.command_json,'$.command.identity.shard_id') IS NOT NEW.shard_id
          OR json_extract(NEW.command_json,'$.command.identity.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.command_json,'$.command.provider.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.command_json,'$.command.provider.policy_revision')
                IS NOT NEW.provider_policy_revision
          OR json_extract(NEW.command_json,'$.command.provider.provider_digest')
                IS NOT NEW.provider_digest
          OR json_extract(NEW.command_json,'$.command.offer.offer_id') IS NOT NEW.offer_id
          OR json_extract(NEW.command_json,'$.command.offer.offer_version') IS NOT NEW.offer_version
          OR json_extract(NEW.command_json,'$.command.offer.offer_digest') IS NOT NEW.offer_digest
          OR json_extract(NEW.command_json,'$.command.job.job_id') IS NOT NEW.job_id
          OR json_extract(NEW.command_json,'$.command.job.job_revision') IS NOT NEW.job_revision
          OR json_extract(NEW.command_json,'$.command.job.job_digest') IS NOT NEW.job_digest
          OR json_extract(NEW.command_json,'$.command.reservation.reservation_id')
                IS NOT NEW.reservation_id
          OR json_extract(NEW.command_json,'$.command.reservation.reservation_revision')
                IS NOT NEW.reservation_revision
          OR json_extract(NEW.command_json,'$.command.reservation.reservation_digest')
                IS NOT NEW.reservation_digest
          OR json_extract(NEW.command_json,'$.command.capacity_claim.claim_id')
                IS NOT NEW.capacity_claim_id
          OR json_extract(NEW.command_json,'$.command.capacity_claim.claim_revision')
                IS NOT NEW.claim_revision
          OR json_extract(NEW.command_json,'$.command.capacity_claim.claim_digest')
                IS NOT NEW.claim_digest
          OR json_extract(NEW.command_json,'$.command.executor_id') IS NOT NEW.executor_id
          OR json_extract(NEW.command_json,'$.command.execution_plan.plan_id')
                IS NOT NEW.execution_plan_id
          OR json_extract(NEW.command_json,'$.command.execution_plan.plan_schema')
                IS NOT NEW.execution_plan_schema
          OR json_extract(NEW.command_json,'$.command.execution_plan.plan_digest')
                IS NOT NEW.execution_plan_digest
          OR json_extract(NEW.command_json,'$.command.lease_expires_at')
                IS NOT NEW.lease_expires_at
          OR json_extract(NEW.command_json,'$.command.hard_deadline_at')
                IS NOT NEW.hard_deadline_at
          OR json_extract(NEW.adapter_binding_json,'$.schema')
                IS NOT 'compute_federation.attempt_adapter_binding.v1'
          OR json_extract(NEW.adapter_binding_json,'$.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.adapter_binding_json,'$.provider_kind') IS NOT NEW.provider_kind
          OR json_extract(NEW.adapter_binding_json,'$.route_kind') IS NOT NEW.route_kind
          OR json_type(NEW.adapter_binding_json,'$.endpoint_id') IS NULL
          OR json_extract(NEW.adapter_binding_json,'$.endpoint_id') IS NOT NEW.endpoint_id
          OR json_type(NEW.adapter_binding_json,'$.endpoint_transport') IS NULL
          OR json_extract(NEW.adapter_binding_json,'$.endpoint_transport')
                IS NOT NEW.endpoint_transport
          OR json_extract(NEW.adapter_binding_json,'$.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.adapter_binding_json,'$.adapter_version') IS NOT NEW.adapter_version
          OR json_extract(NEW.adapter_binding_json,'$.config_revision')
                IS NOT NEW.adapter_config_revision
          OR json_extract(NEW.adapter_binding_json,'$.config_digest')
                IS NOT NEW.adapter_config_digest
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch command projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_commands_current_fence
        BEFORE INSERT ON compute_attempt_dispatch_commands
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_providers p
              JOIN compute_provider_versions pv
                ON pv.provider_id=p.provider_id
               AND pv.policy_revision=p.current_policy_revision
              JOIN compute_offers o ON o.offer_id=NEW.offer_id
              JOIN compute_offer_versions ov
                ON ov.offer_id=NEW.offer_id AND ov.offer_version=NEW.offer_version
              JOIN compute_jobs j ON j.job_id=NEW.job_id
              JOIN compute_reservations r ON r.reservation_id=NEW.reservation_id
              JOIN compute_capacity_claims c ON c.claim_id=NEW.capacity_claim_id
             WHERE p.provider_id=NEW.provider_id
               AND p.status IN ('active','draining')
               AND p.provider_kind=NEW.provider_kind
               AND p.owner_account_id=NEW.activated_by_user_id
               AND (
                    (NEW.route_kind='provider_endpoint'
                        AND json_extract(pv.provider_json,'$.endpoint.endpoint_id')
                            IS NEW.endpoint_id
                        AND json_extract(pv.provider_json,'$.endpoint.transport')
                            IS NEW.endpoint_transport)
                    OR
                    (NEW.route_kind='server_adapter'
                        AND json_extract(pv.provider_json,'$.adapter.adapter_id')
                            IS NEW.adapter_id
                        AND json_extract(pv.provider_json,'$.adapter.adapter_version')
                            IS NEW.adapter_version
                        AND json_extract(pv.provider_json,'$.adapter.config_revision')
                            IS NEW.adapter_config_revision
                        AND json_extract(pv.provider_json,'$.adapter.config_digest')
                            IS NEW.adapter_config_digest)
               )
               AND ov.offer_digest=NEW.offer_digest
               AND ov.provider_id=NEW.provider_id
               AND ov.provider_policy_revision=NEW.provider_policy_revision
               AND ov.provider_digest=NEW.provider_digest
               AND o.provider_id=NEW.provider_id
               AND o.status IN ('active','draining')
               AND j.current_revision=NEW.job_revision
               AND j.current_job_digest=NEW.job_digest
               AND j.status='reserved'
               AND r.job_id=NEW.job_id
               AND r.job_revision=NEW.job_revision
               AND r.job_digest=NEW.job_digest
               AND r.consumer_account_id=j.consumer_account_id
               AND r.current_revision=NEW.reservation_revision
               AND r.current_reservation_digest=NEW.reservation_digest
               AND r.status='active'
               AND r.provider_id=NEW.provider_id
               AND r.offer_id=NEW.offer_id
               AND r.offer_version=NEW.offer_version
               AND r.offer_digest=NEW.offer_digest
               AND r.capacity_claim_id=NEW.capacity_claim_id
               AND r.capacity_claim_revision=NEW.claim_revision
               AND r.capacity_claim_digest=NEW.claim_digest
               AND julianday(NEW.hard_deadline_at)<=julianday(r.expires_at)
               AND c.revision=NEW.claim_revision
               AND c.claim_digest=NEW.claim_digest
               AND c.status='held'
               AND EXISTS (
                    SELECT 1 FROM compute_broker_reserve_receipts b
                     WHERE b.reservation_id=NEW.reservation_id
                       AND b.consumer_account_id=j.consumer_account_id
                       AND b.request_digest=NEW.broker_request_digest
                       AND b.budget_reservation_id=NEW.budget_reservation_id
                       AND b.budget_reserved_fen=NEW.budget_reserved_fen
                       AND b.capacity_claim_id=NEW.capacity_claim_id
                       AND b.capacity_claim_revision=NEW.claim_revision
                       AND b.capacity_claim_digest=NEW.claim_digest
                       AND b.job_id=NEW.job_id
                       AND b.reserved_job_revision=NEW.job_revision
                       AND b.reserved_job_digest=NEW.job_digest
                       AND b.reservation_revision=NEW.reservation_revision
                       AND b.reservation_digest=NEW.reservation_digest
               )
               AND EXISTS (
                    SELECT 1 FROM billing_reservations br
                     WHERE br.id=NEW.budget_reservation_id
                       AND br.user_id=j.consumer_account_id
                       AND br.reserved_fen=NEW.budget_reserved_fen
                       AND br.status='reserved'
                       AND (br.expires_at IS NULL
                            OR julianday(br.expires_at)>=julianday(NEW.created_at))
               )
               AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_activations x
                     WHERE x.reservation_id=NEW.reservation_id OR x.job_id=NEW.job_id
                        OR x.lease_id=NEW.lease_id OR (x.idempotency_scope=
                            'compute_attempt_activation:'||NEW.provider_id
                            AND x.idempotency_key=NEW.activation_idempotency_key)
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch source facts are no longer current');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_acks_projection
        BEFORE INSERT ON compute_attempt_dispatch_acks
        WHEN json_extract(NEW.ack_json,'$.schema')
                IS NOT 'compute_federation.attempt_adapter_ack.v1'
          OR json_extract(NEW.ack_json,'$.ack_id') IS NOT NEW.ack_id
          OR json_extract(NEW.ack_json,'$.adapter_ack_id') IS NOT NEW.adapter_ack_id
          OR json_extract(NEW.ack_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.ack_json,'$.command_digest') IS NOT NEW.command_digest
          OR json_extract(NEW.ack_json,'$.adapter_binding_digest')
                IS NOT NEW.adapter_binding_digest
          OR json_extract(NEW.ack_json,'$.outcome') IS NOT NEW.outcome
          OR json_type(NEW.ack_json,'$.remote_execution_ref') IS NULL
          OR json_extract(NEW.ack_json,'$.remote_execution_ref') IS NOT NEW.remote_execution_ref
          OR json_type(NEW.ack_json,'$.reason_code') IS NULL
          OR json_extract(NEW.ack_json,'$.reason_code') IS NOT NEW.reason_code
          OR json_extract(NEW.ack_json,'$.observed_at') IS NOT NEW.observed_at
          OR json_extract(NEW.ack_json,'$.received_at') IS NOT NEW.received_at
          OR json_extract(NEW.ack_json,'$.ack_digest') IS NOT NEW.ack_digest
          OR NOT EXISTS (
                SELECT 1 FROM compute_attempt_dispatch_commands c
                 WHERE c.command_id=NEW.command_id
                   AND c.command_digest=NEW.command_digest
                   AND c.adapter_binding_digest=NEW.adapter_binding_digest
                   AND c.provider_id=NEW.provider_id
                   AND c.adapter_id=NEW.adapter_id
                   AND c.created_at<=NEW.received_at
          )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt adapter ACK projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_applications_exact
        BEFORE INSERT ON compute_attempt_dispatch_applications
        WHEN json_extract(NEW.application_json,'$.schema')
                IS NOT 'compute_federation.attempt_dispatch_application.v1'
          OR json_extract(NEW.application_json,'$.application_id') IS NOT NEW.application_id
          OR json_extract(NEW.application_json,'$.application_digest')
                IS NOT NEW.application_digest
          OR json_extract(NEW.application_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.application_json,'$.ack_id') IS NOT NEW.ack_id
          OR json_extract(NEW.application_json,'$.action') IS NOT NEW.action
          OR json_extract(NEW.application_json,'$.lease_id') IS NOT NEW.lease_id
          OR json_extract(NEW.application_json,'$.activation_request_digest')
                IS NOT NEW.activation_request_digest
          OR json_extract(NEW.application_json,'$.lease_digest') IS NOT NEW.lease_digest
          OR json_extract(NEW.application_json,'$.applied_at') IS NOT NEW.applied_at
          OR NOT EXISTS (
                SELECT 1
                  FROM compute_attempt_dispatch_commands c
                  JOIN compute_attempt_dispatch_acks a ON a.command_id=c.command_id
                  JOIN compute_attempt_activations x ON x.lease_id=c.lease_id
                 WHERE c.command_id=NEW.command_id
                   AND a.ack_id=NEW.ack_id
                   AND a.application_id=NEW.application_id
                   AND a.outcome='accepted'
                   AND a.disposition='accepted_applied'
                   AND a.activation_lease_id=x.lease_id
                   AND a.remote_execution_ref=x.executor_acceptance_ref
                   AND x.lease_id=NEW.lease_id
                   AND x.request_digest=NEW.activation_request_digest
                   AND x.lease_digest=NEW.lease_digest
                   AND x.activated_at=NEW.applied_at
                   AND a.created_at<=NEW.created_at
                   AND x.reservation_id=c.reservation_id
                   AND x.job_id=c.job_id
                   AND x.provider_id=c.provider_id
                   AND x.executor_id=c.executor_id
                   AND x.attempt_no=c.attempt_no
                   AND x.fencing_generation=c.fencing_generation
          )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch application mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_activation_requires_dispatch_acceptance
        AFTER INSERT ON compute_attempt_activations
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_commands c
              JOIN compute_attempt_dispatch_acks a ON a.command_id=c.command_id
              JOIN compute_jobs j ON j.job_id=c.job_id
             WHERE a.outcome='accepted'
               AND a.disposition='accepted_applied'
               AND a.application_id IS NOT NULL
               AND a.activation_lease_id=NEW.lease_id
               AND a.remote_execution_ref=NEW.executor_acceptance_ref
               AND c.lease_id=NEW.lease_id
               AND c.job_id=NEW.job_id
               AND c.reservation_id=NEW.reservation_id
               AND c.provider_id=NEW.provider_id
               AND c.executor_id=NEW.executor_id
               AND c.attempt_no=NEW.attempt_no
               AND c.fencing_generation=NEW.fencing_generation
               AND c.job_revision=NEW.source_job_revision
               AND c.job_digest=NEW.source_job_digest
               AND c.reservation_revision=NEW.source_reservation_revision
               AND c.reservation_digest=NEW.source_reservation_digest
               AND c.capacity_claim_id=NEW.capacity_claim_id
               AND c.claim_revision=NEW.source_claim_revision
               AND c.claim_digest=NEW.source_claim_digest
               AND NEW.consumer_account_id=j.consumer_account_id
               AND NEW.budget_reservation_id=c.budget_reservation_id
               AND NEW.budget_reserved_fen=c.budget_reserved_fen
               AND NEW.activated_by_user_id=c.activated_by_user_id
               AND NEW.idempotency_scope='compute_attempt_activation:'||c.provider_id
               AND NEW.idempotency_key=c.activation_idempotency_key
               AND NEW.created_at=NEW.activated_at
               AND json_extract(NEW.lease_json,'$.schema')
                    IS 'compute_federation.attempt_lease.v1'
               AND json_extract(NEW.lease_json,'$.lease_id') IS c.lease_id
               AND json_extract(NEW.lease_json,'$.job_id') IS c.job_id
               AND json_extract(NEW.lease_json,'$.reservation_id') IS c.reservation_id
               AND json_extract(NEW.lease_json,'$.attempt_no') IS c.attempt_no
               AND json_type(NEW.lease_json,'$.shard_id') IS NOT NULL
               AND json_extract(NEW.lease_json,'$.shard_id') IS c.shard_id
               AND json_extract(NEW.lease_json,'$.provider_id') IS c.provider_id
               AND json_extract(NEW.lease_json,'$.executor_id') IS c.executor_id
               AND json_extract(NEW.lease_json,'$.status') IS 'staging'
               AND json_extract(NEW.lease_json,'$.fencing_generation')
                    IS c.fencing_generation
               AND json_extract(NEW.lease_json,'$.lease_credential_ref')
                    IS c.lease_credential_ref
               AND json_extract(NEW.lease_json,'$.lease_credential_hint')
                    IS c.lease_credential_hint
               AND json_extract(NEW.lease_json,'$.issued_at') IS NEW.activated_at
               AND json_type(NEW.lease_json,'$.last_heartbeat_at') IS 'null'
               AND json_type(NEW.lease_json,'$.latest_checkpoint') IS 'null'
               AND json_extract(NEW.lease_json,'$.expires_at') IS c.lease_expires_at
               AND json_extract(NEW.lease_json,'$.hard_deadline_at') IS c.hard_deadline_at
               AND json_type(NEW.lease_json,'$.terminal_reason_code') IS 'null'
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt activation requires atomic gateway acceptance');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_commands_no_replace
        BEFORE INSERT ON compute_attempt_dispatch_commands
        WHEN EXISTS (SELECT 1 FROM compute_attempt_dispatch_commands x
             WHERE x.command_id=NEW.command_id OR x.command_digest=NEW.command_digest
                OR x.reservation_id=NEW.reservation_id OR x.lease_id=NEW.lease_id
                OR (x.job_id=NEW.job_id AND x.attempt_no=NEW.attempt_no)
                OR (x.provider_id=NEW.provider_id
                    AND x.activation_idempotency_key=NEW.activation_idempotency_key))
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch command replacement is forbidden');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_activations_no_replace_v211
        BEFORE INSERT ON compute_attempt_activations
        WHEN EXISTS (SELECT 1 FROM compute_attempt_activations x
             WHERE x.lease_id=NEW.lease_id
                OR x.capacity_transaction_id=NEW.capacity_transaction_id
                OR (x.idempotency_scope=NEW.idempotency_scope
                    AND x.idempotency_key=NEW.idempotency_key)
                OR (x.job_id=NEW.job_id AND x.attempt_no=NEW.attempt_no)
                OR (x.reservation_id=NEW.reservation_id
                    AND x.fencing_generation=NEW.fencing_generation))
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt activation replacement is forbidden');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_acks_no_replace
        BEFORE INSERT ON compute_attempt_dispatch_acks
        WHEN EXISTS (SELECT 1 FROM compute_attempt_dispatch_acks x
             WHERE x.ack_id=NEW.ack_id OR x.command_id=NEW.command_id
                OR x.ack_digest=NEW.ack_digest
                OR (x.provider_id=NEW.provider_id AND x.adapter_id=NEW.adapter_id
                    AND x.adapter_ack_id=NEW.adapter_ack_id))
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch ACK replacement is forbidden');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_applications_no_replace
        BEFORE INSERT ON compute_attempt_dispatch_applications
        WHEN EXISTS (SELECT 1 FROM compute_attempt_dispatch_applications x
             WHERE x.application_id=NEW.application_id OR x.command_id=NEW.command_id
                OR x.ack_id=NEW.ack_id OR x.lease_id=NEW.lease_id
                OR x.application_digest=NEW.application_digest)
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch application replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_commands_no_update
        BEFORE UPDATE ON compute_attempt_dispatch_commands BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch commands are append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_commands_no_delete
        BEFORE DELETE ON compute_attempt_dispatch_commands BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch commands are append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_acks_no_update
        BEFORE UPDATE ON compute_attempt_dispatch_acks BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch ACKs are append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_acks_no_delete
        BEFORE DELETE ON compute_attempt_dispatch_acks BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch ACKs are append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_applications_no_update
        BEFORE UPDATE ON compute_attempt_dispatch_applications BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch applications are append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_applications_no_delete
        BEFORE DELETE ON compute_attempt_dispatch_applications BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch applications are append-only');
        END;
        "#,
    )?;
    Ok(())
}
