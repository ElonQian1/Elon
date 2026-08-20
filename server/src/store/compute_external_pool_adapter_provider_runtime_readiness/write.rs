use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::ExternalPoolAdapterInstallationFsError,
        external_pool_adapter_provider_runtime_readiness::*,
    },
    store::{
        compute_external_pool_adapter_runtime_bundle::{
            CurrentExternalPoolAdapterNoWorkProbeObservationAuthority,
            ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        },
        compute_external_pool_adapter_upstream_transport_target::{
            CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
            ExternalPoolAdapterInstallationReopener,
        },
        new_id, Store,
    },
};

use super::{
    error::ExternalPoolAdapterProviderRuntimeReadinessStoreError as StoreError,
    persistence::insert_readiness,
    read::{readiness_by_id_on, readiness_by_idempotency_on, readiness_head_by_binding_on},
    roots::{audit_create_preflight, build_material_from_observation},
    types::*,
};

impl Store {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(crate) async fn create_external_pool_adapter_provider_runtime_readiness(
        &self,
        input: CreateExternalPoolAdapterProviderRuntimeReadiness,
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    ) -> std::result::Result<ExternalPoolAdapterProviderRuntimeReadinessWriteReceipt, StoreError>
    {
        if let Some(replay) = self.exact_readiness_replay(&input)? {
            return Ok(replay);
        }
        let output = self
            .with_current_external_pool_adapter_no_work_probe_observation(
                &input.profile_id,
                &input.companion_id,
                &input.expected_companion_digest,
                &input.target_id,
                &input.expected_target_digest,
                &input.runtime_compatibility_verification_receipt_id,
                &input.expected_runtime_compatibility_verification_receipt_digest,
                reopen_prepared,
                runtime,
                |transaction, target, checked_at| {
                    preflight_create(transaction, &input, target, checked_at)
                },
                |transaction, observation| {
                    finalize_create(transaction, &input, observation, runtime)
                },
                |_connection, receipt| Ok(receipt),
            )
            .await
            .map_err(classify_create_error)?;
        let output = output.ok_or_else(|| {
            StoreError::conflict(anyhow::anyhow!(
                "provider runtime readiness current roots were not found"
            ))
        })?;
        if !output.replayed {
            let committed_seal = runtime
                .process_custody()
                .commit_readiness_seal(
                    &output.readiness.readiness_receipt_id,
                    &output.readiness.readiness_receipt_digest,
                )
                .map_err(StoreError::storage)?;
            if !committed_seal {
                return Err(StoreError::storage(anyhow::anyhow!(
                    "fresh readiness receipt lost its process seal before publication"
                )));
            }
        }
        Ok(output)
    }

    fn exact_readiness_replay(
        &self,
        input: &CreateExternalPoolAdapterProviderRuntimeReadiness,
    ) -> std::result::Result<
        Option<ExternalPoolAdapterProviderRuntimeReadinessWriteReceipt>,
        StoreError,
    > {
        let conn = self.conn().map_err(StoreError::storage)?;
        let stored =
            readiness_by_idempotency_on(&conn, &input.idempotency_scope, &input.idempotency_key)
                .map_err(StoreError::storage)?;
        stored
            .map(|stored| {
                ensure_create_replay(input, &stored).map_err(StoreError::conflict)?;
                Ok(ExternalPoolAdapterProviderRuntimeReadinessWriteReceipt {
                    readiness: provider_runtime_readiness_safe_summary(&stored.receipt),
                    replayed: true,
                })
            })
            .transpose()
    }
}

fn preflight_create(
    transaction: &Transaction<'_>,
    input: &CreateExternalPoolAdapterProviderRuntimeReadiness,
    target: &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    checked_at: &str,
) -> Result<()> {
    audit_create_preflight(transaction, input, target, checked_at)?;
    if readiness_by_idempotency_on(
        transaction,
        &input.idempotency_scope,
        &input.idempotency_key,
    )?
    .is_some()
    {
        bail!("provider runtime readiness idempotency raced before physical execution")
    }
    let head = readiness_head_by_binding_on(transaction, &input.provider_binding_id)?;
    ensure_predecessor(input, head.as_ref())?;
    Ok(())
}

fn finalize_create(
    transaction: &Transaction<'_>,
    input: &CreateExternalPoolAdapterProviderRuntimeReadiness,
    observation: &CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<'_, '_, '_>,
    runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
) -> Result<ExternalPoolAdapterProviderRuntimeReadinessWriteReceipt> {
    if let Some(stored) = readiness_by_idempotency_on(
        transaction,
        &input.idempotency_scope,
        &input.idempotency_key,
    )? {
        ensure_create_replay(input, &stored)?;
        return Ok(ExternalPoolAdapterProviderRuntimeReadinessWriteReceipt {
            readiness: provider_runtime_readiness_safe_summary(&stored.receipt),
            replayed: true,
        });
    }
    let head = readiness_head_by_binding_on(transaction, &input.provider_binding_id)?;
    ensure_predecessor(input, head.as_ref())?;
    let sequence = head.as_ref().map_or(Ok(1), |stored| {
        stored
            .receipt
            .readiness
            .sequence
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("provider runtime readiness sequence overflow"))
    })?;
    let predecessor_id = head
        .as_ref()
        .map(|stored| stored.receipt.readiness_receipt_id.clone());
    let predecessor_digest = head
        .as_ref()
        .map(|stored| stored.receipt.readiness_receipt_digest.clone());
    let material = build_material_from_observation(
        input,
        observation,
        sequence,
        predecessor_id,
        predecessor_digest,
        new_id("external_pool_adapter_provider_runtime_readiness_probe"),
    )?;
    if canonical_time(&material.expires_at)? <= Utc::now() {
        bail!("provider runtime readiness expired before durable insert")
    }
    let receipt = build_external_pool_adapter_provider_runtime_readiness_receipt(
        new_id("external_pool_adapter_provider_runtime_readiness"),
        material,
    )?;
    insert_readiness(transaction, &receipt)?;
    let stored = readiness_by_id_on(transaction, &receipt.readiness_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("provider runtime readiness disappeared after insert"))?;
    let sealed = &stored.receipt.readiness.sealed_bindings;
    runtime.process_custody().remember_readiness_seal(
        &stored.receipt.readiness_receipt_id,
        &stored.receipt.readiness_receipt_digest,
        &sealed.runtime_bundle_identity_commitment,
        &sealed.post_cleanup_observation_commitment,
        &stored.receipt.readiness.expires_at,
    )?;
    Ok(ExternalPoolAdapterProviderRuntimeReadinessWriteReceipt {
        readiness: provider_runtime_readiness_safe_summary(&stored.receipt),
        replayed: false,
    })
}

fn ensure_predecessor(
    input: &CreateExternalPoolAdapterProviderRuntimeReadiness,
    head: Option<&StoredProviderRuntimeReadiness>,
) -> Result<()> {
    match (
        head,
        &input.predecessor_readiness_receipt_id,
        &input.expected_predecessor_readiness_receipt_digest,
    ) {
        (None, None, None) => Ok(()),
        (Some(stored), Some(id), Some(digest))
            if stored.receipt.readiness_receipt_id == *id
                && stored.receipt.readiness_receipt_digest == *digest =>
        {
            Ok(())
        }
        _ => bail!("provider runtime readiness predecessor is missing, stale, or unexpected"),
    }
}

fn ensure_create_replay(
    input: &CreateExternalPoolAdapterProviderRuntimeReadiness,
    stored: &StoredProviderRuntimeReadiness,
) -> Result<()> {
    let r = &stored.receipt.readiness;
    if r.provider_binding_id != input.provider_binding_id
        || r.provider_binding_digest != input.expected_provider_binding_digest
        || r.installation_receipt_id != input.expected_installation_receipt_id
        || r.installation_receipt_digest != input.expected_installation_receipt_digest
        || r.candidate_id != input.candidate_id
        || r.candidate_digest != input.expected_candidate_digest
        || r.profile_id != input.profile_id
        || r.profile_digest != input.expected_profile_digest
        || r.target_id != input.target_id
        || r.target_digest != input.expected_target_digest
        || r.companion_id != input.companion_id
        || r.companion_digest != input.expected_companion_digest
        || r.runtime_compatibility_verification_receipt_id
            != input.runtime_compatibility_verification_receipt_id
        || r.runtime_compatibility_verification_receipt_digest
            != input.expected_runtime_compatibility_verification_receipt_digest
        || r.predecessor_readiness_receipt_id != input.predecessor_readiness_receipt_id
        || r.predecessor_readiness_receipt_digest
            != input.expected_predecessor_readiness_receipt_digest
        || r.recorded_by_actor_kind != input.recorded_by_actor_kind
        || r.recorded_by_actor_user_id != input.recorded_by_actor_user_id
        || r.idempotency_scope != input.idempotency_scope
        || r.idempotency_key != input.idempotency_key
        || r.confirmation != input.confirmation
    {
        bail!("provider runtime readiness replay conflicts with sealed input")
    }
    Ok(())
}

fn classify_create_error(error: anyhow::Error) -> StoreError {
    if error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<ExternalPoolAdapterInstallationFsError>(),
            Some(ExternalPoolAdapterInstallationFsError::Storage(_))
        )
    }) || error
        .chain()
        .any(|cause| cause.downcast_ref::<std::io::Error>().is_some())
    {
        StoreError::storage(error)
    } else {
        StoreError::classify_write(error)
    }
}

fn canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    Ok(parsed.with_timezone(&Utc))
}
