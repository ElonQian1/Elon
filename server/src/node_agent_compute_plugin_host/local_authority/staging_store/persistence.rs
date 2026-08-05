use anyhow::{bail, Context, Result};
use rusqlite::{named_params, params, Transaction};

use super::{
    projection::CandidateStagingProjection,
    types::{
        ComputePluginCandidateStagingReceipt, HashedComputePluginCandidateStagingReceipt,
        CANDIDATE_STAGING_RECEIPT_SCHEMA, HASHED_CANDIDATE_STAGING_RECEIPT_SCHEMA,
    },
};
use crate::node_agent_compute_plugin_host::{
    candidate_staging_contract::ValidatedCandidateStagingStorePermit,
    lifecycle::SLOT_STAGED,
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::plan_application::AuthorityPlanApplicationState;

pub(super) fn insert_candidate_staging_receipt(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidateStagingStorePermit<'_>,
    authority: &AuthorityPlanApplicationState,
    projection: &CandidateStagingProjection,
    staged_at_ms: i64,
) -> Result<HashedComputePluginCandidateStagingReceipt> {
    let key = permit.key();
    let plan = permit.plan();
    let evidence = permit.evidence();
    let seal = permit.seal();
    let receipt = ComputePluginCandidateStagingReceipt {
        schema: CANDIDATE_STAGING_RECEIPT_SCHEMA.to_string(),
        staging_id: permit.staging_id().to_string(),
        candidate_token_digest: key.candidate_token_digest().to_string(),
        owner_plan_id: key.owner_plan_id().to_string(),
        owner_plan_digest: key.owner_plan_digest().to_string(),
        verification_id: key.verification_id().to_string(),
        verification_generation: key.verification_generation(),
        candidate_generation: key.candidate_generation(),
        application_inventory_revision: key.application_inventory_revision(),
        verification_result_digest: permit.binding().verification_result_digest().to_string(),
        root_identity_digest: evidence.evidence.root_identity_digest.clone(),
        staging_run_digest: evidence.evidence.staging_run_digest.clone(),
        extraction_plan_digest: plan.plan_digest.clone(),
        extraction_evidence_digest: evidence.evidence_digest.clone(),
        staging_seal_payload_digest: seal.payload_digest.clone(),
        staging_seal_file_digest: seal.file_digest.clone(),
        staging_seal_identity_digest: seal.file_identity_digest.clone(),
        staging_seal_size_bytes: seal.size_bytes,
        extracted_file_count: evidence.evidence.extracted_file_count,
        extracted_bytes: evidence.evidence.extracted_bytes,
        authority_state_revision_before: authority.state_revision,
        authority_state_revision_after: projection.state_revision,
        inventory_revision_before: authority.inventory.inventory_revision,
        inventory_revision_after: projection.inventory.inventory_revision,
        inventory_digest_before: authority.inventory_digest.clone(),
        inventory_digest_after: projection.inventory_digest.clone(),
        authority_epoch_before: authority.authority_epoch,
        authority_epoch_after: projection.authority_epoch,
        process_owner_epoch: authority.process_owner_epoch,
        staged_at_ms,
        slot_phase_after: SLOT_STAGED.to_string(),
    };
    let receipt_json =
        serde_json::to_string(&receipt).context("COMPUTE_PLUGIN_STAGING_RECEIPT_JSON")?;
    let receipt_digest = jcs_sha256_hex(&receipt)?;
    let extraction_plan_json =
        serde_json::to_string(plan).context("COMPUTE_PLUGIN_STAGING_EXTRACTION_PLAN_JSON")?;
    let extraction_evidence_json = serde_json::to_string(evidence)
        .context("COMPUTE_PLUGIN_STAGING_EXTRACTION_EVIDENCE_JSON")?;
    let staging_seal_json =
        serde_json::to_string(seal).context("COMPUTE_PLUGIN_STAGING_SEAL_JSON")?;

    transaction
        .execute(
            r#"INSERT INTO candidate_staging_receipts (
                staging_id, candidate_token, candidate_token_digest,
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
            ) VALUES (
                :staging_id, :candidate_token, :candidate_token_digest,
                :owner_plan_id, :owner_plan_digest, :verification_id,
                :verification_generation, :candidate_generation,
                :application_inventory_revision, :verification_result_digest,
                :root_identity_digest, :staging_run_digest,
                :extraction_plan_json, :extraction_plan_digest,
                :extraction_evidence_json, :extraction_evidence_digest,
                :staging_seal_json, :staging_seal_payload_digest,
                :staging_seal_file_digest, :staging_seal_identity_digest,
                :staging_seal_size_bytes, :extracted_file_count, :extracted_bytes,
                :authority_state_revision_before, :authority_state_revision_after,
                :inventory_revision_before, :inventory_revision_after,
                :inventory_digest_before, :inventory_digest_after, :inventory_json_after,
                :authority_epoch_before, :authority_epoch_after, :process_owner_epoch,
                :staged_at_ms, :receipt_json, :receipt_digest
            )"#,
            named_params! {
                ":staging_id": receipt.staging_id.as_str(),
                ":candidate_token": key.candidate_token(),
                ":candidate_token_digest": receipt.candidate_token_digest.as_str(),
                ":owner_plan_id": receipt.owner_plan_id.as_str(),
                ":owner_plan_digest": receipt.owner_plan_digest.as_str(),
                ":verification_id": receipt.verification_id.as_str(),
                ":verification_generation": receipt.verification_generation,
                ":candidate_generation": receipt.candidate_generation,
                ":application_inventory_revision": receipt.application_inventory_revision,
                ":verification_result_digest": receipt.verification_result_digest.as_str(),
                ":root_identity_digest": receipt.root_identity_digest.as_str(),
                ":staging_run_digest": receipt.staging_run_digest.as_str(),
                ":extraction_plan_json": extraction_plan_json.as_str(),
                ":extraction_plan_digest": receipt.extraction_plan_digest.as_str(),
                ":extraction_evidence_json": extraction_evidence_json.as_str(),
                ":extraction_evidence_digest": receipt.extraction_evidence_digest.as_str(),
                ":staging_seal_json": staging_seal_json.as_str(),
                ":staging_seal_payload_digest": receipt.staging_seal_payload_digest.as_str(),
                ":staging_seal_file_digest": receipt.staging_seal_file_digest.as_str(),
                ":staging_seal_identity_digest": receipt.staging_seal_identity_digest.as_str(),
                ":staging_seal_size_bytes": receipt.staging_seal_size_bytes,
                ":extracted_file_count": receipt.extracted_file_count,
                ":extracted_bytes": receipt.extracted_bytes,
                ":authority_state_revision_before": receipt.authority_state_revision_before,
                ":authority_state_revision_after": receipt.authority_state_revision_after,
                ":inventory_revision_before": receipt.inventory_revision_before,
                ":inventory_revision_after": receipt.inventory_revision_after,
                ":inventory_digest_before": receipt.inventory_digest_before.as_str(),
                ":inventory_digest_after": receipt.inventory_digest_after.as_str(),
                ":inventory_json_after": projection.inventory_json.as_str(),
                ":authority_epoch_before": receipt.authority_epoch_before,
                ":authority_epoch_after": receipt.authority_epoch_after,
                ":process_owner_epoch": receipt.process_owner_epoch,
                ":staged_at_ms": receipt.staged_at_ms,
                ":receipt_json": receipt_json.as_str(),
                ":receipt_digest": receipt_digest.as_str(),
            },
        )
        .context("COMPUTE_PLUGIN_STAGING_RECEIPT_INSERT")?;

    validate_receipt_readback(
        transaction,
        key.candidate_token(),
        &receipt,
        &receipt_json,
        &receipt_digest,
        &extraction_plan_json,
        &extraction_evidence_json,
        &staging_seal_json,
        &projection.inventory_json,
    )?;
    Ok(HashedComputePluginCandidateStagingReceipt {
        schema: HASHED_CANDIDATE_STAGING_RECEIPT_SCHEMA.to_string(),
        receipt,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        receipt_digest,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_receipt_readback(
    transaction: &Transaction<'_>,
    candidate_token: &str,
    receipt: &ComputePluginCandidateStagingReceipt,
    receipt_json: &str,
    receipt_digest: &str,
    extraction_plan_json: &str,
    extraction_evidence_json: &str,
    staging_seal_json: &str,
    inventory_json_after: &str,
) -> Result<()> {
    let stored = transaction
        .query_row(
            r#"SELECT receipt_json, receipt_digest, extraction_plan_json,
                extraction_evidence_json, staging_seal_json, inventory_json_after
            FROM candidate_staging_receipts
            WHERE staging_id = ?1 AND candidate_token = ?2 AND verification_id = ?3"#,
            params![
                receipt.staging_id.as_str(),
                candidate_token,
                receipt.verification_id.as_str()
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_STAGING_RECEIPT_READBACK")?;
    let parsed: ComputePluginCandidateStagingReceipt =
        serde_json::from_str(&stored.0).context("COMPUTE_PLUGIN_STAGING_RECEIPT_READBACK_JSON")?;
    if &parsed != receipt
        || stored.0 != receipt_json
        || stored.1 != receipt_digest
        || stored.2 != extraction_plan_json
        || stored.3 != extraction_evidence_json
        || stored.4 != staging_seal_json
        || stored.5 != inventory_json_after
        || jcs_sha256_hex(&parsed)? != receipt_digest
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECEIPT_READBACK_CHANGED");
    }
    Ok(())
}
