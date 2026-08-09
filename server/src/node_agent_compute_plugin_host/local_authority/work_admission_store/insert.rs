use anyhow::{bail, Context, Result};
use rusqlite::{named_params, Transaction};

use crate::node_agent_compute_plugin_host::work_admission_contract::ComputePluginWorkAdmissionReceiptPair;

pub(super) fn insert_receipt(
    transaction: &Transaction<'_>,
    pair: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<()> {
    pair.validate()?;
    let hashed_source = pair.source();
    let source = hashed_source.source();
    let plan = source.plan();
    let profile = source.launch_profile();
    let resources = profile.granted_resources();
    let hashed_receipt = pair.receipt();
    let receipt = hashed_receipt.receipt();
    let generations = receipt.generations();
    let quiescence = receipt.quiescence();
    let authority = receipt.authority();

    let source_json = serde_json::to_string(source)?;
    let receipt_json = serde_json::to_string(receipt)?;
    let release_json = serde_json::to_string(source.release())?;
    let target_json = serde_json::to_string(profile.target())?;
    let task_kinds_json = serde_json::to_string(profile.task_kinds())?;
    let entrypoint_arguments_json = serde_json::to_string(profile.entrypoint_arguments())?;
    let health_check_json = serde_json::to_string(profile.health_check())?;
    let granted_permissions_json = serde_json::to_string(profile.granted_permissions())?;

    let changed = transaction
        .execute(
            r#"INSERT INTO compute_plugin_work_admission_receipts (
                work_admission_id, installation_id_digest, clock_epoch_digest, plugin_id,
                slot_ref, release_json, install_receipt_id, install_receipt_digest,
                promotion_receipt_id, promotion_receipt_digest, source_digest, plan_action,
                plan_id, plan_digest, signed_plan_envelope_digest, signed_manifest_set_digest,
                application_request_digest, application_receipt_digest,
                admission_bindings_digest, application_inventory_revision, policy_revision,
                sharing_authorization_ref, sharing_authorization_revision,
                sharing_authorization_digest, policy_binding_receipt_digest,
                policy_revocation_receipt_digest, node_profile_digest,
                manifest_catalog_revision, manifest_catalog_digest,
                manifest_catalog_binding_receipt_digest, keyring_bundle_revision,
                publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest, plugin_version,
                publisher_id, manifest_digest, signed_manifest_envelope_digest, target_id,
                target_json, task_kinds_json, host_api_protocol_id, host_api_revision,
                entrypoint_kind, entrypoint_relative_path, entrypoint_arguments_json,
                entrypoint_arguments_digest, health_check_json, runner_relative_path,
                runner_file_digest, runner_file_size_bytes, runner_file_executable, grant_ref,
                permission_grant_digest, granted_permissions_json,
                authorized_max_cpu_millicores, authorized_max_memory_bytes,
                authorized_max_vram_bytes, authorized_max_disk_bytes,
                authorized_max_processes, authorized_max_sidecar_uptime_seconds,
                install_generation, activation_generation, runtime_generation,
                work_admission_generation_before, work_admission_generation_after,
                previous_work_admission_id, previous_work_admission_receipt_digest,
                desired_presence, desired_activation, slot_phase, admission, runtime_phase,
                candidate_slot_present, runtime_slot_present, runtime_runner_digest_present,
                health_present, active_attempts, authority_state_revision_before,
                authority_state_revision_after, inventory_revision_before,
                inventory_revision_after, inventory_digest_before, inventory_digest_after,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                trusted_time_before_ms, authority_updated_at_ms_before, admitted_at_ms,
                source_json, receipt_json, receipt_digest
            ) VALUES (
                :work_admission_id, :installation_id_digest, :clock_epoch_digest, :plugin_id,
                :slot_ref, :release_json, :install_receipt_id, :install_receipt_digest,
                :promotion_receipt_id, :promotion_receipt_digest, :source_digest, :plan_action,
                :plan_id, :plan_digest, :signed_plan_envelope_digest,
                :signed_manifest_set_digest, :application_request_digest,
                :application_receipt_digest, :admission_bindings_digest,
                :application_inventory_revision, :policy_revision,
                :sharing_authorization_ref, :sharing_authorization_revision,
                :sharing_authorization_digest, :policy_binding_receipt_digest,
                :policy_revocation_receipt_digest, :node_profile_digest,
                :manifest_catalog_revision, :manifest_catalog_digest,
                :manifest_catalog_binding_receipt_digest, :keyring_bundle_revision,
                :publisher_keyring_revision, :publisher_keyring_digest,
                :control_keyring_revision, :control_keyring_digest, :plugin_version,
                :publisher_id, :manifest_digest, :signed_manifest_envelope_digest, :target_id,
                :target_json, :task_kinds_json, :host_api_protocol_id, :host_api_revision,
                :entrypoint_kind, :entrypoint_relative_path, :entrypoint_arguments_json,
                :entrypoint_arguments_digest, :health_check_json, :runner_relative_path,
                :runner_file_digest, :runner_file_size_bytes, :runner_file_executable,
                :grant_ref, :permission_grant_digest, :granted_permissions_json,
                :authorized_max_cpu_millicores, :authorized_max_memory_bytes,
                :authorized_max_vram_bytes, :authorized_max_disk_bytes,
                :authorized_max_processes, :authorized_max_sidecar_uptime_seconds,
                :install_generation, :activation_generation, :runtime_generation,
                :work_admission_generation_before, :work_admission_generation_after,
                :previous_work_admission_id, :previous_work_admission_receipt_digest,
                :desired_presence, :desired_activation, :slot_phase, :admission,
                :runtime_phase, :candidate_slot_present, :runtime_slot_present,
                :runtime_runner_digest_present, :health_present, :active_attempts,
                :authority_state_revision_before, :authority_state_revision_after,
                :inventory_revision_before, :inventory_revision_after,
                :inventory_digest_before, :inventory_digest_after, :authority_epoch_before,
                :authority_epoch_after, :process_owner_epoch, :trusted_time_before_ms,
                :authority_updated_at_ms_before, :admitted_at_ms, :source_json,
                :receipt_json, :receipt_digest
            )"#,
            named_params! {
                ":work_admission_id": receipt.work_admission_id(),
                ":installation_id_digest": receipt.installation_id_digest(),
                ":clock_epoch_digest": receipt.clock_epoch_digest(),
                ":plugin_id": receipt.plugin_id(),
                ":slot_ref": receipt.slot_ref(),
                ":release_json": release_json,
                ":install_receipt_id": receipt.install_receipt_id(),
                ":install_receipt_digest": receipt.install_receipt_digest(),
                ":promotion_receipt_id": receipt.promotion_receipt_id(),
                ":promotion_receipt_digest": receipt.promotion_receipt_digest(),
                ":source_digest": hashed_source.source_digest(),
                ":plan_action": plan.action(),
                ":plan_id": plan.plan_id(),
                ":plan_digest": plan.plan_digest(),
                ":signed_plan_envelope_digest": plan.signed_plan_envelope_digest(),
                ":signed_manifest_set_digest": plan.signed_manifest_set_digest(),
                ":application_request_digest": plan.application_request_digest(),
                ":application_receipt_digest": plan.application_receipt_digest(),
                ":admission_bindings_digest": plan.admission_bindings_digest(),
                ":application_inventory_revision": plan.application_inventory_revision(),
                ":policy_revision": plan.policy_revision(),
                ":sharing_authorization_ref": plan.sharing_authorization_ref(),
                ":sharing_authorization_revision": plan.sharing_authorization_revision(),
                ":sharing_authorization_digest": plan.sharing_authorization_digest(),
                ":policy_binding_receipt_digest": plan.policy_binding_receipt_digest(),
                ":policy_revocation_receipt_digest": plan.policy_revocation_receipt_digest(),
                ":node_profile_digest": plan.node_profile_digest(),
                ":manifest_catalog_revision": plan.manifest_catalog_revision(),
                ":manifest_catalog_digest": plan.manifest_catalog_digest(),
                ":manifest_catalog_binding_receipt_digest": plan.manifest_catalog_binding_receipt_digest(),
                ":keyring_bundle_revision": plan.keyring_bundle_revision(),
                ":publisher_keyring_revision": plan.publisher_keyring_revision(),
                ":publisher_keyring_digest": plan.publisher_keyring_digest(),
                ":control_keyring_revision": plan.control_keyring_revision(),
                ":control_keyring_digest": plan.control_keyring_digest(),
                ":plugin_version": profile.plugin_version(),
                ":publisher_id": profile.publisher_id(),
                ":manifest_digest": profile.manifest_digest(),
                ":signed_manifest_envelope_digest": profile.signed_manifest_envelope_digest(),
                ":target_id": profile.target_id(),
                ":target_json": target_json,
                ":task_kinds_json": task_kinds_json,
                ":host_api_protocol_id": profile.host_api_protocol_id(),
                ":host_api_revision": i64::from(profile.host_api_revision()),
                ":entrypoint_kind": profile.entrypoint_kind(),
                ":entrypoint_relative_path": profile.entrypoint_relative_path(),
                ":entrypoint_arguments_json": entrypoint_arguments_json,
                ":entrypoint_arguments_digest": profile.entrypoint_arguments_digest(),
                ":health_check_json": health_check_json,
                ":runner_relative_path": profile.runner_relative_path(),
                ":runner_file_digest": profile.runner_file_digest(),
                ":runner_file_size_bytes": profile.runner_file_size_bytes(),
                ":runner_file_executable": profile.runner_file_executable(),
                ":grant_ref": profile.grant_ref(),
                ":permission_grant_digest": profile.grant_digest(),
                ":granted_permissions_json": granted_permissions_json,
                ":authorized_max_cpu_millicores": resources.max_cpu_millicores,
                ":authorized_max_memory_bytes": resources.max_memory_bytes,
                ":authorized_max_vram_bytes": resources.max_vram_bytes,
                ":authorized_max_disk_bytes": resources.max_disk_bytes,
                ":authorized_max_processes": resources.max_processes,
                ":authorized_max_sidecar_uptime_seconds": resources.max_sidecar_uptime_seconds,
                ":install_generation": generations.install_generation(),
                ":activation_generation": generations.activation_generation(),
                ":runtime_generation": generations.runtime_generation(),
                ":work_admission_generation_before": generations.work_admission_generation_before(),
                ":work_admission_generation_after": generations.work_admission_generation_after(),
                ":previous_work_admission_id": generations.previous_work_admission_id(),
                ":previous_work_admission_receipt_digest": generations.previous_work_admission_receipt_digest(),
                ":desired_presence": quiescence.desired_presence(),
                ":desired_activation": quiescence.desired_activation(),
                ":slot_phase": quiescence.slot_phase(),
                ":admission": quiescence.admission(),
                ":runtime_phase": quiescence.runtime_phase(),
                ":candidate_slot_present": quiescence.candidate_slot_present(),
                ":runtime_slot_present": quiescence.runtime_slot_present(),
                ":runtime_runner_digest_present": quiescence.runtime_runner_digest_present(),
                ":health_present": quiescence.health_present(),
                ":active_attempts": quiescence.active_attempts(),
                ":authority_state_revision_before": authority.authority_state_revision_before(),
                ":authority_state_revision_after": authority.authority_state_revision_after(),
                ":inventory_revision_before": authority.inventory_revision_before(),
                ":inventory_revision_after": authority.inventory_revision_after(),
                ":inventory_digest_before": authority.inventory_digest_before(),
                ":inventory_digest_after": authority.inventory_digest_after(),
                ":authority_epoch_before": authority.authority_epoch_before(),
                ":authority_epoch_after": authority.authority_epoch_after(),
                ":process_owner_epoch": authority.process_owner_epoch(),
                ":trusted_time_before_ms": authority.trusted_time_high_water_ms_before(),
                ":authority_updated_at_ms_before": authority.authority_updated_at_ms_before(),
                ":admitted_at_ms": receipt.admitted_at_ms(),
                ":source_json": source_json,
                ":receipt_json": receipt_json,
                ":receipt_digest": hashed_receipt.receipt_digest(),
            },
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_RECEIPT_INSERT")?;
    if changed != 1 {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECEIPT_INSERT_CHANGED");
    }
    Ok(())
}
