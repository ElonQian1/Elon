use anyhow::{bail, Context, Result};
use rusqlite::Transaction;
use sha2::{Digest, Sha256};

use super::row::CandidateStagingRow;
use crate::node_agent_compute_plugin_host::{
    candidate_extraction::{
        ComputePluginStagingSealEvidence, HashedComputePluginArchiveExtractionPlan,
        HashedComputePluginExtractedArchiveEvidence, COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA,
        EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA, HASHED_COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA,
        HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA, STAGING_EVIDENCE_CANONICALIZATION,
        STAGING_EVIDENCE_DIGEST_ALGORITHM, STAGING_SEAL_EVIDENCE_SCHEMA,
        STAGING_SEAL_PAYLOAD_SCHEMA,
    },
    candidate_staging_contract::ComputePluginCandidateStagingRecoveryKey,
    candidate_verification_terminal_result::{
        parse_candidate_verification_resolution, CandidateVerificationResolutionKind,
    },
    identity::ComputePluginReleaseRef,
    install_plan_admission::validate_inventory,
    install_plan_admission_validation::is_identifier,
    lifecycle::{ComputePluginInventorySnapshot, SLOT_STAGED, SLOT_VERIFYING},
    manifest_validation::{is_normalized_relative_path, is_sha256},
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::super::plan_application::AuthorityPlanApplicationState;
use super::super::{
    recovery_session::ComputePluginCandidateStagingRecoveryAuthoritySession,
    types::{
        ComputePluginCandidateStagingReceipt, HashedComputePluginCandidateStagingReceipt,
        CANDIDATE_STAGING_RECEIPT_SCHEMA, HASHED_CANDIDATE_STAGING_RECEIPT_SCHEMA,
    },
};

pub(super) fn validate_recovery_provenance(
    session: &ComputePluginCandidateStagingRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
) -> Result<()> {
    let expected = key.receipt_expectation();
    let slot = key.slot_expectation();
    let digests = [
        key.installation_id_digest(),
        key.clock_epoch_digest(),
        key.candidate_token_digest(),
        expected.owner_plan_digest.as_str(),
        expected.verification_result_digest.as_str(),
        expected.root_identity_digest.as_str(),
        expected.staging_run_digest.as_str(),
        expected.extraction_plan_digest.as_str(),
        expected.extraction_evidence_digest.as_str(),
        expected.staging_seal_payload_digest.as_str(),
        expected.staging_seal_file_digest.as_str(),
        expected.staging_seal_identity_digest.as_str(),
        expected.inventory_digest_before.as_str(),
        slot.release.manifest_digest.as_str(),
        slot.release.package_digest.as_str(),
    ];
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.process_owner_epoch() != session.process_owner_epoch()
        || digests.iter().any(|digest| !is_sha256(digest))
        || !is_identifier(key.staging_id())
        || !is_identifier(key.candidate_token())
        || !is_identifier(expected.owner_plan_id.as_str())
        || !is_identifier(expected.verification_id.as_str())
        || !is_identifier(slot.plugin_id.as_str())
        || !is_identifier(slot.slot_ref.as_str())
        || !is_identifier(slot.release.plugin_id.as_str())
        || !is_identifier(slot.release.plugin_version.as_str())
        || !is_identifier(slot.release.target_id.as_str())
        || slot.release.plugin_id != slot.plugin_id
        || expected.verification_generation <= 0
        || expected.candidate_generation <= 0
        || expected.application_inventory_revision <= 0
        || expected.verification_resolved_at_ms < 0
        || expected.staging_seal_size_bytes <= 0
        || expected.staging_seal_size_bytes > 1_048_576
        || expected.extracted_file_count <= 0
        || expected.extracted_file_count > 4_096
        || expected.extracted_bytes <= 0
        || expected.extracted_bytes > 68_719_476_736
        || expected.authority_state_revision_before <= 0
        || expected.inventory_revision_before <= 0
        || expected.authority_epoch_before <= 0
        || expected.process_owner_epoch <= 0
        || expected.trusted_time_high_water_ms_before < 0
        || session.trusted_now_ms() < expected.trusted_time_high_water_ms_before
        || jcs_sha256_hex(&key.candidate_token())? != key.candidate_token_digest()
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_PROVENANCE_CHANGED");
    }
    Ok(())
}

pub(super) fn validate_staged_row(
    session: &ComputePluginCandidateStagingRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
    authority: &AuthorityPlanApplicationState,
    row: &CandidateStagingRow,
) -> Result<HashedComputePluginCandidateStagingReceipt> {
    let receipt: ComputePluginCandidateStagingReceipt = serde_json::from_str(&row.receipt_json)
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_RECEIPT_JSON")?;
    validate_receipt(key, row, &receipt)?;
    validate_evidence_envelopes(key, row, &receipt)?;
    let inventory_after = validate_inventory_after(session, key, row, &receipt)?;
    validate_staged_authority(key, authority, &receipt, &inventory_after)?;
    Ok(HashedComputePluginCandidateStagingReceipt {
        schema: HASHED_CANDIDATE_STAGING_RECEIPT_SCHEMA.to_string(),
        receipt,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        receipt_digest: row.receipt_digest.clone(),
    })
}

fn validate_receipt(
    key: &ComputePluginCandidateStagingRecoveryKey,
    row: &CandidateStagingRow,
    receipt: &ComputePluginCandidateStagingReceipt,
) -> Result<()> {
    let expected = key.receipt_expectation();
    let serialized = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_RECEIPT_SERIALIZE")?;
    if receipt.schema != CANDIDATE_STAGING_RECEIPT_SCHEMA
        || row.candidate_token != key.candidate_token()
        || !row.matches_receipt(receipt)
        || serialized != row.receipt_json
        || !is_sha256(&row.receipt_digest)
        || jcs_sha256_hex(receipt)? != row.receipt_digest
        || receipt.staging_id != key.staging_id()
        || receipt.candidate_token_digest != key.candidate_token_digest()
        || receipt.owner_plan_id != expected.owner_plan_id
        || receipt.owner_plan_digest != expected.owner_plan_digest
        || receipt.verification_id != expected.verification_id
        || receipt.verification_generation != expected.verification_generation
        || receipt.candidate_generation != expected.candidate_generation
        || receipt.application_inventory_revision != expected.application_inventory_revision
        || receipt.verification_result_digest != expected.verification_result_digest
        || receipt.root_identity_digest != expected.root_identity_digest
        || receipt.staging_run_digest != expected.staging_run_digest
        || receipt.extraction_plan_digest != expected.extraction_plan_digest
        || receipt.extraction_evidence_digest != expected.extraction_evidence_digest
        || receipt.staging_seal_payload_digest != expected.staging_seal_payload_digest
        || receipt.staging_seal_file_digest != expected.staging_seal_file_digest
        || receipt.staging_seal_identity_digest != expected.staging_seal_identity_digest
        || receipt.staging_seal_size_bytes != expected.staging_seal_size_bytes
        || receipt.extracted_file_count != expected.extracted_file_count
        || receipt.extracted_bytes != expected.extracted_bytes
        || receipt.authority_state_revision_before != expected.authority_state_revision_before
        || receipt.authority_state_revision_after
            != expected
                .authority_state_revision_before
                .checked_add(1)
                .unwrap_or(-1)
        || receipt.inventory_revision_before != expected.inventory_revision_before
        || receipt.inventory_revision_after
            != expected
                .inventory_revision_before
                .checked_add(1)
                .unwrap_or(-1)
        || receipt.inventory_digest_before != expected.inventory_digest_before
        || !is_sha256(&receipt.inventory_digest_after)
        || receipt.inventory_digest_after == receipt.inventory_digest_before
        || receipt.authority_epoch_before != expected.authority_epoch_before
        || receipt.authority_epoch_after
            != expected.authority_epoch_before.checked_add(1).unwrap_or(-1)
        || receipt.process_owner_epoch != expected.process_owner_epoch
        || receipt.staged_at_ms <= expected.verification_resolved_at_ms
        || receipt.staged_at_ms <= expected.trusted_time_high_water_ms_before
        || receipt.slot_phase_after != SLOT_STAGED
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_RECEIPT_CHANGED");
    }
    Ok(())
}

fn validate_evidence_envelopes(
    key: &ComputePluginCandidateStagingRecoveryKey,
    row: &CandidateStagingRow,
    receipt: &ComputePluginCandidateStagingReceipt,
) -> Result<()> {
    let plan: HashedComputePluginArchiveExtractionPlan =
        serde_json::from_str(&row.extraction_plan_json)
            .context("COMPUTE_PLUGIN_STAGING_RECOVERY_PLAN_JSON")?;
    let evidence: HashedComputePluginExtractedArchiveEvidence =
        serde_json::from_str(&row.extraction_evidence_json)
            .context("COMPUTE_PLUGIN_STAGING_RECOVERY_EVIDENCE_JSON")?;
    let seal: ComputePluginStagingSealEvidence = serde_json::from_str(&row.staging_seal_json)
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_SEAL_JSON")?;
    let plan_json =
        serde_json::to_string(&plan).context("COMPUTE_PLUGIN_STAGING_RECOVERY_PLAN_SERIALIZE")?;
    let evidence_json = serde_json::to_string(&evidence)
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_EVIDENCE_SERIALIZE")?;
    let seal_json = serde_json::to_string(&seal)
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_SEAL_ENVELOPE_SERIALIZE")?;
    let slot = key.slot_expectation();
    let seal_bytes = serde_json::to_vec(&seal.payload)
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_SEAL_SERIALIZE")?;
    let plan_files_match = plan
        .plan
        .files
        .iter()
        .zip(evidence.evidence.files.iter())
        .all(|(planned, extracted)| {
            planned.relative_path == extracted.relative_path
                && planned.expected_digest == extracted.digest
                && planned.expected_size_bytes == extracted.size_bytes
                && is_normalized_relative_path(&extracted.relative_path)
                && is_sha256(&extracted.digest)
                && is_sha256(&extracted.file_identity_digest)
                && extracted.size_bytes >= 0
        });
    if plan.schema != HASHED_COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA
        || plan_json != row.extraction_plan_json
        || plan.plan.schema != COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA
        || plan.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || plan.plan_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || jcs_sha256_hex(&plan.plan)? != plan.plan_digest
        || plan.plan_digest != receipt.extraction_plan_digest
        || plan.plan.release != slot.release
        || plan.plan.package_digest != slot.release.package_digest
        || plan.plan.unpacked_size_bytes != receipt.extracted_bytes
        || plan.plan.files.len() != evidence.evidence.files.len()
        || !plan_files_match
        || evidence.schema != HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA
        || evidence_json != row.extraction_evidence_json
        || evidence.evidence.schema != EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA
        || evidence.canonicalization != STAGING_EVIDENCE_CANONICALIZATION
        || evidence.digest_algorithm != STAGING_EVIDENCE_DIGEST_ALGORITHM
        || jcs_sha256_hex(&evidence.evidence)? != evidence.evidence_digest
        || evidence.evidence_digest != receipt.extraction_evidence_digest
        || evidence.evidence.installation_id_digest != key.installation_id_digest()
        || evidence.evidence.root_identity_digest != receipt.root_identity_digest
        || evidence.evidence.candidate_token_digest != receipt.candidate_token_digest
        || evidence.evidence.staging_run_digest != receipt.staging_run_digest
        || evidence.evidence.extraction_plan_digest != receipt.extraction_plan_digest
        || evidence.evidence.extracted_file_count != receipt.extracted_file_count
        || evidence.evidence.extracted_bytes != receipt.extracted_bytes
        || i64::try_from(evidence.evidence.files.len()).ok() != Some(receipt.extracted_file_count)
        || seal.schema != STAGING_SEAL_EVIDENCE_SCHEMA
        || seal_json != row.staging_seal_json
        || seal.payload.schema != STAGING_SEAL_PAYLOAD_SCHEMA
        || seal.canonicalization != STAGING_EVIDENCE_CANONICALIZATION
        || seal.digest_algorithm != STAGING_EVIDENCE_DIGEST_ALGORITHM
        || jcs_sha256_hex(&seal.payload)? != seal.payload_digest
        || seal.payload_digest != receipt.staging_seal_payload_digest
        || sha256_hex(&seal_bytes) != receipt.staging_seal_file_digest
        || seal.file_digest != receipt.staging_seal_file_digest
        || seal.file_identity_digest != receipt.staging_seal_identity_digest
        || seal.size_bytes != receipt.staging_seal_size_bytes
        || i64::try_from(seal_bytes.len()).ok() != Some(receipt.staging_seal_size_bytes)
        || seal.payload.installation_id_digest != key.installation_id_digest()
        || seal.payload.root_identity_digest != receipt.root_identity_digest
        || seal.payload.candidate_token_digest != receipt.candidate_token_digest
        || seal.payload.staging_run_digest != receipt.staging_run_digest
        || seal.payload.extraction_plan_digest != receipt.extraction_plan_digest
        || seal.payload.extraction_evidence_digest != receipt.extraction_evidence_digest
        || seal.payload.extracted_file_count != receipt.extracted_file_count
        || seal.payload.extracted_bytes != receipt.extracted_bytes
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_EVIDENCE_CHANGED");
    }
    Ok(())
}

fn validate_inventory_after(
    session: &ComputePluginCandidateStagingRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
    row: &CandidateStagingRow,
    receipt: &ComputePluginCandidateStagingReceipt,
) -> Result<ComputePluginInventorySnapshot> {
    let inventory: ComputePluginInventorySnapshot = serde_json::from_str(&row.inventory_json_after)
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_INVENTORY_JSON")?;
    validate_inventory(&inventory, session.trusted_now.clone())?;
    if inventory.inventory_revision != receipt.inventory_revision_after
        || jcs_sha256_hex(&inventory)? != receipt.inventory_digest_after
        || !inventory_has_slot(&inventory, key, SLOT_STAGED)
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_INVENTORY_CHANGED");
    }
    Ok(inventory)
}

fn validate_staged_authority(
    key: &ComputePluginCandidateStagingRecoveryKey,
    authority: &AuthorityPlanApplicationState,
    receipt: &ComputePluginCandidateStagingReceipt,
    inventory_after: &ComputePluginInventorySnapshot,
) -> Result<()> {
    if authority.installation_id_digest != key.installation_id_digest()
        || authority.state_revision < receipt.authority_state_revision_after
        || authority.inventory.inventory_revision < receipt.inventory_revision_after
        || authority.authority_epoch < receipt.authority_epoch_after
        || authority.process_owner_epoch != receipt.process_owner_epoch
        || authority.trusted_time_high_water_ms < receipt.staged_at_ms
        || (authority.inventory.inventory_revision == receipt.inventory_revision_after
            && (authority.inventory_digest != receipt.inventory_digest_after
                || &authority.inventory != inventory_after))
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_AUTHORITY_ROLLBACK");
    }
    Ok(())
}

pub(super) fn validate_not_created(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateStagingRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
    authority: &AuthorityPlanApplicationState,
) -> Result<()> {
    let expected = key.receipt_expectation();
    if authority.installation_id_digest != key.installation_id_digest()
        || authority.state_revision != expected.authority_state_revision_before
        || authority.inventory.inventory_revision != expected.inventory_revision_before
        || authority.inventory_digest != expected.inventory_digest_before
        || authority.authority_epoch != expected.authority_epoch_before
        || authority.process_owner_epoch != expected.process_owner_epoch
        || authority.trusted_time_high_water_ms != expected.trusted_time_high_water_ms_before
        || !inventory_has_slot(&authority.inventory, key, SLOT_VERIFYING)
        || session.trusted_now_ms() < authority.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_NOT_CREATED_AUTHORITY_CHANGED");
    }
    validate_candidate_owner(transaction, key)?;
    validate_verified_run(transaction, key)
}

fn validate_candidate_owner(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
) -> Result<()> {
    let row = transaction
        .query_row(
            r#"SELECT plugin_id, slot_ref, candidate_generation, release_json,
                owner_plan_id, owner_plan_digest, application_inventory_revision, state
            FROM candidate_owners WHERE candidate_token = ?1"#,
            [key.candidate_token()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_OWNER_READ")?;
    let release: ComputePluginReleaseRef =
        serde_json::from_str(&row.3).context("COMPUTE_PLUGIN_STAGING_RECOVERY_OWNER_RELEASE")?;
    let expected = key.receipt_expectation();
    let slot = key.slot_expectation();
    if row.0 != slot.plugin_id
        || row.1 != slot.slot_ref
        || row.2 != expected.candidate_generation
        || release != slot.release
        || row.4 != expected.owner_plan_id
        || row.5 != expected.owner_plan_digest
        || row.6 != expected.application_inventory_revision
        || row.7 != "owned"
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_OWNER_CHANGED");
    }
    Ok(())
}

fn validate_verified_run(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateStagingRecoveryKey,
) -> Result<()> {
    let row = transaction
        .query_row(
            r#"SELECT candidate_token, owner_plan_id, owner_plan_digest,
                verification_generation, candidate_generation,
                application_inventory_revision, authority_state_revision,
                authority_epoch, process_owner_epoch, state, resolved_at_ms,
                resolution_reason, result_json, result_digest
            FROM candidate_verification_runs WHERE verification_id = ?1"#,
            [key.verification_id()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_STAGING_RECOVERY_VERIFICATION_READ")?;
    let expected = key.receipt_expectation();
    let result_json = row.12.as_deref().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_RECOVERY_VERIFICATION_RESULT_MISSING")
    })?;
    let result_digest = row.13.as_deref().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_RECOVERY_VERIFICATION_DIGEST_MISSING")
    })?;
    let result = parse_candidate_verification_resolution(result_json, result_digest)?;
    if row.0 != key.candidate_token()
        || row.1 != expected.owner_plan_id
        || row.2 != expected.owner_plan_digest
        || row.3 != expected.verification_generation
        || row.4 != expected.candidate_generation
        || row.5 != expected.application_inventory_revision
        || row.6.checked_add(1) != Some(expected.authority_state_revision_before)
        || row.7.checked_add(1) != Some(expected.authority_epoch_before)
        || row.8 != expected.process_owner_epoch
        || row.9 != "verified"
        || row.10 != Some(expected.verification_resolved_at_ms)
        || row.11.as_deref() != Some("artifact_set_verified")
        || row.13.as_deref() != Some(expected.verification_result_digest.as_str())
        || result.kind != CandidateVerificationResolutionKind::Verified
        || result.verification_id != expected.verification_id
        || result.candidate_token_digest != key.candidate_token_digest()
        || result.owner_plan_id != expected.owner_plan_id
        || result.owner_plan_digest != expected.owner_plan_digest
        || result.verification_generation != expected.verification_generation
        || result.candidate_generation != expected.candidate_generation
        || result.resolved_at_ms != expected.verification_resolved_at_ms
        || result.authority_state_revision_before != row.6
        || result.authority_state_revision_after != expected.authority_state_revision_before
        || result.inventory_revision_after != expected.inventory_revision_before
        || result.inventory_digest_after != expected.inventory_digest_before
        || result.authority_epoch_before != row.7
        || result.authority_epoch_after != expected.authority_epoch_before
        || result.slot_phase_after != SLOT_VERIFYING
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECOVERY_VERIFICATION_CHANGED");
    }
    Ok(())
}

fn inventory_has_slot(
    inventory: &ComputePluginInventorySnapshot,
    key: &ComputePluginCandidateStagingRecoveryKey,
    phase: &str,
) -> bool {
    let expected = key.slot_expectation();
    let mut records = inventory.plugins.iter().filter(|record| {
        record.plugin_id == expected.plugin_id
            && record.candidate_slot_ref.as_deref() == Some(expected.slot_ref.as_str())
    });
    let Some(record) = records.next() else {
        return false;
    };
    if records.next().is_some() {
        return false;
    }
    let mut slots = record
        .slots
        .iter()
        .filter(|slot| slot.slot_ref == expected.slot_ref && slot.release == expected.release);
    slots.next().is_some_and(|slot| slot.phase == phase) && slots.next().is_none()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
