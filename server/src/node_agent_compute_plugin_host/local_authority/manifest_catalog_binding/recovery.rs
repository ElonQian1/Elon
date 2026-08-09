use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};
use std::time::Instant;

use super::{
    durable,
    types::ManifestCatalogAuthorityState,
    validation::{load_keyring, prepare_request, read_state_at_or_before, validate_session},
    write::{read_binding_by_revision, validate_current_catalog_head, validate_exact_request},
    ComputePluginManifestCatalogBindingRecovery,
    ComputePluginManifestCatalogBindingRecoveryOutcome,
};
use crate::node_agent_compute_plugin_host::{
    keyring::ComputePluginBootstrapRootKeyResolver,
    local_authority::process_ownership::ComputePluginFetchProcessFence,
    trusted_time::ComputePluginTrustedTimeObservation,
};

#[cfg(test)]
mod tests;

enum RecoveryClassification {
    Durable(super::HashedComputePluginManifestCatalogBindingReceipt),
    CommittedHistorical(super::HashedComputePluginManifestCatalogBindingReceipt),
    NotCreated,
    NotCreatedSuperseded,
}

pub(super) fn adopt(
    mut recovery: ComputePluginManifestCatalogBindingRecovery,
    process_fence: &ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
) -> ComputePluginManifestCatalogBindingRecoveryOutcome {
    let session = match validate_recovery_session(&recovery, process_fence, &observation) {
        Ok(session) => session,
        Err(_) => return ComputePluginManifestCatalogBindingRecoveryOutcome::Retained(recovery),
    };
    let classified = recovery.authority.with_immediate(|transaction| {
        process_fence.ensure_process_owner_current()?;
        if let Some(stored) =
            read_binding_by_revision(transaction, recovery.key.request.catalog_revision)?
        {
            validate_exact_request(&stored.request, &recovery.key.request)?;
            if stored.hashed_receipt != recovery.key.hashed_receipt {
                bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_RECEIPT_CHANGED");
            }
            let current = read_state_at_or_before(transaction, &session.trusted_now)?;
            validate_recovery_state_floor(&current, &recovery.key)?;
            let keyring = load_keyring(transaction, &current, session.trusted_now.clone(), roots)?;
            if validate_current_catalog_head(
                transaction,
                &stored.hashed_receipt,
                &session.trusted_now,
            )
            .is_ok()
                && current.process_owner_epoch == process_fence.process_owner_epoch()
            {
                let expected = prepare_request(
                    &recovery.candidate,
                    &current,
                    session.trusted_now.clone(),
                    &keyring,
                )?;
                validate_exact_request(&stored.request, &expected)?;
                observation.ensure_live(Instant::now())?;
                return Ok(RecoveryClassification::Durable(stored.hashed_receipt));
            }
            validate_committed_successor(transaction, &current, &stored.hashed_receipt)?;
            observation.ensure_live(Instant::now())?;
            return Ok(RecoveryClassification::CommittedHistorical(
                stored.hashed_receipt,
            ));
        }
        validate_receipt_absence(transaction, &recovery.key)?;
        let current = read_state_at_or_before(transaction, &session.trusted_now)?;
        load_keyring(transaction, &current, session.trusted_now.clone(), roots)?;
        if exact_prestate(&current, &recovery.key.before)
            && process_fence.process_owner_epoch() == recovery.key.before.process_owner_epoch
        {
            observation.ensure_live(Instant::now())?;
            return Ok(RecoveryClassification::NotCreated);
        }
        validate_not_created_successor(transaction, &current, &recovery.key)?;
        observation.ensure_live(Instant::now())?;
        Ok(RecoveryClassification::NotCreatedSuperseded)
    });
    match classified {
        Ok(RecoveryClassification::Durable(receipt)) => {
            match process_fence
                .ensure_process_owner_current()
                .and_then(|()| observation.ensure_live(Instant::now()))
            {
                Ok(()) => ComputePluginManifestCatalogBindingRecoveryOutcome::Durable(durable(
                    recovery.authority,
                    receipt,
                )),
                Err(_) => ComputePluginManifestCatalogBindingRecoveryOutcome::Retained(recovery),
            }
        }
        Ok(RecoveryClassification::CommittedHistorical(receipt)) => {
            ComputePluginManifestCatalogBindingRecoveryOutcome::CommittedHistorical(receipt)
        }
        Ok(RecoveryClassification::NotCreated) => {
            ComputePluginManifestCatalogBindingRecoveryOutcome::NotCreated {
                authority: recovery.authority,
                candidate: recovery.candidate,
            }
        }
        Ok(RecoveryClassification::NotCreatedSuperseded) => {
            ComputePluginManifestCatalogBindingRecoveryOutcome::NotCreatedSuperseded(
                recovery.authority,
            )
        }
        Err(_) => ComputePluginManifestCatalogBindingRecoveryOutcome::Retained(recovery),
    }
}

fn validate_recovery_session(
    recovery: &ComputePluginManifestCatalogBindingRecovery,
    process_fence: &ComputePluginFetchProcessFence,
    observation: &ComputePluginTrustedTimeObservation,
) -> Result<super::validation::ManifestCatalogBindingSession> {
    let session = validate_session(&recovery.authority, process_fence, observation)?;
    if !recovery
        .authority
        .authority_instance_binding()
        .matches(&recovery.key.authority_instance_binding)
        || recovery.authority.root_identity_digest() != recovery.key.root_identity_digest
        || observation.clock_epoch_digest() != recovery.key.clock_epoch_digest
        || observation.observed_at() <= recovery.key.prepared_at
        || process_fence.process_owner_epoch() < recovery.key.before.process_owner_epoch
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_SESSION_CHANGED");
    }
    Ok(session)
}

fn validate_receipt_absence(
    transaction: &Transaction<'_>,
    key: &super::types::ComputePluginManifestCatalogBindingRecoveryKey,
) -> Result<()> {
    let collisions = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM manifest_catalog_binding_receipts
               WHERE request_id = ?1 OR request_digest = ?2
                  OR catalog_digest = ?3 OR signed_catalog_envelope_digest = ?4
                  OR receipt_digest = ?5"#,
            params![
                &key.request.request_id,
                &key.request.request_digest,
                &key.request.catalog_digest,
                &key.request.signed_catalog_envelope_digest,
                &key.hashed_receipt.receipt_digest,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_COLLISION_READ")?;
    if collisions != 0 {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_COLLISION");
    }
    Ok(())
}

fn validate_recovery_state_floor(
    current: &ManifestCatalogAuthorityState,
    key: &super::types::ComputePluginManifestCatalogBindingRecoveryKey,
) -> Result<()> {
    let receipt = &key.hashed_receipt.receipt;
    if current.installation_id_digest != receipt.installation_id_digest
        || current.state_revision < receipt.state_revision_after
        || current.authority_epoch < receipt.authority_epoch_after
        || current.process_owner_epoch < receipt.process_owner_epoch
        || current.trusted_time_high_water_ms < receipt.bound_at_ms
        || current.updated_at_ms < receipt.bound_at_ms
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_STATE_ROLLBACK");
    }
    Ok(())
}

fn validate_committed_successor(
    transaction: &Transaction<'_>,
    current: &ManifestCatalogAuthorityState,
    committed: &super::HashedComputePluginManifestCatalogBindingReceipt,
) -> Result<()> {
    let committed = &committed.receipt;
    if current.manifest_catalog_revision < committed.catalog_revision {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_REVISION_ROLLBACK");
    }
    if current.manifest_catalog_revision > committed.catalog_revision {
        let head = read_binding_by_revision(transaction, current.manifest_catalog_revision)?
            .ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_MANIFEST_CATALOG_HEAD_RECEIPT_MISSING")
            })?;
        if head.hashed_receipt.receipt.installation_id_digest != committed.installation_id_digest
            || head.hashed_receipt.receipt.catalog_revision != current.manifest_catalog_revision
            || head.hashed_receipt.receipt.state_revision_after > current.state_revision
            || head.hashed_receipt.receipt.authority_epoch_after > current.authority_epoch
            || head.hashed_receipt.receipt.bound_at_ms > current.trusted_time_high_water_ms
        {
            bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_HEAD_RECEIPT_CHANGED");
        }
    }
    Ok(())
}

fn validate_not_created_successor(
    transaction: &Transaction<'_>,
    current: &ManifestCatalogAuthorityState,
    key: &super::types::ComputePluginManifestCatalogBindingRecoveryKey,
) -> Result<()> {
    let before = &key.before;
    if current.installation_id_digest != before.installation_id_digest
        || current.state_revision < before.state_revision
        || current.authority_epoch < before.authority_epoch
        || current.process_owner_epoch < before.process_owner_epoch
        || current.trusted_time_high_water_ms < before.trusted_time_high_water_ms
        || current.updated_at_ms < before.updated_at_ms
        || current.manifest_catalog_revision < before.manifest_catalog_revision
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_SUCCESSOR_INVALID");
    }
    if current.manifest_catalog_revision > key.request.catalog_revision {
        let head = read_binding_by_revision(transaction, current.manifest_catalog_revision)?
            .ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_MANIFEST_CATALOG_HEAD_RECEIPT_MISSING")
            })?;
        if head.hashed_receipt.receipt.installation_id_digest != key.request.installation_id_digest
        {
            bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_HEAD_INSTALLATION_CHANGED");
        }
    } else if current.process_owner_epoch == before.process_owner_epoch
        && current.state_revision == before.state_revision
        && current.authority_epoch == before.authority_epoch
        && current.trusted_time_high_water_ms == before.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECOVERY_SUCCESSOR_UNPROVEN");
    }
    Ok(())
}

fn exact_prestate(
    current: &ManifestCatalogAuthorityState,
    before: &ManifestCatalogAuthorityState,
) -> bool {
    current.installation_id_digest == before.installation_id_digest
        && current.state_revision == before.state_revision
        && current.inventory_revision == before.inventory_revision
        && current.inventory_digest == before.inventory_digest
        && current.inventory_json == before.inventory_json
        && current.desired_policy_revision == before.desired_policy_revision
        && current.sharing_enabled == before.sharing_enabled
        && current.node_profile_digest == before.node_profile_digest
        && current.manifest_catalog_revision == before.manifest_catalog_revision
        && current.target_id == before.target_id
        && current.host_api_protocol_id == before.host_api_protocol_id
        && current.host_api_revision == before.host_api_revision
        && current.authority_epoch == before.authority_epoch
        && current.process_owner_epoch == before.process_owner_epoch
        && current.trusted_time_high_water_ms == before.trusted_time_high_water_ms
        && current.updated_at_ms == before.updated_at_ms
        && current.keyring_bundle_revision == before.keyring_bundle_revision
        && current.publisher_keyring == before.publisher_keyring
        && current.control_keyring == before.control_keyring
}
