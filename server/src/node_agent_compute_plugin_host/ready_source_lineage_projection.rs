use anyhow::{anyhow, Result};
use chrono::SecondsFormat;

use super::{
    ready_capability::ValidatedComputeReadyPublication,
    user_node_ready_source_lineage_contract::{
        build_compute_user_node_ready_source_lineage, ComputeUserNodeReadyGrantedResourceCeilingV1,
        ComputeUserNodeReadyHealthSourceRefV1, ComputeUserNodeReadyPluginReleaseRefV1,
        ComputeUserNodeReadySourceLineageSources, ComputeUserNodeReadyTrustedTimeRefV1,
        ComputeUserNodeReadyWorkAdmissionSourceRefV1, ProjectedComputeUserNodeReadySourceLineageV1,
        UntrustedComputeUserNodeHostRuntimeObservationV1,
        COMPUTE_USER_NODE_READY_WORK_ADMISSION_RECEIPT_SCHEMA,
        COMPUTE_USER_NODE_READY_WORK_ADMISSION_SOURCE_SCHEMA,
    },
    work_admission_contract::DurableWorkAdmittedPluginSlot,
};

/// Projects already-validated local source facts without turning them into Ready authority.
/// The Host observation remains caller-supplied and untrusted; a future Host authority and v15
/// authenticated session must independently reprove this lineage before server adoption.
pub(in crate::node_agent_compute_plugin_host) fn project_user_node_ready_source_lineage(
    admitted: &DurableWorkAdmittedPluginSlot<'_>,
    ready: &ValidatedComputeReadyPublication,
    host_runtime_observation: UntrustedComputeUserNodeHostRuntimeObservationV1,
) -> Result<ProjectedComputeUserNodeReadySourceLineageV1> {
    let receipts = admitted.receipts();
    let source = receipts.source().source();
    let receipt = receipts.receipt().receipt();
    let profile = source.launch_profile();
    let generations = receipt.generations();
    let authority = receipt.authority();
    let granted = profile.granted_resources();
    let work_admission = ComputeUserNodeReadyWorkAdmissionSourceRefV1 {
        source_schema: COMPUTE_USER_NODE_READY_WORK_ADMISSION_SOURCE_SCHEMA.to_string(),
        source_digest: receipts.source().source_digest().to_string(),
        receipt_schema: COMPUTE_USER_NODE_READY_WORK_ADMISSION_RECEIPT_SCHEMA.to_string(),
        work_admission_id: receipt.work_admission_id().to_string(),
        receipt_digest: receipts.receipt().receipt_digest().to_string(),
        clock_epoch_digest: receipt.clock_epoch_digest().to_string(),
        admitted_at_ms: receipt.admitted_at_ms(),
        installation_identity_digest: source.installation_id_digest().to_string(),
        plugin_id: source.plugin_id().to_string(),
        slot_ref: source.slot_ref().to_string(),
        release: release_ref(source.release()),
        install_receipt_id: source.install_receipt_id().to_string(),
        install_receipt_digest: source.install_receipt_digest().to_string(),
        promotion_receipt_id: source.promotion_receipt_id().to_string(),
        promotion_receipt_digest: source.promotion_receipt_digest().to_string(),
        plan_id: source.plan().plan_id().to_string(),
        plan_digest: source.plan().plan_digest().to_string(),
        plan_policy_revision: source.plan().policy_revision(),
        application_receipt_digest: source.plan().application_receipt_digest().to_string(),
        grant_ref: profile.grant_ref().to_string(),
        grant_digest: profile.grant_digest().to_string(),
        install_generation: generations.install_generation(),
        activation_generation: generations.activation_generation(),
        runtime_generation_before_ready: generations.runtime_generation(),
        work_admission_generation: generations.work_admission_generation_after(),
        inventory_revision: authority.inventory_revision_after(),
        inventory_digest: authority.inventory_digest_after().to_string(),
        authority_state_revision: authority.authority_state_revision_after(),
        authority_epoch: authority.authority_epoch_after(),
        process_owner_epoch: authority.process_owner_epoch(),
        runner_digest: profile.runner_file_digest().to_string(),
        target_accelerator_kind: profile.target().accelerator_kind.clone(),
        task_kinds: profile.task_kinds().to_vec(),
        granted_resources: ComputeUserNodeReadyGrantedResourceCeilingV1 {
            max_cpu_millicores: granted.max_cpu_millicores,
            max_memory_bytes: granted.max_memory_bytes,
            max_vram_bytes: granted.max_vram_bytes,
            max_disk_bytes: granted.max_disk_bytes,
            max_processes: granted.max_processes,
            max_sidecar_uptime_seconds: granted.max_sidecar_uptime_seconds,
            allow_network_egress: profile.granted_permissions().allow_network_egress,
        },
    };

    let record = ready.record();
    let health = record
        .health
        .as_ref()
        .ok_or_else(|| anyhow!("validated Ready publication lost its health source"))?;
    let active_slot = record
        .active_slot_ref
        .as_deref()
        .and_then(|slot_ref| record.slots.iter().find(|slot| slot.slot_ref == slot_ref))
        .ok_or_else(|| anyhow!("validated Ready publication lost its active slot"))?;
    let permission_grant_digest = record
        .permission_grant_digest
        .as_deref()
        .ok_or_else(|| anyhow!("validated Ready publication lost its grant digest"))?;
    let last_plan_id = record
        .last_plan_id
        .as_deref()
        .ok_or_else(|| anyhow!("validated Ready publication lost its Plan binding"))?;
    let trusted = ready.trusted_time();
    let ready_health = ComputeUserNodeReadyHealthSourceRefV1 {
        inventory_revision: ready.inventory_revision(),
        desired_policy_revision: ready.desired_policy_revision(),
        installation_identity_digest: trusted.installation_id_digest().to_string(),
        plugin_id: record.plugin_id.clone(),
        last_plan_id: last_plan_id.to_string(),
        slot_ref: health.slot_ref.clone(),
        release: release_ref(&active_slot.release),
        permission_grant_digest: permission_grant_digest.to_string(),
        install_generation: record.install_generation,
        activation_generation: record.activation_generation,
        runtime_generation: record.runtime.runtime_generation,
        runner_digest: health.runner_digest.clone(),
        health_observation_digest: health.observation_digest.clone(),
        health_reason_codes: health.reason_codes.clone(),
        health_observed_at: health.observed_at.clone(),
        health_expires_at: health.expires_at.clone(),
        trusted_time: ComputeUserNodeReadyTrustedTimeRefV1 {
            trusted_now: trusted
                .trusted_now()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            clock_epoch_digest: trusted.clock_epoch_digest().to_string(),
            time_authority_id: trusted.time_authority_id().to_string(),
            attestation_digest: trusted.attestation_digest().to_string(),
            attestation_sequence: trusted.attestation_sequence(),
            signing_key_fingerprint: trusted.signing_key_fingerprint().to_string(),
        },
    };

    build_compute_user_node_ready_source_lineage(ComputeUserNodeReadySourceLineageSources {
        work_admission,
        ready_health,
        host_runtime_observation,
    })
}

fn release_ref(
    value: &crate::compute_attempt_contract::ComputePluginReleaseRef,
) -> ComputeUserNodeReadyPluginReleaseRefV1 {
    ComputeUserNodeReadyPluginReleaseRefV1 {
        plugin_id: value.plugin_id.clone(),
        plugin_version: value.plugin_version.clone(),
        target_id: value.target_id.clone(),
        manifest_digest: value.manifest_digest.clone(),
        package_digest: value.package_digest.clone(),
    }
}
