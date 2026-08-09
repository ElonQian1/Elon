/// SQLite does not fire DELETE triggers for the implicit delete performed by `OR REPLACE` unless
/// recursive triggers are enabled. Reject every identity collision before replacement can rewrite
/// either half of the immutable receipt pair.
pub(super) const CANDIDATE_PROMOTION_REPLACEMENT_GUARDS_V7: &str = r#"
CREATE TRIGGER candidate_install_receipts_no_replace
BEFORE INSERT ON candidate_install_receipts
WHEN EXISTS (
    SELECT 1 FROM candidate_install_receipts AS stored
    WHERE stored.install_id = NEW.install_id
       OR stored.promotion_id = NEW.promotion_id
       OR stored.candidate_token = NEW.candidate_token
       OR stored.staging_id = NEW.staging_id
       OR stored.health_id = NEW.health_id
       OR stored.install_evidence_digest = NEW.install_evidence_digest
       OR stored.receipt_digest = NEW.receipt_digest
)
BEGIN
    SELECT RAISE(ABORT, 'candidate install receipt replacement is forbidden');
END;

CREATE TRIGGER candidate_promotion_receipts_no_replace
BEFORE INSERT ON candidate_promotion_receipts
WHEN EXISTS (
    SELECT 1 FROM candidate_promotion_receipts AS stored
    WHERE stored.promotion_id = NEW.promotion_id
       OR stored.install_id = NEW.install_id
       OR stored.install_receipt_digest = NEW.install_receipt_digest
       OR stored.candidate_token = NEW.candidate_token
       OR stored.staging_id = NEW.staging_id
       OR stored.health_id = NEW.health_id
       OR stored.active_provenance_digest = NEW.active_provenance_digest
       OR stored.receipt_digest = NEW.receipt_digest
)
BEGIN
    SELECT RAISE(ABORT, 'candidate promotion receipt replacement is forbidden');
END;
"#;
