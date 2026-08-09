use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_service_actor_authorizations_no_replace
        BEFORE INSERT ON compute_service_actor_authorizations
        WHEN EXISTS (
            SELECT 1 FROM compute_service_actor_authorizations x
             WHERE x.actor_authorization_id=NEW.actor_authorization_id
                OR x.actor_authorization_digest=NEW.actor_authorization_digest
        )
        BEGIN SELECT RAISE(ABORT, 'compute service actor replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapters_no_replace
        BEFORE INSERT ON compute_route_adapters
        WHEN EXISTS (
            SELECT 1 FROM compute_route_adapters x WHERE x.adapter_id=NEW.adapter_id
        )
        BEGIN SELECT RAISE(ABORT, 'compute route Adapter root replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapter_versions_no_replace
        BEFORE INSERT ON compute_route_adapter_versions
        WHEN EXISTS (
            SELECT 1 FROM compute_route_adapter_versions x
             WHERE (x.adapter_id=NEW.adapter_id AND x.adapter_revision=NEW.adapter_revision)
                OR x.adapter_digest=NEW.adapter_digest
        )
        BEGIN SELECT RAISE(ABORT, 'compute route Adapter version replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credentials_no_replace
        BEFORE INSERT ON compute_route_credentials
        WHEN EXISTS (
            SELECT 1 FROM compute_route_credentials x WHERE x.credential_id=NEW.credential_id
        )
        BEGIN SELECT RAISE(ABORT, 'compute route credential root replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_versions_no_replace
        BEFORE INSERT ON compute_route_credential_versions
        WHEN EXISTS (
            SELECT 1 FROM compute_route_credential_versions x
             WHERE (x.credential_id=NEW.credential_id
                    AND x.credential_revision=NEW.credential_revision)
                OR x.credential_digest=NEW.credential_digest
        )
        BEGIN SELECT RAISE(ABORT, 'compute route credential version replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_revocations_no_replace
        BEFORE INSERT ON compute_route_credential_revocations
        WHEN EXISTS (
            SELECT 1 FROM compute_route_credential_revocations x
             WHERE x.revocation_id=NEW.revocation_id
                OR x.revocation_digest=NEW.revocation_digest
                OR (x.credential_id=NEW.credential_id
                    AND x.credential_revision=NEW.credential_revision)
        )
        BEGIN SELECT RAISE(ABORT, 'compute route credential revocation replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorizations_no_replace
        BEFORE INSERT ON compute_route_authorization_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_route_authorization_receipts x
             WHERE x.route_authorization_id=NEW.route_authorization_id
                OR x.route_authorization_digest=NEW.route_authorization_digest
        )
        BEGIN SELECT RAISE(ABORT, 'compute route authorization replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_caps_no_replace
        BEFORE INSERT ON compute_route_authorization_capabilities
        WHEN EXISTS (
            SELECT 1 FROM compute_route_authorization_capabilities x
             WHERE (x.route_authorization_id=NEW.route_authorization_id
                    AND x.ordinal=NEW.ordinal)
                OR (x.route_authorization_id=NEW.route_authorization_id
                    AND x.capability_id=NEW.capability_id)
        )
        BEGIN SELECT RAISE(ABORT, 'compute route capability replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_seals_no_replace
        BEFORE INSERT ON compute_route_authorization_seals
        WHEN EXISTS (
            SELECT 1 FROM compute_route_authorization_seals x
             WHERE x.route_authorization_id=NEW.route_authorization_id
                OR x.seal_id=NEW.seal_id OR x.seal_digest=NEW.seal_digest
        )
        BEGIN SELECT RAISE(ABORT, 'compute route authorization seal replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_actor_no_replace
        BEFORE INSERT ON compute_attempt_dispatch_actor_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_dispatch_actor_receipts x
             WHERE x.actor_receipt_id=NEW.actor_receipt_id
                OR x.actor_receipt_digest=NEW.actor_receipt_digest
                OR (x.command_id=NEW.command_id AND x.actor_phase=NEW.actor_phase)
        )
        BEGIN SELECT RAISE(ABORT, 'compute dispatch actor replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_lease_authority_no_replace
        BEFORE INSERT ON compute_attempt_lease_authority_bindings
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_lease_authority_bindings x
             WHERE (x.lease_authority_id=NEW.lease_authority_id
                    AND x.authority_revision=NEW.authority_revision)
                OR x.lease_authority_digest=NEW.lease_authority_digest
                OR x.application_id=NEW.application_id
                OR (x.lease_id=NEW.lease_id
                    AND x.fencing_generation=NEW.fencing_generation)
        )
        BEGIN SELECT RAISE(ABORT, 'compute lease authority replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_outbox_no_replace
        BEFORE INSERT ON compute_attempt_start_outbox
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_start_outbox x
             WHERE x.outbox_id=NEW.outbox_id OR x.outbox_digest=NEW.outbox_digest
                OR (x.command_id=NEW.command_id
                    AND x.operation_kind=NEW.operation_kind
                    AND x.operation_generation=NEW.operation_generation)
                OR (NEW.operation_kind='prepare' AND x.operation_kind='prepare'
                    AND x.command_id=NEW.command_id)
                OR (NEW.operation_kind='commit' AND x.operation_kind='commit'
                    AND x.application_id=NEW.application_id)
        )
        BEGIN SELECT RAISE(ABORT, 'compute Start outbox replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_send_no_replace
        BEFORE INSERT ON compute_attempt_start_send_attempts
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_start_send_attempts x
             WHERE x.send_attempt_id=NEW.send_attempt_id
                OR x.send_attempt_digest=NEW.send_attempt_digest
                OR (x.outbox_id=NEW.outbox_id AND x.attempt_no=NEW.attempt_no)
        )
        BEGIN SELECT RAISE(ABORT, 'compute Start send-attempt replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_start_observation_no_replace
        BEFORE INSERT ON compute_attempt_start_remote_observations
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_start_remote_observations x
             WHERE x.observation_id=NEW.observation_id
                OR x.observation_digest=NEW.observation_digest
                OR (x.provider_id=NEW.provider_id AND x.adapter_id=NEW.adapter_id
                    AND x.adapter_observation_id=NEW.adapter_observation_id)
                OR (x.send_attempt_id=NEW.send_attempt_id
                    AND x.observation_id=NEW.observation_id)
        )
        BEGIN SELECT RAISE(ABORT, 'compute Start observation replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_no_start_proof_no_replace
        BEFORE INSERT ON compute_attempt_no_start_proofs
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_no_start_proofs x
             WHERE x.proof_id=NEW.proof_id OR x.proof_digest=NEW.proof_digest
                OR x.command_id=NEW.command_id OR x.reservation_id=NEW.reservation_id
                OR x.lease_id=NEW.lease_id
        )
        BEGIN SELECT RAISE(ABORT, 'compute no-start proof replacement is forbidden'); END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_broker_finish_no_replace_v213
        BEFORE INSERT ON compute_broker_finish_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_broker_finish_receipts x
             WHERE x.reservation_id=NEW.reservation_id
                OR (x.consumer_account_id=NEW.consumer_account_id
                    AND x.idempotency_key=NEW.idempotency_key)
        )
        BEGIN SELECT RAISE(ABORT, 'compute broker finish receipt replacement is forbidden'); END;
        "#,
    )?;
    Ok(())
}
