/// Relational projections over the canonical inventory JSON.
///
/// These guards keep generation and active-slot facts anchored to the unique current plugin
/// record. The core receipt DDL remains separately bounded so neither schema leaf becomes large.
pub(super) const CANDIDATE_PROMOTION_PROJECTION_SCHEMA_V7: &str = r#"
CREATE TRIGGER candidate_install_inventory_projection_fenced
BEFORE INSERT ON candidate_install_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN json_each(meta.inventory_json, '$.plugins') AS record
    WHERE meta.singleton = 1
      AND json_extract(record.value, '$.plugin_id') = NEW.plugin_id
      AND json_type(record.value, '$.install_generation') = 'integer'
      AND json_extract(record.value, '$.install_generation') =
          NEW.install_generation_before
      AND json_extract(record.value, '$.candidate_slot_ref') = NEW.slot_ref
      AND (SELECT COUNT(*)
           FROM json_each(meta.inventory_json, '$.plugins') AS peer
           WHERE json_extract(peer.value, '$.plugin_id') = NEW.plugin_id) = 1
      AND EXISTS (
          SELECT 1
          FROM json_each(record.value, '$.slots') AS slot
          WHERE json_extract(slot.value, '$.slot_ref') = NEW.slot_ref
            AND json_extract(slot.value, '$.phase') = 'staged'
            AND json_extract(slot.value, '$.release') = NEW.release_json
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate install generations lost the current inventory record');
END;

CREATE TRIGGER candidate_promotion_inventory_projection_fenced
BEFORE INSERT ON candidate_promotion_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM candidate_install_receipts AS installation
    JOIN authority_meta AS meta ON meta.singleton = 1
    JOIN json_each(meta.inventory_json, '$.plugins') AS record
    WHERE installation.install_id = NEW.install_id
      AND installation.receipt_digest = NEW.install_receipt_digest
      AND json_extract(record.value, '$.plugin_id') = NEW.plugin_id
      AND json_type(record.value, '$.install_generation') = 'integer'
      AND json_type(record.value, '$.activation_generation') = 'integer'
      AND json_extract(record.value, '$.install_generation') =
          installation.install_generation_before
      AND json_extract(record.value, '$.activation_generation') =
          NEW.activation_generation_before
      AND json_extract(record.value, '$.candidate_slot_ref') = NEW.slot_ref
      AND (SELECT COUNT(*)
           FROM json_each(meta.inventory_json, '$.plugins') AS peer
           WHERE json_extract(peer.value, '$.plugin_id') = NEW.plugin_id) = 1
      AND (
          (NEW.previous_active_slot_ref IS NULL
           AND json_extract(record.value, '$.active_slot_ref') IS NULL)
          OR
          (json_extract(record.value, '$.active_slot_ref') =
               NEW.previous_active_slot_ref
           AND EXISTS (
               SELECT 1
               FROM json_each(record.value, '$.slots') AS active_slot
               WHERE json_extract(active_slot.value, '$.slot_ref') =
                     NEW.previous_active_slot_ref
                 AND json_extract(active_slot.value, '$.phase') = 'installed'
                 AND json_extract(active_slot.value, '$.release') =
                     NEW.previous_active_release_json
           ))
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate activation lost the current inventory record');
END;

CREATE TRIGGER candidate_promotion_inventory_after_fenced
BEFORE UPDATE OF state, closed_at_ms, closed_by_plan_id, closed_by_plan_digest, close_reason
ON candidate_owners
WHEN NEW.state = 'promoted' AND NOT EXISTS (
    SELECT 1
    FROM candidate_promotion_receipts AS promotion
    JOIN json_each(promotion.inventory_json_after, '$.plugins') AS record
    WHERE promotion.candidate_token = OLD.candidate_token
      AND json_extract(record.value, '$.plugin_id') = OLD.plugin_id
      AND json_type(record.value, '$.install_generation') = 'integer'
      AND json_type(record.value, '$.activation_generation') = 'integer'
      AND json_extract(record.value, '$.install_generation') =
          promotion.install_generation_after
      AND json_extract(record.value, '$.activation_generation') =
          promotion.activation_generation_after
      AND json_extract(record.value, '$.active_slot_ref') = OLD.slot_ref
      AND json_extract(record.value, '$.candidate_slot_ref') IS NULL
      AND json_extract(record.value, '$.permission_grant_digest') =
          OLD.permission_grant_digest
      AND (SELECT COUNT(*)
           FROM json_each(promotion.inventory_json_after, '$.plugins') AS peer
           WHERE json_extract(peer.value, '$.plugin_id') = OLD.plugin_id) = 1
      AND EXISTS (
          SELECT 1
          FROM json_each(record.value, '$.slots') AS slot
          WHERE json_extract(slot.value, '$.slot_ref') = OLD.slot_ref
            AND json_extract(slot.value, '$.phase') = 'installed'
            AND json_extract(slot.value, '$.release') = OLD.release_json
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate promotion inventory is not exact installed and active state');
END;
"#;
