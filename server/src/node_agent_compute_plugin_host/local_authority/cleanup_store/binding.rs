use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{
    ComputePluginCandidateCleanupAuthorityFacts, ComputePluginCandidateCleanupAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::{
        validate_hashed_candidate_health_failure_observation, DurableCandidateHealthQuarantine,
    },
    identity::ComputePluginReleaseRef,
    lifecycle::{ComputePluginInventorySnapshot, SLOT_FAILED},
    local_authority::plan_application::read_authority_plan_application_state,
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn read_candidate_cleanup_binding(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupAuthoritySession<'_>,
    quarantined: &DurableCandidateHealthQuarantine<'_>,
) -> Result<ComputePluginCandidateCleanupAuthorityFacts> {
    let quarantine = quarantined.receipt();
    let receipt = quarantine.receipt();
    let staged = quarantined.staged();
    let staging = staged.recovery_key();
    let slot = staging.slot_expectation();
    validate_hashed_candidate_health_failure_observation(quarantine.observation())?;
    if receipt.slot_phase_after() != SLOT_FAILED
        || !is_sha256(quarantine.receipt_digest())
        || jcs_sha256_hex(receipt)? != quarantine.receipt_digest()
        || receipt.failure_observation_digest() != quarantine.observation().observation_digest
        || receipt.candidate_token_digest() != staging.candidate_token_digest()
        || receipt.staging_id() != staging.staging_id()
        || receipt.staging_receipt_digest() != staged.receipt().receipt_digest()
        || receipt.staging_run_digest() != staging.staging_run_digest()
        || receipt.process_owner_epoch() != session.process_owner_epoch()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_QUARANTINE_CHANGED");
    }

    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_owner_epoch()
        || authority.state_revision < receipt.authority_state_revision_after()
        || authority.inventory.inventory_revision < receipt.inventory_revision_after()
        || authority.authority_epoch < receipt.authority_epoch_after()
        || authority.trusted_time_high_water_ms < receipt.failed_at_ms()
        || session.trusted_now_ms() <= authority.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_AUTHORITY_CHANGED");
    }
    validate_failed_candidate_inventory(
        &authority.inventory,
        &slot.plugin_id,
        &slot.slot_ref,
        &slot.release,
    )?;
    validate_quarantine_row(transaction, quarantined)?;
    validate_owner_and_idle_state(transaction, staging.candidate_token())?;

    Ok(ComputePluginCandidateCleanupAuthorityFacts {
        authority_state_revision_before: authority.state_revision,
        authority_state_revision_after: authority
            .state_revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_STATE_EXHAUSTED"))?,
        inventory_revision: authority.inventory.inventory_revision,
        inventory_digest: authority.inventory_digest,
        authority_epoch_before: authority.authority_epoch,
        authority_epoch_after: authority
            .authority_epoch
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_EPOCH_EXHAUSTED"))?,
        process_owner_epoch: authority.process_owner_epoch,
        trusted_time_high_water_ms_before: authority.trusted_time_high_water_ms,
        authorized_at_ms: session.trusted_now_ms(),
        candidate_token_digest: staging.candidate_token_digest().to_string(),
        quarantine_id: receipt.quarantine_id().to_string(),
        quarantine_receipt_digest: quarantine.receipt_digest().to_string(),
        staging_id: staging.staging_id().to_string(),
        staging_run_digest: staging.staging_run_digest().to_string(),
    })
}

pub(super) fn validate_failed_candidate_inventory(
    inventory: &ComputePluginInventorySnapshot,
    plugin_id: &str,
    slot_ref: &str,
    release: &ComputePluginReleaseRef,
) -> Result<()> {
    let plugin = inventory
        .plugins
        .iter()
        .find(|plugin| plugin.plugin_id == plugin_id)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PLUGIN_MISSING"))?;
    let slot = plugin
        .slots
        .iter()
        .find(|slot| slot.slot_ref == slot_ref)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_SLOT_MISSING"))?;
    if plugin.candidate_slot_ref.as_deref() != Some(slot_ref)
        || slot.phase != SLOT_FAILED
        || &slot.release != release
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_SLOT_CHANGED");
    }
    Ok(())
}

fn validate_quarantine_row(
    transaction: &Transaction<'_>,
    quarantined: &DurableCandidateHealthQuarantine<'_>,
) -> Result<()> {
    let quarantine = quarantined.receipt();
    let receipt = quarantine.receipt();
    let staging = quarantined.staged().recovery_key();
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_QUARANTINE_SERIALIZE")?;
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_health_quarantine_receipts
            WHERE quarantine_id = ?1 AND candidate_token = ?2
              AND candidate_token_digest = ?3 AND staging_id = ?4
              AND staging_receipt_digest = ?5 AND staging_run_digest = ?6
              AND failure_observation_digest = ?7
              AND authority_state_revision_after = ?8
              AND inventory_revision_after = ?9 AND inventory_digest_after = ?10
              AND authority_epoch_after = ?11 AND process_owner_epoch = ?12
              AND failed_at_ms = ?13 AND slot_phase_after = 'failed'
              AND receipt_json = ?14 AND receipt_digest = ?15"#,
            params![
                receipt.quarantine_id(),
                staging.candidate_token(),
                receipt.candidate_token_digest(),
                receipt.staging_id(),
                receipt.staging_receipt_digest(),
                receipt.staging_run_digest(),
                receipt.failure_observation_digest(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision_after(),
                receipt.inventory_digest_after(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.failed_at_ms(),
                receipt_json,
                quarantine.receipt_digest(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_QUARANTINE_READ")?;
    if count != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_QUARANTINE_ROW_CHANGED");
    }
    Ok(())
}

fn validate_owner_and_idle_state(
    transaction: &Transaction<'_>,
    candidate_token: &str,
) -> Result<()> {
    let owner_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_owners
               WHERE candidate_token = ?1 AND state = 'owned'
                 AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
                 AND closed_by_plan_digest IS NULL AND close_reason IS NULL"#,
            params![candidate_token],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OWNER_READ")?;
    let conflicting_count = transaction
        .query_row(
            r#"SELECT
                (SELECT COUNT(*) FROM candidate_cleanup_authorizations
                 WHERE candidate_token = ?1)
              + (SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared')
              + (SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared')"#,
            params![candidate_token],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_CONFLICT_READ")?;
    if owner_count != 1 || conflicting_count != 0 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OWNER_CHANGED");
    }
    Ok(())
}
