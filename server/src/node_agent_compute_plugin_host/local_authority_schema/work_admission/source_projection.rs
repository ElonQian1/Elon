/// Exact projection of the canonical Source, PlanBinding and LaunchProfile bodies.
///
/// SQLite deliberately does not recompute RFC8785/JCS digests. The typed Store must verify the
/// digest before insertion and after readback; these guards prevent any JSON/column split-brain.
pub(super) const WORK_ADMISSION_SOURCE_PROJECTION_SCHEMA_V8: &str = r#"
CREATE TRIGGER compute_plugin_work_admission_source_projection_fenced
BEFORE INSERT ON compute_plugin_work_admission_receipts
WHEN NOT EXISTS (
    SELECT 1
    WHERE json_type(NEW.source_json) = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.source_json)) = 11
      AND json_extract(NEW.source_json, '$.schema') =
          'elon.compute_plugin.work_admission_source.v1'
      AND json_extract(NEW.source_json, '$.installation_id_digest') =
          NEW.installation_id_digest
      AND json_extract(NEW.source_json, '$.plugin_id') = NEW.plugin_id
      AND json_extract(NEW.source_json, '$.slot_ref') = NEW.slot_ref
      AND json_type(NEW.source_json, '$.release') = 'object'
      AND json_extract(NEW.source_json, '$.release') = NEW.release_json
      AND json_extract(NEW.source_json, '$.install_receipt_id') =
          NEW.install_receipt_id
      AND json_extract(NEW.source_json, '$.install_receipt_digest') =
          NEW.install_receipt_digest
      AND json_extract(NEW.source_json, '$.promotion_receipt_id') =
          NEW.promotion_receipt_id
      AND json_extract(NEW.source_json, '$.promotion_receipt_digest') =
          NEW.promotion_receipt_digest
      AND json_type(NEW.source_json, '$.plan') = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.source_json, '$.plan')) = 25
      AND json_extract(NEW.source_json, '$.plan.schema') =
          'elon.compute_plugin.work_admission_plan_binding.v1'
      AND json_extract(NEW.source_json, '$.plan.action') = NEW.plan_action
      AND json_extract(NEW.source_json, '$.plan.plan_id') = NEW.plan_id
      AND json_extract(NEW.source_json, '$.plan.plan_digest') = NEW.plan_digest
      AND json_extract(NEW.source_json, '$.plan.signed_plan_envelope_digest') =
          NEW.signed_plan_envelope_digest
      AND json_extract(NEW.source_json, '$.plan.signed_manifest_set_digest') =
          NEW.signed_manifest_set_digest
      AND json_extract(NEW.source_json, '$.plan.application_request_digest') =
          NEW.application_request_digest
      AND json_extract(NEW.source_json, '$.plan.application_receipt_digest') =
          NEW.application_receipt_digest
      AND json_extract(NEW.source_json, '$.plan.admission_bindings_digest') =
          NEW.admission_bindings_digest
      AND json_extract(NEW.source_json, '$.plan.application_inventory_revision') =
          NEW.application_inventory_revision
      AND json_extract(NEW.source_json, '$.plan.policy_revision') = NEW.policy_revision
      AND json_extract(NEW.source_json, '$.plan.sharing_authorization_ref') =
          NEW.sharing_authorization_ref
      AND json_extract(NEW.source_json, '$.plan.sharing_authorization_revision') =
          NEW.sharing_authorization_revision
      AND json_extract(NEW.source_json, '$.plan.sharing_authorization_digest') =
          NEW.sharing_authorization_digest
      AND json_extract(NEW.source_json, '$.plan.policy_binding_receipt_digest') =
          NEW.policy_binding_receipt_digest
      AND json_extract(NEW.source_json, '$.plan.policy_revocation_receipt_digest') =
          NEW.policy_revocation_receipt_digest
      AND json_extract(NEW.source_json, '$.plan.node_profile_digest') =
          NEW.node_profile_digest
      AND json_extract(NEW.source_json, '$.plan.manifest_catalog_revision') =
          NEW.manifest_catalog_revision
      AND json_extract(NEW.source_json, '$.plan.manifest_catalog_digest') =
          NEW.manifest_catalog_digest
      AND json_extract(
          NEW.source_json, '$.plan.manifest_catalog_binding_receipt_digest'
      ) = NEW.manifest_catalog_binding_receipt_digest
      AND json_extract(NEW.source_json, '$.plan.keyring_bundle_revision') =
          NEW.keyring_bundle_revision
      AND json_extract(NEW.source_json, '$.plan.publisher_keyring_revision') =
          NEW.publisher_keyring_revision
      AND json_extract(NEW.source_json, '$.plan.publisher_keyring_digest') =
          NEW.publisher_keyring_digest
      AND json_extract(NEW.source_json, '$.plan.control_keyring_revision') =
          NEW.control_keyring_revision
      AND json_extract(NEW.source_json, '$.plan.control_keyring_digest') =
          NEW.control_keyring_digest
      AND json_type(NEW.source_json, '$.launch_profile') = 'object'
      AND (SELECT COUNT(*) FROM json_each(NEW.source_json, '$.launch_profile')) = 24
      AND json_extract(NEW.source_json, '$.launch_profile.schema') =
          'elon.compute_plugin.work_admission_launch_profile.v1'
      AND json_extract(NEW.source_json, '$.launch_profile.plugin_id') = NEW.plugin_id
      AND json_extract(NEW.source_json, '$.launch_profile.plugin_version') =
          NEW.plugin_version
      AND json_extract(NEW.source_json, '$.launch_profile.publisher_id') = NEW.publisher_id
      AND json_extract(NEW.source_json, '$.launch_profile.manifest_digest') =
          NEW.manifest_digest
      AND json_extract(
          NEW.source_json, '$.launch_profile.signed_manifest_envelope_digest'
      ) = NEW.signed_manifest_envelope_digest
      AND json_extract(NEW.source_json, '$.launch_profile.target_id') = NEW.target_id
      AND json_type(NEW.source_json, '$.launch_profile.target') = 'object'
      AND json_extract(NEW.source_json, '$.launch_profile.target') = NEW.target_json
      AND json_type(NEW.source_json, '$.launch_profile.task_kinds') = 'array'
      AND json_extract(NEW.source_json, '$.launch_profile.task_kinds') = NEW.task_kinds_json
      AND json_extract(NEW.source_json, '$.launch_profile.host_api_protocol_id') =
          NEW.host_api_protocol_id
      AND json_extract(NEW.source_json, '$.launch_profile.host_api_revision') =
          NEW.host_api_revision
      AND json_extract(NEW.source_json, '$.launch_profile.entrypoint_kind') =
          NEW.entrypoint_kind
      AND json_extract(NEW.source_json, '$.launch_profile.entrypoint_relative_path') =
          NEW.entrypoint_relative_path
      AND json_type(NEW.source_json, '$.launch_profile.entrypoint_arguments') = 'array'
      AND json_extract(NEW.source_json, '$.launch_profile.entrypoint_arguments') =
          NEW.entrypoint_arguments_json
      AND json_extract(NEW.source_json, '$.launch_profile.entrypoint_arguments_digest') =
          NEW.entrypoint_arguments_digest
      AND json_type(NEW.source_json, '$.launch_profile.health_check') = 'object'
      AND json_extract(NEW.source_json, '$.launch_profile.health_check') =
          NEW.health_check_json
      AND json_extract(NEW.source_json, '$.launch_profile.runner_relative_path') =
          NEW.runner_relative_path
      AND json_extract(NEW.source_json, '$.launch_profile.runner_file_digest') =
          NEW.runner_file_digest
      AND json_extract(NEW.source_json, '$.launch_profile.runner_file_size_bytes') =
          NEW.runner_file_size_bytes
      AND json_type(NEW.source_json, '$.launch_profile.runner_file_executable') = 'true'
      AND json_extract(NEW.source_json, '$.launch_profile.runner_file_executable') =
          NEW.runner_file_executable
      AND json_extract(NEW.source_json, '$.launch_profile.grant_ref') = NEW.grant_ref
      AND json_extract(NEW.source_json, '$.launch_profile.grant_digest') =
          NEW.permission_grant_digest
      AND json_type(NEW.source_json, '$.launch_profile.granted_permissions') = 'object'
      AND json_extract(NEW.source_json, '$.launch_profile.granted_permissions') =
          NEW.granted_permissions_json
      AND json_type(NEW.source_json, '$.launch_profile.granted_resources') = 'object'
      AND (SELECT COUNT(*)
           FROM json_each(NEW.source_json, '$.launch_profile.granted_resources')) = 6
      AND json_extract(
          NEW.source_json, '$.launch_profile.granted_resources.max_cpu_millicores'
      ) = NEW.authorized_max_cpu_millicores
      AND json_extract(
          NEW.source_json, '$.launch_profile.granted_resources.max_memory_bytes'
      ) = NEW.authorized_max_memory_bytes
      AND json_extract(
          NEW.source_json, '$.launch_profile.granted_resources.max_vram_bytes'
      ) = NEW.authorized_max_vram_bytes
      AND json_extract(
          NEW.source_json, '$.launch_profile.granted_resources.max_disk_bytes'
      ) = NEW.authorized_max_disk_bytes
      AND json_extract(
          NEW.source_json, '$.launch_profile.granted_resources.max_processes'
      ) = NEW.authorized_max_processes
      AND json_extract(
          NEW.source_json, '$.launch_profile.granted_resources.max_sidecar_uptime_seconds'
      ) = NEW.authorized_max_sidecar_uptime_seconds
)
BEGIN
    SELECT RAISE(ABORT, 'work admission source JSON projection changed');
END;
"#;
