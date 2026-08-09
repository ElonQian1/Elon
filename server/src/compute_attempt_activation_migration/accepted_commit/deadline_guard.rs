use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_application_live_authority_v215
        BEFORE INSERT ON compute_attempt_dispatch_applications
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_actor_receipts actor
              JOIN compute_attempt_dispatch_commands command
                ON command.command_id=NEW.command_id
               AND command.command_digest=actor.command_digest
               AND command.lease_id=NEW.lease_id
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=actor.route_authorization_id
               AND route.route_authorization_digest=actor.route_authorization_digest
              JOIN compute_attempt_lease_authority_bindings authority
                ON authority.application_id=NEW.application_id
               AND authority.application_digest=NEW.application_digest
              JOIN compute_attempt_start_outbox commit_intent
                ON commit_intent.command_id=NEW.command_id
               AND commit_intent.operation_kind='commit'
               AND commit_intent.ack_id=NEW.ack_id
               AND commit_intent.application_id=NEW.application_id
               AND commit_intent.application_digest=NEW.application_digest
             WHERE actor.command_id=NEW.command_id
               AND actor.actor_phase='application'
               AND actor.ack_id=NEW.ack_id
               AND actor.application_id=NEW.application_id
               AND actor.application_digest=NEW.application_digest
               AND route.verified_by_service_actor_id=actor.service_actor_id
               AND route.actor_authorization_id=actor.actor_authorization_id
               AND route.actor_authorization_digest=actor.actor_authorization_digest
               AND authority.command_id=NEW.command_id
               AND authority.ack_id=NEW.ack_id
               AND authority.lease_id=NEW.lease_id
               AND authority.application_actor_receipt_id=actor.actor_receipt_id
               AND authority.application_actor_receipt_digest=actor.actor_receipt_digest
               AND commit_intent.lease_id=NEW.lease_id
               AND commit_intent.actor_receipt_id=actor.actor_receipt_id
               AND commit_intent.actor_receipt_digest=actor.actor_receipt_digest
               AND commit_intent.lease_authority_id=authority.lease_authority_id
               AND commit_intent.lease_authority_revision=authority.authority_revision
               AND commit_intent.lease_authority_digest=authority.lease_authority_digest
               AND commit_intent.route_authorization_id=actor.route_authorization_id
               AND commit_intent.route_authorization_digest=actor.route_authorization_digest
               AND commit_intent.created_at<=NEW.created_at
               AND NEW.created_at<commit_intent.not_after
               AND authority.recorded_at<=NEW.created_at
               AND actor.recorded_at<=NEW.created_at
               AND NEW.applied_at<route.expires_at
               AND NEW.created_at<route.expires_at
               AND NEW.applied_at<actor.valid_until
               AND NEW.created_at<actor.valid_until
               AND NEW.applied_at<command.lease_expires_at
               AND NEW.created_at<command.lease_expires_at
               AND NEW.applied_at<authority.expires_at
               AND NEW.created_at<authority.expires_at
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute Attempt application is outside live actor authority');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_commit_live_authority_v215
        BEFORE INSERT ON compute_attempt_start_outbox
        WHEN NEW.operation_kind='commit' AND NOT EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_actor_receipts actor
              JOIN compute_attempt_dispatch_commands command
                ON command.command_id=NEW.command_id
               AND command.command_digest=NEW.command_digest
               AND command.lease_id=NEW.lease_id
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=NEW.route_authorization_id
               AND route.route_authorization_digest=NEW.route_authorization_digest
              JOIN compute_attempt_lease_authority_bindings authority
                ON authority.lease_authority_id=NEW.lease_authority_id
               AND authority.authority_revision=NEW.lease_authority_revision
               AND authority.lease_authority_digest=NEW.lease_authority_digest
             WHERE actor.actor_receipt_id=NEW.actor_receipt_id
               AND actor.actor_receipt_digest=NEW.actor_receipt_digest
               AND actor.actor_phase='application'
               AND actor.command_id=NEW.command_id
               AND actor.command_digest=NEW.command_digest
               AND actor.route_authorization_id=NEW.route_authorization_id
               AND actor.route_authorization_digest=NEW.route_authorization_digest
               AND actor.ack_id=NEW.ack_id
               AND actor.ack_digest=NEW.ack_digest
               AND actor.application_id=NEW.application_id
               AND actor.application_digest=NEW.application_digest
               AND route.verified_by_service_actor_id=actor.service_actor_id
               AND route.actor_authorization_id=actor.actor_authorization_id
               AND route.actor_authorization_digest=actor.actor_authorization_digest
               AND authority.command_id=NEW.command_id
               AND authority.command_digest=NEW.command_digest
               AND authority.ack_id=NEW.ack_id
               AND authority.ack_digest=NEW.ack_digest
               AND authority.application_id=NEW.application_id
               AND authority.application_digest=NEW.application_digest
               AND authority.lease_id=NEW.lease_id
               AND authority.route_authorization_id=NEW.route_authorization_id
               AND authority.route_authorization_digest=NEW.route_authorization_digest
               AND authority.application_actor_receipt_id=actor.actor_receipt_id
               AND authority.application_actor_receipt_digest=actor.actor_receipt_digest
               AND authority.recorded_at<=NEW.created_at
               AND actor.recorded_at<=NEW.created_at
               AND NEW.created_at<route.expires_at
               AND NEW.created_at<actor.valid_until
               AND NEW.not_after<=route.expires_at
               AND NEW.not_after<=actor.valid_until
               AND NEW.created_at<NEW.not_after
               AND NEW.not_before<NEW.not_after
               AND NEW.created_at<command.lease_expires_at
               AND NEW.not_after<=command.lease_expires_at
               AND NEW.created_at<authority.expires_at
               AND NEW.not_after<=authority.expires_at
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute Attempt commit is outside live actor authority');
        END;
        "#,
    )?;
    Ok(())
}
