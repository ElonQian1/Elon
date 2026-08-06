use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    binding::validate_staged_inventory,
    types::{
        ComputePluginCandidateHealthReceipt, HashedComputePluginCandidateHealthReceipt,
        CANDIDATE_HEALTH_RECEIPT_CANONICALIZATION, CANDIDATE_HEALTH_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_HEALTH_RECEIPT_SCHEMA, HASHED_CANDIDATE_HEALTH_RECEIPT_SCHEMA,
    },
};
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::{
        validate_hashed_candidate_health_observation, ComputePluginCandidateHealthRecoveryKey,
        HashedComputePluginCandidateHealthObservation,
    },
    local_authority::{
        plan_application::read_authority_plan_application_state,
        ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateHealthRecoveryOutcome {
    NotCreated,
    Recorded(HashedComputePluginCandidateHealthReceipt),
}

struct StoredHealthRow {
    health_id: String,
    evaluation_id: String,
    candidate_token: String,
    candidate_token_digest: String,
    staging_id: String,
    staging_receipt_digest: String,
    staging_run_digest: String,
    observation_json: String,
    observation_digest: String,
    authority_state_revision: i64,
    inventory_revision: i64,
    inventory_digest: String,
    authority_epoch: i64,
    process_owner_epoch: i64,
    recorded_at_ms: i64,
    expires_at_ms: i64,
    receipt_json: String,
    receipt_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_health_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateHealthRecoveryAuthoritySession<'authority>> {
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
            bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateHealthRecoveryAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        })
    }
}

impl ComputePluginCandidateHealthRecoveryAuthoritySession<'_> {
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

    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_health_outcome(
        &self,
        key: &ComputePluginCandidateHealthRecoveryKey,
    ) -> Result<ComputePluginCandidateHealthRecoveryOutcome> {
        validate_recovery_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

impl ComputePluginCandidateHealthRecoveryOutcome {
    pub(in crate::node_agent_compute_plugin_host) fn is_not_created(&self) -> bool {
        matches!(self, Self::NotCreated)
    }

    pub(in crate::node_agent_compute_plugin_host) fn recorded_receipt(
        &self,
    ) -> Option<&HashedComputePluginCandidateHealthReceipt> {
        match self {
            Self::NotCreated => None,
            Self::Recorded(receipt) => Some(receipt),
        }
    }
}

fn validate_recovery_provenance(
    session: &ComputePluginCandidateHealthRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidateHealthRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.receipt_expectation().process_owner_epoch != session.process_owner_epoch()
        || session.observed_at <= session.process_fence.acquired_observed_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateHealthRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidateHealthRecoveryKey,
) -> Result<ComputePluginCandidateHealthRecoveryOutcome> {
    let stored = read_exact_row(transaction, key)?;
    let identity_matches = count_identity_matches(transaction, key)?;
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let staging = key.staging_expectation();
    validate_staged_inventory(
        &authority.inventory,
        &staging.plugin_id,
        &staging.slot_ref,
        &staging.release,
    )?;
    let expected = key.receipt_expectation();
    if authority.installation_id_digest != key.installation_id_digest()
        || authority.state_revision != expected.authority_state_revision
        || authority.inventory.inventory_revision != expected.inventory_revision
        || authority.inventory_digest != expected.inventory_digest
        || authority.authority_epoch != expected.authority_epoch
        || authority.process_owner_epoch != expected.process_owner_epoch
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_AUTHORITY_CHANGED");
    }

    match stored {
        Some(row) => {
            if identity_matches != 1
                || authority.trusted_time_high_water_ms < expected.recorded_at_ms
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_IDENTITY_COLLISION");
            }
            Ok(ComputePluginCandidateHealthRecoveryOutcome::Recorded(
                decode_recorded_row(key, row)?,
            ))
        }
        None => {
            if identity_matches != 0 {
                bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_IDENTITY_COLLISION");
            }
            if authority.trusted_time_high_water_ms != expected.trusted_time_high_water_ms_before {
                bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            Ok(ComputePluginCandidateHealthRecoveryOutcome::NotCreated)
        }
    }
}

fn read_exact_row(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateHealthRecoveryKey,
) -> Result<Option<StoredHealthRow>> {
    transaction
        .query_row(
            r#"SELECT health_id, evaluation_id, candidate_token, candidate_token_digest,
                staging_id, staging_receipt_digest, staging_run_digest,
                health_observation_json, health_observation_digest,
                authority_state_revision, inventory_revision, inventory_digest,
                authority_epoch, process_owner_epoch, recorded_at_ms, expires_at_ms,
                receipt_json, receipt_digest
            FROM candidate_health_receipts WHERE health_id = ?1"#,
            params![key.health_id()],
            |row| {
                Ok(StoredHealthRow {
                    health_id: row.get(0)?,
                    evaluation_id: row.get(1)?,
                    candidate_token: row.get(2)?,
                    candidate_token_digest: row.get(3)?,
                    staging_id: row.get(4)?,
                    staging_receipt_digest: row.get(5)?,
                    staging_run_digest: row.get(6)?,
                    observation_json: row.get(7)?,
                    observation_digest: row.get(8)?,
                    authority_state_revision: row.get(9)?,
                    inventory_revision: row.get(10)?,
                    inventory_digest: row.get(11)?,
                    authority_epoch: row.get(12)?,
                    process_owner_epoch: row.get(13)?,
                    recorded_at_ms: row.get(14)?,
                    expires_at_ms: row.get(15)?,
                    receipt_json: row.get(16)?,
                    receipt_digest: row.get(17)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_ROW_READ")
}

fn count_identity_matches(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateHealthRecoveryKey,
) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_health_receipts
            WHERE health_id = ?1 OR evaluation_id = ?2 OR health_observation_digest = ?3"#,
            params![
                key.health_id(),
                key.receipt_expectation().evaluation_id,
                key.receipt_expectation().health_observation_digest,
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_IDENTITY_READ")
}

fn decode_recorded_row(
    key: &ComputePluginCandidateHealthRecoveryKey,
    row: StoredHealthRow,
) -> Result<HashedComputePluginCandidateHealthReceipt> {
    let observation: HashedComputePluginCandidateHealthObservation =
        serde_json::from_str(&row.observation_json)
            .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_OBSERVATION_PARSE")?;
    let receipt: ComputePluginCandidateHealthReceipt = serde_json::from_str(&row.receipt_json)
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_RECEIPT_PARSE")?;
    validate_hashed_candidate_health_observation(&observation)?;
    let expected = key.receipt_expectation();
    let staging = key.staging_expectation();
    if serde_json::to_string(&observation)? != row.observation_json
        || serde_json::to_string(&receipt)? != row.receipt_json
        || receipt.schema != CANDIDATE_HEALTH_RECEIPT_SCHEMA
        || row.health_id != key.health_id()
        || row.evaluation_id != expected.evaluation_id
        || row.candidate_token != staging.candidate_token
        || row.candidate_token_digest != staging.candidate_token_digest
        || row.staging_id != staging.staging_id
        || row.staging_receipt_digest != staging.staging_receipt_digest
        || row.staging_run_digest != staging.staging_run_digest
        || row.observation_digest != expected.health_observation_digest
        || row.authority_state_revision != expected.authority_state_revision
        || row.inventory_revision != expected.inventory_revision
        || row.inventory_digest != expected.inventory_digest
        || row.authority_epoch != expected.authority_epoch
        || row.process_owner_epoch != expected.process_owner_epoch
        || row.recorded_at_ms != expected.recorded_at_ms
        || row.expires_at_ms != expected.expires_at_ms
        || receipt.health_id != row.health_id
        || receipt.evaluation_id != row.evaluation_id
        || receipt.candidate_token_digest != row.candidate_token_digest
        || receipt.staging_id != row.staging_id
        || receipt.staging_receipt_digest != row.staging_receipt_digest
        || receipt.staging_run_digest != row.staging_run_digest
        || receipt.health_observation_digest != row.observation_digest
        || receipt.authority_state_revision != row.authority_state_revision
        || receipt.inventory_revision != row.inventory_revision
        || receipt.inventory_digest != row.inventory_digest
        || receipt.authority_epoch != row.authority_epoch
        || receipt.process_owner_epoch != row.process_owner_epoch
        || receipt.recorded_at_ms != row.recorded_at_ms
        || receipt.expires_at_ms != row.expires_at_ms
        || observation.observation_digest != row.observation_digest
        || !is_sha256(&row.receipt_digest)
        || jcs_sha256_hex(&receipt)? != row.receipt_digest
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECOVERY_ROW_CHANGED");
    }
    Ok(HashedComputePluginCandidateHealthReceipt {
        schema: HASHED_CANDIDATE_HEALTH_RECEIPT_SCHEMA.to_string(),
        observation,
        receipt,
        canonicalization: CANDIDATE_HEALTH_RECEIPT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_HEALTH_RECEIPT_DIGEST_ALGORITHM.to_string(),
        receipt_digest: row.receipt_digest,
    })
}
