use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Transaction;

use super::{outcome, read, ComputePluginCandidateVerificationAuthorityFacts};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::{
        ComputePluginCandidateVerificationOutcome, ComputePluginCandidateVerificationOutcomeKind,
        ComputePluginCandidateVerificationRecoveryKey,
    },
    keyring::ComputePluginBootstrapRootKeyResolver,
    lifecycle::SLOT_VERIFYING,
};

use super::super::ComputePluginFetchProcessFence;

pub(in crate::node_agent_compute_plugin_host::local_authority) struct VerifiedCandidateStagingSnapshot
{
    pub outcome: ComputePluginCandidateVerificationOutcome,
    pub current: ComputePluginCandidateVerificationAuthorityFacts,
}

pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_verified_candidate_staging_snapshot(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    expected_result_digest: &str,
) -> Result<VerifiedCandidateStagingSnapshot> {
    let outcome_snapshot = outcome::read_outcome_snapshot(transaction, process_fence, key)?;
    let resolved_at_ms = outcome_snapshot
        .outcome
        .resolved_at_ms()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_VERIFICATION_TIME_MISSING"))?;
    if outcome_snapshot.outcome.kind() != ComputePluginCandidateVerificationOutcomeKind::Verified
        || outcome_snapshot.outcome.resolution_reason() != Some("artifact_set_verified")
        || outcome_snapshot.outcome.result_digest() != Some(expected_result_digest)
        || outcome_snapshot.outcome.mismatch().is_some()
        || outcome_snapshot.outcome.slot_phase_after() != Some(SLOT_VERIFYING)
        || resolved_at_ms >= trusted_now.timestamp_millis()
    {
        bail!("COMPUTE_PLUGIN_STAGING_VERIFICATION_OUTCOME_CHANGED");
    }

    let current = read::read_fresh_verified_candidate_staging_authority(
        transaction,
        process_fence,
        trusted_now,
        roots,
        key,
        expected_result_digest,
    )?;
    if current.installation_id_digest != key.installation_id_digest()
        || current.applied_plan_id != key.owner_plan_id()
        || current.applied_plan_digest != key.owner_plan_digest()
        || current.candidate_owner_plan_id != key.owner_plan_id()
        || current.candidate_owner_plan_digest != key.owner_plan_digest()
        || current.candidate_token_digest != key.candidate_token_digest()
        || current.candidate_generation != key.candidate_generation()
        || current.candidate_application_inventory_revision != key.application_inventory_revision()
        || current.candidate_state != "owned"
        || current.next_verification_generation != key.verification_generation()
        || current.authority_state_revision
            != outcome_snapshot
                .outcome
                .authority_state_revision_after()
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_VERIFICATION_STATE_MISSING")
                })?
        || current.execution_inventory_revision
            != outcome_snapshot
                .outcome
                .inventory_revision_after()
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_VERIFICATION_INVENTORY_MISSING")
                })?
        || current.inventory_digest
            != outcome_snapshot
                .outcome
                .inventory_digest_after()
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_VERIFICATION_INVENTORY_MISSING")
                })?
        || current.authority_epoch
            != outcome_snapshot
                .outcome
                .authority_epoch_after()
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_VERIFICATION_EPOCH_MISSING")
                })?
        || current.process_owner_epoch != key.process_owner_epoch()
        || current.observed_trusted_time_high_water_ms < resolved_at_ms
        || current.observed_trusted_time_high_water_ms >= current.trusted_now.timestamp_millis()
    {
        bail!("COMPUTE_PLUGIN_STAGING_VERIFIED_AUTHORITY_CHANGED");
    }
    let existing = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_staging_receipts
            WHERE candidate_token = ?1 OR verification_id = ?2"#,
            [key.candidate_token(), key.verification_id()],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_STAGING_RECEIPT_EXISTENCE_READ")?;
    if existing != 0 {
        bail!("COMPUTE_PLUGIN_STAGING_RECEIPT_ALREADY_EXISTS");
    }
    Ok(VerifiedCandidateStagingSnapshot {
        outcome: outcome_snapshot.outcome,
        current,
    })
}
