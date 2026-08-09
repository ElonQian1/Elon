/// Exact JSON-to-column projections for the two canonical receipt bodies.
///
/// SQLite does not recompute RFC8785/JCS SHA-256. It instead locks every security fact to the
/// canonical body, binds the signed envelope to the sealed plan and active catalog, and leaves the
/// Store's typed construction plus read-back digest verification as the cryptographic second gate.
pub(super) const CANDIDATE_PROMOTION_RECEIPT_PROJECTION_SCHEMA_V7: &str = r#"
CREATE TRIGGER candidate_install_receipt_projection_fenced
BEFORE INSERT ON candidate_install_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM candidate_staging_receipts AS staging
    JOIN plan_applications AS application
      ON application.plan_id = NEW.owner_plan_id
     AND application.plan_digest = NEW.owner_plan_digest
     AND application.application_inventory_revision = NEW.application_inventory_revision
    JOIN plan_application_seals AS seal
      ON seal.plan_id = application.plan_id
     AND seal.plan_digest = application.plan_digest
    JOIN authority_meta AS meta ON meta.singleton = 1
    JOIN manifest_catalog_binding_receipts AS catalog
      ON catalog.catalog_revision = meta.manifest_catalog_revision
    WHERE staging.staging_id = NEW.staging_id
      AND staging.candidate_token = NEW.candidate_token
      AND staging.receipt_digest = NEW.staging_receipt_digest
      AND json_type(NEW.receipt_json) = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.receipt_json)) = 41
      AND json_extract(NEW.receipt_json, '$.schema') =
          'elon.compute_plugin.install_receipt.v1'
      AND json_extract(NEW.receipt_json, '$.install_receipt_id') = NEW.install_id
      AND json_extract(NEW.receipt_json, '$.promotion_id') = NEW.promotion_id
      AND json_extract(NEW.receipt_json, '$.installation_id_digest') =
          NEW.installation_id_digest
      AND json_extract(NEW.receipt_json, '$.candidate_token_digest') =
          NEW.candidate_token_digest
      AND json_extract(NEW.receipt_json, '$.candidate_generation') =
          NEW.candidate_generation
      AND json_extract(NEW.receipt_json, '$.plugin_id') = NEW.plugin_id
      AND json_extract(NEW.receipt_json, '$.slot_ref') = NEW.slot_ref
      AND json_type(NEW.receipt_json, '$.release') = 'object'
      AND json_extract(NEW.receipt_json, '$.release') = NEW.release_json
      AND json_extract(NEW.receipt_json, '$.staging_id') = NEW.staging_id
      AND json_extract(NEW.receipt_json, '$.staging_receipt_digest') =
          NEW.staging_receipt_digest
      AND json_extract(NEW.receipt_json, '$.staging_run_digest') = NEW.staging_run_digest
      AND json_extract(NEW.receipt_json, '$.extraction_plan_digest') =
          staging.extraction_plan_digest
      AND json_extract(NEW.receipt_json, '$.extraction_evidence_digest') =
          staging.extraction_evidence_digest
      AND json_extract(NEW.receipt_json, '$.staging_seal_payload_digest') =
          staging.staging_seal_payload_digest
      AND json_extract(NEW.receipt_json, '$.staging_seal_file_digest') =
          staging.staging_seal_file_digest
      AND json_extract(NEW.receipt_json, '$.staging_seal_identity_digest') =
          staging.staging_seal_identity_digest
      AND json_extract(NEW.receipt_json, '$.health_id') = NEW.health_id
      AND json_extract(NEW.receipt_json, '$.health_receipt_digest') =
          NEW.health_receipt_digest
      AND json_extract(NEW.receipt_json, '$.health_observation_digest') =
          NEW.health_observation_digest
      AND json_extract(NEW.receipt_json, '$.owner_plan_id') = NEW.owner_plan_id
      AND json_extract(NEW.receipt_json, '$.owner_plan_digest') = NEW.owner_plan_digest
      AND json_extract(NEW.receipt_json, '$.application_inventory_revision') =
          NEW.application_inventory_revision
      AND json_extract(NEW.receipt_json, '$.permission_grant_digest') =
          NEW.permission_grant_digest
      AND json_extract(NEW.receipt_json, '$.signed_manifest_envelope_digest') =
          NEW.signed_manifest_envelope_digest
      AND json_extract(NEW.receipt_json, '$.authority_state_revision_before') =
          NEW.authority_state_revision_before
      AND json_extract(NEW.receipt_json, '$.authority_state_revision_after') =
          NEW.authority_state_revision_after
      AND json_extract(NEW.receipt_json, '$.inventory_revision_before') =
          NEW.inventory_revision_before
      AND json_extract(NEW.receipt_json, '$.inventory_revision_after') =
          NEW.inventory_revision_after
      AND json_extract(NEW.receipt_json, '$.inventory_digest_before') =
          NEW.inventory_digest_before
      AND json_extract(NEW.receipt_json, '$.inventory_digest_after') =
          NEW.inventory_digest_after
      AND json_extract(NEW.receipt_json, '$.authority_epoch_before') =
          NEW.authority_epoch_before
      AND json_extract(NEW.receipt_json, '$.authority_epoch_after') =
          NEW.authority_epoch_after
      AND json_extract(NEW.receipt_json, '$.process_owner_epoch') = NEW.process_owner_epoch
      AND json_extract(NEW.receipt_json, '$.trusted_time_high_water_ms_before') =
          NEW.trusted_time_before_ms
      AND json_extract(NEW.receipt_json, '$.authority_updated_at_ms_before') =
          NEW.authority_updated_at_ms_before
      AND json_extract(NEW.receipt_json, '$.installed_at_ms') = NEW.installed_at_ms
      AND json_extract(NEW.receipt_json, '$.install_generation_before') =
          NEW.install_generation_before
      AND json_extract(NEW.receipt_json, '$.install_generation_after') =
          NEW.install_generation_after
      AND json_extract(NEW.receipt_json, '$.slot_phase_before') = 'staged'
      AND json_extract(NEW.receipt_json, '$.slot_phase_after') = 'installed'
      AND NEW.install_evidence_json = NEW.receipt_json
      AND NEW.install_evidence_digest = NEW.receipt_digest
      AND application.expires_at_ms > NEW.installed_at_ms
      AND (SELECT COUNT(*)
           FROM json_each(application.signed_manifests_json) AS signed
           JOIN json_each(catalog.signed_manifests_json) AS catalog_signed
             ON catalog_signed.value = signed.value
           WHERE json_extract(signed.value, '$.manifest.plugin_id') = NEW.plugin_id
             AND json_extract(signed.value, '$.manifest.plugin_version') =
                 json_extract(NEW.release_json, '$.plugin_version')
             AND json_extract(signed.value, '$.manifest.target.target_id') =
                 json_extract(NEW.release_json, '$.target_id')
             AND json_extract(signed.value, '$.manifest_digest') =
                 json_extract(NEW.release_json, '$.manifest_digest')
             AND json_extract(signed.value, '$.manifest.package.package_digest') =
                 json_extract(NEW.release_json, '$.package_digest')) = 1
      AND (SELECT COUNT(*)
           FROM json_each(catalog.catalog_json, '$.entries') AS entry
           WHERE json_extract(entry.value, '$.release') = NEW.release_json
             AND json_extract(entry.value, '$.signed_manifest_envelope_digest') =
                 NEW.signed_manifest_envelope_digest) = 1
)
BEGIN
    SELECT RAISE(ABORT, 'candidate install receipt JSON or signed manifest projection changed');
END;

CREATE TRIGGER candidate_promotion_receipt_projection_fenced
BEFORE INSERT ON candidate_promotion_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM candidate_install_receipts AS installation
    WHERE installation.install_id = NEW.install_id
      AND installation.receipt_digest = NEW.install_receipt_digest
      AND json_type(NEW.receipt_json) = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.receipt_json)) = 38
      AND json_extract(NEW.receipt_json, '$.schema') =
          'elon.compute_plugin.promotion_receipt.v1'
      AND json_extract(NEW.receipt_json, '$.promotion_receipt_id') = NEW.promotion_id
      AND json_extract(NEW.receipt_json, '$.install_receipt_id') = NEW.install_id
      AND json_extract(NEW.receipt_json, '$.install_receipt_digest') =
          NEW.install_receipt_digest
      AND json_extract(NEW.receipt_json, '$.installation_id_digest') =
          NEW.installation_id_digest
      AND json_extract(NEW.receipt_json, '$.candidate_token_digest') =
          NEW.candidate_token_digest
      AND json_extract(NEW.receipt_json, '$.plugin_id') = NEW.plugin_id
      AND json_extract(NEW.receipt_json, '$.slot_ref') = NEW.slot_ref
      AND json_type(NEW.receipt_json, '$.release') = 'object'
      AND json_extract(NEW.receipt_json, '$.release') = NEW.release_json
      AND json_extract(NEW.receipt_json, '$.health_id') = NEW.health_id
      AND json_extract(NEW.receipt_json, '$.health_receipt_digest') =
          NEW.health_receipt_digest
      AND json_extract(NEW.receipt_json, '$.owner_plan_id') = NEW.owner_plan_id
      AND json_extract(NEW.receipt_json, '$.owner_plan_digest') = NEW.owner_plan_digest
      AND json_extract(NEW.receipt_json, '$.application_inventory_revision') =
          NEW.application_inventory_revision
      AND json_extract(NEW.receipt_json, '$.permission_grant_digest') =
          NEW.permission_grant_digest
      AND json_extract(NEW.receipt_json, '$.signed_manifest_envelope_digest') =
          NEW.signed_manifest_envelope_digest
      AND json_extract(NEW.receipt_json, '$.authority_state_revision_before') =
          NEW.authority_state_revision_before
      AND json_extract(NEW.receipt_json, '$.authority_state_revision_after') =
          NEW.authority_state_revision_after
      AND json_extract(NEW.receipt_json, '$.inventory_revision_before') =
          NEW.inventory_revision_before
      AND json_extract(NEW.receipt_json, '$.inventory_revision_after') =
          NEW.inventory_revision_after
      AND json_extract(NEW.receipt_json, '$.inventory_digest_before') =
          NEW.inventory_digest_before
      AND json_extract(NEW.receipt_json, '$.inventory_digest_after') =
          NEW.inventory_digest_after
      AND json_extract(NEW.receipt_json, '$.authority_epoch_before') =
          NEW.authority_epoch_before
      AND json_extract(NEW.receipt_json, '$.authority_epoch_after') =
          NEW.authority_epoch_after
      AND json_extract(NEW.receipt_json, '$.process_owner_epoch') = NEW.process_owner_epoch
      AND json_extract(NEW.receipt_json, '$.trusted_time_high_water_ms_before') =
          NEW.trusted_time_before_ms
      AND json_extract(NEW.receipt_json, '$.authority_updated_at_ms_before') =
          NEW.authority_updated_at_ms_before
      AND json_extract(NEW.receipt_json, '$.promoted_at_ms') = NEW.promoted_at_ms
      AND json_extract(NEW.receipt_json, '$.install_generation_after') =
          NEW.install_generation_after
      AND json_extract(NEW.receipt_json, '$.activation_generation_before') =
          NEW.activation_generation_before
      AND json_extract(NEW.receipt_json, '$.activation_generation_after') =
          NEW.activation_generation_after
      AND json_extract(NEW.receipt_json, '$.previous_active_slot_ref') IS
          NEW.previous_active_slot_ref
      AND json_extract(NEW.receipt_json, '$.previous_active_release') IS
          NEW.previous_active_release_json
      AND json_extract(NEW.receipt_json, '$.previous_active_install_receipt_digest') IS
          NEW.previous_active_install_receipt_digest
      AND json_extract(NEW.receipt_json, '$.previous_active_promotion_receipt_digest') IS
          NEW.previous_active_promotion_receipt_digest
      AND (
          (NEW.previous_active_slot_ref IS NULL
           AND json_type(NEW.receipt_json, '$.previous_active_slot_ref') = 'null'
           AND json_type(NEW.receipt_json, '$.previous_active_release') = 'null'
           AND json_type(
               NEW.receipt_json, '$.previous_active_install_receipt_digest'
           ) = 'null'
           AND json_type(
               NEW.receipt_json, '$.previous_active_promotion_receipt_digest'
           ) = 'null')
          OR
          (NEW.previous_active_slot_ref IS NOT NULL
           AND json_type(NEW.receipt_json, '$.previous_active_slot_ref') = 'text'
           AND json_type(NEW.receipt_json, '$.previous_active_release') = 'object'
           AND json_type(
               NEW.receipt_json, '$.previous_active_install_receipt_digest'
           ) = 'text'
           AND json_type(
               NEW.receipt_json, '$.previous_active_promotion_receipt_digest'
           ) = 'text')
      )
      AND json_extract(NEW.receipt_json, '$.active_slot_ref_after') = NEW.slot_ref
      AND json_type(NEW.receipt_json, '$.active_release_after') = 'object'
      AND json_extract(NEW.receipt_json, '$.active_release_after') = NEW.release_json
      AND json_extract(NEW.receipt_json, '$.slot_phase_after') = 'installed'
      AND NEW.active_provenance_json = NEW.receipt_json
      AND NEW.active_provenance_digest = NEW.receipt_digest
)
BEGIN
    SELECT RAISE(ABORT, 'candidate promotion receipt JSON or active provenance changed');
END;
"#;
