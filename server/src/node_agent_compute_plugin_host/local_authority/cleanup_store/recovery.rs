use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    binding::validate_failed_candidate_inventory,
    types::{
        ComputePluginCandidateCleanupAuthorizationReceipt,
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
        HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
    },
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::CandidateCleanupAuthorizationRecoveryKey,
    lifecycle::SLOT_FAILED,
    local_authority::{
        plan_application::read_authority_plan_application_state,
        ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateCleanupRecoveryOutcome {
    NotCreated,
    Authorized(HashedComputePluginCandidateCleanupAuthorizationReceipt),
}

struct StoredCleanupAuthorizationRow {
    cleanup_id: String,
    candidate_token: String,
    candidate_token_digest: String,
    quarantine_id: String,
    quarantine_receipt_digest: String,
    staging_id: String,
    staging_run_digest: String,
    state_before: i64,
    state_after: i64,
    inventory_revision: i64,
    inventory_digest: String,
    epoch_before: i64,
    epoch_after: i64,
    process_owner_epoch: i64,
    time_before: i64,
    authorized_at_ms: i64,
    slot_phase_before: String,
    receipt_json: String,
    receipt_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateCleanupRecoveryAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || !is_sha256(observation.installation_id_digest())
            || !is_sha256(observation.clock_epoch_digest())
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateCleanupRecoveryAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        })
    }
}

impl ComputePluginCandidateCleanupRecoveryAuthoritySession<'_> {
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
    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_cleanup_authorization_outcome(
        &self,
        key: &CandidateCleanupAuthorizationRecoveryKey,
    ) -> Result<ComputePluginCandidateCleanupRecoveryOutcome> {
        validate_recovery_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

impl ComputePluginCandidateCleanupRecoveryOutcome {
    pub(in crate::node_agent_compute_plugin_host) fn is_not_created(&self) -> bool {
        matches!(self, Self::NotCreated)
    }
    pub(in crate::node_agent_compute_plugin_host) fn authorization_receipt(
        &self,
    ) -> Option<&HashedComputePluginCandidateCleanupAuthorizationReceipt> {
        match self {
            Self::NotCreated => None,
            Self::Authorized(receipt) => Some(receipt),
        }
    }
}

fn validate_recovery_provenance(
    session: &ComputePluginCandidateCleanupRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupAuthorizationRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.receipt_expectation().process_owner_epoch != session.process_owner_epoch()
        || session.observed_at <= session.process_fence.acquired_observed_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupAuthorizationRecoveryKey,
) -> Result<ComputePluginCandidateCleanupRecoveryOutcome> {
    let stored = read_exact_row(transaction, key)?;
    let identity_matches = count_identity_matches(transaction, key)?;
    let owner_state = read_owner_state(transaction, key.candidate_token())?;
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let expected = key.receipt_expectation();
    let slot = key.slot_expectation();
    if authority.installation_id_digest != key.installation_id_digest()
        || authority.process_owner_epoch != expected.process_owner_epoch
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_AUTHORITY_CHANGED");
    }
    validate_failed_candidate_inventory(
        &authority.inventory,
        &slot.plugin_id,
        &slot.slot_ref,
        &slot.release,
    )?;

    match stored {
        Some(row) => {
            if identity_matches != 1
                || owner_state.as_deref() != Some("cleanup_pending")
                || authority.state_revision != expected.authority_state_revision_after
                || authority.inventory.inventory_revision != expected.inventory_revision
                || authority.inventory_digest != expected.inventory_digest
                || authority.authority_epoch != expected.authority_epoch_after
                || authority.trusted_time_high_water_ms != expected.authorized_at_ms
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_RESULT_AMBIGUOUS");
            }
            Ok(ComputePluginCandidateCleanupRecoveryOutcome::Authorized(
                decode_recorded_row(key, row)?,
            ))
        }
        None => {
            if identity_matches != 0
                || owner_state.as_deref() != Some("owned")
                || authority.state_revision != expected.authority_state_revision_before
                || authority.inventory.inventory_revision != expected.inventory_revision
                || authority.inventory_digest != expected.inventory_digest
                || authority.authority_epoch != expected.authority_epoch_before
                || authority.trusted_time_high_water_ms
                    != expected.trusted_time_high_water_ms_before
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            Ok(ComputePluginCandidateCleanupRecoveryOutcome::NotCreated)
        }
    }
}

fn read_owner_state(
    transaction: &Transaction<'_>,
    candidate_token: &str,
) -> Result<Option<String>> {
    transaction
        .query_row(
            "SELECT state FROM candidate_owners WHERE candidate_token = ?1",
            params![candidate_token],
            |row| row.get(0),
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_OWNER_READ")
}

fn read_exact_row(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupAuthorizationRecoveryKey,
) -> Result<Option<StoredCleanupAuthorizationRow>> {
    transaction
        .query_row(
            r#"SELECT cleanup_id, candidate_token, candidate_token_digest,
                quarantine_id, quarantine_receipt_digest, staging_id, staging_run_digest,
                authority_state_revision_before, authority_state_revision_after,
                inventory_revision, inventory_digest,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                trusted_time_high_water_ms_before, authorized_at_ms, slot_phase_before,
                receipt_json, receipt_digest
            FROM candidate_cleanup_authorizations WHERE cleanup_id = ?1"#,
            params![key.cleanup_id()],
            |row| {
                Ok(StoredCleanupAuthorizationRow {
                    cleanup_id: row.get(0)?,
                    candidate_token: row.get(1)?,
                    candidate_token_digest: row.get(2)?,
                    quarantine_id: row.get(3)?,
                    quarantine_receipt_digest: row.get(4)?,
                    staging_id: row.get(5)?,
                    staging_run_digest: row.get(6)?,
                    state_before: row.get(7)?,
                    state_after: row.get(8)?,
                    inventory_revision: row.get(9)?,
                    inventory_digest: row.get(10)?,
                    epoch_before: row.get(11)?,
                    epoch_after: row.get(12)?,
                    process_owner_epoch: row.get(13)?,
                    time_before: row.get(14)?,
                    authorized_at_ms: row.get(15)?,
                    slot_phase_before: row.get(16)?,
                    receipt_json: row.get(17)?,
                    receipt_digest: row.get(18)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_ROW_READ")
}

fn count_identity_matches(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupAuthorizationRecoveryKey,
) -> Result<i64> {
    let expected = key.receipt_expectation();
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_authorizations
               WHERE cleanup_id = ?1 OR candidate_token = ?2
                  OR quarantine_id = ?3 OR staging_id = ?4"#,
            params![
                key.cleanup_id(),
                key.candidate_token(),
                expected.quarantine_id,
                expected.staging_id,
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_IDENTITY_READ")
}

fn decode_recorded_row(
    key: &CandidateCleanupAuthorizationRecoveryKey,
    row: StoredCleanupAuthorizationRow,
) -> Result<HashedComputePluginCandidateCleanupAuthorizationReceipt> {
    let receipt: ComputePluginCandidateCleanupAuthorizationReceipt =
        serde_json::from_str(&row.receipt_json)
            .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_RECEIPT_PARSE")?;
    let expected = key.receipt_expectation();
    if serde_json::to_string(&receipt)? != row.receipt_json
        || receipt.schema != CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA
        || row.cleanup_id != key.cleanup_id()
        || row.candidate_token != key.candidate_token()
        || row.candidate_token_digest != expected.candidate_token_digest
        || row.quarantine_id != expected.quarantine_id
        || row.quarantine_receipt_digest != expected.quarantine_receipt_digest
        || row.staging_id != expected.staging_id
        || row.staging_run_digest != expected.staging_run_digest
        || row.state_before != expected.authority_state_revision_before
        || row.state_after != expected.authority_state_revision_after
        || row.inventory_revision != expected.inventory_revision
        || row.inventory_digest != expected.inventory_digest
        || row.epoch_before != expected.authority_epoch_before
        || row.epoch_after != expected.authority_epoch_after
        || row.process_owner_epoch != expected.process_owner_epoch
        || row.time_before != expected.trusted_time_high_water_ms_before
        || row.authorized_at_ms != expected.authorized_at_ms
        || row.slot_phase_before != SLOT_FAILED
        || receipt.cleanup_id != row.cleanup_id
        || receipt.candidate_token_digest != row.candidate_token_digest
        || receipt.quarantine_id != row.quarantine_id
        || receipt.quarantine_receipt_digest != row.quarantine_receipt_digest
        || receipt.staging_id != row.staging_id
        || receipt.staging_run_digest != row.staging_run_digest
        || receipt.authority_state_revision_before != row.state_before
        || receipt.authority_state_revision_after != row.state_after
        || receipt.inventory_revision != row.inventory_revision
        || receipt.inventory_digest != row.inventory_digest
        || receipt.authority_epoch_before != row.epoch_before
        || receipt.authority_epoch_after != row.epoch_after
        || receipt.process_owner_epoch != row.process_owner_epoch
        || receipt.trusted_time_high_water_ms_before != row.time_before
        || receipt.authorized_at_ms != row.authorized_at_ms
        || receipt.slot_phase_before != row.slot_phase_before
        || !is_sha256(&row.receipt_digest)
        || jcs_sha256_hex(&receipt)? != row.receipt_digest
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_RECOVERY_ROW_CHANGED");
    }
    Ok(HashedComputePluginCandidateCleanupAuthorizationReceipt {
        schema: HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA.to_string(),
        receipt,
        canonicalization: CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM.to_string(),
        receipt_digest: row.receipt_digest,
    })
}
