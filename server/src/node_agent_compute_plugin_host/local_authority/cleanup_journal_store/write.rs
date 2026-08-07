use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{
    validation::{read_exact_step_event, validate_authority_and_owner, validate_unstored_intent},
    ComputePluginCandidateCleanupDeleteIntentAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        HashedComputePluginCandidateCleanupStepEvent, ValidatedCandidateCleanupDeleteIntentPermit,
    },
    local_authority::keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
};

pub(super) fn persist_candidate_cleanup_delete_intent(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupDeleteIntentAuthoritySession<'_>,
    permit: ValidatedCandidateCleanupDeleteIntentPermit<'_>,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    let sealed = permit.sealed();
    let event = permit.event();
    session.validate_source(sealed.state().cancellation_guard())?;
    validate_unstored_intent(transaction, session, sealed, event)?;
    let plan = sealed.plan();
    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.trusted_time_high_water_ms != Some(plan.plan().planned_at_ms())
        || time_state.clock_status != "trusted"
        || event.event().recorded_at_ms() <= plan.plan().planned_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, event.event().recorded_at_ms())?;
    session.validate_source(sealed.state().cancellation_guard())?;
    insert_event(transaction, event)?;
    validate_authority_and_owner(transaction, session, sealed, event.event().recorded_at_ms())?;
    let stored = read_exact_step_event(transaction, event)?.ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_READBACK_MISSING")
    })?;
    session.validate_source(sealed.state().cancellation_guard())?;
    Ok(stored)
}

pub(super) fn insert_event(
    transaction: &Transaction<'_>,
    hashed: &HashedComputePluginCandidateCleanupStepEvent,
) -> Result<()> {
    let event = hashed.event();
    let event_json = serde_json::to_string(event)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_SERIALIZE")?;
    transaction
        .execute(
            r#"INSERT INTO candidate_cleanup_step_events (
                   cleanup_id, plan_digest, event_sequence, step_ordinal, event_kind,
                   object_digest, observed_identity_digest,
                   observed_parent_identity_digest, namespace_durability_kind,
                   namespace_durability_evidence_digest, previous_event_digest,
                   process_owner_epoch, recorded_at_ms, event_json, event_digest
               ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"#,
            params![
                event.cleanup_id(),
                event.plan_digest(),
                event.event_sequence(),
                event.step_ordinal(),
                event.event_kind(),
                event.object_digest(),
                event.observed_identity_digest(),
                event.observed_parent_identity_digest(),
                event.namespace_durability_kind(),
                event.namespace_durability_evidence_digest(),
                event.previous_event_digest(),
                event.process_owner_epoch(),
                event.recorded_at_ms(),
                event_json,
                hashed.event_digest(),
            ],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_INSERT")?;
    Ok(())
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
                "cleanup_id": "cca_intent_store_test",
                "plan_digest": "1".repeat(64),
                "event_sequence": 1,
                "step_ordinal": 0,
                "event_kind": "delete_intent",
                "object_digest": "2".repeat(64),
                "observed_identity_digest": "3".repeat(64),
                "observed_parent_identity_digest": "4".repeat(64),
                "namespace_durability_kind": null,
                "namespace_durability_evidence_digest": null,
                "previous_event_digest": "1".repeat(64),
                "process_owner_epoch": 7,
                "recorded_at_ms": 2_001
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
    fn cleanup_delete_intent_store_round_trips_exact_event() {
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
    fn cleanup_delete_intent_store_rejects_changed_event_column() {
        let mut connection = connection();
        let transaction = connection.transaction().unwrap();
        let event = event();
        insert_event(&transaction, &event).unwrap();
        transaction
            .execute(
                "UPDATE candidate_cleanup_step_events SET observed_parent_identity_digest = ?1",
                params!["9".repeat(64)],
            )
            .unwrap();

        let error = read_exact_step_event(&transaction, &event).unwrap_err();

        assert!(error.to_string().contains("STEP_EVENT_ROW_CHANGED"));
    }
}
