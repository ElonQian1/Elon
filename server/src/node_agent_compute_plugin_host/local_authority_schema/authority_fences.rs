/// Cross-cutting monotonic fences for authority and process ownership transitions.
pub(super) const AUTHORITY_FENCE_SCHEMA_V2: &str = r#"
CREATE TRIGGER authority_state_revision_monotonic
BEFORE UPDATE OF state_revision ON authority_meta
WHEN NEW.state_revision IS NOT OLD.state_revision
 AND NEW.state_revision <> OLD.state_revision + 1
BEGIN
    SELECT RAISE(ABORT, 'authority state revision must advance exactly once');
END;

CREATE TRIGGER authority_epoch_transition_fenced
BEFORE UPDATE OF authority_epoch ON authority_meta
WHEN NEW.authority_epoch IS NOT OLD.authority_epoch
 AND (
    NEW.authority_epoch <> OLD.authority_epoch + 1
    OR NEW.state_revision <> OLD.state_revision + 1
    OR NEW.process_owner_epoch <> OLD.process_owner_epoch
    OR EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
 )
BEGIN
    SELECT RAISE(ABORT, 'authority epoch change must fence every prepared claim');
END;

CREATE TRIGGER authority_process_owner_transition_fenced
BEFORE UPDATE OF process_owner_epoch ON authority_meta
WHEN NEW.process_owner_epoch IS NOT OLD.process_owner_epoch
 AND (
    NEW.process_owner_epoch <> OLD.process_owner_epoch + 1
    OR NEW.state_revision <> OLD.state_revision + 1
    OR NEW.authority_epoch <> OLD.authority_epoch
    OR EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
 )
BEGIN
    SELECT RAISE(ABORT, 'process owner change must fence every prepared claim');
END;
"#;
