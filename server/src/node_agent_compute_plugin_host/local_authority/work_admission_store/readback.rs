use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::node_agent_compute_plugin_host::work_admission_contract::{
    ComputePluginWorkAdmissionReceipt, ComputePluginWorkAdmissionReceiptPair,
    ComputePluginWorkAdmissionSource, HashedComputePluginWorkAdmissionReceipt,
    HashedComputePluginWorkAdmissionSource,
};

mod row;
mod validation;

use row::StoredWorkAdmissionRow;

pub(super) fn read_pair(
    transaction: &Transaction<'_>,
    work_admission_id: &str,
    receipt_digest: &str,
) -> Result<Option<ComputePluginWorkAdmissionReceiptPair>> {
    let stored = transaction
        .query_row(
            r#"SELECT
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
            FROM compute_plugin_work_admission_receipts
            WHERE work_admission_id = ?1 AND receipt_digest = ?2"#,
            params![work_admission_id, receipt_digest],
            StoredWorkAdmissionRow::read,
        )
        .optional()
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_RECEIPT_READBACK")?;
    stored.map(decode_row).transpose()
}

pub(super) fn read_pair_required(
    transaction: &Transaction<'_>,
    work_admission_id: &str,
    receipt_digest: &str,
) -> Result<ComputePluginWorkAdmissionReceiptPair> {
    read_pair(transaction, work_admission_id, receipt_digest)?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_RECEIPT_READBACK_MISSING"))
}

fn decode_row(stored: StoredWorkAdmissionRow) -> Result<ComputePluginWorkAdmissionReceiptPair> {
    let source: ComputePluginWorkAdmissionSource = serde_json::from_str(&stored.source_json)
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_SOURCE_READBACK_PARSE")?;
    let receipt: ComputePluginWorkAdmissionReceipt = serde_json::from_str(&stored.receipt_json)
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_RECEIPT_READBACK_PARSE")?;
    let source = HashedComputePluginWorkAdmissionSource::from_store_readback(
        source,
        stored.source_digest.clone(),
    )?;
    let receipt = HashedComputePluginWorkAdmissionReceipt::from_store_readback(
        receipt,
        stored.receipt_digest.clone(),
    )?;
    let pair = ComputePluginWorkAdmissionReceiptPair::new(source, receipt)?;
    validation::validate_row(&stored, &pair)?;
    Ok(pair)
}
