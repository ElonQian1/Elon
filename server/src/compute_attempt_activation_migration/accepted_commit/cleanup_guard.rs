use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_accepted_ack_blocks_cleanup_v215
        BEFORE INSERT ON compute_attempt_dispatch_acks
        WHEN NEW.outcome='accepted' AND NEW.disposition='accepted_applied'
          AND EXISTS (
            SELECT 1 FROM compute_attempt_start_outbox cleanup
             WHERE cleanup.command_id=NEW.command_id
               AND cleanup.operation_kind IN ('cancel','reconcile')
          )
        BEGIN
            SELECT RAISE(ABORT, 'accepted Attempt ACK conflicts with cleanup custody');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_activation_blocks_cleanup_v215
        BEFORE INSERT ON compute_attempt_activations
        WHEN EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_commands command
              JOIN compute_attempt_start_outbox cleanup
                ON cleanup.command_id=command.command_id
               AND cleanup.operation_kind IN ('cancel','reconcile')
             WHERE command.lease_id=NEW.lease_id
                OR command.reservation_id=NEW.reservation_id
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute Attempt activation conflicts with cleanup custody');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_application_blocks_cleanup_v215
        BEFORE INSERT ON compute_attempt_dispatch_applications
        WHEN EXISTS (
            SELECT 1 FROM compute_attempt_start_outbox cleanup
             WHERE cleanup.command_id=NEW.command_id
               AND cleanup.operation_kind IN ('cancel','reconcile')
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute Attempt application conflicts with cleanup custody');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_commit_blocks_cleanup_v215
        BEFORE INSERT ON compute_attempt_start_outbox
        WHEN NEW.operation_kind='commit' AND EXISTS (
            SELECT 1 FROM compute_attempt_start_outbox cleanup
             WHERE cleanup.command_id=NEW.command_id
               AND cleanup.operation_kind IN ('cancel','reconcile')
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute Attempt commit conflicts with cleanup custody');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_cleanup_blocks_accepted_v215
        BEFORE INSERT ON compute_attempt_start_outbox
        WHEN NEW.operation_kind IN ('cancel','reconcile') AND (
            EXISTS (
                SELECT 1 FROM compute_attempt_dispatch_acks ack
                 WHERE ack.command_id=NEW.command_id
                   AND ack.outcome='accepted'
                   AND ack.disposition='accepted_applied'
            )
            OR EXISTS (
                SELECT 1 FROM compute_attempt_dispatch_applications application
                 WHERE application.command_id=NEW.command_id
            )
            OR EXISTS (
                SELECT 1 FROM compute_attempt_start_outbox commit_intent
                 WHERE commit_intent.command_id=NEW.command_id
                   AND commit_intent.operation_kind='commit'
            )
            OR EXISTS (
                SELECT 1
                  FROM compute_attempt_dispatch_commands command
                  JOIN compute_attempt_activations activation
                    ON activation.lease_id=command.lease_id
                    OR activation.reservation_id=command.reservation_id
                 WHERE command.command_id=NEW.command_id
            )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute Attempt cleanup conflicts with accepted custody');
        END;
        "#,
    )?;
    Ok(())
}
