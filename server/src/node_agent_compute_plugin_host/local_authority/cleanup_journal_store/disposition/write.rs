use anyhow::{bail, Result};
use rusqlite::Transaction;

use super::{
    validation::{validate_authority_and_owner, validate_unstored_disposition},
    ComputePluginCandidateCleanupDispositionAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        HashedComputePluginCandidateCleanupStepEvent, ValidatedCandidateCleanupDispositionPermit,
    },
    local_authority::{
        cleanup_journal_store::{validation::read_exact_step_event, write::insert_event},
        keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    },
};

pub(super) fn persist_candidate_cleanup_disposition(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupDispositionAuthoritySession<'_>,
    permit: ValidatedCandidateCleanupDispositionPermit<'_>,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    let physical = permit.physical();
    let event = permit.event();
    session.validate_source(physical.state().cancellation_guard())?;
    validate_unstored_disposition(transaction, session, physical, event)?;
    let intent_time = physical.intent_event().event().recorded_at_ms();
    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.trusted_time_high_water_ms != Some(intent_time)
        || time_state.clock_status != "trusted"
        || event.event().recorded_at_ms() <= intent_time
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, event.event().recorded_at_ms())?;
    session.validate_source(physical.state().cancellation_guard())?;
    insert_event(transaction, event)?;
    validate_authority_and_owner(
        transaction,
        session,
        physical,
        event.event().recorded_at_ms(),
    )?;
    let stored = read_exact_step_event(transaction, event)?.ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_READBACK_MISSING")
    })?;
    session.validate_source(physical.state().cancellation_guard())?;
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::*;
    use crate::node_agent_compute_plugin_host::{
        candidate_cleanup_contract::{
            restore_hashed_cleanup_step_event, ComputePluginCandidateCleanupStepEvent,
        },
        signed_artifact_verification::jcs_sha256_hex,
    };

    fn event() -> HashedComputePluginCandidateCleanupStepEvent {
        let event: ComputePluginCandidateCleanupStepEvent =
            serde_json::from_value(serde_json::json!({
                "schema": "elon.compute_plugin.candidate_cleanup_step_event.v1",
                "cleanup_id": "cca_disposition_store_test",
                "plan_digest": "1".repeat(64),
                "event_sequence": 2,
                "step_ordinal": 0,
                "event_kind": "exact_handle_disposition_set",
                "object_digest": "2".repeat(64),
                "observed_identity_digest": "3".repeat(64),
                "observed_parent_identity_digest": "4".repeat(64),
                "namespace_durability_kind": null,
                "namespace_durability_evidence_digest": null,
                "previous_event_digest": "5".repeat(64),
                "process_owner_epoch": 7,
                "recorded_at_ms": 2_002
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
    fn cleanup_disposition_store_round_trips_exact_event() {
        let mut connection = connection();
        let transaction = connection.transaction().unwrap();
        let event = event();
        insert_event(&transaction, &event).unwrap();

        let stored = read_exact_step_event(&transaction, &event)
            .unwrap()
            .unwrap();

        assert_eq!(stored, event);
    }

    #[test]
    fn cleanup_disposition_store_rejects_changed_event_column() {
        let mut connection = connection();
        let transaction = connection.transaction().unwrap();
        let event = event();
        insert_event(&transaction, &event).unwrap();
        transaction
            .execute(
                "UPDATE candidate_cleanup_step_events SET previous_event_digest = ?1",
                params!["9".repeat(64)],
            )
            .unwrap();

        let error = read_exact_step_event(&transaction, &event).unwrap_err();

        assert!(error.to_string().contains("STEP_EVENT_ROW_CHANGED"));
    }
}
