use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::ComputePluginCandidateCleanupDeleteIntentAuthoritySession;
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        restore_hashed_cleanup_step_event, validate_hashed_cleanup_step_event,
        ComputePluginCandidateCleanupStepEvent, HashedComputePluginCandidateCleanupStepEvent,
        SealedCandidateCleanupTopology,
    },
    local_authority::{
        cleanup_topology_store::read_exact_sealed_plan,
        plan_application::read_authority_plan_application_state,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn validate_unstored_intent(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupDeleteIntentAuthoritySession<'_>,
    sealed: &SealedCandidateCleanupTopology,
    event: &HashedComputePluginCandidateCleanupStepEvent,
) -> Result<()> {
    validate_hashed_cleanup_step_event(event)?;
    let plan = sealed.plan();
    let first_object = plan
        .objects()
        .first()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_OBJECT_MISSING"))?;
    if event.event().cleanup_id() != plan.plan().cleanup_id()
        || event.event().plan_digest() != plan.plan_digest()
        || event.event().event_sequence() != 1
        || event.event().step_ordinal() != 0
        || event.event().event_kind() != "delete_intent"
        || event.event().object_digest() != first_object.object_digest()
        || event.event().observed_identity_digest()
            != Some(first_object.object().expected_identity_digest())
        || event.event().observed_parent_identity_digest()
            != first_object.object().expected_parent_identity_digest()
        || event.event().previous_event_digest() != plan.plan_digest()
        || event.event().process_owner_epoch() != session.process_owner_epoch()
        || event.event().recorded_at_ms() != session.trusted_now_ms()
        || event.event().recorded_at_ms() <= plan.plan().planned_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_BINDING_CHANGED");
    }
    validate_authority_and_owner(transaction, session, sealed, plan.plan().planned_at_ms())?;
    let stored = read_exact_sealed_plan(
        transaction,
        plan,
        sealed.state().staging_recovery_key().candidate_token(),
    )?;
    if stored.as_ref() != Some(plan)
        || count_event_identity_matches(transaction, event)? != 0
        || count_events(transaction, plan.plan().cleanup_id())? != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_ALREADY_EXISTS");
    }
    Ok(())
}

pub(super) fn validate_authority_and_owner(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupDeleteIntentAuthoritySession<'_>,
    sealed: &SealedCandidateCleanupTopology,
    expected_high_water_ms: i64,
) -> Result<()> {
    let state = sealed.state();
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let recovery = state.staging_recovery_key();
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_owner_epoch()
        || authority.state_revision != receipt.authority_state_revision_after()
        || authority.inventory.inventory_revision != receipt.inventory_revision()
        || authority.inventory_digest != receipt.inventory_digest()
        || authority.authority_epoch != receipt.authority_epoch_after()
        || authority.trusted_time_high_water_ms != expected_high_water_ms
        || count_exact_authorization(transaction, sealed)? != 1
        || count_pending_owner(transaction, recovery.candidate_token())? != 1
        || count_completion(transaction, recovery.candidate_token())? != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_AUTHORITY_CHANGED");
    }
    Ok(())
}

pub(super) fn read_exact_step_event(
    transaction: &Transaction<'_>,
    expected: &HashedComputePluginCandidateCleanupStepEvent,
) -> Result<Option<HashedComputePluginCandidateCleanupStepEvent>> {
    let expected_event = expected.event();
    let row = transaction
        .query_row(
            r#"SELECT plan_digest, step_ordinal, event_kind, object_digest,
                      observed_identity_digest, observed_parent_identity_digest,
                      namespace_durability_kind, namespace_durability_evidence_digest,
                      previous_event_digest, process_owner_epoch, recorded_at_ms,
                      event_json, event_digest
               FROM candidate_cleanup_step_events
               WHERE cleanup_id = ?1 AND event_sequence = ?2"#,
            params![expected_event.cleanup_id(), expected_event.event_sequence()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_READ")?;
    let Some(row) = row else { return Ok(None) };
    let event: ComputePluginCandidateCleanupStepEvent =
        serde_json::from_str(&row.11).context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_DECODE")?;
    if row.0 != event.plan_digest()
        || row.1 != event.step_ordinal()
        || row.2 != event.event_kind()
        || row.3 != event.object_digest()
        || row.4.as_deref() != event.observed_identity_digest()
        || row.5 != event.observed_parent_identity_digest()
        || row.6.as_deref() != event.namespace_durability_kind()
        || row.7.as_deref() != event.namespace_durability_evidence_digest()
        || row.8 != event.previous_event_digest()
        || row.9 != event.process_owner_epoch()
        || row.10 != event.recorded_at_ms()
        || event.cleanup_id() != expected_event.cleanup_id()
        || event.event_sequence() != expected_event.event_sequence()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_ROW_CHANGED");
    }
    let restored = restore_hashed_cleanup_step_event(event, row.12)?;
    if &restored != expected {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_READBACK_CHANGED");
    }
    Ok(Some(restored))
}

pub(super) fn count_event_identity_matches(
    transaction: &Transaction<'_>,
    event: &HashedComputePluginCandidateCleanupStepEvent,
) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_step_events
               WHERE (cleanup_id = ?1 AND event_sequence = ?2) OR event_digest = ?3"#,
            params![
                event.event().cleanup_id(),
                event.event().event_sequence(),
                event.event_digest()
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_IDENTITY_READ")
}

pub(super) fn count_events(transaction: &Transaction<'_>, cleanup_id: &str) -> Result<i64> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_cleanup_step_events WHERE cleanup_id = ?1",
            params![cleanup_id],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STEP_EVENT_COUNT")
}

fn count_exact_authorization(
    transaction: &Transaction<'_>,
    sealed: &SealedCandidateCleanupTopology,
) -> Result<i64> {
    let state = sealed.state();
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_AUTHORIZATION_SERIALIZE")?;
    if jcs_sha256_hex(receipt)? != authorization.receipt_digest() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_AUTHORIZATION_CHANGED");
    }
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_authorizations
               WHERE cleanup_id = ?1 AND candidate_token = ?2
                 AND candidate_token_digest = ?3 AND receipt_json = ?4 AND receipt_digest = ?5
                 AND process_owner_epoch = ?6 AND authorized_at_ms = ?7
                 AND slot_phase_before = 'failed'"#,
            params![
                receipt.cleanup_id(),
                state.staging_recovery_key().candidate_token(),
                receipt.candidate_token_digest(),
                receipt_json,
                authorization.receipt_digest(),
                receipt.process_owner_epoch(),
                receipt.authorized_at_ms()
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_AUTHORIZATION_READ")
}

fn count_pending_owner(transaction: &Transaction<'_>, candidate_token: &str) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_owners
               WHERE candidate_token = ?1 AND state = 'cleanup_pending'
                 AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
                 AND closed_by_plan_digest IS NULL AND close_reason IS NULL"#,
            params![candidate_token],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_OWNER_READ")
}

fn count_completion(transaction: &Transaction<'_>, candidate_token: &str) -> Result<i64> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_cleanup_completions WHERE candidate_token = ?1",
            params![candidate_token],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_INTENT_COMPLETION_READ")
}
