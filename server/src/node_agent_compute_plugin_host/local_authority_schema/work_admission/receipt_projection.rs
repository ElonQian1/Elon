/// Exact projection of the canonical Receipt and all nested transition bodies.
pub(super) const WORK_ADMISSION_RECEIPT_PROJECTION_SCHEMA_V8: &str = r#"
CREATE TRIGGER compute_plugin_work_admission_receipt_projection_fenced
BEFORE INSERT ON compute_plugin_work_admission_receipts
WHEN NOT EXISTS (
    SELECT 1
    WHERE json_type(NEW.receipt_json) = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.receipt_json)) = 16
      AND json_extract(NEW.receipt_json, '$.schema') =
          'elon.compute_plugin.work_admission_receipt.v1'
      AND json_extract(NEW.receipt_json, '$.work_admission_id') = NEW.work_admission_id
      AND json_extract(NEW.receipt_json, '$.installation_id_digest') =
          NEW.installation_id_digest
      AND json_extract(NEW.receipt_json, '$.clock_epoch_digest') = NEW.clock_epoch_digest
      AND json_extract(NEW.receipt_json, '$.plugin_id') = NEW.plugin_id
      AND json_extract(NEW.receipt_json, '$.slot_ref') = NEW.slot_ref
      AND json_type(NEW.receipt_json, '$.release') = 'object'
      AND json_extract(NEW.receipt_json, '$.release') = NEW.release_json
      AND json_extract(NEW.receipt_json, '$.install_receipt_id') =
          NEW.install_receipt_id
      AND json_extract(NEW.receipt_json, '$.install_receipt_digest') =
          NEW.install_receipt_digest
      AND json_extract(NEW.receipt_json, '$.promotion_receipt_id') =
          NEW.promotion_receipt_id
      AND json_extract(NEW.receipt_json, '$.promotion_receipt_digest') =
          NEW.promotion_receipt_digest
      AND json_extract(NEW.receipt_json, '$.source_digest') = NEW.source_digest
      AND json_type(NEW.receipt_json, '$.generations') = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.receipt_json, '$.generations')) = 7
      AND json_extract(NEW.receipt_json, '$.generations.install_generation') =
          NEW.install_generation
      AND json_extract(NEW.receipt_json, '$.generations.activation_generation') =
          NEW.activation_generation
      AND json_extract(NEW.receipt_json, '$.generations.runtime_generation') =
          NEW.runtime_generation
      AND json_extract(
          NEW.receipt_json, '$.generations.work_admission_generation_before'
      ) = NEW.work_admission_generation_before
      AND json_extract(
          NEW.receipt_json, '$.generations.work_admission_generation_after'
      ) = NEW.work_admission_generation_after
      AND json_extract(NEW.receipt_json, '$.generations.previous_work_admission_id') IS
          NEW.previous_work_admission_id
      AND json_extract(
          NEW.receipt_json, '$.generations.previous_work_admission_receipt_digest'
      ) IS NEW.previous_work_admission_receipt_digest
      AND (
          (NEW.work_admission_generation_before = 0
           AND json_type(
               NEW.receipt_json, '$.generations.previous_work_admission_id'
           ) = 'null'
           AND json_type(
               NEW.receipt_json,
               '$.generations.previous_work_admission_receipt_digest'
           ) = 'null')
          OR
          (NEW.work_admission_generation_before > 0
           AND json_type(
               NEW.receipt_json, '$.generations.previous_work_admission_id'
           ) = 'text'
           AND json_type(
               NEW.receipt_json,
               '$.generations.previous_work_admission_receipt_digest'
           ) = 'text')
      )
      AND json_type(NEW.receipt_json, '$.quiescence') = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.receipt_json, '$.quiescence')) = 10
      AND json_extract(NEW.receipt_json, '$.quiescence.desired_presence') =
          NEW.desired_presence
      AND json_extract(NEW.receipt_json, '$.quiescence.desired_activation') =
          NEW.desired_activation
      AND json_extract(NEW.receipt_json, '$.quiescence.slot_phase') = NEW.slot_phase
      AND json_extract(NEW.receipt_json, '$.quiescence.admission') = NEW.admission
      AND json_extract(NEW.receipt_json, '$.quiescence.runtime_phase') = NEW.runtime_phase
      AND json_type(NEW.receipt_json, '$.quiescence.candidate_slot_present') = 'false'
      AND json_extract(NEW.receipt_json, '$.quiescence.candidate_slot_present') =
          NEW.candidate_slot_present
      AND json_type(NEW.receipt_json, '$.quiescence.runtime_slot_present') = 'false'
      AND json_extract(NEW.receipt_json, '$.quiescence.runtime_slot_present') =
          NEW.runtime_slot_present
      AND json_type(
          NEW.receipt_json, '$.quiescence.runtime_runner_digest_present'
      ) = 'false'
      AND json_extract(
          NEW.receipt_json, '$.quiescence.runtime_runner_digest_present'
      ) = NEW.runtime_runner_digest_present
      AND json_type(NEW.receipt_json, '$.quiescence.health_present') = 'false'
      AND json_extract(NEW.receipt_json, '$.quiescence.health_present') =
          NEW.health_present
      AND json_extract(NEW.receipt_json, '$.quiescence.active_attempts') =
          NEW.active_attempts
      AND json_type(NEW.receipt_json, '$.authority') = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.receipt_json, '$.authority')) = 11
      AND json_extract(
          NEW.receipt_json, '$.authority.authority_state_revision_before'
      ) = NEW.authority_state_revision_before
      AND json_extract(
          NEW.receipt_json, '$.authority.authority_state_revision_after'
      ) = NEW.authority_state_revision_after
      AND json_extract(NEW.receipt_json, '$.authority.inventory_revision_before') =
          NEW.inventory_revision_before
      AND json_extract(NEW.receipt_json, '$.authority.inventory_revision_after') =
          NEW.inventory_revision_after
      AND json_extract(NEW.receipt_json, '$.authority.inventory_digest_before') =
          NEW.inventory_digest_before
      AND json_extract(NEW.receipt_json, '$.authority.inventory_digest_after') =
          NEW.inventory_digest_after
      AND json_extract(NEW.receipt_json, '$.authority.authority_epoch_before') =
          NEW.authority_epoch_before
      AND json_extract(NEW.receipt_json, '$.authority.authority_epoch_after') =
          NEW.authority_epoch_after
      AND json_extract(NEW.receipt_json, '$.authority.process_owner_epoch') =
          NEW.process_owner_epoch
      AND json_extract(
          NEW.receipt_json, '$.authority.trusted_time_high_water_ms_before'
      ) = NEW.trusted_time_before_ms
      AND json_extract(
          NEW.receipt_json, '$.authority.authority_updated_at_ms_before'
      ) = NEW.authority_updated_at_ms_before
      AND json_extract(NEW.receipt_json, '$.admitted_at_ms') = NEW.admitted_at_ms
)
BEGIN
    SELECT RAISE(ABORT, 'work admission receipt JSON projection changed');
END;
"#;
