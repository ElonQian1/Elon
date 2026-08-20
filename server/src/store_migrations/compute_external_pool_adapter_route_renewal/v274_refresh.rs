use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "DROP TRIGGER IF EXISTS v278_provider_active_successor_refresh_pending_plan;
         DROP TRIGGER IF EXISTS v274_provider_active_successor_receipt_pending_seal;
         CREATE TRIGGER v274_provider_active_successor_receipt_pending_seal
         BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_receipts
         WHEN elon_v274_provider_active_successor_pending_process_seal_is_exact(
                'provider_active_successor_receipt',NEW.active_successor_receipt_id,
                NEW.receipt_digest,NEW.process_custody_epoch_digest,
                NEW.process_custody_nonce_digest,NEW.process_custody_seal_digest,
                NEW.receipt_integrity_digest) IS NOT 1
           OR (NEW.successor_sequence > 1 AND
               elon_v278_external_pool_adapter_provider_active_successor_refresh_pending_plan_matches(
                 'provider_active_successor_refresh',NEW.active_successor_receipt_id,
                 NEW.receipt_digest,NEW.receipt_json,NEW.provider_binding_id,
                 NEW.activation_root_digest,NEW.successor_sequence,
                 NEW.predecessor_active_successor_receipt_id,
                 NEW.predecessor_active_successor_receipt_digest,
                 NEW.activation_target_updated_at,NEW.evidence_checked_at,NEW.created_at,
                 NEW.observation_expires_at,NEW.process_custody_epoch_digest,
                 NEW.process_custody_nonce_digest,NEW.process_custody_seal_digest,
                 NEW.receipt_integrity_digest) IS NOT 1)
         BEGIN SELECT RAISE(ABORT,'V274 successor lacks exact pending process seal or V278 refresh plan'); END;",
    )?;
    Ok(())
}
