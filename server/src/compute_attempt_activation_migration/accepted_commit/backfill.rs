use anyhow::{bail, Result};
use rusqlite::Connection;

pub(super) fn ensure_no_unsafe_backfill(conn: &Connection) -> Result<()> {
    let cleanup_conflict = conn.query_row(CLEANUP_CONFLICT, [], |row| row.get::<_, bool>(0))?;
    let actor_mismatch = conn.query_row(ACTOR_MISMATCH, [], |row| row.get::<_, bool>(0))?;
    let expired_write = conn.query_row(EXPIRED_WRITE, [], |row| row.get::<_, bool>(0))?;
    if cleanup_conflict || actor_mismatch || expired_write {
        bail!("COMPUTE_ATTEMPT_ACCEPTED_COMMIT_BACKFILL_REQUIRED");
    }
    Ok(())
}

const CLEANUP_CONFLICT: &str = r#"
    SELECT EXISTS(
        SELECT 1
          FROM compute_attempt_start_outbox cleanup
         WHERE cleanup.operation_kind IN ('cancel','reconcile')
           AND (
                EXISTS (
                    SELECT 1 FROM compute_attempt_dispatch_acks ack
                     WHERE ack.command_id=cleanup.command_id
                       AND ack.outcome='accepted'
                       AND ack.disposition='accepted_applied'
                )
                OR EXISTS (
                    SELECT 1 FROM compute_attempt_dispatch_applications application
                     WHERE application.command_id=cleanup.command_id
                )
                OR EXISTS (
                    SELECT 1 FROM compute_attempt_start_outbox commit_intent
                     WHERE commit_intent.command_id=cleanup.command_id
                       AND commit_intent.operation_kind='commit'
                )
                OR EXISTS (
                    SELECT 1
                      FROM compute_attempt_dispatch_commands command
                      JOIN compute_attempt_activations activation
                        ON activation.lease_id=command.lease_id
                        OR activation.reservation_id=command.reservation_id
                     WHERE command.command_id=cleanup.command_id
                )
           )
    )
"#;

const ACTOR_MISMATCH: &str = r#"
    SELECT EXISTS(
        SELECT 1
          FROM compute_attempt_dispatch_actor_receipts actor
         WHERE actor.actor_phase='application'
           AND NOT EXISTS (
                SELECT 1
                  FROM compute_attempt_dispatch_commands command
                  JOIN compute_route_authorization_receipts route
                    ON route.route_authorization_id=actor.route_authorization_id
                   AND route.route_authorization_digest=actor.route_authorization_digest
                  JOIN compute_service_actor_authorizations authority
                    ON authority.actor_authorization_id=actor.actor_authorization_id
                   AND authority.actor_authorization_digest=actor.actor_authorization_digest
                  JOIN compute_attempt_start_outbox prepare
                    ON prepare.command_id=command.command_id
                   AND prepare.command_digest=command.command_digest
                   AND prepare.operation_kind='prepare'
                   AND prepare.operation_generation=1
                   AND prepare.route_authorization_id=route.route_authorization_id
                   AND prepare.route_authorization_digest=route.route_authorization_digest
                 WHERE command.command_id=actor.command_id
                   AND command.command_digest=actor.command_digest
                   AND command.provider_id=actor.provider_id
                   AND command.activated_by_user_id=actor.provider_owner_account_id
                   AND route.provider_id=actor.provider_id
                   AND route.provider_owner_account_id=actor.provider_owner_account_id
                   AND route.executor_id=command.executor_id
                   AND route.adapter_id=command.adapter_id
                   AND route.adapter_binding_digest=command.adapter_binding_digest
                   AND route.verified_by_service_actor_id=actor.service_actor_id
                   AND route.actor_authorization_id=actor.actor_authorization_id
                   AND route.actor_authorization_digest=actor.actor_authorization_digest
                   AND authority.provider_id=actor.provider_id
                   AND authority.provider_owner_account_id=actor.provider_owner_account_id
                   AND authority.service_actor_id=actor.service_actor_id
                   AND authority.issued_at<=actor.issued_at
                   AND authority.recorded_at<=actor.issued_at
                   AND actor.valid_until<=authority.valid_until
                   AND route.authorized_at<=actor.issued_at
                   AND route.recorded_at<=actor.issued_at
                   AND actor.recorded_at<route.expires_at
                   AND actor.valid_until<=route.expires_at
                   AND EXISTS (
                        SELECT 1 FROM json_each(authority.allowed_actor_phases_json) allowed
                         WHERE allowed.type='text' AND allowed.value=actor.actor_phase
                   )
           )
    )
"#;

const EXPIRED_WRITE: &str = r#"
    SELECT EXISTS(
        SELECT 1
          FROM compute_attempt_dispatch_applications application
         WHERE NOT EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_actor_receipts actor
              JOIN compute_attempt_dispatch_commands command
                ON command.command_id=application.command_id
               AND command.command_digest=actor.command_digest
               AND command.lease_id=application.lease_id
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=actor.route_authorization_id
               AND route.route_authorization_digest=actor.route_authorization_digest
              JOIN compute_attempt_lease_authority_bindings authority
                ON authority.application_id=application.application_id
               AND authority.application_digest=application.application_digest
              JOIN compute_attempt_start_outbox commit_intent
                ON commit_intent.command_id=application.command_id
               AND commit_intent.operation_kind='commit'
               AND commit_intent.ack_id=application.ack_id
               AND commit_intent.application_id=application.application_id
               AND commit_intent.application_digest=application.application_digest
             WHERE actor.command_id=application.command_id
               AND actor.actor_phase='application'
               AND actor.ack_id=application.ack_id
               AND actor.application_id=application.application_id
               AND actor.application_digest=application.application_digest
               AND route.verified_by_service_actor_id=actor.service_actor_id
               AND route.actor_authorization_id=actor.actor_authorization_id
               AND route.actor_authorization_digest=actor.actor_authorization_digest
               AND authority.command_id=application.command_id
               AND authority.ack_id=application.ack_id
               AND authority.lease_id=application.lease_id
               AND authority.application_actor_receipt_id=actor.actor_receipt_id
               AND authority.application_actor_receipt_digest=actor.actor_receipt_digest
               AND commit_intent.lease_id=application.lease_id
               AND commit_intent.actor_receipt_id=actor.actor_receipt_id
               AND commit_intent.actor_receipt_digest=actor.actor_receipt_digest
               AND commit_intent.lease_authority_id=authority.lease_authority_id
               AND commit_intent.lease_authority_revision=authority.authority_revision
               AND commit_intent.lease_authority_digest=authority.lease_authority_digest
               AND commit_intent.route_authorization_id=actor.route_authorization_id
               AND commit_intent.route_authorization_digest=actor.route_authorization_digest
               AND commit_intent.created_at<=application.created_at
               AND application.created_at<commit_intent.not_after
               AND authority.recorded_at<=application.created_at
               AND actor.recorded_at<=application.created_at
               AND application.applied_at<route.expires_at
               AND application.created_at<route.expires_at
               AND application.applied_at<actor.valid_until
               AND application.created_at<actor.valid_until
               AND application.applied_at<command.lease_expires_at
               AND application.created_at<command.lease_expires_at
               AND application.applied_at<authority.expires_at
               AND application.created_at<authority.expires_at
         )
    ) OR EXISTS(
        SELECT 1
          FROM compute_attempt_start_outbox commit_intent
         WHERE commit_intent.operation_kind='commit'
           AND NOT EXISTS (
                SELECT 1
                  FROM compute_attempt_dispatch_actor_receipts actor
                  JOIN compute_attempt_dispatch_commands command
                    ON command.command_id=commit_intent.command_id
                   AND command.command_digest=commit_intent.command_digest
                   AND command.lease_id=commit_intent.lease_id
                  JOIN compute_route_authorization_receipts route
                    ON route.route_authorization_id=commit_intent.route_authorization_id
                   AND route.route_authorization_digest=commit_intent.route_authorization_digest
                  JOIN compute_attempt_lease_authority_bindings authority
                    ON authority.lease_authority_id=commit_intent.lease_authority_id
                   AND authority.authority_revision=commit_intent.lease_authority_revision
                   AND authority.lease_authority_digest=commit_intent.lease_authority_digest
                 WHERE actor.actor_receipt_id=commit_intent.actor_receipt_id
                   AND actor.actor_receipt_digest=commit_intent.actor_receipt_digest
                   AND actor.actor_phase='application'
                   AND actor.command_id=commit_intent.command_id
                   AND actor.command_digest=commit_intent.command_digest
                   AND actor.route_authorization_id=commit_intent.route_authorization_id
                   AND actor.route_authorization_digest=
                        commit_intent.route_authorization_digest
                   AND actor.ack_id=commit_intent.ack_id
                   AND actor.ack_digest=commit_intent.ack_digest
                   AND actor.application_id=commit_intent.application_id
                   AND actor.application_digest=commit_intent.application_digest
                   AND route.verified_by_service_actor_id=actor.service_actor_id
                   AND route.actor_authorization_id=actor.actor_authorization_id
                   AND route.actor_authorization_digest=actor.actor_authorization_digest
                   AND authority.command_id=commit_intent.command_id
                   AND authority.command_digest=commit_intent.command_digest
                   AND authority.ack_id=commit_intent.ack_id
                   AND authority.ack_digest=commit_intent.ack_digest
                   AND authority.application_id=commit_intent.application_id
                   AND authority.application_digest=commit_intent.application_digest
                   AND authority.lease_id=commit_intent.lease_id
                   AND authority.route_authorization_id=commit_intent.route_authorization_id
                   AND authority.route_authorization_digest=commit_intent.route_authorization_digest
                   AND authority.application_actor_receipt_id=actor.actor_receipt_id
                   AND authority.application_actor_receipt_digest=actor.actor_receipt_digest
                   AND authority.recorded_at<=commit_intent.created_at
                   AND actor.recorded_at<=commit_intent.created_at
                   AND commit_intent.created_at<route.expires_at
                   AND commit_intent.created_at<actor.valid_until
                   AND commit_intent.not_after<=route.expires_at
                   AND commit_intent.not_after<=actor.valid_until
                   AND commit_intent.created_at<commit_intent.not_after
                   AND commit_intent.not_before<commit_intent.not_after
                   AND commit_intent.created_at<command.lease_expires_at
                   AND commit_intent.not_after<=command.lease_expires_at
                   AND commit_intent.created_at<authority.expires_at
                   AND commit_intent.not_after<=authority.expires_at
           )
    )
"#;
