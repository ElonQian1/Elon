use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    for (prefix, table, identity) in [
        (
            "external_pool_adapter_credential_reattestation_challenge",
            "compute_external_pool_adapter_credential_reattestation_challenges",
            "old.challenge_id=NEW.challenge_id OR old.challenge_nonce_digest=NEW.challenge_nonce_digest",
        ),
        (
            "external_pool_adapter_credential_reattestation_receipt",
            "compute_external_pool_adapter_credential_reattestation_receipts",
            "old.reattestation_receipt_id=NEW.reattestation_receipt_id OR old.reattestation_receipt_digest=NEW.reattestation_receipt_digest OR old.challenge_id=NEW.challenge_id OR old.verifier_report_id=NEW.verifier_report_id OR (old.provider_binding_id=NEW.provider_binding_id AND old.sequence=NEW.sequence) OR old.predecessor_receipt_id=NEW.predecessor_receipt_id OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)",
        ),
        (
            "external_pool_adapter_credential_reattestation_revocation",
            "compute_external_pool_adapter_credential_reattestation_revocations",
            "old.revocation_receipt_id=NEW.revocation_receipt_id OR old.revocation_receipt_digest=NEW.revocation_receipt_digest OR old.reattestation_receipt_id=NEW.reattestation_receipt_id OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)",
        ),
    ] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS {prefix}_no_update BEFORE UPDATE ON {table}
             BEGIN SELECT RAISE(ABORT,'V253 authority is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS {prefix}_no_delete BEFORE DELETE ON {table}
             BEGIN SELECT RAISE(ABORT,'V253 authority is append-only'); END;
             CREATE TRIGGER IF NOT EXISTS {prefix}_no_replace BEFORE INSERT ON {table}
             WHEN EXISTS(SELECT 1 FROM {table} old WHERE {identity})
             BEGIN SELECT RAISE(ABORT,'V253 authority cannot replace immutable history'); END;"
        ))?;
    }
    Ok(())
}
