use anyhow::{anyhow, bail, Result};

use super::types::{
    hash_cleanup_step_event, ComputePluginCandidateCleanupStepEvent,
    HashedComputePluginCandidateCleanupStepEvent, CANDIDATE_CLEANUP_STEP_EVENT_SCHEMA,
};
use crate::node_agent_compute_plugin_host::candidate_cleanup_contract::{
    validate_hashed_execution_plan, HashedComputePluginCandidateCleanupExecutionPlan,
};

pub(super) fn build_initial_delete_intent(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_compute_plugin_host::{
        candidate_cleanup_contract::{
            restore_hashed_execution_plan, restore_hashed_expected_object,
            ComputePluginCandidateCleanupExecutionPlan,
            ComputePluginCandidateCleanupExpectedObject,
        },
        signed_artifact_verification::jcs_sha256_hex,
    };

    fn plan() -> HashedComputePluginCandidateCleanupExecutionPlan {
        let object: ComputePluginCandidateCleanupExpectedObject = serde_json::from_value(
            serde_json::json!({
                "schema": "elon.compute_plugin.candidate_cleanup_expected_object.v1",
                "cleanup_id": "cca_intent_test",
                "step_ordinal": 0,
                "parent_step_ordinal": null,
                "topology_depth": 0,
                "object_kind": "directory",
                "logical_kind": "candidate_directory",
                "relative_name": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "relative_path": "compute-plugin/candidates/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "relative_path_digest": "1".repeat(64),
                "expected_identity_digest": "2".repeat(64),
                "expected_parent_identity_digest": "3".repeat(64),
                "expected_content_digest": null,
                "expected_size_bytes": null
            }),
        )
        .unwrap();
        let object_digest = jcs_sha256_hex(&object).unwrap();
        let object = restore_hashed_expected_object(object, object_digest.clone()).unwrap();
        let plan: ComputePluginCandidateCleanupExecutionPlan =
            serde_json::from_value(serde_json::json!({
                "schema": "elon.compute_plugin.candidate_cleanup_execution_plan.v1",
                "cleanup_id": "cca_intent_test",
                "candidate_token_digest": "a".repeat(64),
                "authorization_receipt_digest": "4".repeat(64),
                "installation_id_digest": "5".repeat(64),
                "root_identity_digest": "6".repeat(64),
                "candidate_parent_anchor_relative_path": "compute-plugin/candidates",
                "candidate_parent_anchor_identity_digest": "3".repeat(64),
                "object_count": 1,
                "file_count": 0,
                "directory_count": 1,
                "expected_file_bytes": 0,
                "process_owner_epoch": 7,
                "planned_at_ms": 2_000,
                "object_digests": [object_digest]
            }))
            .unwrap();
        let plan_digest = jcs_sha256_hex(&plan).unwrap();
        restore_hashed_execution_plan(plan, vec![object], plan_digest).unwrap()
    }

    #[test]
    fn cleanup_delete_intent_is_deterministic_and_plan_anchored() {
        let first = build_initial_delete_intent(&plan(), 2_001).unwrap();
        let second = build_initial_delete_intent(&plan(), 2_001).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.event().event_sequence(), 1);
        assert_eq!(first.event().step_ordinal(), 0);
        assert_eq!(first.event().event_kind(), "delete_intent");
        assert_eq!(
            first.event().previous_event_digest(),
            first.event().plan_digest()
        );
    }

    #[test]
    fn cleanup_delete_intent_requires_time_after_plan_seal() {
        let error = build_initial_delete_intent(&plan(), 2_000).unwrap_err();

        assert!(error.to_string().contains("INTENT_BINDING_INVALID"));
    }
}
