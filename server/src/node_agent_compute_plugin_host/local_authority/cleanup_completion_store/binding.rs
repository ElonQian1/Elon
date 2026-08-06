use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{
    projection::project_candidate_cleanup_completion, terminal::validate_terminal_journal,
    ComputePluginCandidateCleanupCompletionAuthorityFacts,
    ComputePluginCandidateCleanupCompletionAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        validate_hashed_execution_evidence, DurableCandidateCleanupTerminalJournal,
        PhysicallyExecutedCandidateCleanup,
    },
    lifecycle::SLOT_FAILED,
    local_authority::{
        cleanup_store::binding::validate_failed_candidate_inventory,
        plan_application::read_authority_plan_application_state,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn read_candidate_cleanup_completion_binding(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupCompletionAuthoritySession<'_>,
    terminal: &DurableCandidateCleanupTerminalJournal,
) -> Result<ComputePluginCandidateCleanupCompletionAuthorityFacts> {
    let physical = terminal.physical();
    session.validate_source(physical.cancellation_guard())?;
    validate_physical_chain(session, physical)?;
    validate_terminal_journal(transaction, terminal)?;

    let staging = physical.staging_recovery_key();
    let slot = staging.slot_expectation();
    let authorization = physical.authorization_receipt();
    let authorization_receipt = authorization.receipt();
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_owner_epoch()
        || authority.state_revision < authorization_receipt.authority_state_revision_after()
        || authority.inventory.inventory_revision < authorization_receipt.inventory_revision()
        || authority.authority_epoch < authorization_receipt.authority_epoch_after()
        || authority.trusted_time_high_water_ms < authorization_receipt.authorized_at_ms()
        || session.trusted_now_ms() <= authority.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_AUTHORITY_CHANGED");
    }
    validate_failed_candidate_inventory(
        &authority.inventory,
        &slot.plugin_id,
        &slot.slot_ref,
        &slot.release,
    )?;
    validate_authorization_row(transaction, physical)?;
    validate_pending_owner_and_idle_state(transaction, physical)?;
    let projection = project_candidate_cleanup_completion(
        &authority,
        &slot.plugin_id,
        &slot.slot_ref,
        &slot.release,
        &session.trusted_now,
    )?;
    session.validate_source(physical.cancellation_guard())?;

    Ok(ComputePluginCandidateCleanupCompletionAuthorityFacts {
        authority_state_revision_before: authority.state_revision,
        authority_state_revision_after: projection.state_revision,
        inventory_revision_before: authority.inventory.inventory_revision,
        inventory_revision_after: projection.inventory.inventory_revision,
        inventory_digest_before: authority.inventory_digest,
        inventory_digest_after: projection.inventory_digest,
        inventory_json_after: projection.inventory_json,
        inventory_after: projection.inventory,
        authority_epoch_before: authority.authority_epoch,
        authority_epoch_after: projection.authority_epoch,
        process_owner_epoch: authority.process_owner_epoch,
        trusted_time_high_water_ms_before: authority.trusted_time_high_water_ms,
        completed_at_ms: session.trusted_now_ms(),
        candidate_token_digest: staging.candidate_token_digest().to_string(),
        cleanup_id: authorization_receipt.cleanup_id().to_string(),
        authorization_receipt_digest: authorization.receipt_digest().to_string(),
        execution_plan_digest: terminal.execution_plan_digest().to_string(),
        execution_evidence_digest: physical.evidence().evidence_digest.clone(),
        terminal_journal_digest: terminal.terminal_journal_digest().to_string(),
    })
}

fn validate_physical_chain(
    session: &ComputePluginCandidateCleanupCompletionAuthoritySession<'_>,
    physical: &PhysicallyExecutedCandidateCleanup,
) -> Result<()> {
    let staging_key = physical.staging_recovery_key();
    let staging = physical.staging_receipt();
    let quarantine = physical.quarantine_receipt();
    let authorization = physical.authorization_receipt();
    let evidence = physical.evidence();
    let authorization_receipt = authorization.receipt();
    let quarantine_receipt = quarantine.receipt();
    let staging_receipt = staging.receipt();
    validate_hashed_execution_evidence(evidence)?;
    if !staging_key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || staging_key.installation_id_digest() != session.installation_id_digest()
        || staging_key.clock_epoch_digest() != session.clock_epoch_digest()
        || staging_key.process_owner_epoch() != session.process_owner_epoch()
        || session.observed_at() <= physical.physical_completed_at()
        || !is_sha256(staging.receipt_digest())
        || jcs_sha256_hex(staging_receipt)? != staging.receipt_digest()
        || !is_sha256(quarantine.receipt_digest())
        || jcs_sha256_hex(quarantine_receipt)? != quarantine.receipt_digest()
        || !is_sha256(authorization.receipt_digest())
        || jcs_sha256_hex(authorization_receipt)? != authorization.receipt_digest()
        || staging_receipt.staging_id() != staging_key.staging_id()
        || staging_receipt.candidate_token_digest() != staging_key.candidate_token_digest()
        || staging_receipt.staging_run_digest() != staging_key.staging_run_digest()
        || quarantine_receipt.staging_id() != staging_key.staging_id()
        || quarantine_receipt.staging_receipt_digest() != staging.receipt_digest()
        || quarantine_receipt.staging_run_digest() != staging_key.staging_run_digest()
        || quarantine_receipt.candidate_token_digest() != staging_key.candidate_token_digest()
        || quarantine_receipt.slot_phase_after() != SLOT_FAILED
        || authorization_receipt.cleanup_id() != evidence.evidence.cleanup_id
        || authorization_receipt.candidate_token_digest() != staging_key.candidate_token_digest()
        || authorization_receipt.quarantine_id() != quarantine_receipt.quarantine_id()
        || authorization_receipt.quarantine_receipt_digest() != quarantine.receipt_digest()
        || authorization_receipt.staging_id() != staging_key.staging_id()
        || authorization_receipt.staging_run_digest() != staging_key.staging_run_digest()
        || authorization_receipt.process_owner_epoch() != session.process_owner_epoch()
        || authorization_receipt.slot_phase_before() != SLOT_FAILED
        || evidence.evidence.cleanup_authorization_receipt_digest != authorization.receipt_digest()
        || evidence.evidence.candidate_token_digest != staging_key.candidate_token_digest()
        || evidence.evidence.quarantine_receipt_digest != quarantine.receipt_digest()
        || evidence.evidence.staging_receipt_digest != staging.receipt_digest()
        || evidence.evidence.extraction_evidence_digest
            != staging_key.receipt_expectation().extraction_evidence_digest
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_PHYSICAL_CHAIN_CHANGED");
    }
    Ok(())
}

fn validate_authorization_row(
    transaction: &Transaction<'_>,
    physical: &PhysicallyExecutedCandidateCleanup,
) -> Result<()> {
    let staging = physical.staging_recovery_key();
    let authorization = physical.authorization_receipt();
    let receipt = authorization.receipt();
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_AUTHORIZATION_SERIALIZE")?;
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_authorizations
               WHERE cleanup_id = ?1 AND candidate_token = ?2
                 AND candidate_token_digest = ?3 AND quarantine_id = ?4
                 AND quarantine_receipt_digest = ?5 AND staging_id = ?6
                 AND staging_run_digest = ?7
                 AND authority_state_revision_after = ?8
                 AND inventory_revision = ?9 AND inventory_digest = ?10
                 AND authority_epoch_after = ?11 AND process_owner_epoch = ?12
                 AND authorized_at_ms = ?13 AND slot_phase_before = 'failed'
                 AND receipt_json = ?14 AND receipt_digest = ?15"#,
            params![
                receipt.cleanup_id(),
                staging.candidate_token(),
                receipt.candidate_token_digest(),
                receipt.quarantine_id(),
                receipt.quarantine_receipt_digest(),
                receipt.staging_id(),
                receipt.staging_run_digest(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision(),
                receipt.inventory_digest(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.authorized_at_ms(),
                receipt_json,
                authorization.receipt_digest(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_AUTHORIZATION_READ")?;
    if count != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_AUTHORIZATION_CHANGED");
    }
    Ok(())
}

fn validate_pending_owner_and_idle_state(
    transaction: &Transaction<'_>,
    physical: &PhysicallyExecutedCandidateCleanup,
) -> Result<()> {
    let staging = physical.staging_recovery_key();
    let slot = staging.slot_expectation();
    let expected = staging.receipt_expectation();
    let release_json = serde_json::to_string(&slot.release)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_OWNER_RELEASE_SERIALIZE")?;
    let owner_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_owners
               WHERE candidate_token = ?1 AND plugin_id = ?2 AND slot_ref = ?3
                 AND candidate_generation = ?4 AND release_json = ?5
                 AND owner_plan_id = ?6 AND owner_plan_digest = ?7
                 AND application_inventory_revision = ?8 AND state = 'cleanup_pending'
                 AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
                 AND closed_by_plan_digest IS NULL AND close_reason IS NULL"#,
            params![
                staging.candidate_token(),
                slot.plugin_id.as_str(),
                slot.slot_ref.as_str(),
                expected.candidate_generation,
                release_json,
                expected.owner_plan_id.as_str(),
                expected.owner_plan_digest.as_str(),
                expected.application_inventory_revision,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_OWNER_READ")?;
    let conflicting_count = transaction
        .query_row(
            r#"SELECT
                (SELECT COUNT(*) FROM candidate_cleanup_completions
                 WHERE candidate_token = ?1)
              + (SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared')
              + (SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared')"#,
            params![staging.candidate_token()],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_CONFLICT_READ")?;
    if owner_count != 1 || conflicting_count != 0 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_OWNER_CHANGED");
    }
    Ok(())
}
