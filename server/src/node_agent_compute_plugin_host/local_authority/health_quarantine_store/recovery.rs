use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::types::{
    ComputePluginCandidateHealthQuarantineReceipt,
    HashedComputePluginCandidateHealthQuarantineReceipt,
    CANDIDATE_HEALTH_QUARANTINE_RECEIPT_CANONICALIZATION,
    CANDIDATE_HEALTH_QUARANTINE_RECEIPT_DIGEST_ALGORITHM,
    CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA, HASHED_CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA,
};
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::{
        validate_hashed_candidate_health_failure_observation, CandidateHealthQuarantineRecoveryKey,
        HashedComputePluginCandidateHealthFailureObservation,
    },
    lifecycle::{ComputePluginInventorySnapshot, SLOT_FAILED, SLOT_STAGED},
    local_authority::{
        plan_application::read_authority_plan_application_state,
        ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateHealthQuarantineRecoveryOutcome
{
    NotCreated,
    Quarantined(HashedComputePluginCandidateHealthQuarantineReceipt),
}

struct StoredQuarantineRow {
    quarantine_id: String,
    evaluation_id: String,
    candidate_token: String,
    candidate_token_digest: String,
    staging_id: String,
    staging_receipt_digest: String,
    staging_run_digest: String,
    observation_json: String,
    observation_digest: String,
    state_before: i64,
    state_after: i64,
    inventory_before: i64,
    inventory_after: i64,
    inventory_digest_before: String,
    inventory_digest_after: String,
    epoch_before: i64,
    epoch_after: i64,
    process_owner_epoch: i64,
    time_before: i64,
    failed_at_ms: i64,
    slot_phase_after: String,
    receipt_json: String,
    receipt_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_health_quarantine_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || process_fence.process_owner_epoch() <= 0
            || observed_at <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_SESSION_INVALID");
        }
        Ok(
            ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession {
                authority: self,
                process_fence,
                trusted_now,
                observed_at,
                clock_epoch_digest: observation.clock_epoch_digest().to_string(),
            },
        )
    }
}

impl ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }
    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_fence.process_owner_epoch()
    }
    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }
    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_health_quarantine_outcome(
        &self,
        key: &CandidateHealthQuarantineRecoveryKey,
    ) -> Result<ComputePluginCandidateHealthQuarantineRecoveryOutcome> {
        validate_recovery_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

impl ComputePluginCandidateHealthQuarantineRecoveryOutcome {
    pub(in crate::node_agent_compute_plugin_host) fn is_not_created(&self) -> bool {
        matches!(self, Self::NotCreated)
    }
    pub(in crate::node_agent_compute_plugin_host) fn quarantined_receipt(
        &self,
    ) -> Option<&HashedComputePluginCandidateHealthQuarantineReceipt> {
        match self {
            Self::NotCreated => None,
            Self::Quarantined(receipt) => Some(receipt),
        }
    }
}

fn validate_recovery_provenance(
    session: &ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession<'_>,
    key: &CandidateHealthQuarantineRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.receipt_expectation().process_owner_epoch != session.process_owner_epoch()
        || session.observed_at <= session.process_fence.acquired_observed_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession<'_>,
    key: &CandidateHealthQuarantineRecoveryKey,
) -> Result<ComputePluginCandidateHealthQuarantineRecoveryOutcome> {
    let stored = read_exact_row(transaction, key)?;
    let identity_matches = count_identity_matches(transaction, key)?;
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let expected = key.receipt_expectation();
    let staging = key.staging_expectation();
    if authority.installation_id_digest != key.installation_id_digest()
        || authority.process_owner_epoch != expected.process_owner_epoch
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_AUTHORITY_CHANGED");
    }

    match stored {
        Some(row) => {
            if identity_matches != 1
                || authority.state_revision != expected.authority_state_revision_after
                || authority.inventory.inventory_revision != expected.inventory_revision_after
                || authority.inventory_digest != expected.inventory_digest_after
                || authority.authority_epoch != expected.authority_epoch_after
                || authority.trusted_time_high_water_ms != expected.failed_at_ms
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_RESULT_AMBIGUOUS");
            }
            validate_slot_phase(
                &authority.inventory,
                &staging.plugin_id,
                &staging.slot_ref,
                &staging.release,
                SLOT_FAILED,
            )?;
            Ok(
                ComputePluginCandidateHealthQuarantineRecoveryOutcome::Quarantined(
                    decode_recorded_row(key, row)?,
                ),
            )
        }
        None => {
            if identity_matches != 0
                || authority.state_revision != expected.authority_state_revision_before
                || authority.inventory.inventory_revision != expected.inventory_revision_before
                || authority.inventory_digest != expected.inventory_digest_before
                || authority.authority_epoch != expected.authority_epoch_before
                || authority.trusted_time_high_water_ms
                    != expected.trusted_time_high_water_ms_before
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            validate_slot_phase(
                &authority.inventory,
                &staging.plugin_id,
                &staging.slot_ref,
                &staging.release,
                SLOT_STAGED,
            )?;
            Ok(ComputePluginCandidateHealthQuarantineRecoveryOutcome::NotCreated)
        }
    }
}

fn validate_slot_phase(
    inventory: &ComputePluginInventorySnapshot,
    plugin_id: &str,
    slot_ref: &str,
    release: &crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
    expected_phase: &str,
) -> Result<()> {
    let plugin = inventory
        .plugins
        .iter()
        .find(|plugin| plugin.plugin_id == plugin_id)
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_PLUGIN_MISSING")
        })?;
    let slot = plugin
        .slots
        .iter()
        .find(|slot| slot.slot_ref == slot_ref)
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_SLOT_MISSING")
        })?;
    if plugin.candidate_slot_ref.as_deref() != Some(slot_ref)
        || &slot.release != release
        || slot.phase != expected_phase
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_SLOT_CHANGED");
    }
    Ok(())
}

fn read_exact_row(
    transaction: &Transaction<'_>,
    key: &CandidateHealthQuarantineRecoveryKey,
) -> Result<Option<StoredQuarantineRow>> {
    transaction
        .query_row(
            r#"SELECT quarantine_id, evaluation_id, candidate_token, candidate_token_digest,
            staging_id, staging_receipt_digest, staging_run_digest,
            failure_observation_json, failure_observation_digest,
            authority_state_revision_before, authority_state_revision_after,
            inventory_revision_before, inventory_revision_after,
            inventory_digest_before, inventory_digest_after,
            authority_epoch_before, authority_epoch_after, process_owner_epoch,
            trusted_time_high_water_ms_before, failed_at_ms, slot_phase_after,
            receipt_json, receipt_digest
        FROM candidate_health_quarantine_receipts WHERE quarantine_id = ?1"#,
            params![key.quarantine_id()],
            |row| {
                Ok(StoredQuarantineRow {
                    quarantine_id: row.get(0)?,
                    evaluation_id: row.get(1)?,
                    candidate_token: row.get(2)?,
                    candidate_token_digest: row.get(3)?,
                    staging_id: row.get(4)?,
                    staging_receipt_digest: row.get(5)?,
                    staging_run_digest: row.get(6)?,
                    observation_json: row.get(7)?,
                    observation_digest: row.get(8)?,
                    state_before: row.get(9)?,
                    state_after: row.get(10)?,
                    inventory_before: row.get(11)?,
                    inventory_after: row.get(12)?,
                    inventory_digest_before: row.get(13)?,
                    inventory_digest_after: row.get(14)?,
                    epoch_before: row.get(15)?,
                    epoch_after: row.get(16)?,
                    process_owner_epoch: row.get(17)?,
                    time_before: row.get(18)?,
                    failed_at_ms: row.get(19)?,
                    slot_phase_after: row.get(20)?,
                    receipt_json: row.get(21)?,
                    receipt_digest: row.get(22)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_ROW_READ")
}

fn count_identity_matches(
    transaction: &Transaction<'_>,
    key: &CandidateHealthQuarantineRecoveryKey,
) -> Result<i64> {
    let expected = key.receipt_expectation();
    let staging = key.staging_expectation();
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_health_quarantine_receipts
           WHERE quarantine_id = ?1 OR evaluation_id = ?2 OR failure_observation_digest = ?3
              OR candidate_token = ?4 OR staging_id = ?5"#,
            params![
                key.quarantine_id(),
                expected.evaluation_id,
                expected.failure_observation_digest,
                staging.candidate_token,
                staging.staging_id
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_IDENTITY_READ")
}

fn decode_recorded_row(
    key: &CandidateHealthQuarantineRecoveryKey,
    row: StoredQuarantineRow,
) -> Result<HashedComputePluginCandidateHealthQuarantineReceipt> {
    let observation: HashedComputePluginCandidateHealthFailureObservation =
        serde_json::from_str(&row.observation_json)
            .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_OBSERVATION_PARSE")?;
    let receipt: ComputePluginCandidateHealthQuarantineReceipt =
        serde_json::from_str(&row.receipt_json)
            .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_RECEIPT_PARSE")?;
    validate_hashed_candidate_health_failure_observation(&observation)?;
    let expected = key.receipt_expectation();
    let staging = key.staging_expectation();
    if serde_json::to_string(&observation)? != row.observation_json
        || serde_json::to_string(&receipt)? != row.receipt_json
        || receipt.schema != CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA
        || row.quarantine_id != key.quarantine_id()
        || row.evaluation_id != expected.evaluation_id
        || row.candidate_token != staging.candidate_token
        || row.candidate_token_digest != staging.candidate_token_digest
        || row.staging_id != staging.staging_id
        || row.staging_receipt_digest != staging.staging_receipt_digest
        || row.staging_run_digest != staging.staging_run_digest
        || row.observation_digest != expected.failure_observation_digest
        || row.state_before != expected.authority_state_revision_before
        || row.state_after != expected.authority_state_revision_after
        || row.inventory_before != expected.inventory_revision_before
        || row.inventory_after != expected.inventory_revision_after
        || row.inventory_digest_before != expected.inventory_digest_before
        || row.inventory_digest_after != expected.inventory_digest_after
        || row.epoch_before != expected.authority_epoch_before
        || row.epoch_after != expected.authority_epoch_after
        || row.process_owner_epoch != expected.process_owner_epoch
        || row.time_before != expected.trusted_time_high_water_ms_before
        || row.failed_at_ms != expected.failed_at_ms
        || row.slot_phase_after != "failed"
        || receipt.quarantine_id != row.quarantine_id
        || receipt.evaluation_id != row.evaluation_id
        || receipt.candidate_token_digest != row.candidate_token_digest
        || receipt.staging_id != row.staging_id
        || receipt.staging_receipt_digest != row.staging_receipt_digest
        || receipt.staging_run_digest != row.staging_run_digest
        || receipt.failure_observation_digest != row.observation_digest
        || receipt.authority_state_revision_before != row.state_before
        || receipt.authority_state_revision_after != row.state_after
        || receipt.inventory_revision_before != row.inventory_before
        || receipt.inventory_revision_after != row.inventory_after
        || receipt.inventory_digest_before != row.inventory_digest_before
        || receipt.inventory_digest_after != row.inventory_digest_after
        || receipt.authority_epoch_before != row.epoch_before
        || receipt.authority_epoch_after != row.epoch_after
        || receipt.process_owner_epoch != row.process_owner_epoch
        || receipt.trusted_time_high_water_ms_before != row.time_before
        || receipt.failed_at_ms != row.failed_at_ms
        || receipt.slot_phase_after != row.slot_phase_after
        || observation.observation_digest != row.observation_digest
        || !is_sha256(&row.receipt_digest)
        || jcs_sha256_hex(&receipt)? != row.receipt_digest
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECOVERY_ROW_CHANGED");
    }
    Ok(HashedComputePluginCandidateHealthQuarantineReceipt {
        schema: HASHED_CANDIDATE_HEALTH_QUARANTINE_RECEIPT_SCHEMA.to_string(),
        observation,
        receipt,
        canonicalization: CANDIDATE_HEALTH_QUARANTINE_RECEIPT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_HEALTH_QUARANTINE_RECEIPT_DIGEST_ALGORITHM.to_string(),
        receipt_digest: row.receipt_digest,
    })
}
