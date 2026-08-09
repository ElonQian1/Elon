use anyhow::{bail, Result};

use super::StoredWorkAdmissionRow;
use crate::node_agent_compute_plugin_host::work_admission_contract::{
    ComputePluginWorkAdmissionReceiptPair, ComputePluginWorkAdmissionSource,
};

pub(super) fn validate_row(
    row: &StoredWorkAdmissionRow,
    pair: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<()> {
    pair.validate()?;
    let source = pair.source().source();
    let receipt = pair.receipt().receipt();
    if row.source_json != serde_json::to_string(source)?
        || row.receipt_json != serde_json::to_string(receipt)?
        || row.source_digest != pair.source().source_digest()
        || row.receipt_digest != pair.receipt().receipt_digest()
        || !source_projection_matches(row, source)?
        || !receipt_projection_matches(row, receipt)?
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_READBACK_CHANGED");
    }
    Ok(())
}

fn source_projection_matches(
    row: &StoredWorkAdmissionRow,
    source: &ComputePluginWorkAdmissionSource,
) -> Result<bool> {
    let plan = source.plan();
    let profile = source.launch_profile();
    let resources = profile.granted_resources();
    Ok(
        row.installation_id_digest == source.installation_id_digest()
            && row.plugin_id == source.plugin_id()
            && row.slot_ref == source.slot_ref()
            && row.release_json == serde_json::to_string(source.release())?
            && row.install_receipt_id == source.install_receipt_id()
            && row.install_receipt_digest == source.install_receipt_digest()
            && row.promotion_receipt_id == source.promotion_receipt_id()
            && row.promotion_receipt_digest == source.promotion_receipt_digest()
            && row.plan_action == plan.action()
            && row.plan_id == plan.plan_id()
            && row.plan_digest == plan.plan_digest()
            && row.signed_plan_envelope_digest == plan.signed_plan_envelope_digest()
            && row.signed_manifest_set_digest == plan.signed_manifest_set_digest()
            && row.application_request_digest == plan.application_request_digest()
            && row.application_receipt_digest == plan.application_receipt_digest()
            && row.admission_bindings_digest == plan.admission_bindings_digest()
            && row.application_inventory_revision == plan.application_inventory_revision()
            && row.policy_revision == plan.policy_revision()
            && row.sharing_authorization_ref == plan.sharing_authorization_ref()
            && row.sharing_authorization_revision == plan.sharing_authorization_revision()
            && row.sharing_authorization_digest == plan.sharing_authorization_digest()
            && row.policy_binding_receipt_digest == plan.policy_binding_receipt_digest()
            && row.policy_revocation_receipt_digest == plan.policy_revocation_receipt_digest()
            && row.node_profile_digest == plan.node_profile_digest()
            && row.manifest_catalog_revision == plan.manifest_catalog_revision()
            && row.manifest_catalog_digest == plan.manifest_catalog_digest()
            && row.manifest_catalog_binding_receipt_digest
                == plan.manifest_catalog_binding_receipt_digest()
            && row.keyring_bundle_revision == plan.keyring_bundle_revision()
            && row.publisher_keyring_revision == plan.publisher_keyring_revision()
            && row.publisher_keyring_digest == plan.publisher_keyring_digest()
            && row.control_keyring_revision == plan.control_keyring_revision()
            && row.control_keyring_digest == plan.control_keyring_digest()
            && row.plugin_version == profile.plugin_version()
            && row.publisher_id == profile.publisher_id()
            && row.manifest_digest == profile.manifest_digest()
            && row.signed_manifest_envelope_digest == profile.signed_manifest_envelope_digest()
            && row.target_id == profile.target_id()
            && row.target_json == serde_json::to_string(profile.target())?
            && row.task_kinds_json == serde_json::to_string(profile.task_kinds())?
            && row.host_api_protocol_id == profile.host_api_protocol_id()
            && row.host_api_revision == i64::from(profile.host_api_revision())
            && row.entrypoint_kind == profile.entrypoint_kind()
            && row.entrypoint_relative_path == profile.entrypoint_relative_path()
            && row.entrypoint_arguments_json
                == serde_json::to_string(profile.entrypoint_arguments())?
            && row.entrypoint_arguments_digest == profile.entrypoint_arguments_digest()
            && row.health_check_json == serde_json::to_string(profile.health_check())?
            && row.runner_relative_path == profile.runner_relative_path()
            && row.runner_file_digest == profile.runner_file_digest()
            && row.runner_file_size_bytes == profile.runner_file_size_bytes()
            && row.runner_file_executable == profile.runner_file_executable()
            && row.grant_ref == profile.grant_ref()
            && row.permission_grant_digest == profile.grant_digest()
            && row.granted_permissions_json
                == serde_json::to_string(profile.granted_permissions())?
            && row.authorized_max_cpu_millicores == resources.max_cpu_millicores
            && row.authorized_max_memory_bytes == resources.max_memory_bytes
            && row.authorized_max_vram_bytes == resources.max_vram_bytes
            && row.authorized_max_disk_bytes == resources.max_disk_bytes
            && row.authorized_max_processes == resources.max_processes
            && row.authorized_max_sidecar_uptime_seconds == resources.max_sidecar_uptime_seconds,
    )
}

fn receipt_projection_matches(
    row: &StoredWorkAdmissionRow,
    receipt: &crate::node_agent_compute_plugin_host::work_admission_contract::ComputePluginWorkAdmissionReceipt,
) -> Result<bool> {
    let generations = receipt.generations();
    let quiescence = receipt.quiescence();
    let authority = receipt.authority();
    Ok(row.work_admission_id == receipt.work_admission_id()
        && row.installation_id_digest == receipt.installation_id_digest()
        && row.clock_epoch_digest == receipt.clock_epoch_digest()
        && row.plugin_id == receipt.plugin_id()
        && row.slot_ref == receipt.slot_ref()
        && row.release_json == serde_json::to_string(receipt.release())?
        && row.install_receipt_id == receipt.install_receipt_id()
        && row.install_receipt_digest == receipt.install_receipt_digest()
        && row.promotion_receipt_id == receipt.promotion_receipt_id()
        && row.promotion_receipt_digest == receipt.promotion_receipt_digest()
        && row.source_digest == receipt.source_digest()
        && row.install_generation == generations.install_generation()
        && row.activation_generation == generations.activation_generation()
        && row.runtime_generation == generations.runtime_generation()
        && row.work_admission_generation_before == generations.work_admission_generation_before()
        && row.work_admission_generation_after == generations.work_admission_generation_after()
        && row.previous_work_admission_id.as_deref() == generations.previous_work_admission_id()
        && row.previous_work_admission_receipt_digest.as_deref()
            == generations.previous_work_admission_receipt_digest()
        && row.desired_presence == quiescence.desired_presence()
        && row.desired_activation == quiescence.desired_activation()
        && row.slot_phase == quiescence.slot_phase()
        && row.admission == quiescence.admission()
        && row.runtime_phase == quiescence.runtime_phase()
        && row.candidate_slot_present == quiescence.candidate_slot_present()
        && row.runtime_slot_present == quiescence.runtime_slot_present()
        && row.runtime_runner_digest_present == quiescence.runtime_runner_digest_present()
        && row.health_present == quiescence.health_present()
        && row.active_attempts == quiescence.active_attempts()
        && row.authority_state_revision_before == authority.authority_state_revision_before()
        && row.authority_state_revision_after == authority.authority_state_revision_after()
        && row.inventory_revision_before == authority.inventory_revision_before()
        && row.inventory_revision_after == authority.inventory_revision_after()
        && row.inventory_digest_before == authority.inventory_digest_before()
        && row.inventory_digest_after == authority.inventory_digest_after()
        && row.authority_epoch_before == authority.authority_epoch_before()
        && row.authority_epoch_after == authority.authority_epoch_after()
        && row.process_owner_epoch == authority.process_owner_epoch()
        && row.trusted_time_before_ms == authority.trusted_time_high_water_ms_before()
        && row.authority_updated_at_ms_before == authority.authority_updated_at_ms_before()
        && row.admitted_at_ms == receipt.admitted_at_ms())
}
