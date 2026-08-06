use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{
    binding::read_candidate_health_quarantine_binding,
    meta::update_quarantine_authority_meta,
    projection::project_candidate_health_quarantine,
    types::{
        ComputePluginCandidateHealthQuarantineReceipt,
        HashedComputePluginCandidateHealthQuarantineReceipt,
        CANDIDATE_HEALTH_QUARANTINE_RECEIPT_CANONICALIZATION,
        CANDIDATE_HEALTH_QUARANTINE_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA,
        HASHED_CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA,
    },
    ComputePluginCandidateHealthQuarantineAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::ValidatedCandidateHealthQuarantinePermit,
    local_authority::{
        keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
        plan_application::read_authority_plan_application_state,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn persist_candidate_health_quarantine(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateHealthQuarantineAuthoritySession<'_>,
    permit: ValidatedCandidateHealthQuarantinePermit<'_, '_>,
) -> Result<HashedComputePluginCandidateHealthQuarantineReceipt> {
    let publication = permit.publication();
    let guard = publication.staged().archive().snapshot_cancellation_guard();
    session.validate_source(&guard)?;
    let current = read_candidate_health_quarantine_binding(transaction, session, publication)?;
    if &current != permit.facts() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_AUTHORITY_CHANGED");
    }
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let projection = project_candidate_health_quarantine(
        &authority,
        &current.plugin_id,
        &current.slot_ref,
        &current.release,
        &session.trusted_now,
    )?;
    if projection.state_revision != current.authority_state_revision_after
        || projection.inventory.inventory_revision != current.inventory_revision_after
        || projection.inventory_digest != current.inventory_digest_after
        || projection.authority_epoch != current.authority_epoch_after
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_PROJECTION_CHANGED");
    }

    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.state_revision != current.authority_state_revision_before
        || time_state.authority_epoch != current.authority_epoch_before
        || time_state.trusted_time_high_water_ms != Some(current.trusted_time_high_water_ms_before)
        || time_state.clock_status != "trusted"
        || current.failed_at_ms <= current.trusted_time_high_water_ms_before
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, current.failed_at_ms)?;
    update_quarantine_authority_meta(transaction, &authority, &projection, current.failed_at_ms)?;
    session.validate_source(&guard)?;

    let observation = publication.observation().clone();
    let receipt = ComputePluginCandidateHealthQuarantineReceipt {
        schema: CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA.to_string(),
        quarantine_id: permit.quarantine_id().to_string(),
        evaluation_id: observation.observation.evaluation_id.clone(),
        candidate_token_digest: current.candidate_token_digest.clone(),
        staging_id: current.staging_id.clone(),
        staging_receipt_digest: current.staging_receipt_digest.clone(),
        staging_run_digest: current.staging_run_digest.clone(),
        failure_observation_digest: observation.observation_digest.clone(),
        authority_state_revision_before: current.authority_state_revision_before,
        authority_state_revision_after: current.authority_state_revision_after,
        inventory_revision_before: current.inventory_revision_before,
        inventory_revision_after: current.inventory_revision_after,
        inventory_digest_before: current.inventory_digest_before.clone(),
        inventory_digest_after: current.inventory_digest_after.clone(),
        authority_epoch_before: current.authority_epoch_before,
        authority_epoch_after: current.authority_epoch_after,
        process_owner_epoch: current.process_owner_epoch,
        trusted_time_high_water_ms_before: current.trusted_time_high_water_ms_before,
        failed_at_ms: current.failed_at_ms,
        slot_phase_after: "failed".to_string(),
    };
    let hashed = HashedComputePluginCandidateHealthQuarantineReceipt {
        schema: HASHED_CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA.to_string(),
        observation,
        receipt_digest: jcs_sha256_hex(&receipt)?,
        receipt,
        canonicalization: CANDIDATE_HEALTH_QUARANTINE_RECEIPT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_HEALTH_QUARANTINE_RECEIPT_DIGEST_ALGORITHM.to_string(),
    };
    insert_receipt(transaction, publication, &hashed)?;
    validate_readback(transaction, &hashed)?;
    session.validate_source(&guard)?;
    Ok(hashed)
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    publication: &crate::node_agent_compute_plugin_host::candidate_health_contract::ValidatedCandidateHealthFailurePublication<'_>,
    hashed: &HashedComputePluginCandidateHealthQuarantineReceipt,
) -> Result<()> {
    let staging_key = publication.staged().recovery_key();
    let receipt = hashed.receipt();
    let observation_json = serde_json::to_string(hashed.observation())
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_OBSERVATION_SERIALIZE")?;
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECEIPT_SERIALIZE")?;
    transaction
        .execute(
            r#"INSERT INTO candidate_health_quarantine_receipts (
            quarantine_id, evaluation_id, candidate_token, candidate_token_digest,
            staging_id, staging_receipt_digest, staging_run_digest,
            failure_observation_json, failure_observation_digest,
            authority_state_revision_before, authority_state_revision_after,
            inventory_revision_before, inventory_revision_after,
            inventory_digest_before, inventory_digest_after,
            authority_epoch_before, authority_epoch_after, process_owner_epoch,
            trusted_time_high_water_ms_before, failed_at_ms, slot_phase_after,
            receipt_json, receipt_digest
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
        )"#,
            params![
                receipt.quarantine_id(),
                receipt.evaluation_id(),
                staging_key.candidate_token(),
                receipt.candidate_token_digest(),
                receipt.staging_id(),
                receipt.staging_receipt_digest(),
                receipt.staging_run_digest(),
                observation_json,
                receipt.failure_observation_digest(),
                receipt.authority_state_revision_before(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision_before(),
                receipt.inventory_revision_after(),
                receipt.inventory_digest_before(),
                receipt.inventory_digest_after(),
                receipt.authority_epoch_before(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.trusted_time_high_water_ms_before(),
                receipt.failed_at_ms(),
                receipt.slot_phase_after(),
                receipt_json,
                hashed.receipt_digest(),
            ],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECEIPT_INSERT")?;
    Ok(())
}

fn validate_readback(
    transaction: &Transaction<'_>,
    hashed: &HashedComputePluginCandidateHealthQuarantineReceipt,
) -> Result<()> {
    let receipt = hashed.receipt();
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_health_quarantine_receipts
           WHERE quarantine_id = ?1 AND evaluation_id = ?2
             AND candidate_token_digest = ?3 AND staging_id = ?4
             AND failure_observation_digest = ?5
             AND authority_state_revision_after = ?6
             AND inventory_revision_after = ?7 AND inventory_digest_after = ?8
             AND authority_epoch_after = ?9 AND process_owner_epoch = ?10
             AND failed_at_ms = ?11 AND slot_phase_after = 'failed'
             AND receipt_digest = ?12"#,
            params![
                receipt.quarantine_id(),
                receipt.evaluation_id(),
                receipt.candidate_token_digest(),
                receipt.staging_id(),
                receipt.failure_observation_digest(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision_after(),
                receipt.inventory_digest_after(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.failed_at_ms(),
                hashed.receipt_digest(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECEIPT_READBACK")?;
    let meta_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta WHERE singleton = 1
          AND state_revision = ?1 AND inventory_revision = ?2
          AND inventory_digest = ?3 AND authority_epoch = ?4
          AND process_owner_epoch = ?5 AND trusted_time_high_water_ms = ?6
          AND updated_at_ms = ?6 AND clock_status = 'trusted'"#,
            params![
                receipt.authority_state_revision_after(),
                receipt.inventory_revision_after(),
                receipt.inventory_digest_after(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.failed_at_ms(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_META_READBACK")?;
    if count != 1 || meta_count != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECEIPT_READBACK_CHANGED");
    }
    Ok(())
}
