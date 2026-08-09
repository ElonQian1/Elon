use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};
use std::time::Instant;

mod read;

#[cfg(test)]
mod tests;

pub(super) use read::{
    read_binding_by_revision, validate_current_catalog_head, validate_exact_request,
};

use super::{
    durable, recovery, rejected,
    types::{
        ComputePluginManifestCatalogBindingRecoveryKey, PreparedManifestCatalogBindingRequest,
    },
    validation::{
        load_keyring, prepare_request, project, read_state_strict, validate_authority_after,
        validate_hashed_receipt, validate_session,
    },
    ComputePluginManifestCatalogBindingStoreResult,
};
use crate::node_agent_compute_plugin_host::{
    keyring::ComputePluginBootstrapRootKeyResolver,
    local_authority::{
        opened_authority::OpenedComputePluginLocalAuthority,
        process_ownership::ComputePluginFetchProcessFence,
    },
    manifest_catalog::ComputePluginManifestCatalogCandidate,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(super) fn bind(
    mut authority: OpenedComputePluginLocalAuthority,
    candidate: ComputePluginManifestCatalogCandidate,
    process_fence: &ComputePluginFetchProcessFence,
    observation: ComputePluginTrustedTimeObservation,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
) -> ComputePluginManifestCatalogBindingStoreResult {
    let session = match validate_session(&authority, process_fence, &observation) {
        Ok(session) => session,
        Err(error) => return rejected(authority, candidate, error),
    };
    let authority_instance_binding = authority.authority_instance_binding().clone();
    let root_identity_digest = authority.root_identity_digest().to_string();
    let mut recovery_key = None;
    let outcome = authority.with_immediate(|transaction| {
        process_fence.ensure_process_owner_current()?;
        let before = read_state_strict(transaction, &session.trusted_now)?;
        validate_state_session(&before, process_fence)?;
        let keyring = load_keyring(transaction, &before, session.trusted_now.clone(), roots)?;
        let request = prepare_request(&candidate, &before, session.trusted_now.clone(), &keyring)?;
        observation.ensure_live(Instant::now())?;
        if let Some(stored) = read_binding_by_revision(transaction, request.catalog_revision)? {
            validate_exact_request(&stored.request, &request)?;
            validate_current_catalog_head(
                transaction,
                &stored.hashed_receipt,
                &session.trusted_now,
            )?;
            process_fence.ensure_process_owner_current()?;
            return Ok(stored.hashed_receipt);
        }
        validate_absence_for_new_binding(transaction, &request, &before)?;
        // Catalog replacement invalidates every byte-fetch authority derived from the old catalog.
        // This source is terminal even if the SQLite commit later becomes uncertain.
        process_fence.close_fetch_cancellation();
        process_fence.ensure_process_owner_current()?;
        let projected = project(request, before, session.trusted_now.timestamp_millis())?;
        recovery_key = Some(ComputePluginManifestCatalogBindingRecoveryKey {
            authority_instance_binding: authority_instance_binding.clone(),
            root_identity_digest: root_identity_digest.clone(),
            clock_epoch_digest: session.clock_epoch_digest.clone(),
            prepared_at: session.prepared_at,
            request: projected.request.clone(),
            before: projected.before.clone(),
            hashed_receipt: projected.hashed_receipt.clone(),
        });
        observation.ensure_live(Instant::now())?;
        insert_receipt(transaction, &projected)?;
        let stored = read_binding_by_revision(transaction, projected.request.catalog_revision)?
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_MISSING"))?;
        validate_exact_request(&stored.request, &projected.request)?;
        if stored.hashed_receipt != projected.hashed_receipt {
            bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_CHANGED");
        }
        validate_authority_after(transaction, &projected, &session.trusted_now)?;
        observation.ensure_live(Instant::now())?;
        process_fence.ensure_process_owner_current()?;
        Ok(stored.hashed_receipt)
    });
    match outcome {
        Ok(receipt) => match process_fence
            .ensure_process_owner_current()
            .and_then(|()| observation.ensure_live(Instant::now()))
        {
            Ok(()) => {
                ComputePluginManifestCatalogBindingStoreResult::Durable(durable(authority, receipt))
            }
            Err(error) => match recovery_key {
                Some(key) => recovery(authority, candidate, key, error),
                None => rejected(authority, candidate, error),
            },
        },
        Err(error) => match recovery_key {
            Some(key) => recovery(authority, candidate, key, error),
            None => rejected(authority, candidate, error),
        },
    }
}

fn validate_state_session(
    state: &super::types::ManifestCatalogAuthorityState,
    process_fence: &ComputePluginFetchProcessFence,
) -> Result<()> {
    if state.installation_id_digest != process_fence.installation_id_digest()
        || state.process_owner_epoch != process_fence.process_owner_epoch()
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_PROCESS_FENCE_CHANGED");
    }
    Ok(())
}

fn validate_absence_for_new_binding(
    transaction: &Transaction<'_>,
    request: &PreparedManifestCatalogBindingRequest,
    before: &super::types::ManifestCatalogAuthorityState,
) -> Result<()> {
    let maximum = transaction
        .query_row(
            "SELECT MAX(catalog_revision) FROM manifest_catalog_binding_receipts",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_MAX_REVISION_READ")?;
    let request_id_conflict = transaction
        .query_row(
            "SELECT COUNT(*) FROM manifest_catalog_binding_receipts WHERE request_id = ?1 OR request_digest = ?2",
            params![&request.request_id, &request.request_digest],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_REQUEST_CONFLICT_READ")?;
    if request_id_conflict != 0
        || maximum.is_some_and(|revision| request.catalog_revision <= revision)
        || maximum.is_none() && request.catalog_revision < before.manifest_catalog_revision
        || maximum.is_some() && request.catalog_revision <= before.manifest_catalog_revision
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_REVISION_STALE_OR_CONFLICT");
    }
    Ok(())
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    projected: &super::types::ProjectedManifestCatalogBinding,
) -> Result<()> {
    let request = &projected.request;
    let receipt = &projected.hashed_receipt.receipt;
    let receipt_json =
        serde_json::to_string(receipt).context("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_JSON")?;
    let inserted = transaction
        .execute(
            r#"INSERT INTO manifest_catalog_binding_receipts (
                catalog_revision, manifest_catalog_revision_before,
                request_digest, request_id, installation_id_digest,
                catalog_json, catalog_digest, signed_catalog_json,
                signed_catalog_envelope_digest, control_signing_key_id,
                control_signing_key_fingerprint, signed_manifests_json,
                signed_manifest_set_digest,
                catalog_entry_count, node_profile_digest, target_id,
                host_api_protocol_id, host_api_revision, keyring_bundle_revision,
                publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest,
                state_revision_before, state_revision_after,
                inventory_revision, inventory_digest,
                authority_epoch_before, authority_epoch_after, process_owner_epoch,
                trusted_time_before_ms, clock_status_before,
                authority_updated_at_ms_before, bound_at_ms,
                receipt_json, receipt_digest
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19,
                ?20, ?21, ?22, ?23, ?24, ?25,
                ?26, ?27, ?28, ?29, ?30, ?31, 'trusted',
                ?32, ?33, ?34, ?35
            )"#,
            params![
                receipt.catalog_revision,
                receipt.manifest_catalog_revision_before,
                &request.request_digest,
                &request.request_id,
                &request.installation_id_digest,
                &request.catalog_json,
                &request.catalog_digest,
                &request.signed_catalog_json,
                &request.signed_catalog_envelope_digest,
                &request.control_signing_key_id,
                &request.control_signing_key_fingerprint,
                &request.signed_manifests_json,
                &request.signed_manifest_set_digest,
                request.catalog_entry_count,
                &request.node_profile_digest,
                &request.target_id,
                &request.host_api_protocol_id,
                request.host_api_revision,
                request.keyring_bundle_revision,
                request.publisher_keyring.revision,
                &request.publisher_keyring.digest,
                request.control_keyring.revision,
                &request.control_keyring.digest,
                receipt.state_revision_before,
                receipt.state_revision_after,
                receipt.inventory_revision,
                &receipt.inventory_digest,
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
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_INSERT")?;
    if inserted != 1 {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_RECEIPT_CAS");
    }
    Ok(())
}
