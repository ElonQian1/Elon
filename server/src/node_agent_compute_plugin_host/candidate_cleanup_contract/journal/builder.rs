use anyhow::{anyhow, bail, Result};
use serde::Serialize;

use super::types::{
    hash_cleanup_step_event, validate_hashed_cleanup_step_event,
    ComputePluginCandidateCleanupStepEvent, HashedComputePluginCandidateCleanupStepEvent,
    CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA,
};
use crate::node_agent_compute_plugin_host::candidate_cleanup_contract::{
    validate_hashed_execution_plan, HashedComputePluginCandidateCleanupExecutionPlan,
};
use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;
use crate::node_agent_managed_fs::{ManagedNamespaceDurable, ManagedObjectBinding};

const CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_EVIDENCE_SCHEMA: &str =
    "elon.compute_plugin.candidate_cleanup_namespace_durability_evidence.v1";
const CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_KIND: &str =
    "windows_nt_flush_buffers_file_ex_normal_parent_directory_v1";

#[derive(Serialize)]
struct ComputePluginCandidateCleanupNamespaceDurabilityEvidence<'evidence> {
    schema: &'static str,
    cleanup_id: &'evidence str,
    plan_digest: &'evidence str,
    event_sequence: i64,
    step_ordinal: i64,
    object_digest: &'evidence str,
    observed_parent_identity_digest: &'evidence str,
    previous_event_digest: &'evidence str,
    namespace_durability_kind: &'evidence str,
    filesystem_kind: &'evidence str,
    process_owner_epoch: i64,
    recorded_at_ms: i64,
}

pub(in crate::node_agent_compute_plugin_host) fn build_namespace_durability_evidence_digest(
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    absence: &HashedComputePluginCandidateCleanupStepEvent,
    namespace_durability_kind: &str,
    filesystem_kind: &str,
    recorded_at_ms: i64,
) -> Result<String> {
    validate_hashed_execution_plan(plan)?;
    validate_hashed_cleanup_step_event(absence)?;
    let object = plan
        .objects()
        .first()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DURABILITY_OBJECT_MISSING"))?;
    let absence_event = absence.event();
    if absence_event.cleanup_id() != plan.plan().cleanup_id()
        || absence_event.plan_digest() != plan.plan_digest()
        || absence_event.event_sequence() != 3
        || absence_event.step_ordinal() != 0
        || absence_event.event_kind() != "parent_namespace_absence_observed"
        || absence_event.object_digest() != object.object_digest()
        || absence_event.observed_parent_identity_digest()
            != object.object().expected_parent_identity_digest()
        || namespace_durability_kind != CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_KIND
        || !matches!(filesystem_kind, "ntfs" | "refs" | "fat" | "fat32" | "exfat")
        || recorded_at_ms <= absence_event.recorded_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DURABILITY_EVIDENCE_BINDING_INVALID");
    }
    jcs_sha256_hex(&ComputePluginCandidateCleanupNamespaceDurabilityEvidence {
        schema: CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_EVIDENCE_SCHEMA,
        cleanup_id: plan.plan().cleanup_id(),
        plan_digest: plan.plan_digest(),
        event_sequence: 4,
        step_ordinal: 0,
        object_digest: object.object_digest(),
        observed_parent_identity_digest: object.object().expected_parent_identity_digest(),
        previous_event_digest: absence.event_digest(),
        namespace_durability_kind,
        filesystem_kind,
        process_owner_epoch: plan.plan().process_owner_epoch(),
        recorded_at_ms,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn build_initial_delete_intent(
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    recorded_at_ms: i64,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    validate_hashed_execution_plan(plan)?;
    let object = plan
        .objects()
        .first()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_OBJECT_MISSING"))?;
    if object.object().step_ordinal() != 0
        || plan.plan().object_count() <= 0
        || recorded_at_ms <= plan.plan().planned_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_BINDING_INVALID");
    }
    hash_cleanup_step_event(ComputePluginCandidateCleanupStepEvent {
        schema: CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA.to_string(),
        cleanup_id: plan.plan().cleanup_id().to_string(),
        plan_digest: plan.plan_digest().to_string(),
        event_sequence: 1,
        step_ordinal: 0,
        event_kind: "delete_intent".to_string(),
        object_digest: object.object_digest().to_string(),
        observed_identity_digest: Some(object.object().expected_identity_digest().to_string()),
        observed_parent_identity_digest: object
            .object()
            .expected_parent_identity_digest()
            .to_string(),
        namespace_durability_kind: None,
        namespace_durability_evidence_digest: None,
        previous_event_digest: plan.plan_digest().to_string(),
        process_owner_epoch: plan.plan().process_owner_epoch(),
        recorded_at_ms,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn build_exact_handle_disposition_event(
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    intent: &HashedComputePluginCandidateCleanupStepEvent,
    binding: &ManagedObjectBinding,
    recorded_at_ms: i64,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    let relative_name = binding
        .relative_name()
        .to_str()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_NAME_INVALID"))?;
    build_exact_handle_disposition_event_from_fields(
        plan,
        intent,
        binding.is_directory(),
        relative_name,
        binding.identity_digest(),
        binding.parent_identity_digest(),
        recorded_at_ms,
    )
}

pub(in crate::node_agent_compute_plugin_host) fn build_parent_namespace_absence_event(
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    intent: &HashedComputePluginCandidateCleanupStepEvent,
    disposition: &HashedComputePluginCandidateCleanupStepEvent,
    binding: &ManagedObjectBinding,
    recorded_at_ms: i64,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    validate_hashed_execution_plan(plan)?;
    let expected_disposition = build_exact_handle_disposition_event(
        plan,
        intent,
        binding,
        disposition.event().recorded_at_ms(),
    )?;
    let object = plan
        .objects()
        .first()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ABSENCE_OBJECT_MISSING"))?;
    if expected_disposition != *disposition
        || disposition.event().event_sequence() != 2
        || disposition.event().step_ordinal() != 0
        || recorded_at_ms <= disposition.event().recorded_at_ms()
        || binding.parent_identity_digest() != object.object().expected_parent_identity_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ABSENCE_BINDING_INVALID");
    }
    hash_cleanup_step_event(ComputePluginCandidateCleanupStepEvent {
        schema: CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA.to_string(),
        cleanup_id: plan.plan().cleanup_id().to_string(),
        plan_digest: plan.plan_digest().to_string(),
        event_sequence: 3,
        step_ordinal: 0,
        event_kind: "parent_namespace_absence_observed".to_string(),
        object_digest: object.object_digest().to_string(),
        observed_identity_digest: None,
        observed_parent_identity_digest: binding.parent_identity_digest().to_string(),
        namespace_durability_kind: None,
        namespace_durability_evidence_digest: None,
        previous_event_digest: disposition.event_digest().to_string(),
        process_owner_epoch: plan.plan().process_owner_epoch(),
        recorded_at_ms,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn build_namespace_durable_event(
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    intent: &HashedComputePluginCandidateCleanupStepEvent,
    disposition: &HashedComputePluginCandidateCleanupStepEvent,
    absence: &HashedComputePluginCandidateCleanupStepEvent,
    namespace: &ManagedNamespaceDurable,
    recorded_at_ms: i64,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    validate_hashed_execution_plan(plan)?;
    let binding = namespace.object_binding();
    let expected_absence = build_parent_namespace_absence_event(
        plan,
        intent,
        disposition,
        binding,
        absence.event().recorded_at_ms(),
    )?;
    let object = plan
        .objects()
        .first()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DURABILITY_OBJECT_MISSING"))?;
    if expected_absence != *absence
        || absence.event().event_sequence() != 3
        || absence.event().step_ordinal() != 0
        || recorded_at_ms <= absence.event().recorded_at_ms()
        || namespace.barrier_completed_at() >= namespace.post_absence_observed_at()
        || namespace.post_absence_observed_at() > namespace.completed_at()
        || binding.parent_identity_digest() != object.object().expected_parent_identity_digest()
        || binding.identity_digest() != object.object().expected_identity_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DURABILITY_BINDING_INVALID");
    }
    let namespace_durability_evidence_digest = build_namespace_durability_evidence_digest(
        plan,
        absence,
        namespace.namespace_durability_kind(),
        namespace.filesystem_kind(),
        recorded_at_ms,
    )?;
    hash_cleanup_step_event(ComputePluginCandidateCleanupStepEvent {
        schema: CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA.to_string(),
        cleanup_id: plan.plan().cleanup_id().to_string(),
        plan_digest: plan.plan_digest().to_string(),
        event_sequence: 4,
        step_ordinal: 0,
        event_kind: "namespace_durable".to_string(),
        object_digest: object.object_digest().to_string(),
        observed_identity_digest: None,
        observed_parent_identity_digest: binding.parent_identity_digest().to_string(),
        namespace_durability_kind: Some(namespace.namespace_durability_kind().to_string()),
        namespace_durability_evidence_digest: Some(namespace_durability_evidence_digest),
        previous_event_digest: absence.event_digest().to_string(),
        process_owner_epoch: plan.plan().process_owner_epoch(),
        recorded_at_ms,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_exact_handle_disposition_event_from_fields(
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    intent: &HashedComputePluginCandidateCleanupStepEvent,
    is_directory: bool,
    relative_name: &str,
    identity_digest: &str,
    parent_identity_digest: &str,
    recorded_at_ms: i64,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    validate_hashed_execution_plan(plan)?;
    let expected_intent = build_initial_delete_intent(plan, intent.event().recorded_at_ms())?;
    let object = plan
        .objects()
        .first()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_OBJECT_MISSING"))?;
    let expected = object.object();
    if expected_intent != *intent
        || expected.step_ordinal() != 0
        || (expected.object_kind() == "directory") != is_directory
        || expected.relative_name() != relative_name
        || expected.expected_identity_digest() != identity_digest
        || expected.expected_parent_identity_digest() != parent_identity_digest
        || recorded_at_ms <= intent.event().recorded_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_BINDING_INVALID");
    }
    hash_cleanup_step_event(ComputePluginCandidateCleanupStepEvent {
        schema: CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA.to_string(),
        cleanup_id: plan.plan().cleanup_id().to_string(),
        plan_digest: plan.plan_digest().to_string(),
        event_sequence: 2,
        step_ordinal: 0,
        event_kind: "exact_handle_disposition_set".to_string(),
        object_digest: object.object_digest().to_string(),
        observed_identity_digest: Some(identity_digest.to_string()),
        observed_parent_identity_digest: parent_identity_digest.to_string(),
        namespace_durability_kind: None,
        namespace_durability_evidence_digest: None,
        previous_event_digest: intent.event_digest().to_string(),
        process_owner_epoch: plan.plan().process_owner_epoch(),
        recorded_at_ms,
    })
}

#[cfg(test)]
mod tests;
