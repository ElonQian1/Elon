use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_terminal_exact_source
        BEFORE INSERT ON compute_external_pool_adapter_release_admission_terminal_receipts
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_release_admissions admission
              JOIN compute_external_pool_adapter_release_requests request
                ON request.request_id=admission.request_id
               AND request.request_digest=admission.request_digest
               AND request.request_material_digest=admission.request_material_digest
              JOIN compute_external_pool_adapter_release_reviews review
                ON review.review_id=admission.review_id
               AND review.review_digest=admission.review_digest
               AND review.request_id=admission.request_id
               AND review.request_digest=admission.request_digest
               AND review.request_material_digest=admission.request_material_digest
             WHERE admission.admission_id=NEW.admission_id
               AND admission.admission_digest=NEW.admission_digest
               AND admission.adapter_id=NEW.adapter_id
               AND admission.release_version=NEW.release_version
               AND admission.status='staged'
               AND admission.applied_at<=NEW.occurred_at
               AND request.adapter_id=NEW.adapter_id
               AND request.release_version=NEW.release_version
               AND request.status='staged'
               AND request.reviewed_by_admin_user_id=admission.reviewed_by_admin_user_id
               AND request.reviewed_at=review.reviewed_at
               AND request.applied_by_admin_user_id=admission.applied_by_admin_user_id
               AND request.applied_at=admission.applied_at
               AND review.adapter_id=NEW.adapter_id
               AND review.release_version=NEW.release_version
               AND review.decision='approved'
               AND review.reviewed_by_admin_user_id=admission.reviewed_by_admin_user_id
               AND review.reviewed_at<=admission.applied_at
        )
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release terminal lacks exact staged admission');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_terminal_successor
        BEFORE INSERT ON compute_external_pool_adapter_release_admission_terminal_receipts
        WHEN NEW.terminal_status='superseded' AND NOT EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_release_admissions source
              JOIN compute_external_pool_adapter_release_admissions successor
                ON successor.admission_id=NEW.successor_admission_id
               AND successor.admission_digest=NEW.successor_admission_digest
               AND successor.release_version=NEW.successor_release_version
             WHERE source.admission_id=NEW.admission_id
               AND source.admission_digest=NEW.admission_digest
               AND source.adapter_id=NEW.adapter_id
               AND source.release_version=NEW.release_version
               AND successor.adapter_id=source.adapter_id
               AND successor.admission_id<>source.admission_id
               AND successor.release_version<>source.release_version
               AND successor.status='staged'
               AND successor.applied_at>=source.applied_at
               AND successor.applied_at<=NEW.occurred_at
               AND NOT EXISTS (
                    SELECT 1
                      FROM compute_external_pool_adapter_release_admission_terminal_receipts
                           successor_terminal
                     WHERE successor_terminal.admission_id=successor.admission_id)
        )
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release terminal lacks an exact current successor');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_artifact_source_current_admission
        BEFORE INSERT ON compute_external_pool_adapter_artifact_source_receipts
        WHEN EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_release_admission_terminal_receipts terminal
             WHERE terminal.admission_id=NEW.admission_id)
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter artifact source admission is terminal');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_terminal_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_release_admission_terminal_receipts
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release admission terminals are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_terminal_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_release_admission_terminal_receipts
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release admission terminals are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_terminal_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_release_admission_terminal_receipts
        WHEN EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_release_admission_terminal_receipts existing
             WHERE existing.terminal_receipt_id=NEW.terminal_receipt_id
                OR existing.terminal_receipt_digest=NEW.terminal_receipt_digest
                OR existing.admission_id=NEW.admission_id
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release admission terminals cannot be replaced'); END;
        "#,
    )?;
    Ok(())
}
