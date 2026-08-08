use rusqlite::{params, Connection};

use super::*;
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        restore_hashed_cleanup_step_event, ComputePluginCandidateCleanupStepEvent,
    },
    local_authority::cleanup_journal_store::{
        validation::read_exact_step_event, write::insert_event,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

fn event() -> HashedComputePluginCandidateCleanupStepEvent {
    let event: ComputePluginCandidateCleanupStepEvent = serde_json::from_value(serde_json::json!({
        "schema": "elon.compute_plugin.candidate_cleanup_step_event.v1",
        "cleanup_id": "cca_parent_absence_store_test",
        "plan_digest": "1".repeat(64),
        "event_sequence": 3,
        "step_ordinal": 0,
        "event_kind": "parent_namespace_absence_observed",
        "object_digest": "2".repeat(64),
        "observed_identity_digest": null,
        "observed_parent_identity_digest": "4".repeat(64),
        "namespace_durability_kind": null,
        "namespace_durability_evidence_digest": null,
        "previous_event_digest": "5".repeat(64),
        "process_owner_epoch": 7,
        "recorded_at_ms": 2_003
    }))
    .unwrap();
    let event_digest = jcs_sha256_hex(&event).unwrap();
    restore_hashed_cleanup_step_event(event, event_digest).unwrap()
}

fn connection() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE candidate_cleanup_step_events (
                cleanup_id TEXT, plan_digest TEXT, event_sequence INTEGER,
                step_ordinal INTEGER, event_kind TEXT, object_digest TEXT,
                observed_identity_digest TEXT, observed_parent_identity_digest TEXT,
                namespace_durability_kind TEXT,
                namespace_durability_evidence_digest TEXT,
                previous_event_digest TEXT, process_owner_epoch INTEGER,
                recorded_at_ms INTEGER, event_json TEXT, event_digest TEXT
            );
            "#,
        )
        .unwrap();
    connection
}

#[test]
fn cleanup_parent_absence_store_round_trips_exact_event() {
    let mut connection = connection();
    let transaction = connection.transaction().unwrap();
    let event = event();
    insert_event(&transaction, &event).unwrap();

    let stored = read_exact_step_event(&transaction, &event)
        .unwrap()
        .unwrap();

    assert_eq!(stored, event);
    assert_eq!(stored.event().observed_identity_digest(), None);
}

#[test]
fn cleanup_parent_absence_store_rejects_observed_identity_tamper() {
    let mut connection = connection();
    let transaction = connection.transaction().unwrap();
    let event = event();
    insert_event(&transaction, &event).unwrap();
    transaction
        .execute(
            "UPDATE candidate_cleanup_step_events SET observed_identity_digest = ?1",
            params!["9".repeat(64)],
        )
        .unwrap();

    let error = read_exact_step_event(&transaction, &event).unwrap_err();

    assert!(error.to_string().contains("STEP_EVENT_ROW_CHANGED"));
}
