use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{
    super::{process_ownership::ComputePluginFetchProcessFence, ComputePluginLocalAuthority},
    durable, recovery_error, retained,
    types::{
        ComputePluginSharingPolicyBindingReceipt, ComputePluginSharingPolicyBindingRecoveryKey,
        PreparedSharingPolicyBindingRequest, ProjectedSharingPolicyBinding,
    },
    validation::{read_state, validate_session_and_prepare_request},
    write::{read_exact_receipt, validate_authority_after},
    ComputePluginSharingPolicyBindingRecovery, ComputePluginSharingPolicyBindingRecoveryOutcome,
};
use crate::node_agent_compute_plugin_host::{
    fetch_file::PinnedComputePluginRoot, trusted_time::ComputePluginTrustedTimeObservation,
};

pub(super) fn adopt(
    authority: &ComputePluginLocalAuthority,
    mut recovery: ComputePluginSharingPolicyBindingRecovery,
    root: &PinnedComputePluginRoot,
    process_fence: &ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
) -> ComputePluginSharingPolicyBindingRecoveryOutcome {
    let validation =
        validate_recovery_session(authority, &recovery, root, process_fence, &observation);
    if let Err(error) = validation {
        recovery.error = error;
        return retained(recovery);
    }
    let outcome = authority.with_deferred(|transaction| {
        match read_exact_receipt(transaction, &recovery.key.request)? {
            Some(stored) => {
                if stored != recovery.key.hashed_receipt {
                    bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_RECEIPT_CHANGED");
                }
                let projected = ProjectedSharingPolicyBinding {
                    request: recovery.key.request.clone(),
                    before: recovery.key.before.clone(),
                    inventory_after_json: recovery.key.inventory_after_json.clone(),
                    hashed_receipt: recovery.key.hashed_receipt.clone(),
                };
                if recovery.intent.ensure_current().is_ok()
                    && validate_authority_after(transaction, &projected).is_ok()
                {
                    Ok(RecoveryClassification::DurableCurrent)
                } else {
                    validate_committed_history(
                        transaction,
                        &recovery.key,
                        observation.trusted_now(),
                    )?;
                    Ok(RecoveryClassification::CommittedHistorical)
                }
            }
            None => {
                validate_not_created(transaction, &recovery.key)?;
                if recovery.intent.ensure_current().is_ok() {
                    Ok(RecoveryClassification::NotCreatedCurrent)
                } else {
                    Ok(RecoveryClassification::NotCreatedSuperseded)
                }
            }
        }
    });
    match outcome {
        Ok(RecoveryClassification::DurableCurrent) => {
            ComputePluginSharingPolicyBindingRecoveryOutcome::Durable(durable(
                recovery.intent,
                recovery.key.hashed_receipt,
                recovery.root_lock,
            ))
        }
        Ok(RecoveryClassification::CommittedHistorical) => {
            ComputePluginSharingPolicyBindingRecoveryOutcome::CommittedHistorical(
                recovery.key.hashed_receipt,
            )
        }
        Ok(RecoveryClassification::NotCreatedCurrent) => {
            ComputePluginSharingPolicyBindingRecoveryOutcome::NotCreated(recovery.intent)
        }
        Ok(RecoveryClassification::NotCreatedSuperseded) => {
            ComputePluginSharingPolicyBindingRecoveryOutcome::NotCreatedSuperseded
        }
        Err(error) => {
            recovery.error = error;
            retained(recovery)
        }
    }
}

fn validate_committed_history(
    transaction: &Transaction<'_>,
    key: &ComputePluginSharingPolicyBindingRecoveryKey,
    trusted_now: &chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let current = read_state(transaction, trusted_now)?;
    let mut chain = read_receipt_chain(transaction, key.request.policy_revision)?;
    let Some((first_request, first_receipt)) = chain.first() else {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_MISSING");
    };
    if first_request.policy_revision != key.request.policy_revision
        || first_receipt != &key.hashed_receipt
    {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_ORIGIN_CHANGED");
    }

    let mut previous = &first_receipt.receipt;
    for (request, hashed) in chain.iter().skip(1) {
        let receipt = &hashed.receipt;
        if request.policy_revision <= previous.policy_revision
            || receipt.installation_id_digest != previous.installation_id_digest
            || receipt.state_revision_before < previous.state_revision_after
            || receipt.inventory_revision_before < previous.inventory_revision_after
            || receipt.authority_epoch_before < previous.authority_epoch_after
            || receipt.process_owner_epoch < previous.process_owner_epoch
            || receipt.trusted_time_before_ms < previous.bound_at_ms
        {
            bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_CHAIN_CHANGED");
        }
        previous = receipt;
    }

    let (latest_request, latest_hashed) = chain
        .pop()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_MISSING"))?;
    let latest = &latest_hashed.receipt;
    let authority = &current.authority;
    if authority.installation_id_digest != latest.installation_id_digest
        || authority.desired_policy_revision != latest_request.policy_revision
        || authority.sharing_enabled != latest_request.sharing_enabled
        || authority.sharing_authorization_ref != latest_request.sharing_authorization_ref
        || authority.sharing_authorization_revision != latest_request.sharing_authorization_revision
        || authority.sharing_authorization_digest != latest_request.sharing_authorization_digest
        || authority.state_revision < latest.state_revision_after
        || authority.inventory_revision < latest.inventory_revision_after
        || authority.authority_epoch < latest.authority_epoch_after
        || authority.process_owner_epoch < latest.process_owner_epoch
        || authority.trusted_time_high_water_ms < latest.bound_at_ms
    {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_HEAD_CHANGED");
    }
    Ok(())
}

fn read_receipt_chain(
    transaction: &Transaction<'_>,
    first_policy_revision: i64,
) -> Result<
    Vec<(
        PreparedSharingPolicyBindingRequest,
        super::HashedComputePluginSharingPolicyBindingReceipt,
    )>,
> {
    let mut statement = transaction
        .prepare(
            r#"SELECT policy_snapshot_json, policy_snapshot_digest, receipt_json
               FROM sharing_policy_binding_receipts
               WHERE policy_revision >= ?1
               ORDER BY policy_revision ASC"#,
        )
        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_PREPARE")?;
    let rows = statement
        .query_map([first_policy_revision], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_READ")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_ROWS")?;
    drop(statement);

    rows.into_iter()
        .map(
            |(policy_snapshot_json, policy_snapshot_digest, receipt_json)| {
                let snapshot: homecli_proto::ComputePluginSharingPolicySnapshotV1 =
                    serde_json::from_str(&policy_snapshot_json)
                        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_SNAPSHOT_JSON")?;
                let receipt: ComputePluginSharingPolicyBindingReceipt =
                    serde_json::from_str(&receipt_json)
                        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_RECEIPT_JSON")?;
                let (
                    sharing_authorization_ref,
                    sharing_authorization_revision,
                    sharing_authorization_digest,
                ) = snapshot
                    .authorization
                    .as_ref()
                    .map_or((None, None, None), |authorization| {
                        (
                            Some(authorization.authorization_ref.clone()),
                            i64::try_from(authorization.revision).ok(),
                            Some(authorization.digest.clone()),
                        )
                    });
                let request = PreparedSharingPolicyBindingRequest {
                    node_id: snapshot.node_id.clone(),
                    owner_user_id: snapshot.owner_user_id.clone(),
                    installation_id_digest: snapshot.installation_identity_digest.clone(),
                    policy_revision: i64::try_from(snapshot.policy_revision)
                        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_REVISION_RANGE")?,
                    policy_digest: snapshot.policy_digest.clone(),
                    policy_snapshot_json,
                    policy_snapshot_digest,
                    sharing_enabled: snapshot.plugin_runtime_requested,
                    sharing_authorization_ref,
                    sharing_authorization_revision,
                    sharing_authorization_digest,
                    source_preparation_id: receipt.source_preparation_id.clone(),
                    source_bootstrap_instance_id: receipt.source_bootstrap_instance_id.clone(),
                    source_configuration_generation: receipt.source_configuration_generation,
                    source_cancellation_generation: receipt.source_cancellation_generation,
                    request_digest: receipt.request_digest.clone(),
                };
                let hashed = read_exact_receipt(transaction, &request)?.ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_HISTORY_ROW_MISSING")
                })?;
                Ok((request, hashed))
            },
        )
        .collect()
}

enum RecoveryClassification {
    DurableCurrent,
    CommittedHistorical,
    NotCreatedCurrent,
    NotCreatedSuperseded,
}

fn validate_recovery_session(
    authority: &ComputePluginLocalAuthority,
    recovery: &ComputePluginSharingPolicyBindingRecovery,
    root: &PinnedComputePluginRoot,
    process_fence: &ComputePluginFetchProcessFence,
    observation: &ComputePluginTrustedTimeObservation,
) -> Result<()> {
    let (request, session) = validate_session_and_prepare_request(
        authority,
        &recovery.intent,
        root,
        process_fence,
        observation,
    )?;
    if request != recovery.key.request
        || !recovery
            .key
            .authority_instance_binding
            .matches(authority.instance_binding())
        || recovery.key.clock_epoch_digest != session.clock_epoch_digest
        || recovery.key.root_identity_digest != root.root_identity_digest()
        || session.prepared_at <= recovery.key.prepared_at
        || session.trusted_now.timestamp_millis() <= recovery.key.hashed_receipt.receipt.bound_at_ms
        || process_fence.process_owner_epoch()
            != recovery.key.hashed_receipt.receipt.process_owner_epoch
    {
        recovery_error("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_PROVENANCE_CHANGED")?;
    }
    Ok(())
}

fn validate_not_created(
    transaction: &Transaction<'_>,
    key: &ComputePluginSharingPolicyBindingRecoveryKey,
) -> Result<()> {
    let before = &key.before;
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
              AND trusted_time_high_water_ms = ?13 AND updated_at_ms = ?14
              AND clock_status = 'trusted'
              AND NOT EXISTS (
                  SELECT 1 FROM sharing_policy_binding_receipts
                  WHERE policy_revision >= ?15 OR request_digest = ?16
              )"#,
            params![
                &before.installation_id_digest,
                before.state_revision,
                before.inventory_revision,
                &before.inventory_digest,
                &before.inventory_json,
                before.desired_policy_revision,
                i64::from(before.sharing_enabled),
                before.sharing_authorization_ref.as_deref(),
                before.sharing_authorization_revision,
                before.sharing_authorization_digest.as_deref(),
                before.authority_epoch,
                before.process_owner_epoch,
                before.trusted_time_high_water_ms,
                before.updated_at_ms,
                key.request.policy_revision,
                &key.request.request_digest,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_BEFORE_READ")?;
    if matches != 1 {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECOVERY_OUTCOME_AMBIGUOUS");
    }
    Ok(())
}
