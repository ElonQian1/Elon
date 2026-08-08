use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::types::{
    ComputePluginSharingPolicyCapabilityRevocationReceipt,
    HashedComputePluginSharingPolicyCapabilityRevocationReceipt, PolicyPreparedWorkSet,
    PreparedPolicyCapabilityRevocation, StoredPolicyCapabilityRevocation,
    COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA,
    COMPUTE_PLUGIN_SHARING_POLICY_PREPARED_WORK_SET_SCHEMA, FETCH_POLICY_TERMINAL_REASON,
    HASHED_COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA,
    VERIFICATION_POLICY_TERMINAL_REASON,
};
use super::work_set::{
    read_prepared_work_set, read_terminalized_work_set, MAX_PREPARED_FETCH_CLAIMS,
    MAX_PREPARED_VERIFICATIONS,
};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_terminal_result::{
        encode_candidate_verification_abort, validate_candidate_verification_terminal_result,
        CandidateVerificationTerminalKind,
    },
    local_authority::sharing_policy_binding::types::{
        HashedComputePluginSharingPolicyBindingReceipt, PreparedSharingPolicyBindingRequest,
        ProjectedSharingPolicyBinding,
    },
    manifest_validation::is_sha256,
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

pub(in super::super) fn prepare_revocation(
    transaction: &Transaction<'_>,
    projected: &ProjectedSharingPolicyBinding,
) -> Result<PreparedPolicyCapabilityRevocation> {
    let (work_set, fetch_count, verification_count) =
        read_prepared_work_set(transaction, projected)?;
    let fetch_claim_count =
        i64::try_from(fetch_count).context("COMPUTE_PLUGIN_POLICY_REVOCATION_FETCH_COUNT_RANGE")?;
    let verification_count = i64::try_from(verification_count)
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_VERIFICATION_COUNT_RANGE")?;
    let work_item_count = fetch_claim_count
        .checked_add(verification_count)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_REVOCATION_COUNT_EXHAUSTED"))?;
    let work_set_json =
        serde_json::to_string(&work_set).context("COMPUTE_PLUGIN_POLICY_REVOCATION_WORK_JSON")?;
    let work_set_digest = jcs_sha256_hex(&work_set)?;
    let bound_at_ms = projected.hashed_receipt.receipt.bound_at_ms;
    let (verification_result_json, verification_result_digest) =
        encode_candidate_verification_abort(VERIFICATION_POLICY_TERMINAL_REASON, bound_at_ms)?;
    let receipt = ComputePluginSharingPolicyCapabilityRevocationReceipt {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA.to_string(),
        policy_revision: projected.request.policy_revision,
        request_digest: projected.request.request_digest.clone(),
        policy_binding_receipt_digest: projected.hashed_receipt.receipt_digest.clone(),
        installation_id_digest: projected.request.installation_id_digest.clone(),
        authority_epoch_before: projected.before.authority_epoch,
        process_owner_epoch: projected.before.process_owner_epoch,
        trusted_time_before_ms: projected.before.trusted_time_high_water_ms,
        bound_at_ms,
        work_item_count,
        fetch_claim_count,
        verification_count,
        work_set_digest,
        fetch_resolution_reason: FETCH_POLICY_TERMINAL_REASON.to_string(),
        verification_resolution_reason: VERIFICATION_POLICY_TERMINAL_REASON.to_string(),
        verification_result_digest,
    };
    let hashed_receipt = HashedComputePluginSharingPolicyCapabilityRevocationReceipt {
        schema: HASHED_COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA
            .to_string(),
        receipt_digest: jcs_sha256_hex(&receipt)?,
        receipt,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        receipt_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
    };
    validate_hashed_revocation(&hashed_receipt)?;
    Ok(PreparedPolicyCapabilityRevocation {
        hashed_receipt,
        work_set,
        work_set_json,
        verification_result_json,
    })
}

pub(in super::super) fn insert_prepared_revocation(
    transaction: &Transaction<'_>,
    prepared: &PreparedPolicyCapabilityRevocation,
) -> Result<()> {
    let receipt = &prepared.hashed_receipt.receipt;
    let receipt_json =
        serde_json::to_string(receipt).context("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_JSON")?;
    let inserted = transaction
        .execute(
            r#"INSERT INTO sharing_policy_binding_revocation_receipts (
                policy_revision, request_digest, policy_binding_receipt_digest,
                installation_id_digest, authority_epoch_before, process_owner_epoch,
                trusted_time_before_ms, bound_at_ms, work_item_count,
                fetch_claim_count, verification_count, work_set_json, work_set_digest,
                fetch_resolution_reason, verification_resolution_reason,
                verification_result_json, verification_result_digest,
                receipt_json, receipt_digest
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
            )"#,
            params![
                receipt.policy_revision,
                &receipt.request_digest,
                &receipt.policy_binding_receipt_digest,
                &receipt.installation_id_digest,
                receipt.authority_epoch_before,
                receipt.process_owner_epoch,
                receipt.trusted_time_before_ms,
                receipt.bound_at_ms,
                receipt.work_item_count,
                receipt.fetch_claim_count,
                receipt.verification_count,
                &prepared.work_set_json,
                &receipt.work_set_digest,
                &receipt.fetch_resolution_reason,
                &receipt.verification_resolution_reason,
                &prepared.verification_result_json,
                &receipt.verification_result_digest,
                receipt_json,
                &prepared.hashed_receipt.receipt_digest,
            ],
        )
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_INSERT")?;
    if inserted != 1 {
        bail!("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_CAS");
    }
    Ok(())
}

pub(in super::super) fn read_exact_revocation(
    transaction: &Transaction<'_>,
    request: &PreparedSharingPolicyBindingRequest,
    binding_receipt: &HashedComputePluginSharingPolicyBindingReceipt,
) -> Result<Option<StoredPolicyCapabilityRevocation>> {
    let row = transaction
        .query_row(
            r#"SELECT request_digest, policy_binding_receipt_digest,
                installation_id_digest, authority_epoch_before, process_owner_epoch,
                trusted_time_before_ms, bound_at_ms, work_item_count,
                fetch_claim_count, verification_count, work_set_json, work_set_digest,
                fetch_resolution_reason, verification_resolution_reason,
                verification_result_json, verification_result_digest,
                receipt_json, receipt_digest
            FROM sharing_policy_binding_revocation_receipts WHERE policy_revision = ?1"#,
            [request.policy_revision],
            |row| {
                Ok(StoredRevocationRow {
                    request_digest: row.get(0)?,
                    policy_binding_receipt_digest: row.get(1)?,
                    installation_id_digest: row.get(2)?,
                    authority_epoch_before: row.get(3)?,
                    process_owner_epoch: row.get(4)?,
                    trusted_time_before_ms: row.get(5)?,
                    bound_at_ms: row.get(6)?,
                    work_item_count: row.get(7)?,
                    fetch_claim_count: row.get(8)?,
                    verification_count: row.get(9)?,
                    work_set_json: row.get(10)?,
                    work_set_digest: row.get(11)?,
                    fetch_resolution_reason: row.get(12)?,
                    verification_resolution_reason: row.get(13)?,
                    verification_result_json: row.get(14)?,
                    verification_result_digest: row.get(15)?,
                    receipt_json: row.get(16)?,
                    receipt_digest: row.get(17)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_READ")?;
    row.map(|row| validate_stored_revocation(row, request, binding_receipt))
        .transpose()
}

pub(in super::super) fn validate_terminalized_work(
    transaction: &Transaction<'_>,
    stored: &StoredPolicyCapabilityRevocation,
) -> Result<()> {
    let receipt = &stored.hashed_receipt.receipt;
    let actual = read_terminalized_work_set(transaction, stored)?;
    if actual != stored.work_set
        || serde_json::to_string(&actual)? != stored.work_set_json
        || jcs_sha256_hex(&actual)? != receipt.work_set_digest
    {
        bail!("COMPUTE_PLUGIN_POLICY_REVOCATION_TERMINAL_SET_CHANGED");
    }
    Ok(())
}

struct StoredRevocationRow {
    request_digest: String,
    policy_binding_receipt_digest: String,
    installation_id_digest: String,
    authority_epoch_before: i64,
    process_owner_epoch: i64,
    trusted_time_before_ms: i64,
    bound_at_ms: i64,
    work_item_count: i64,
    fetch_claim_count: i64,
    verification_count: i64,
    work_set_json: String,
    work_set_digest: String,
    fetch_resolution_reason: String,
    verification_resolution_reason: String,
    verification_result_json: String,
    verification_result_digest: String,
    receipt_json: String,
    receipt_digest: String,
}

fn validate_stored_revocation(
    row: StoredRevocationRow,
    request: &PreparedSharingPolicyBindingRequest,
    binding_receipt: &HashedComputePluginSharingPolicyBindingReceipt,
) -> Result<StoredPolicyCapabilityRevocation> {
    let work_set: PolicyPreparedWorkSet = serde_json::from_str(&row.work_set_json)
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_STORED_WORK_JSON")?;
    let receipt: ComputePluginSharingPolicyCapabilityRevocationReceipt =
        serde_json::from_str(&row.receipt_json)
            .context("COMPUTE_PLUGIN_POLICY_REVOCATION_STORED_RECEIPT_JSON")?;
    let hashed_receipt = HashedComputePluginSharingPolicyCapabilityRevocationReceipt {
        schema: HASHED_COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA
            .to_string(),
        receipt,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        receipt_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        receipt_digest: row.receipt_digest.clone(),
    };
    validate_hashed_revocation(&hashed_receipt)?;
    let receipt = &hashed_receipt.receipt;
    let base_receipt = &binding_receipt.receipt;
    let stored_fetch_count = work_set
        .items
        .iter()
        .filter(|item| {
            matches!(
                item,
                super::types::PolicyPreparedWorkItem::FetchClaim { .. }
            )
        })
        .count();
    let stored_verification_count = work_set.items.len() - stored_fetch_count;
    if work_set.schema != COMPUTE_PLUGIN_SHARING_POLICY_PREPARED_WORK_SET_SCHEMA
        || serde_json::to_string(&work_set)? != row.work_set_json
        || jcs_sha256_hex(&work_set)? != row.work_set_digest
        || serde_json::to_string(receipt)? != row.receipt_json
        || receipt.policy_revision != request.policy_revision
        || receipt.request_digest != request.request_digest
        || receipt.policy_binding_receipt_digest != binding_receipt.receipt_digest
        || receipt.installation_id_digest != request.installation_id_digest
        || receipt.authority_epoch_before != base_receipt.authority_epoch_before
        || receipt.process_owner_epoch != base_receipt.process_owner_epoch
        || receipt.trusted_time_before_ms != base_receipt.trusted_time_before_ms
        || receipt.bound_at_ms != base_receipt.bound_at_ms
        || receipt.request_digest != row.request_digest
        || receipt.policy_binding_receipt_digest != row.policy_binding_receipt_digest
        || receipt.installation_id_digest != row.installation_id_digest
        || receipt.authority_epoch_before != row.authority_epoch_before
        || receipt.process_owner_epoch != row.process_owner_epoch
        || receipt.trusted_time_before_ms != row.trusted_time_before_ms
        || receipt.bound_at_ms != row.bound_at_ms
        || receipt.work_item_count != row.work_item_count
        || receipt.fetch_claim_count != row.fetch_claim_count
        || receipt.verification_count != row.verification_count
        || receipt.work_set_digest != row.work_set_digest
        || receipt.fetch_resolution_reason != row.fetch_resolution_reason
        || receipt.verification_resolution_reason != row.verification_resolution_reason
        || receipt.verification_result_digest != row.verification_result_digest
        || usize::try_from(receipt.work_item_count).ok() != Some(work_set.items.len())
        || usize::try_from(receipt.fetch_claim_count).ok() != Some(stored_fetch_count)
        || usize::try_from(receipt.verification_count).ok() != Some(stored_verification_count)
    {
        bail!("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_EXACT_READBACK_FAILED");
    }
    validate_candidate_verification_terminal_result(
        CandidateVerificationTerminalKind::Aborted,
        VERIFICATION_POLICY_TERMINAL_REASON,
        receipt.bound_at_ms,
        &row.verification_result_json,
        &receipt.verification_result_digest,
    )?;
    Ok(StoredPolicyCapabilityRevocation {
        hashed_receipt,
        work_set,
        work_set_json: row.work_set_json,
        verification_result_json: row.verification_result_json,
    })
}

fn validate_hashed_revocation(
    hashed: &HashedComputePluginSharingPolicyCapabilityRevocationReceipt,
) -> Result<()> {
    let receipt = &hashed.receipt;
    if hashed.schema != HASHED_COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA
        || hashed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || hashed.receipt_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&hashed.receipt_digest)
        || jcs_sha256_hex(receipt)? != hashed.receipt_digest
        || receipt.schema != COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA
        || receipt.policy_revision <= 0
        || !is_sha256(&receipt.request_digest)
        || !is_sha256(&receipt.policy_binding_receipt_digest)
        || !is_sha256(&receipt.installation_id_digest)
        || receipt.authority_epoch_before < 0
        || receipt.process_owner_epoch <= 0
        || receipt.trusted_time_before_ms < 0
        || receipt.bound_at_ms <= receipt.trusted_time_before_ms
        || receipt.fetch_claim_count < 0
        || receipt.verification_count < 0
        || receipt.work_item_count
            != receipt
                .fetch_claim_count
                .checked_add(receipt.verification_count)
                .unwrap_or(-1)
        || usize::try_from(receipt.fetch_claim_count)
            .ok()
            .is_none_or(|count| count > MAX_PREPARED_FETCH_CLAIMS)
        || usize::try_from(receipt.verification_count)
            .ok()
            .is_none_or(|count| count > MAX_PREPARED_VERIFICATIONS)
        || !is_sha256(&receipt.work_set_digest)
        || receipt.fetch_resolution_reason != FETCH_POLICY_TERMINAL_REASON
        || receipt.verification_resolution_reason != VERIFICATION_POLICY_TERMINAL_REASON
        || !is_sha256(&receipt.verification_result_digest)
    {
        bail!("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_INVALID");
    }
    Ok(())
}
