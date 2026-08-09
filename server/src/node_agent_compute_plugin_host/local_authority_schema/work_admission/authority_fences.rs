/// Exact current-source, quiescence and authority CAS fences for V8 work admission.
///
/// Only a freshly applied signed `reauthorize_existing` Plan may authorize this receipt. Candidate
/// health remains historical installation evidence; current runtime health must still be absent.
pub(super) const WORK_ADMISSION_AUTHORITY_FENCES_SCHEMA_V8: &str = r#"
CREATE TRIGGER compute_plugin_work_admission_insert_fenced
BEFORE INSERT ON compute_plugin_work_admission_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN compute_plugin_work_admission_heads AS head
      ON head.plugin_id = NEW.plugin_id
     AND head.installation_id_digest = NEW.installation_id_digest
     AND head.work_admission_generation = NEW.work_admission_generation_after
     AND head.work_admission_id = NEW.work_admission_id
     AND head.receipt_digest = NEW.receipt_digest
     AND head.previous_work_admission_id IS NEW.previous_work_admission_id
     AND head.previous_work_admission_receipt_digest IS
         NEW.previous_work_admission_receipt_digest
     AND head.updated_at_ms = NEW.admitted_at_ms
    JOIN candidate_install_receipts AS installation
      ON installation.install_id = NEW.install_receipt_id
     AND installation.receipt_digest = NEW.install_receipt_digest
    JOIN candidate_promotion_receipts AS promotion
      ON promotion.promotion_id = NEW.promotion_receipt_id
     AND promotion.receipt_digest = NEW.promotion_receipt_digest
     AND promotion.install_id = installation.install_id
     AND promotion.install_receipt_digest = installation.receipt_digest
     AND promotion.candidate_token = installation.candidate_token
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = promotion.candidate_token
    JOIN plan_applications AS application
      ON application.plan_id = NEW.plan_id
     AND application.plan_digest = NEW.plan_digest
    JOIN plan_application_seals AS application_seal
      ON application_seal.plan_id = application.plan_id
     AND application_seal.plan_digest = application.plan_digest
    JOIN sharing_policy_binding_receipts AS policy
      ON policy.policy_revision = NEW.policy_revision
    JOIN sharing_policy_binding_revocation_receipts AS revocation
      ON revocation.policy_revision = policy.policy_revision
    JOIN manifest_catalog_binding_receipts AS catalog
      ON catalog.catalog_revision = NEW.manifest_catalog_revision
    JOIN keyring_bundles AS bundle
      ON bundle.bundle_revision = NEW.keyring_bundle_revision
    JOIN keyring_seals AS bundle_seal
      ON bundle_seal.bundle_revision = bundle.bundle_revision
    JOIN json_each(meta.inventory_json, '$.plugins') AS record
    JOIN json_each(application.signed_plan_json, '$.plan.items') AS item
    JOIN json_each(application.signed_manifests_json) AS signed_manifest
    JOIN json_each(catalog.signed_manifests_json) AS catalog_manifest
      ON catalog_manifest.value = signed_manifest.value
    JOIN json_each(signed_manifest.value, '$.manifest.package.files') AS runner
    JOIN json_each(catalog.catalog_json, '$.entries') AS catalog_entry
    WHERE meta.singleton = 1
      AND meta.schema_version = 3
      AND meta.installation_id_digest = NEW.installation_id_digest
      AND meta.state_revision = NEW.authority_state_revision_before
      AND meta.inventory_revision = NEW.inventory_revision_before
      AND meta.inventory_digest = NEW.inventory_digest_before
      AND meta.authority_epoch = NEW.authority_epoch_before
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND meta.updated_at_ms = NEW.authority_updated_at_ms_before
      AND meta.clock_status = 'trusted'
      AND meta.sharing_enabled = 1
      AND meta.desired_policy_revision = NEW.policy_revision
      AND meta.sharing_authorization_ref = NEW.sharing_authorization_ref
      AND meta.sharing_authorization_revision = NEW.sharing_authorization_revision
      AND meta.sharing_authorization_digest = NEW.sharing_authorization_digest
      AND meta.node_profile_digest = NEW.node_profile_digest
      AND meta.manifest_catalog_revision = NEW.manifest_catalog_revision
      AND meta.target_id = NEW.target_id
      AND meta.host_api_protocol_id = NEW.host_api_protocol_id
      AND meta.host_api_revision = NEW.host_api_revision
      AND meta.active_bundle_revision = NEW.keyring_bundle_revision
      AND meta.publisher_keyring_revision = NEW.publisher_keyring_revision
      AND meta.publisher_keyring_digest = NEW.publisher_keyring_digest
      AND meta.control_keyring_revision = NEW.control_keyring_revision
      AND meta.control_keyring_digest = NEW.control_keyring_digest
      AND json_extract(NEW.release_json, '$.plugin_id') = NEW.plugin_id
      AND json_extract(NEW.release_json, '$.plugin_version') = NEW.plugin_version
      AND json_extract(NEW.release_json, '$.target_id') = NEW.target_id
      AND json_extract(NEW.release_json, '$.manifest_digest') = NEW.manifest_digest
      AND json_extract(NEW.target_json, '$.target_id') = NEW.target_id
      AND application.application_request_digest = NEW.application_request_digest
      AND application.receipt_digest = NEW.application_receipt_digest
      AND application.signed_plan_envelope_digest = NEW.signed_plan_envelope_digest
      AND application.signed_manifest_set_digest = NEW.signed_manifest_set_digest
      AND application.admission_bindings_digest = NEW.admission_bindings_digest
      AND application.application_inventory_revision =
          NEW.application_inventory_revision
      AND application.application_inventory_revision = NEW.inventory_revision_before
      AND application.inventory_after_digest = NEW.inventory_digest_before
      AND application.inventory_after_json = meta.inventory_json
      AND application.application_state_revision = NEW.authority_state_revision_before
      AND application.authority_epoch_at_apply = NEW.authority_epoch_before
      AND application.keyring_bundle_revision = NEW.keyring_bundle_revision
      AND application.publisher_keyring_revision = NEW.publisher_keyring_revision
      AND application.publisher_keyring_digest = NEW.publisher_keyring_digest
      AND application.control_keyring_revision = NEW.control_keyring_revision
      AND application.control_keyring_digest = NEW.control_keyring_digest
      AND application.applied_at_ms = NEW.trusted_time_before_ms
      AND application.applied_at_ms = NEW.authority_updated_at_ms_before
      AND application.applied_at_ms < NEW.admitted_at_ms
      AND NEW.admitted_at_ms < application.expires_at_ms
      AND application_seal.application_request_digest = NEW.application_request_digest
      AND application_seal.receipt_digest = NEW.application_receipt_digest
      AND application_seal.sealed_at_ms = application.applied_at_ms
      AND json_extract(application.signed_plan_json, '$.plan.plan_id') = NEW.plan_id
      AND json_extract(application.signed_plan_json, '$.plan_digest') = NEW.plan_digest
      AND json_extract(application.signed_plan_json, '$.plan.desired_policy_revision') =
          NEW.policy_revision
      AND json_type(application.signed_plan_json, '$.plan.sharing_enabled') = 'true'
      AND json_extract(
          application.signed_plan_json, '$.plan.sharing_authorization.authorization_ref'
      ) = NEW.sharing_authorization_ref
      AND json_extract(
          application.signed_plan_json, '$.plan.sharing_authorization.revision'
      ) = NEW.sharing_authorization_revision
      AND json_extract(
          application.signed_plan_json, '$.plan.sharing_authorization.digest'
      ) = NEW.sharing_authorization_digest
      AND json_extract(application.signed_plan_json, '$.plan.node_profile_digest') =
          NEW.node_profile_digest
      AND json_extract(application.signed_plan_json, '$.plan.manifest_catalog_revision') =
          NEW.manifest_catalog_revision
      AND json_extract(
          application.signed_plan_json, '$.plan.publisher_keyring.revision'
      ) = NEW.publisher_keyring_revision
      AND json_extract(
          application.signed_plan_json, '$.plan.publisher_keyring.digest'
      ) = NEW.publisher_keyring_digest
      AND json_extract(
          application.signed_plan_json, '$.plan.control_keyring.revision'
      ) = NEW.control_keyring_revision
      AND json_extract(
          application.signed_plan_json, '$.plan.control_keyring.digest'
      ) = NEW.control_keyring_digest
      AND json_extract(item.value, '$.action') = 'reauthorize_existing'
      AND json_extract(item.value, '$.expected_current_release') = NEW.release_json
      AND json_type(item.value, '$.expected_candidate_release') = 'null'
      AND json_extract(item.value, '$.expected_install_generation') =
          NEW.install_generation
      AND json_type(item.value, '$.target_release') = 'null'
      AND json_array_length(json_extract(item.value, '$.downloads')) = 0
      AND json_extract(item.value, '$.target_activation') = 'enabled'
      AND json_type(item.value, '$.grant') = 'object'
      AND json_extract(item.value, '$.grant.grant_ref') = NEW.grant_ref
      AND json_extract(item.value, '$.grant.grant_digest') =
          NEW.permission_grant_digest
      AND json_extract(item.value, '$.grant.granted_permissions') =
          NEW.granted_permissions_json
      AND json_extract(item.value, '$.grant.granted_resources.max_cpu_millicores') =
          NEW.authorized_max_cpu_millicores
      AND json_extract(item.value, '$.grant.granted_resources.max_memory_bytes') =
          NEW.authorized_max_memory_bytes
      AND json_extract(item.value, '$.grant.granted_resources.max_vram_bytes') =
          NEW.authorized_max_vram_bytes
      AND json_extract(item.value, '$.grant.granted_resources.max_disk_bytes') =
          NEW.authorized_max_disk_bytes
      AND json_extract(item.value, '$.grant.granted_resources.max_processes') =
          NEW.authorized_max_processes
      AND json_extract(
          item.value, '$.grant.granted_resources.max_sidecar_uptime_seconds'
      ) = NEW.authorized_max_sidecar_uptime_seconds
      AND policy.installation_id_digest = NEW.installation_id_digest
      AND policy.receipt_digest = NEW.policy_binding_receipt_digest
      AND policy.sharing_enabled = 1
      AND policy.sharing_authorization_ref = NEW.sharing_authorization_ref
      AND policy.sharing_authorization_revision = NEW.sharing_authorization_revision
      AND policy.sharing_authorization_digest = NEW.sharing_authorization_digest
      AND policy.policy_digest = NEW.sharing_authorization_digest
      AND policy.bound_at_ms <= application.applied_at_ms
      AND revocation.installation_id_digest = NEW.installation_id_digest
      AND revocation.policy_binding_receipt_digest = policy.receipt_digest
      AND revocation.receipt_digest = NEW.policy_revocation_receipt_digest
      AND revocation.bound_at_ms = policy.bound_at_ms
      AND catalog.installation_id_digest = NEW.installation_id_digest
      AND catalog.catalog_digest = NEW.manifest_catalog_digest
      AND catalog.receipt_digest = NEW.manifest_catalog_binding_receipt_digest
      AND catalog.node_profile_digest = NEW.node_profile_digest
      AND catalog.target_id = NEW.target_id
      AND catalog.host_api_protocol_id = NEW.host_api_protocol_id
      AND catalog.host_api_revision = NEW.host_api_revision
      AND catalog.keyring_bundle_revision = NEW.keyring_bundle_revision
      AND catalog.publisher_keyring_revision = NEW.publisher_keyring_revision
      AND catalog.publisher_keyring_digest = NEW.publisher_keyring_digest
      AND catalog.control_keyring_revision = NEW.control_keyring_revision
      AND catalog.control_keyring_digest = NEW.control_keyring_digest
      AND catalog.bound_at_ms <= application.applied_at_ms
      AND bundle.publisher_revision = NEW.publisher_keyring_revision
      AND bundle.publisher_digest = NEW.publisher_keyring_digest
      AND bundle.control_revision = NEW.control_keyring_revision
      AND bundle.control_digest = NEW.control_keyring_digest
      AND NEW.admitted_at_ms < bundle.expires_at_ms
      AND installation.promotion_id = promotion.promotion_id
      AND installation.install_state = 'installed'
      AND installation.installation_id_digest = NEW.installation_id_digest
      AND installation.plugin_id = NEW.plugin_id
      AND installation.slot_ref = NEW.slot_ref
      AND installation.release_json = NEW.release_json
      AND installation.install_generation_after = NEW.install_generation
      AND installation.signed_manifest_envelope_digest =
          NEW.signed_manifest_envelope_digest
      AND promotion.installation_id_digest = NEW.installation_id_digest
      AND promotion.promotion_state = 'active'
      AND promotion.plugin_id = NEW.plugin_id
      AND promotion.slot_ref = NEW.slot_ref
      AND promotion.release_json = NEW.release_json
      AND promotion.install_generation_after = NEW.install_generation
      AND promotion.activation_generation_after = NEW.activation_generation
      AND promotion.signed_manifest_envelope_digest =
          NEW.signed_manifest_envelope_digest
      AND promotion.promoted_at_ms < application.applied_at_ms
      AND candidate.state = 'promoted'
      AND candidate.plugin_id = NEW.plugin_id
      AND candidate.slot_ref = NEW.slot_ref
      AND candidate.release_json = NEW.release_json
      AND json_extract(record.value, '$.plugin_id') = NEW.plugin_id
      AND json_extract(record.value, '$.last_plan_id') = NEW.plan_id
      AND json_extract(record.value, '$.install_generation') = NEW.install_generation
      AND json_extract(record.value, '$.activation_generation') = NEW.activation_generation
      AND json_extract(record.value, '$.active_slot_ref') = NEW.slot_ref
      AND json_type(record.value, '$.candidate_slot_ref') = 'null'
      AND json_extract(record.value, '$.desired_presence') = NEW.desired_presence
      AND json_extract(record.value, '$.desired_activation') = NEW.desired_activation
      AND json_extract(record.value, '$.admission') = NEW.admission
      AND json_extract(record.value, '$.permission_grant_digest') =
          NEW.permission_grant_digest
      AND json_extract(record.value, '$.active_attempts') = NEW.active_attempts
      AND json_type(record.value, '$.health') = 'null'
      AND json_extract(record.value, '$.runtime.phase') = NEW.runtime_phase
      AND json_extract(record.value, '$.runtime.runtime_generation') =
          NEW.runtime_generation
      AND json_type(record.value, '$.runtime.slot_ref') = 'null'
      AND json_type(record.value, '$.runtime.runner_digest') = 'null'
      AND EXISTS (
          SELECT 1 FROM json_each(record.value, '$.slots') AS slot
          WHERE json_extract(slot.value, '$.slot_ref') = NEW.slot_ref
            AND json_extract(slot.value, '$.phase') = NEW.slot_phase
            AND json_extract(slot.value, '$.release') = NEW.release_json
      )
      AND json_extract(signed_manifest.value, '$.manifest.plugin_id') = NEW.plugin_id
      AND json_extract(signed_manifest.value, '$.manifest.plugin_version') =
          NEW.plugin_version
      AND json_extract(signed_manifest.value, '$.manifest.publisher_id') = NEW.publisher_id
      AND json_extract(signed_manifest.value, '$.manifest_digest') = NEW.manifest_digest
      AND json_extract(signed_manifest.value, '$.manifest.target') = NEW.target_json
      AND json_extract(signed_manifest.value, '$.manifest.target.target_id') = NEW.target_id
      AND json_extract(signed_manifest.value, '$.manifest.task_kinds') = NEW.task_kinds_json
      AND json_extract(signed_manifest.value, '$.manifest.host_api.protocol_id') =
          NEW.host_api_protocol_id
      AND NEW.host_api_revision >= json_extract(
          signed_manifest.value, '$.manifest.host_api.minimum_revision'
      )
      AND NEW.host_api_revision <= json_extract(
          signed_manifest.value, '$.manifest.host_api.maximum_revision'
      )
      AND json_extract(signed_manifest.value, '$.manifest.entrypoint.entrypoint_kind') =
          NEW.entrypoint_kind
      AND json_extract(signed_manifest.value, '$.manifest.entrypoint.relative_path') =
          NEW.entrypoint_relative_path
      AND json_extract(signed_manifest.value, '$.manifest.entrypoint.arguments') =
          NEW.entrypoint_arguments_json
      AND json_extract(signed_manifest.value, '$.manifest.entrypoint.health_check') =
          NEW.health_check_json
      AND json_extract(signed_manifest.value, '$.manifest.package.package_digest') =
          json_extract(NEW.release_json, '$.package_digest')
      AND json_extract(runner.value, '$.relative_path') = NEW.runner_relative_path
      AND json_extract(runner.value, '$.digest') = NEW.runner_file_digest
      AND json_extract(runner.value, '$.size_bytes') = NEW.runner_file_size_bytes
      AND json_type(runner.value, '$.executable') = 'true'
      AND json_extract(runner.value, '$.executable') = NEW.runner_file_executable
      AND NEW.authorized_max_cpu_millicores <= json_extract(
          signed_manifest.value, '$.manifest.requested_resources.max_cpu_millicores'
      )
      AND NEW.authorized_max_memory_bytes <= json_extract(
          signed_manifest.value, '$.manifest.requested_resources.max_memory_bytes'
      )
      AND NEW.authorized_max_vram_bytes <= json_extract(
          signed_manifest.value, '$.manifest.requested_resources.max_vram_bytes'
      )
      AND NEW.authorized_max_disk_bytes <= json_extract(
          signed_manifest.value, '$.manifest.requested_resources.max_disk_bytes'
      )
      AND NEW.authorized_max_processes <= json_extract(
          signed_manifest.value, '$.manifest.requested_resources.max_processes'
      )
      AND NEW.authorized_max_sidecar_uptime_seconds <= json_extract(
          signed_manifest.value, '$.manifest.requested_resources.max_sidecar_uptime_seconds'
      )
      AND json_extract(catalog_entry.value, '$.release') = NEW.release_json
      AND json_extract(catalog_entry.value, '$.publisher_id') = NEW.publisher_id
      AND json_extract(catalog_entry.value, '$.signed_manifest_envelope_digest') =
          NEW.signed_manifest_envelope_digest
      AND (SELECT COUNT(*) FROM json_each(meta.inventory_json, '$.plugins') AS peer
           WHERE json_extract(peer.value, '$.plugin_id') = NEW.plugin_id) = 1
      AND (SELECT COUNT(*)
           FROM json_each(application.signed_plan_json, '$.plan.items') AS peer
           WHERE json_extract(peer.value, '$.action') = 'reauthorize_existing'
             AND json_extract(peer.value, '$.expected_current_release') =
                 NEW.release_json) = 1
      AND (SELECT COUNT(*) FROM json_each(application.signed_manifests_json) AS peer
           WHERE json_extract(peer.value, '$.manifest.plugin_id') = NEW.plugin_id
             AND json_extract(peer.value, '$.manifest_digest') = NEW.manifest_digest) = 1
      AND (SELECT COUNT(*) FROM json_each(catalog.signed_manifests_json) AS peer
           WHERE peer.value = signed_manifest.value) = 1
      AND (SELECT COUNT(*)
           FROM json_each(signed_manifest.value, '$.manifest.package.files') AS peer
           WHERE json_extract(peer.value, '$.relative_path') =
                 NEW.runner_relative_path) = 1
      AND (SELECT COUNT(*) FROM json_each(catalog.catalog_json, '$.entries') AS peer
           WHERE json_extract(peer.value, '$.release') = NEW.release_json) = 1
      AND (
          (NEW.work_admission_generation_before = 0
           AND NEW.previous_work_admission_id IS NULL
           AND NEW.previous_work_admission_receipt_digest IS NULL)
          OR EXISTS (
              SELECT 1 FROM compute_plugin_work_admission_receipts AS previous
              WHERE previous.work_admission_id = NEW.previous_work_admission_id
                AND previous.receipt_digest =
                    NEW.previous_work_admission_receipt_digest
                AND previous.installation_id_digest = NEW.installation_id_digest
                AND previous.plugin_id = NEW.plugin_id
                AND previous.work_admission_generation_after =
                    NEW.work_admission_generation_before
          )
      )
      AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared'
      )
)
BEGIN
    SELECT RAISE(ABORT, 'work admission lost its exact current authority source');
END;

CREATE TRIGGER compute_plugin_work_admission_apply_authority
AFTER INSERT ON compute_plugin_work_admission_receipts
BEGIN
    UPDATE authority_meta SET
        state_revision = NEW.authority_state_revision_after,
        authority_epoch = NEW.authority_epoch_after,
        trusted_time_high_water_ms = NEW.admitted_at_ms,
        clock_status = 'trusted',
        updated_at_ms = NEW.admitted_at_ms
    WHERE singleton = 1 AND schema_version = 3
      AND installation_id_digest = NEW.installation_id_digest
      AND state_revision = NEW.authority_state_revision_before
      AND inventory_revision = NEW.inventory_revision_before
      AND inventory_digest = NEW.inventory_digest_before
      AND authority_epoch = NEW.authority_epoch_before
      AND process_owner_epoch = NEW.process_owner_epoch
      AND trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND updated_at_ms = NEW.authority_updated_at_ms_before
      AND clock_status = 'trusted'
      AND desired_policy_revision = NEW.policy_revision
      AND sharing_enabled = 1
      AND sharing_authorization_ref = NEW.sharing_authorization_ref
      AND sharing_authorization_revision = NEW.sharing_authorization_revision
      AND sharing_authorization_digest = NEW.sharing_authorization_digest
      AND node_profile_digest = NEW.node_profile_digest
      AND manifest_catalog_revision = NEW.manifest_catalog_revision
      AND target_id = NEW.target_id
      AND host_api_protocol_id = NEW.host_api_protocol_id
      AND host_api_revision = NEW.host_api_revision
      AND active_bundle_revision = NEW.keyring_bundle_revision
      AND publisher_keyring_revision = NEW.publisher_keyring_revision
      AND publisher_keyring_digest = NEW.publisher_keyring_digest
      AND control_keyring_revision = NEW.control_keyring_revision
      AND control_keyring_digest = NEW.control_keyring_digest
      AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared'
      );
    SELECT RAISE(ABORT, 'work admission authority CAS did not update exactly once')
    WHERE changes() <> 1;
END;
"#;
