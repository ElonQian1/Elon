use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Transaction};

use super::{
    meta::update_staging_authority_meta,
    persistence::insert_candidate_staging_receipt,
    projection::{project_candidate_staging, CandidateStagingProjection},
    ComputePluginPostRevalidationStagingAuthoritySession,
    HashedComputePluginCandidateStagingReceipt,
};
use crate::node_agent_compute_plugin_host::{
    candidate_staging_contract::ValidatedCandidateStagingStorePermit,
    candidate_verification_contract::ComputePluginCandidateVerificationOutcomeKind,
    install_plan_admission_validation::is_identifier, lifecycle::SLOT_VERIFYING,
    manifest_validation::is_sha256,
};

use super::super::{
    fetch_claim_revocation::revoke_for_verification_authority_epoch_advance,
    keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    plan_application::{read_authority_plan_application_state, AuthorityPlanApplicationState},
    verification_store::{
        read_verified_candidate_staging_snapshot, VerifiedCandidateStagingSnapshot,
    },
};

pub(super) fn persist_candidate_staging(
    transaction: &Transaction<'_>,
    session: &ComputePluginPostRevalidationStagingAuthoritySession<'_>,
    permit: ValidatedCandidateStagingStorePermit<'_>,
) -> Result<HashedComputePluginCandidateStagingReceipt> {
    permit.cancellation_guard().ensure_current()?;
    let snapshot = read_verified_candidate_staging_snapshot(
        transaction,
        session.process_fence,
        session.trusted_now.clone(),
        session.roots,
        permit.key(),
        permit.binding().verification_result_digest(),
    )?;
    validate_bound_snapshot(session, &permit, &snapshot)?;
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    validate_authority_alignment(&authority, &snapshot, &permit)?;
    let projection = project_candidate_staging(&authority, &snapshot, &session.trusted_now)?;
    let staged_at_ms = session.trusted_now.timestamp_millis();
    permit.cancellation_guard().ensure_current()?;

    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.state_revision != authority.state_revision
        || time_state.authority_epoch != authority.authority_epoch
        || time_state.trusted_time_high_water_ms != Some(authority.trusted_time_high_water_ms)
        || time_state.clock_status != "trusted"
        || staged_at_ms <= authority.trusted_time_high_water_ms
        || staged_at_ms <= permit.binding().verification_resolved_at_ms()
    {
        bail!("COMPUTE_PLUGIN_STAGING_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, staged_at_ms)?;
    permit.cancellation_guard().ensure_current()?;
    revoke_for_verification_authority_epoch_advance(
        transaction,
        authority.authority_epoch,
        projection.authority_epoch,
        staged_at_ms,
    )?;
    permit.cancellation_guard().ensure_current()?;
    update_staging_authority_meta(transaction, &authority, &projection, staged_at_ms)?;
    let receipt = insert_candidate_staging_receipt(
        transaction,
        &permit,
        &authority,
        &projection,
        staged_at_ms,
    )?;
    validate_readback(
        transaction,
        &authority,
        &projection,
        &permit,
        &session.trusted_now,
    )?;
    permit.cancellation_guard().ensure_current()?;
    Ok(receipt)
}

fn validate_bound_snapshot(
    session: &ComputePluginPostRevalidationStagingAuthoritySession<'_>,
    permit: &ValidatedCandidateStagingStorePermit<'_>,
    snapshot: &VerifiedCandidateStagingSnapshot,
) -> Result<()> {
    let key = permit.key();
    let binding = permit.binding();
    let outcome = &snapshot.outcome;
    let current = &snapshot.current;
    let plan = permit.plan();
    let evidence = permit.evidence();
    let seal = permit.seal();
    let expected_state_revision = key
        .authority_state_revision()
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_STATE_FENCE_EXHAUSTED"))?;
    let expected_inventory_revision = key
        .execution_inventory_revision()
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_INVENTORY_FENCE_EXHAUSTED"))?;
    let expected_authority_epoch = key
        .authority_epoch()
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_EPOCH_FENCE_EXHAUSTED"))?;
    let file_count = i64::try_from(evidence.evidence.files.len())
        .context("COMPUTE_PLUGIN_STAGING_EXTRACTED_FILE_COUNT")?;
    if !is_identifier(permit.staging_id())
        || outcome.kind() != ComputePluginCandidateVerificationOutcomeKind::Verified
        || outcome.resolution_reason() != Some("artifact_set_verified")
        || outcome.result_digest() != Some(binding.verification_result_digest())
        || outcome.resolved_at_ms() != Some(binding.verification_resolved_at_ms())
        || outcome.slot_phase_after() != Some(SLOT_VERIFYING)
        || binding.authority_state_revision() != expected_state_revision
        || binding.authority_state_revision() != current.authority_state_revision
        || binding.inventory_revision() != expected_inventory_revision
        || binding.inventory_revision() != current.execution_inventory_revision
        || binding.inventory_digest() != current.inventory_digest.as_str()
        || binding.authority_epoch() != expected_authority_epoch
        || binding.authority_epoch() != current.authority_epoch
        || binding.process_owner_epoch() != current.process_owner_epoch
        || binding.trusted_time_high_water_ms() != current.observed_trusted_time_high_water_ms
        || binding.candidate_token_digest() != current.candidate_token_digest.as_str()
        || binding.candidate_generation() != current.candidate_generation
        || binding.application_inventory_revision()
            != current.candidate_application_inventory_revision
        || binding.candidate_plugin_id() != current.candidate_plugin_id.as_str()
        || binding.candidate_slot_ref() != current.candidate_slot_ref.as_str()
        || binding.candidate_release() != &current.candidate_release
        || current.trusted_now.timestamp_millis() != session.trusted_now.timestamp_millis()
        || &plan.plan.release != &current.candidate_release
        || plan.plan.release.plugin_id.as_str() != current.candidate_plugin_id.as_str()
        || !is_sha256(&plan.plan_digest)
        || plan.plan_digest.as_str() != evidence.evidence.extraction_plan_digest.as_str()
        || evidence.evidence.installation_id_digest.as_str() != session.installation_id_digest()
        || evidence.evidence.root_identity_digest.as_str() != key.root_identity_digest()
        || evidence.evidence.candidate_token_digest.as_str() != key.candidate_token_digest()
        || evidence.evidence.staging_run_digest.as_str() != seal.payload.staging_run_digest.as_str()
        || evidence.evidence.extraction_plan_digest.as_str()
            != seal.payload.extraction_plan_digest.as_str()
        || evidence.evidence_digest.as_str() != seal.payload.extraction_evidence_digest.as_str()
        || evidence.evidence.extracted_file_count != file_count
        || evidence.evidence.extracted_file_count != seal.payload.extracted_file_count
        || evidence.evidence.extracted_bytes != plan.plan.unpacked_size_bytes
        || evidence.evidence.extracted_bytes != seal.payload.extracted_bytes
        || evidence.evidence.root_identity_digest.as_str()
            != seal.payload.root_identity_digest.as_str()
        || evidence.evidence.candidate_token_digest.as_str()
            != seal.payload.candidate_token_digest.as_str()
        || !is_sha256(&evidence.evidence_digest)
        || !is_sha256(&seal.payload_digest)
        || !is_sha256(&seal.file_digest)
        || !is_sha256(&seal.file_identity_digest)
        || seal.size_bytes <= 0
    {
        bail!("COMPUTE_PLUGIN_STAGING_BOUND_CONTENT_CHANGED");
    }
    session.validate_source(permit.cancellation_guard())
}

fn validate_authority_alignment(
    authority: &AuthorityPlanApplicationState,
    snapshot: &VerifiedCandidateStagingSnapshot,
    permit: &ValidatedCandidateStagingStorePermit<'_>,
) -> Result<()> {
    let current = &snapshot.current;
    let binding = permit.binding();
    if authority.installation_id_digest != current.installation_id_digest
        || authority.state_revision != binding.authority_state_revision()
        || authority.inventory != current.inventory
        || authority.inventory.inventory_revision != binding.inventory_revision()
        || authority.inventory_digest.as_str() != binding.inventory_digest()
        || authority.authority_epoch != binding.authority_epoch()
        || authority.process_owner_epoch != binding.process_owner_epoch()
        || authority.trusted_time_high_water_ms != binding.trusted_time_high_water_ms()
    {
        bail!("COMPUTE_PLUGIN_STAGING_AUTHORITY_CHANGED");
    }
    Ok(())
}

fn validate_readback(
    transaction: &Transaction<'_>,
    authority_before: &AuthorityPlanApplicationState,
    projection: &CandidateStagingProjection,
    permit: &ValidatedCandidateStagingStorePermit<'_>,
    trusted_now: &DateTime<Utc>,
) -> Result<()> {
    let authority_after = read_authority_plan_application_state(transaction, trusted_now)?;
    let staged_at_ms = trusted_now.timestamp_millis();
    let prepared = transaction
        .query_row(
            "SELECT (SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared') + (SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_STAGING_PREPARED_READBACK")?;
    let verified = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_verification_runs
            WHERE verification_id = ?1 AND candidate_token = ?2
              AND state = 'verified' AND result_digest = ?3"#,
            params![
                permit.key().verification_id(),
                permit.key().candidate_token(),
                permit.binding().verification_result_digest(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_STAGING_VERIFIED_READBACK")?;
    if authority_after.installation_id_digest != authority_before.installation_id_digest
        || authority_after.state_revision != projection.state_revision
        || authority_after.inventory != projection.inventory
        || authority_after.inventory_digest != projection.inventory_digest
        || authority_after.authority_epoch != projection.authority_epoch
        || authority_after.process_owner_epoch != authority_before.process_owner_epoch
        || authority_after.trusted_time_high_water_ms != staged_at_ms
        || prepared != 0
        || verified != 1
    {
        bail!("COMPUTE_PLUGIN_STAGING_READBACK_CHANGED");
    }
    Ok(())
}
