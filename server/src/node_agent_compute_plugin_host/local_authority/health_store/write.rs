use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{
    binding::read_candidate_health_binding,
    types::{
        ComputePluginCandidateHealthReceipt, HashedComputePluginCandidateHealthReceipt,
        CANDIDATE_HEALTH_RECEIPT_CANONICALIZATION, CANDIDATE_HEALTH_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_HEALTH_RECEIPT_SCHEMA, HASHED_CANDIDATE_HEALTH_RECEIPT_SCHEMA,
    },
    ComputePluginCandidateHealthAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::ValidatedCandidateHealthStorePermit,
    local_authority::keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn persist_candidate_health(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateHealthAuthoritySession<'_>,
    permit: ValidatedCandidateHealthStorePermit<'_, '_>,
) -> Result<HashedComputePluginCandidateHealthReceipt> {
    let publication = permit.publication();
    let guard = publication.staged().archive().snapshot_cancellation_guard();
    session.validate_source(&guard)?;
    let current = read_candidate_health_binding(transaction, session, publication)?;
    if &current != permit.facts() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_AUTHORITY_CHANGED");
    }

    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.state_revision != current.authority_state_revision()
        || time_state.authority_epoch != current.authority_epoch()
        || time_state.trusted_time_high_water_ms != Some(current.trusted_time_high_water_ms())
        || time_state.clock_status != "trusted"
        || current.recorded_at_ms() <= current.trusted_time_high_water_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, current.recorded_at_ms())?;
    session.validate_source(&guard)?;

    let observation = publication.observation().clone();
    let receipt = ComputePluginCandidateHealthReceipt {
        schema: CANDIDATE_HEALTH_RECEIPT_SCHEMA.to_string(),
        health_id: permit.health_id().to_string(),
        evaluation_id: observation.observation.evaluation_id.clone(),
        candidate_token_digest: current.candidate_token_digest().to_string(),
        staging_id: current.staging_id().to_string(),
        staging_receipt_digest: current.staging_receipt_digest().to_string(),
        staging_run_digest: current.staging_run_digest().to_string(),
        health_observation_digest: observation.observation_digest.clone(),
        authority_state_revision: current.authority_state_revision(),
        inventory_revision: current.inventory_revision(),
        inventory_digest: current.inventory_digest().to_string(),
        authority_epoch: current.authority_epoch(),
        process_owner_epoch: current.process_owner_epoch(),
        recorded_at_ms: current.recorded_at_ms(),
        expires_at_ms: current.expires_at_ms(),
    };
    let receipt_digest = jcs_sha256_hex(&receipt)?;
    let hashed = HashedComputePluginCandidateHealthReceipt {
        schema: HASHED_CANDIDATE_HEALTH_RECEIPT_SCHEMA.to_string(),
        observation,
        receipt,
        canonicalization: CANDIDATE_HEALTH_RECEIPT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_HEALTH_RECEIPT_DIGEST_ALGORITHM.to_string(),
        receipt_digest,
    };
    insert_receipt(transaction, publication, &hashed)?;
    validate_readback(transaction, &hashed)?;
    session.validate_source(&guard)?;
    Ok(hashed)
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    publication: &crate::node_agent_compute_plugin_host::candidate_health_contract::ValidatedCandidateHealthPublication<'_>,
    hashed: &HashedComputePluginCandidateHealthReceipt,
) -> Result<()> {
    let staging_key = publication.staged().recovery_key();
    let receipt = hashed.receipt();
    let observation_json = serde_json::to_string(hashed.observation())
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_OBSERVATION_SERIALIZE")?;
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECEIPT_SERIALIZE")?;
    transaction
        .execute(
            r#"INSERT INTO candidate_health_receipts (
                health_id, evaluation_id, candidate_token, candidate_token_digest,
                staging_id, staging_receipt_digest, staging_run_digest,
                health_observation_json, health_observation_digest,
                authority_state_revision, inventory_revision, inventory_digest,
                authority_epoch, process_owner_epoch, recorded_at_ms, expires_at_ms,
                receipt_json, receipt_digest
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
            )"#,
            params![
                receipt.health_id(),
                receipt.evaluation_id(),
                staging_key.candidate_token(),
                receipt.candidate_token_digest(),
                receipt.staging_id(),
                receipt.staging_receipt_digest(),
                receipt.staging_run_digest(),
                observation_json,
                receipt.health_observation_digest(),
                receipt.authority_state_revision(),
                receipt.inventory_revision(),
                receipt.inventory_digest(),
                receipt.authority_epoch(),
                receipt.process_owner_epoch(),
                receipt.recorded_at_ms(),
                receipt.expires_at_ms(),
                receipt_json,
                hashed.receipt_digest(),
            ],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECEIPT_INSERT")?;
    Ok(())
}

fn validate_readback(
    transaction: &Transaction<'_>,
    hashed: &HashedComputePluginCandidateHealthReceipt,
) -> Result<()> {
    let receipt = hashed.receipt();
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_health_receipts
            WHERE health_id = ?1 AND evaluation_id = ?2
              AND candidate_token_digest = ?3 AND staging_id = ?4
              AND staging_receipt_digest = ?5 AND staging_run_digest = ?6
              AND health_observation_digest = ?7
              AND authority_state_revision = ?8 AND inventory_revision = ?9
              AND inventory_digest = ?10 AND authority_epoch = ?11
              AND process_owner_epoch = ?12 AND recorded_at_ms = ?13
              AND expires_at_ms = ?14 AND receipt_digest = ?15"#,
            params![
                receipt.health_id(),
                receipt.evaluation_id(),
                receipt.candidate_token_digest(),
                receipt.staging_id(),
                receipt.staging_receipt_digest(),
                receipt.staging_run_digest(),
                receipt.health_observation_digest(),
                receipt.authority_state_revision(),
                receipt.inventory_revision(),
                receipt.inventory_digest(),
                receipt.authority_epoch(),
                receipt.process_owner_epoch(),
                receipt.recorded_at_ms(),
                receipt.expires_at_ms(),
                hashed.receipt_digest(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECEIPT_READBACK")?;
    let meta_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta WHERE singleton = 1
              AND state_revision = ?1 AND inventory_revision = ?2
              AND inventory_digest = ?3 AND authority_epoch = ?4
              AND process_owner_epoch = ?5 AND trusted_time_high_water_ms = ?6
              AND updated_at_ms = ?6 AND clock_status = 'trusted'"#,
            params![
                receipt.authority_state_revision(),
                receipt.inventory_revision(),
                receipt.inventory_digest(),
                receipt.authority_epoch(),
                receipt.process_owner_epoch(),
                receipt.recorded_at_ms(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_META_READBACK")?;
    if count != 1 || meta_count != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECEIPT_READBACK_CHANGED");
    }
    Ok(())
}
