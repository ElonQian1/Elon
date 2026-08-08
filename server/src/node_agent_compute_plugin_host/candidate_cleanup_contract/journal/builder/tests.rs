use super::*;
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        restore_hashed_execution_plan, restore_hashed_expected_object,
        ComputePluginCandidateCleanupExecutionPlan, ComputePluginCandidateCleanupExpectedObject,
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

#[test]
fn cleanup_disposition_is_deterministic_and_chains_exact_intent() {
    let plan = plan();
    let intent = build_initial_delete_intent(&plan, 2_001).unwrap();
    let first = build_exact_handle_disposition_event_from_fields(
        &plan,
        &intent,
        true,
        plan.objects()[0].object().relative_name(),
        plan.objects()[0].object().expected_identity_digest(),
        plan.objects()[0].object().expected_parent_identity_digest(),
        2_002,
    )
    .unwrap();
    let second = build_exact_handle_disposition_event_from_fields(
        &plan,
        &intent,
        true,
        plan.objects()[0].object().relative_name(),
        plan.objects()[0].object().expected_identity_digest(),
        plan.objects()[0].object().expected_parent_identity_digest(),
        2_002,
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.event().event_sequence(), 2);
    assert_eq!(first.event().event_kind(), "exact_handle_disposition_set");
    assert_eq!(first.event().previous_event_digest(), intent.event_digest());
}

#[test]
fn cleanup_disposition_rejects_stale_time_or_changed_binding() {
    let plan = plan();
    let intent = build_initial_delete_intent(&plan, 2_001).unwrap();
    let stale = build_exact_handle_disposition_event_from_fields(
        &plan,
        &intent,
        true,
        plan.objects()[0].object().relative_name(),
        plan.objects()[0].object().expected_identity_digest(),
        plan.objects()[0].object().expected_parent_identity_digest(),
        2_001,
    )
    .unwrap_err();
    let changed = build_exact_handle_disposition_event_from_fields(
        &plan,
        &intent,
        false,
        plan.objects()[0].object().relative_name(),
        plan.objects()[0].object().expected_identity_digest(),
        plan.objects()[0].object().expected_parent_identity_digest(),
        2_002,
    )
    .unwrap_err();

    assert!(stale.to_string().contains("DISPOSITION_BINDING_INVALID"));
    assert!(changed.to_string().contains("DISPOSITION_BINDING_INVALID"));
}

fn disposition_chain() -> (
    HashedComputePluginCandidateCleanupExecutionPlan,
    HashedComputePluginCandidateCleanupStepEvent,
    HashedComputePluginCandidateCleanupStepEvent,
) {
    let plan = plan();
    let intent = build_initial_delete_intent(&plan, 2_001).unwrap();
    let disposition = build_exact_handle_disposition_event_from_fields(
        &plan,
        &intent,
        true,
        plan.objects()[0].object().relative_name(),
        plan.objects()[0].object().expected_identity_digest(),
        plan.objects()[0].object().expected_parent_identity_digest(),
        2_002,
    )
    .unwrap();
    (plan, intent, disposition)
}

fn absence_chain() -> (
    HashedComputePluginCandidateCleanupExecutionPlan,
    HashedComputePluginCandidateCleanupStepEvent,
    HashedComputePluginCandidateCleanupStepEvent,
    HashedComputePluginCandidateCleanupStepEvent,
) {
    let (plan, intent, disposition) = disposition_chain();
    let absence = build_parent_namespace_absence_event_from_fields(
        &plan,
        &intent,
        &disposition,
        true,
        plan.objects()[0].object().relative_name(),
        plan.objects()[0].object().expected_identity_digest(),
        plan.objects()[0].object().expected_parent_identity_digest(),
        2_003,
    )
    .unwrap();
    (plan, intent, disposition, absence)
}

#[test]
fn cleanup_parent_absence_is_deterministic_and_chains_exact_disposition() {
    let (plan, intent, disposition) = disposition_chain();
    let build = || {
        build_parent_namespace_absence_event_from_fields(
            &plan,
            &intent,
            &disposition,
            true,
            plan.objects()[0].object().relative_name(),
            plan.objects()[0].object().expected_identity_digest(),
            plan.objects()[0].object().expected_parent_identity_digest(),
            2_003,
        )
        .unwrap()
    };

    let first = build();
    let second = build();

    assert_eq!(first, second);
    assert_eq!(first.event().event_sequence(), 3);
    assert_eq!(
        first.event().event_kind(),
        "parent_namespace_absence_observed"
    );
    assert_eq!(first.event().observed_identity_digest(), None);
    assert_eq!(
        first.event().previous_event_digest(),
        disposition.event_digest()
    );
}

#[test]
fn cleanup_parent_absence_rejects_stale_time_or_changed_binding() {
    let (plan, intent, disposition) = disposition_chain();
    let stale = build_parent_namespace_absence_event_from_fields(
        &plan,
        &intent,
        &disposition,
        true,
        plan.objects()[0].object().relative_name(),
        plan.objects()[0].object().expected_identity_digest(),
        plan.objects()[0].object().expected_parent_identity_digest(),
        2_002,
    )
    .unwrap_err();
    let changed = build_parent_namespace_absence_event_from_fields(
        &plan,
        &intent,
        &disposition,
        false,
        plan.objects()[0].object().relative_name(),
        plan.objects()[0].object().expected_identity_digest(),
        plan.objects()[0].object().expected_parent_identity_digest(),
        2_003,
    )
    .unwrap_err();

    assert!(stale.to_string().contains("ABSENCE_BINDING_INVALID"));
    assert!(changed.to_string().contains("DISPOSITION_BINDING_INVALID"));
}

#[test]
fn cleanup_namespace_durable_is_deterministic_and_chains_exact_absence() {
    let (plan, intent, disposition, absence) = absence_chain();
    let build = || {
        build_namespace_durable_event_from_fields(
            &plan,
            &intent,
            &disposition,
            &absence,
            true,
            plan.objects()[0].object().relative_name(),
            plan.objects()[0].object().expected_identity_digest(),
            plan.objects()[0].object().expected_parent_identity_digest(),
            CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_KIND,
            "ntfs",
            2_004,
        )
        .unwrap()
    };

    let first = build();
    let second = build();

    assert_eq!(first, second);
    assert_eq!(first.event().event_sequence(), 4);
    assert_eq!(first.event().event_kind(), "namespace_durable");
    assert_eq!(first.event().observed_identity_digest(), None);
    assert_eq!(
        first.event().namespace_durability_kind(),
        Some(CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_KIND)
    );
    assert_eq!(
        first.event().previous_event_digest(),
        absence.event_digest()
    );
    assert_eq!(
        first
            .event()
            .namespace_durability_evidence_digest()
            .map(str::len),
        Some(64)
    );
}

#[test]
fn cleanup_namespace_durable_rejects_stale_time_or_unsupported_filesystem() {
    let (plan, intent, disposition, absence) = absence_chain();
    let build = |filesystem_kind, recorded_at_ms| {
        build_namespace_durable_event_from_fields(
            &plan,
            &intent,
            &disposition,
            &absence,
            true,
            plan.objects()[0].object().relative_name(),
            plan.objects()[0].object().expected_identity_digest(),
            plan.objects()[0].object().expected_parent_identity_digest(),
            CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_KIND,
            filesystem_kind,
            recorded_at_ms,
        )
    };

    let stale = build("ntfs", 2_003).unwrap_err();
    let unsupported = build("ext4", 2_004).unwrap_err();

    assert!(stale.to_string().contains("DURABILITY_BINDING_INVALID"));
    assert!(unsupported
        .to_string()
        .contains("DURABILITY_EVIDENCE_BINDING_INVALID"));
}
