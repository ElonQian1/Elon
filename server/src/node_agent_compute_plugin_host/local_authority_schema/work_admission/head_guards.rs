/// Strict generation and replacement guards for the mutable current-head projection.
///
/// `INSERT OR REPLACE` may bypass delete triggers when recursive triggers are disabled, so every
/// identity collision is rejected before SQLite can perform an implicit delete.
pub(super) const WORK_ADMISSION_HEAD_GUARDS_SCHEMA_V8: &str = r#"
CREATE TRIGGER compute_plugin_work_admission_heads_initial
BEFORE INSERT ON compute_plugin_work_admission_heads
WHEN NEW.work_admission_generation <> 1
  OR NEW.previous_work_admission_id IS NOT NULL
  OR NEW.previous_work_admission_receipt_digest IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'work admission head must start at generation one');
END;

CREATE TRIGGER compute_plugin_work_admission_heads_no_replace
BEFORE INSERT ON compute_plugin_work_admission_heads
WHEN EXISTS (
    SELECT 1 FROM compute_plugin_work_admission_heads AS stored
    WHERE stored.plugin_id = NEW.plugin_id
       OR stored.work_admission_id = NEW.work_admission_id
       OR stored.receipt_digest = NEW.receipt_digest
)
BEGIN
    SELECT RAISE(ABORT, 'work admission head replacement is forbidden');
END;

CREATE TRIGGER compute_plugin_work_admission_heads_linear_update
BEFORE UPDATE ON compute_plugin_work_admission_heads
WHEN NEW.installation_id_digest <> OLD.installation_id_digest
  OR NEW.plugin_id <> OLD.plugin_id
  OR NEW.work_admission_generation <> OLD.work_admission_generation + 1
  OR NEW.work_admission_id = OLD.work_admission_id
  OR NEW.receipt_digest = OLD.receipt_digest
  OR NEW.previous_work_admission_id <> OLD.work_admission_id
  OR NEW.previous_work_admission_receipt_digest <> OLD.receipt_digest
  OR NEW.updated_at_ms <= OLD.updated_at_ms
  OR EXISTS (
      SELECT 1 FROM compute_plugin_work_admission_heads AS peer
      WHERE peer.plugin_id <> OLD.plugin_id
        AND (peer.work_admission_id = NEW.work_admission_id
             OR peer.receipt_digest = NEW.receipt_digest)
  )
BEGIN
    SELECT RAISE(ABORT, 'work admission head CAS is not the exact next generation');
END;

CREATE TRIGGER compute_plugin_work_admission_heads_delete_forbidden
BEFORE DELETE ON compute_plugin_work_admission_heads
BEGIN
    SELECT RAISE(ABORT, 'work admission current heads cannot be deleted');
END;

CREATE TRIGGER compute_plugin_work_admission_receipts_no_replace
BEFORE INSERT ON compute_plugin_work_admission_receipts
WHEN EXISTS (
    SELECT 1 FROM compute_plugin_work_admission_receipts AS stored
    WHERE stored.work_admission_id = NEW.work_admission_id
       OR stored.source_digest = NEW.source_digest
       OR stored.receipt_digest = NEW.receipt_digest
       OR (stored.installation_id_digest = NEW.installation_id_digest
           AND stored.plugin_id = NEW.plugin_id
           AND stored.work_admission_generation_after =
               NEW.work_admission_generation_after)
)
BEGIN
    SELECT RAISE(ABORT, 'work admission receipt replacement is forbidden');
END;

CREATE TRIGGER compute_plugin_work_admission_receipts_update_forbidden
BEFORE UPDATE ON compute_plugin_work_admission_receipts
BEGIN
    SELECT RAISE(ABORT, 'work admission receipts are immutable');
END;

CREATE TRIGGER compute_plugin_work_admission_receipts_delete_forbidden
BEFORE DELETE ON compute_plugin_work_admission_receipts
BEGIN
    SELECT RAISE(ABORT, 'work admission receipts are append-only');
END;
"#;
