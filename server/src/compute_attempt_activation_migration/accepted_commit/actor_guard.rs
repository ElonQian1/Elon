use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS trg_compute_attempt_dispatch_actor_exact_source;
        CREATE TRIGGER trg_compute_attempt_dispatch_actor_exact_source
        BEFORE INSERT ON compute_attempt_dispatch_actor_receipts
        WHEN NEW.actor_phase='application' AND NOT EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_commands command
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=NEW.route_authorization_id
               AND route.route_authorization_digest=NEW.route_authorization_digest
              JOIN compute_service_actor_authorizations authority
                ON authority.actor_authorization_id=NEW.actor_authorization_id
               AND authority.actor_authorization_digest=NEW.actor_authorization_digest
              JOIN compute_attempt_start_outbox prepare
                ON prepare.command_id=command.command_id
               AND prepare.command_digest=command.command_digest
               AND prepare.operation_kind='prepare'
               AND prepare.operation_generation=1
               AND prepare.route_authorization_id=route.route_authorization_id
               AND prepare.route_authorization_digest=route.route_authorization_digest
             WHERE command.command_id=NEW.command_id
               AND command.command_digest=NEW.command_digest
               AND command.provider_id=NEW.provider_id
               AND command.activated_by_user_id=NEW.provider_owner_account_id
               AND route.provider_id=NEW.provider_id
               AND route.provider_owner_account_id=NEW.provider_owner_account_id
               AND route.executor_id=command.executor_id
               AND route.adapter_id=command.adapter_id
               AND route.adapter_binding_digest=command.adapter_binding_digest
               AND route.verified_by_service_actor_id=NEW.service_actor_id
               AND route.actor_authorization_id=NEW.actor_authorization_id
               AND route.actor_authorization_digest=NEW.actor_authorization_digest
               AND authority.provider_id=NEW.provider_id
               AND authority.provider_owner_account_id=NEW.provider_owner_account_id
               AND authority.service_actor_id=NEW.service_actor_id
               AND authority.issued_at<=NEW.issued_at
               AND authority.recorded_at<=NEW.issued_at
               AND NEW.valid_until<=authority.valid_until
               AND route.authorized_at<=NEW.issued_at
               AND route.recorded_at<=NEW.issued_at
               AND NEW.recorded_at<route.expires_at
               AND NEW.valid_until<=route.expires_at
               AND EXISTS (
                    SELECT 1 FROM json_each(authority.allowed_actor_phases_json) allowed
                     WHERE allowed.type='text' AND allowed.value=NEW.actor_phase
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt actor receipt lacks exact live route authority');
        END;
        "#,
    )?;
    Ok(())
}
