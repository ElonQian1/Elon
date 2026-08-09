use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    super::{process_ownership::ComputePluginFetchProcessFence, ComputePluginLocalAuthority},
    durable, recovery, rejected,
    revocation::{
        insert_prepared_revocation, prepare_revocation, read_exact_revocation,
        validate_terminalized_work,
    },
    types::{
        ComputePluginSharingPolicyBindingRecoveryKey,
        HashedComputePluginSharingPolicyBindingReceipt, PreparedSharingPolicyBindingRequest,
        ProjectedSharingPolicyBinding, SharingPolicyBindingRequestDigest,
        COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA,
    },
    validation::{
        project, read_state, validate_hashed_receipt, validate_session_and_prepare_request,
    },
    ComputePluginSharingPolicyBindingStoreResult,
};
use crate::{
    compute_plugin_sharing_directive::compute_plugin_sharing_policy_snapshot_digest,
    node_agent_compute_plugin_host::{
        bootstrap::ComputePluginLocalPolicyBindingIntent,
        fetch_file::PinnedComputePluginRoot,
        lifecycle::ComputePluginInventorySnapshot,
        plugin_manifest::{
            COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION,
        },
        signed_artifact_verification::jcs_sha256_hex,
        trusted_time::ComputePluginTrustedTimeObservation,
    },
};

#[cfg(test)]
mod tests;

pub(super) fn bind(
    authority: &ComputePluginLocalAuthority,
    intent: ComputePluginLocalPolicyBindingIntent,
    root: &PinnedComputePluginRoot,
    process_fence: &ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> ComputePluginSharingPolicyBindingStoreResult {
    if let Err(error) = intent.ensure_current() {
        return rejected(intent, error);
    }
    let (request, session) = match validate_session_and_prepare_request(
        authority,
        &intent,
        root,
        process_fence,
        &observation,
    ) {
        Ok(value) => value,
        Err(error) => return rejected(intent, error),
    };
    let mut root_lock = Some(root.root_lock_lease());
    let mut recovery_key = None;
    let outcome = authority.with_immediate(|transaction| {
        let current = read_state(transaction, &session.trusted_now)?;
        if current.authority.installation_id_digest != request.installation_id_digest
            || current.authority.process_owner_epoch != process_fence.process_owner_epoch()
        {
            bail!("COMPUTE_PLUGIN_POLICY_BINDING_PROCESS_FENCE_CHANGED");
        }
        if let Some(replayed) = read_exact_receipt(transaction, &request)? {
            validate_current_policy_head(transaction, &request)?;
            let revocation =
                read_exact_revocation(transaction, &request, &replayed)?.ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_MISSING")
                })?;
            validate_terminalized_work(transaction, &revocation)?;
            intent.ensure_current()?;
            return Ok((replayed, revocation.hashed_receipt));
        }
        validate_revision_absence(transaction, &request)?;
        // Closing is terminal for this process fence: even if the database transition later
        // rejects or becomes uncertain, the old Plan cannot mint a fresh cancellation guard.
        process_fence.close_fetch_cancellation();
        intent.ensure_current()?;
        let projected = project(request.clone(), current, &session.trusted_now)?;
        let prepared_revocation = prepare_revocation(transaction, &projected)?;
        recovery_key = Some(ComputePluginSharingPolicyBindingRecoveryKey {
            authority_instance_binding: authority.instance_binding().clone(),
            root_identity_digest: root.root_identity_digest().to_string(),
            clock_epoch_digest: session.clock_epoch_digest.clone(),
            prepared_at: session.prepared_at,
            request: projected.request.clone(),
            before: projected.before.clone(),
            inventory_after_json: projected.inventory_after_json.clone(),
            hashed_receipt: projected.hashed_receipt.clone(),
            prepared_revocation: prepared_revocation.clone(),
        });
        intent.ensure_current()?;
        insert_prepared_revocation(transaction, &prepared_revocation)?;
        insert_receipt(transaction, &projected)?;
        let stored = read_exact_receipt(transaction, &projected.request)?
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_MISSING"))?;
        if stored != projected.hashed_receipt {
            bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_CHANGED");
        }
        let stored_revocation = read_exact_revocation(transaction, &projected.request, &stored)?
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_MISSING"))?;
        if stored_revocation != prepared_revocation {
            bail!("COMPUTE_PLUGIN_POLICY_REVOCATION_RECEIPT_CHANGED");
        }
        validate_terminalized_work(transaction, &stored_revocation)?;
        validate_authority_after(transaction, &projected)?;
        intent.ensure_current()?;
        Ok((stored, stored_revocation.hashed_receipt))
    });
    match outcome {
        Ok((receipt, revocation_receipt)) => match intent.ensure_current() {
            Ok(()) => ComputePluginSharingPolicyBindingStoreResult::Durable(durable(
                intent,
                receipt,
                revocation_receipt,
                root_lock
                    .take()
                    .expect("policy binding root lock must be retained"),
            )),
            Err(error) => match recovery_key {
                Some(key) => recovery(
                    intent,
                    key,
                    error,
                    root_lock
                        .take()
                        .expect("policy binding root lock must be retained"),
                ),
                None => rejected(intent, error),
            },
        },
        Err(error) => match recovery_key {
            Some(key) => recovery(
                intent,
                key,
                error,
                root_lock
                    .take()
                    .expect("policy binding root lock must be retained"),
            ),
            None => rejected(intent, error),
        },
    }
}

fn validate_revision_absence(
    transaction: &Transaction<'_>,
    request: &PreparedSharingPolicyBindingRequest,
) -> Result<()> {
    let maximum = transaction
        .query_row(
            "SELECT MAX(policy_revision) FROM sharing_policy_binding_receipts",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .context("COMPUTE_PLUGIN_POLICY_BINDING_REVISION_READ")?;
    if maximum.is_some_and(|revision| request.policy_revision <= revision) {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_REVISION_STALE_OR_CONFLICT");
    }
    Ok(())
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    projected: &ProjectedSharingPolicyBinding,
) -> Result<()> {
    let request = &projected.request;
    let receipt = &projected.hashed_receipt.receipt;
    let receipt_json =
        serde_json::to_string(receipt).context("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_JSON")?;
    let inserted = transaction
        .execute(
            r#"INSERT INTO sharing_policy_binding_receipts (
                policy_revision, request_digest, node_id, owner_user_id,
                installation_id_digest, policy_digest, policy_snapshot_json,
                policy_snapshot_digest, sharing_enabled, sharing_authorization_ref,
                sharing_authorization_revision, sharing_authorization_digest,
                state_revision_before, state_revision_after,
                inventory_revision_before, inventory_revision_after,
                inventory_digest_before, inventory_digest_after, inventory_after_json,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                trusted_time_before_ms, clock_status_before,
                authority_updated_at_ms_before, bound_at_ms, receipt_json, receipt_digest
            ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7,
                ?8, ?9, ?10,
                ?11, ?12,
                ?13, ?14,
                ?15, ?16,
                ?17, ?18, ?19,
                ?20, ?21, ?22,
                ?23, 'trusted',
                ?24, ?25, ?26, ?27
            )"#,
            params![
                request.policy_revision,
                &request.request_digest,
                &request.node_id,
                &request.owner_user_id,
                &request.installation_id_digest,
                &request.policy_digest,
                &request.policy_snapshot_json,
                &request.policy_snapshot_digest,
                i64::from(request.sharing_enabled),
                request.sharing_authorization_ref.as_deref(),
                request.sharing_authorization_revision,
                request.sharing_authorization_digest.as_deref(),
                receipt.state_revision_before,
                receipt.state_revision_after,
                receipt.inventory_revision_before,
                receipt.inventory_revision_after,
                &receipt.inventory_digest_before,
                &receipt.inventory_digest_after,
                &projected.inventory_after_json,
                receipt.authority_epoch_before,
                receipt.authority_epoch_after,
                receipt.process_owner_epoch,
                receipt.trusted_time_before_ms,
                projected.before.updated_at_ms,
                receipt.bound_at_ms,
                receipt_json,
                &projected.hashed_receipt.receipt_digest,
            ],
        )
        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_INSERT")?;
    if inserted != 1 {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_CAS");
    }
    Ok(())
}

pub(super) fn read_exact_receipt(
    transaction: &Transaction<'_>,
    request: &PreparedSharingPolicyBindingRequest,
) -> Result<Option<HashedComputePluginSharingPolicyBindingReceipt>> {
    let row = transaction
        .query_row(
            r#"SELECT request_digest, node_id, owner_user_id, installation_id_digest,
                policy_digest, policy_snapshot_json, policy_snapshot_digest, sharing_enabled,
                sharing_authorization_ref, sharing_authorization_revision,
                sharing_authorization_digest, state_revision_before, state_revision_after,
                inventory_revision_before, inventory_revision_after,
                inventory_digest_before, inventory_digest_after, inventory_after_json,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                trusted_time_before_ms, clock_status_before, authority_updated_at_ms_before,
                bound_at_ms, receipt_json, receipt_digest
            FROM sharing_policy_binding_receipts WHERE policy_revision = ?1"#,
            [request.policy_revision],
            |row| {
                Ok(StoredReceiptRow {
                    request_digest: row.get(0)?,
                    node_id: row.get(1)?,
                    owner_user_id: row.get(2)?,
                    installation_id_digest: row.get(3)?,
                    policy_digest: row.get(4)?,
                    policy_snapshot_json: row.get(5)?,
                    policy_snapshot_digest: row.get(6)?,
                    sharing_enabled: row.get(7)?,
                    sharing_authorization_ref: row.get(8)?,
                    sharing_authorization_revision: row.get(9)?,
                    sharing_authorization_digest: row.get(10)?,
                    state_revision_before: row.get(11)?,
                    state_revision_after: row.get(12)?,
                    inventory_revision_before: row.get(13)?,
                    inventory_revision_after: row.get(14)?,
                    inventory_digest_before: row.get(15)?,
                    inventory_digest_after: row.get(16)?,
                    inventory_after_json: row.get(17)?,
                    authority_epoch_before: row.get(18)?,
                    authority_epoch_after: row.get(19)?,
                    process_owner_epoch: row.get(20)?,
                    trusted_time_before_ms: row.get(21)?,
                    clock_status_before: row.get(22)?,
                    authority_updated_at_ms_before: row.get(23)?,
                    bound_at_ms: row.get(24)?,
                    receipt_json: row.get(25)?,
                    receipt_digest: row.get(26)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_READ")?;
    row.map(|row| validate_stored_receipt(row, request))
        .transpose()
}

struct StoredReceiptRow {
    request_digest: String,
    node_id: String,
    owner_user_id: String,
    installation_id_digest: String,
    policy_digest: String,
    policy_snapshot_json: String,
    policy_snapshot_digest: String,
    sharing_enabled: i64,
    sharing_authorization_ref: Option<String>,
    sharing_authorization_revision: Option<i64>,
    sharing_authorization_digest: Option<String>,
    state_revision_before: i64,
    state_revision_after: i64,
    inventory_revision_before: i64,
    inventory_revision_after: i64,
    inventory_digest_before: String,
    inventory_digest_after: String,
    inventory_after_json: String,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    process_owner_epoch: i64,
    trusted_time_before_ms: i64,
    clock_status_before: String,
    authority_updated_at_ms_before: i64,
    bound_at_ms: i64,
    receipt_json: String,
    receipt_digest: String,
}

fn validate_stored_receipt(
    row: StoredReceiptRow,
    request: &PreparedSharingPolicyBindingRequest,
) -> Result<HashedComputePluginSharingPolicyBindingReceipt> {
    let snapshot: homecli_proto::ComputePluginSharingPolicySnapshotV1 =
        serde_json::from_str(&row.policy_snapshot_json)
            .context("COMPUTE_PLUGIN_POLICY_BINDING_STORED_SNAPSHOT_JSON")?;
    let receipt: super::types::ComputePluginSharingPolicyBindingReceipt =
        serde_json::from_str(&row.receipt_json)
            .context("COMPUTE_PLUGIN_POLICY_BINDING_STORED_RECEIPT_JSON")?;
    let hashed = HashedComputePluginSharingPolicyBindingReceipt {
        schema: super::types::HASHED_COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA
            .to_string(),
        receipt,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        receipt_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        receipt_digest: row.receipt_digest.clone(),
    };
    validate_hashed_receipt(&hashed)?;
    let receipt = &hashed.receipt;
    let calculated_request_digest = jcs_sha256_hex(&SharingPolicyBindingRequestDigest {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA,
        policy_snapshot: &snapshot,
        policy_snapshot_digest: &row.policy_snapshot_digest,
    })?;
    let inventory_after: ComputePluginInventorySnapshot =
        serde_json::from_str(&row.inventory_after_json)
            .context("COMPUTE_PLUGIN_POLICY_BINDING_STORED_INVENTORY_JSON")?;
    if serde_json::to_string(&snapshot)? != row.policy_snapshot_json
        || compute_plugin_sharing_policy_snapshot_digest(&snapshot)
            .map_err(|error| anyhow::anyhow!(error.code()))?
            != row.policy_snapshot_digest
        || serde_json::to_string(receipt)? != row.receipt_json
        || serde_json::to_string(&inventory_after)? != row.inventory_after_json
        || jcs_sha256_hex(&inventory_after)? != row.inventory_digest_after
        || calculated_request_digest != row.request_digest
        || row.request_digest != request.request_digest
        || row.node_id != request.node_id
        || row.owner_user_id != request.owner_user_id
        || row.installation_id_digest != request.installation_id_digest
        || row.policy_digest != request.policy_digest
        || row.policy_snapshot_json != request.policy_snapshot_json
        || row.policy_snapshot_digest != request.policy_snapshot_digest
        || row.sharing_enabled != i64::from(request.sharing_enabled)
        || row.sharing_authorization_ref != request.sharing_authorization_ref
        || row.sharing_authorization_revision != request.sharing_authorization_revision
        || row.sharing_authorization_digest != request.sharing_authorization_digest
        || row.clock_status_before != "trusted"
        || receipt.request_digest != row.request_digest
        || receipt.node_id != row.node_id
        || receipt.owner_user_id != row.owner_user_id
        || receipt.installation_id_digest != row.installation_id_digest
        || receipt.policy_revision != request.policy_revision
        || receipt.policy_digest != row.policy_digest
        || receipt.policy_snapshot_digest != row.policy_snapshot_digest
        || receipt.sharing_enabled != request.sharing_enabled
        || receipt.sharing_authorization_ref != row.sharing_authorization_ref
        || receipt.sharing_authorization_revision != row.sharing_authorization_revision
        || receipt.sharing_authorization_digest != row.sharing_authorization_digest
        || receipt.state_revision_before != row.state_revision_before
        || receipt.state_revision_after != row.state_revision_after
        || receipt.inventory_revision_before != row.inventory_revision_before
        || receipt.inventory_revision_after != row.inventory_revision_after
        || receipt.inventory_digest_before != row.inventory_digest_before
        || receipt.inventory_digest_after != row.inventory_digest_after
        || receipt.authority_epoch_before != row.authority_epoch_before
        || receipt.authority_epoch_after != row.authority_epoch_after
        || receipt.process_owner_epoch != row.process_owner_epoch
        || receipt.trusted_time_before_ms != row.trusted_time_before_ms
        || receipt.bound_at_ms != row.bound_at_ms
        || row.authority_updated_at_ms_before != row.trusted_time_before_ms
    {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_EXACT_READBACK_FAILED");
    }
    Ok(hashed)
}

pub(super) fn validate_authority_after(
    transaction: &Transaction<'_>,
    projected: &ProjectedSharingPolicyBinding,
) -> Result<()> {
    let request = &projected.request;
    let receipt = &projected.hashed_receipt.receipt;
    let matches = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta WHERE singleton = 1
              AND installation_id_digest = ?1
              AND state_revision = ?2
              AND inventory_revision = ?3 AND inventory_digest = ?4 AND inventory_json = ?5
              AND desired_policy_revision = ?6 AND sharing_enabled = ?7
              AND sharing_authorization_ref IS ?8
              AND sharing_authorization_revision IS ?9
              AND sharing_authorization_digest IS ?10
              AND authority_epoch = ?11 AND process_owner_epoch = ?12
              AND trusted_time_high_water_ms = ?13 AND updated_at_ms = ?13
              AND clock_status = 'trusted'"#,
            params![
                &request.installation_id_digest,
                receipt.state_revision_after,
                receipt.inventory_revision_after,
                &receipt.inventory_digest_after,
                &projected.inventory_after_json,
                request.policy_revision,
                i64::from(request.sharing_enabled),
                request.sharing_authorization_ref.as_deref(),
                request.sharing_authorization_revision,
                request.sharing_authorization_digest.as_deref(),
                receipt.authority_epoch_after,
                receipt.process_owner_epoch,
                receipt.bound_at_ms,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_AFTER_READ")?;
    if matches != 1 {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_AFTER_CHANGED");
    }
    Ok(())
}

fn validate_current_policy_head(
    transaction: &Transaction<'_>,
    request: &PreparedSharingPolicyBindingRequest,
) -> Result<()> {
    let matches = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta WHERE singleton = 1
              AND installation_id_digest = ?1
              AND desired_policy_revision = ?2 AND sharing_enabled = ?3
              AND sharing_authorization_ref IS ?4
              AND sharing_authorization_revision IS ?5
              AND sharing_authorization_digest IS ?6
              AND clock_status = 'trusted'
              AND trusted_time_high_water_ms IS NOT NULL
              AND updated_at_ms = trusted_time_high_water_ms"#,
            params![
                &request.installation_id_digest,
                request.policy_revision,
                i64::from(request.sharing_enabled),
                request.sharing_authorization_ref.as_deref(),
                request.sharing_authorization_revision,
                request.sharing_authorization_digest.as_deref(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_POLICY_BINDING_CURRENT_HEAD_READ")?;
    if matches != 1 {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_NOT_CURRENT");
    }
    Ok(())
}
