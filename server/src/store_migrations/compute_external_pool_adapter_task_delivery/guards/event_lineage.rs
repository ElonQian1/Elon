use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS v273_task_event_batch_source
        BEFORE INSERT ON compute_external_pool_adapter_task_event_batches
        WHEN NOT ((NEW.event_count=0 AND NEW.replay_classification='empty')
               OR (NEW.event_count>0 AND NEW.replay_classification='new'))
          OR NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_task_event_polls poll
            JOIN compute_external_pool_adapter_task_exchange_receipts receipt
              ON receipt.source_kind='event_poll'
             AND receipt.source_id=poll.event_poll_id
             AND receipt.source_digest=poll.event_poll_digest
             AND receipt.operation_kind='authenticated_events'
             AND receipt.exchange_receipt_id=NEW.exchange_receipt_id
             AND receipt.exchange_receipt_digest=NEW.exchange_receipt_digest
             AND receipt.semantic_observation_sha256=NEW.authenticated_observation_sha256
             WHERE poll.event_poll_id=NEW.event_poll_id
               AND poll.event_poll_digest=NEW.event_poll_digest
               AND poll.remote_execution_id=NEW.remote_execution_id
               AND poll.remote_identity_digest=NEW.remote_identity_digest
               AND poll.executor_binding_digest=NEW.executor_binding_digest
               AND ((poll.remote_execution_state='committed'
                     AND NEW.remote_execution_state IN ('committed','running','terminal_after_run'))
                 OR (poll.remote_execution_state='running'
                     AND NEW.remote_execution_state IN ('running','terminal_after_run')))
               AND poll.requested_remote_sequence=NEW.cursor_before_remote_sequence
               AND poll.requested_previous_event_root IS NEW.cursor_before_previous_event_root
               AND poll.requested_cursor_digest=NEW.cursor_before_digest
               AND poll.claim_status='claimed')
          OR (NEW.predecessor_event_batch_id IS NULL AND EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_task_event_batches prior
            JOIN compute_external_pool_adapter_task_event_polls prior_poll
              ON prior_poll.event_poll_id=prior.event_poll_id
            JOIN compute_external_pool_adapter_task_event_polls current_poll
              ON current_poll.event_poll_id=NEW.event_poll_id
             AND current_poll.event_poll_digest=NEW.event_poll_digest
             WHERE prior.remote_identity_digest=NEW.remote_identity_digest
               AND prior.executor_binding_digest=NEW.executor_binding_digest
               AND prior_poll.poll_ordinal<current_poll.poll_ordinal))
          OR (NEW.predecessor_event_batch_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_task_event_batches predecessor
            JOIN compute_external_pool_adapter_task_event_polls predecessor_poll
              ON predecessor_poll.event_poll_id=predecessor.event_poll_id
            JOIN compute_external_pool_adapter_task_event_polls current_poll
              ON current_poll.event_poll_id=NEW.event_poll_id
             AND current_poll.event_poll_digest=NEW.event_poll_digest
             WHERE predecessor.event_batch_id=NEW.predecessor_event_batch_id
               AND predecessor.event_batch_digest=NEW.predecessor_event_batch_digest
               AND predecessor.remote_execution_id=NEW.remote_execution_id
               AND predecessor.remote_identity_digest=NEW.remote_identity_digest
               AND predecessor.executor_binding_digest=NEW.executor_binding_digest
               AND predecessor.batch_root=NEW.previous_batch_root
               AND predecessor.cursor_after_remote_sequence=NEW.cursor_before_remote_sequence
               AND predecessor.cursor_after_previous_event_root IS NEW.cursor_before_previous_event_root
               AND predecessor.cursor_after_digest=NEW.cursor_before_digest
               AND predecessor_poll.poll_ordinal<current_poll.poll_ordinal
               AND NOT EXISTS (
                 SELECT 1 FROM compute_external_pool_adapter_task_event_batches later
                 JOIN compute_external_pool_adapter_task_event_polls later_poll
                   ON later_poll.event_poll_id=later.event_poll_id
                  WHERE later.remote_identity_digest=NEW.remote_identity_digest
                    AND later.executor_binding_digest=NEW.executor_binding_digest
                    AND predecessor_poll.poll_ordinal<later_poll.poll_ordinal
                    AND later_poll.poll_ordinal<current_poll.poll_ordinal)))
        BEGIN SELECT RAISE(ABORT,'V273 event batch lacks exact poll/receipt/cursor lineage'); END;

        CREATE TRIGGER IF NOT EXISTS v273_task_event_exact_batch_position
        BEFORE INSERT ON compute_external_pool_adapter_task_events
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_task_event_batches batch
           WHERE batch.event_batch_id=NEW.event_batch_id
             AND batch.event_batch_digest=NEW.event_batch_digest
             AND batch.remote_identity_digest=NEW.remote_identity_digest
             AND NEW.event_ordinal<=batch.event_count
             AND NEW.remote_sequence=batch.cursor_before_remote_sequence+NEW.event_ordinal
             AND json_extract(batch.event_roots_json,'$['||(NEW.event_ordinal-1)||']')=NEW.event_root
             AND NEW.recorded_at=batch.recorded_at
             AND ((NEW.event_ordinal=1 AND NEW.previous_event_root IS batch.cursor_before_previous_event_root)
               OR (NEW.event_ordinal>1 AND EXISTS (
                 SELECT 1 FROM compute_external_pool_adapter_task_events predecessor
                  WHERE predecessor.event_batch_id=NEW.event_batch_id
                    AND predecessor.event_ordinal=NEW.event_ordinal-1
                    AND predecessor.remote_sequence=NEW.remote_sequence-1
                    AND predecessor.event_root=NEW.previous_event_root))))
        BEGIN SELECT RAISE(ABORT,'V273 event is not the exact next batch event'); END;

        CREATE TRIGGER IF NOT EXISTS v273_task_event_remote_sequence_no_fork
        BEFORE INSERT ON compute_external_pool_adapter_task_events
        WHEN EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_task_events prior
          JOIN compute_external_pool_adapter_task_event_batches prior_batch
            ON prior_batch.event_batch_id=prior.event_batch_id
          JOIN compute_external_pool_adapter_task_event_batches new_batch
            ON new_batch.event_batch_id=NEW.event_batch_id
           WHERE prior_batch.remote_identity_digest=new_batch.remote_identity_digest
             AND prior.remote_sequence=NEW.remote_sequence
             AND prior.event_root<>NEW.event_root)
        BEGIN SELECT RAISE(ABORT,'V273 remote event sequence fork detected'); END;

        CREATE TRIGGER IF NOT EXISTS v273_task_event_remote_id_no_conflict
        BEFORE INSERT ON compute_external_pool_adapter_task_events
        WHEN EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_task_events prior
          JOIN compute_external_pool_adapter_task_event_batches prior_batch
            ON prior_batch.event_batch_id=prior.event_batch_id
          JOIN compute_external_pool_adapter_task_event_batches new_batch
            ON new_batch.event_batch_id=NEW.event_batch_id
           WHERE prior_batch.remote_identity_digest=new_batch.remote_identity_digest
             AND prior.remote_event_id=NEW.remote_event_id
             AND (prior.remote_sequence<>NEW.remote_sequence
               OR prior.event_root<>NEW.event_root
               OR prior.event_digest<>NEW.event_digest
               OR prior.canonical_event_digest<>NEW.canonical_event_digest))
        BEGIN SELECT RAISE(ABORT,'V273 remote event id conflicting replay detected'); END;
        "#,
    )?;
    Ok(())
}
