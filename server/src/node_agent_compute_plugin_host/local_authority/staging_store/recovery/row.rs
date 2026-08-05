use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::node_agent_compute_plugin_host::candidate_staging_contract::ComputePluginCandidateStagingRecoveryKey;

use super::super::types::ComputePluginCandidateStagingReceipt;

pub(super) struct CandidateStagingRow {
    pub(super) staging_id: String,
    pub(super) candidate_token: String,
    pub(super) candidate_token_digest: String,
    pub(super) owner_plan_id: String,
    pub(super) owner_plan_digest: String,
    pub(super) verification_id: String,
    pub(super) verification_generation: i64,
    pub(super) candidate_generation: i64,
    pub(super) application_inventory_revision: i64,
    pub(super) verification_result_digest: String,
    pub(super) root_identity_digest: String,
    pub(super) staging_run_digest: String,
    pub(super) extraction_plan_json: String,
    pub(super) extraction_plan_digest: String,
    pub(super) extraction_evidence_json: String,
    pub(super) extraction_evidence_digest: String,
    pub(super) staging_seal_json: String,
    pub(super) staging_seal_payload_digest: String,
    pub(super) staging_seal_file_digest: String,
    pub(super) staging_seal_identity_digest: String,
    pub(super) staging_seal_size_bytes: i64,
    pub(super) extracted_file_count: i64,
    pub(super) extracted_bytes: i64,
    pub(super) authority_state_revision_before: i64,
    pub(super) authority_state_revision_after: i64,
    pub(super) inventory_revision_before: i64,
    pub(super) inventory_revision_after: i64,
    pub(super) inventory_digest_before: String,
    pub(super) inventory_digest_after: String,
    pub(super) inventory_json_after: String,
    pub(super) authority_epoch_before: i64,
    pub(super) authority_epoch_after: i64,
    pub(super) process_owner_epoch: i64,
    pub(super) staged_at_ms: i64,
    pub(super) receipt_json: String,
    pub(super) receipt_digest: String,
}

pub(super) fn read_exact_candidate_staging(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
) -> Result<Option<CandidateStagingRow>> {
    transaction
        .query_row(
            r#"SELECT staging_id, candidate_token, candidate_token_digest,
                owner_plan_id, owner_plan_digest, verification_id,
                verification_generation, candidate_generation,
                application_inventory_revision, verification_result_digest,
                root_identity_digest, staging_run_digest,
                extraction_plan_json, extraction_plan_digest,
                extraction_evidence_json, extraction_evidence_digest,
                staging_seal_json, staging_seal_payload_digest,
                staging_seal_file_digest, staging_seal_identity_digest,
                staging_seal_size_bytes, extracted_file_count, extracted_bytes,
                authority_state_revision_before, authority_state_revision_after,
                inventory_revision_before, inventory_revision_after,
                inventory_digest_before, inventory_digest_after, inventory_json_after,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                staged_at_ms, receipt_json, receipt_digest
            FROM candidate_staging_receipts
            WHERE staging_id = ?1 AND candidate_token = ?2 AND verification_id = ?3"#,
            params![
                key.staging_id(),
                key.candidate_token(),
                key.verification_id()
            ],
            |row| {
                Ok(CandidateStagingRow {
                    staging_id: row.get(0)?,
                    candidate_token: row.get(1)?,
                    candidate_token_digest: row.get(2)?,
                    owner_plan_id: row.get(3)?,
                    owner_plan_digest: row.get(4)?,
                    verification_id: row.get(5)?,
                    verification_generation: row.get(6)?,
                    candidate_generation: row.get(7)?,
                    application_inventory_revision: row.get(8)?,
                    verification_result_digest: row.get(9)?,
                    root_identity_digest: row.get(10)?,
                    staging_run_digest: row.get(11)?,
                    extraction_plan_json: row.get(12)?,
                    extraction_plan_digest: row.get(13)?,
                    extraction_evidence_json: row.get(14)?,
                    extraction_evidence_digest: row.get(15)?,
                    staging_seal_json: row.get(16)?,
                    staging_seal_payload_digest: row.get(17)?,
                    staging_seal_file_digest: row.get(18)?,
                    staging_seal_identity_digest: row.get(19)?,
                    staging_seal_size_bytes: row.get(20)?,
                    extracted_file_count: row.get(21)?,
                    extracted_bytes: row.get(22)?,
                    authority_state_revision_before: row.get(23)?,
                    authority_state_revision_after: row.get(24)?,
                    inventory_revision_before: row.get(25)?,
                    inventory_revision_after: row.get(26)?,
                    inventory_digest_before: row.get(27)?,
                    inventory_digest_after: row.get(28)?,
                    inventory_json_after: row.get(29)?,
                    authority_epoch_before: row.get(30)?,
                    authority_epoch_after: row.get(31)?,
                    process_owner_epoch: row.get(32)?,
                    staged_at_ms: row.get(33)?,
                    receipt_json: row.get(34)?,
                    receipt_digest: row.get(35)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_EXACT_READ")
}

pub(super) fn count_candidate_staging_identity_matches(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_staging_receipts
            WHERE staging_id = ?1 OR candidate_token = ?2 OR candidate_token_digest = ?3
               OR verification_id = ?4 OR staging_run_digest = ?5"#,
            params![
                key.staging_id(),
                key.candidate_token(),
                key.candidate_token_digest(),
                key.verification_id(),
                key.staging_run_digest(),
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_COLLISION_READ")
}

impl CandidateStagingRow {
    pub(super) fn matches_receipt(&self, receipt: &ComputePluginCandidateStagingReceipt) -> bool {
        self.staging_id == receipt.staging_id
            && self.candidate_token_digest == receipt.candidate_token_digest
            && self.owner_plan_id == receipt.owner_plan_id
            && self.owner_plan_digest == receipt.owner_plan_digest
            && self.verification_id == receipt.verification_id
            && self.verification_generation == receipt.verification_generation
            && self.candidate_generation == receipt.candidate_generation
            && self.application_inventory_revision == receipt.application_inventory_revision
            && self.verification_result_digest == receipt.verification_result_digest
            && self.root_identity_digest == receipt.root_identity_digest
            && self.staging_run_digest == receipt.staging_run_digest
            && self.extraction_plan_digest == receipt.extraction_plan_digest
            && self.extraction_evidence_digest == receipt.extraction_evidence_digest
            && self.staging_seal_payload_digest == receipt.staging_seal_payload_digest
            && self.staging_seal_file_digest == receipt.staging_seal_file_digest
            && self.staging_seal_identity_digest == receipt.staging_seal_identity_digest
            && self.staging_seal_size_bytes == receipt.staging_seal_size_bytes
            && self.extracted_file_count == receipt.extracted_file_count
            && self.extracted_bytes == receipt.extracted_bytes
            && self.authority_state_revision_before == receipt.authority_state_revision_before
            && self.authority_state_revision_after == receipt.authority_state_revision_after
            && self.inventory_revision_before == receipt.inventory_revision_before
            && self.inventory_revision_after == receipt.inventory_revision_after
            && self.inventory_digest_before == receipt.inventory_digest_before
            && self.inventory_digest_after == receipt.inventory_digest_after
            && self.authority_epoch_before == receipt.authority_epoch_before
            && self.authority_epoch_after == receipt.authority_epoch_after
            && self.process_owner_epoch == receipt.process_owner_epoch
            && self.staged_at_ms == receipt.staged_at_ms
    }
}
