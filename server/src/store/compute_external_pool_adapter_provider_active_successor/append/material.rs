//! Canonical V274 receipt material and its one-shot pending process seal.

use anyhow::{bail, Result};

use crate::{
    compute_federation::external_pool_adapter_provider_active_successor::{
        canonical_external_pool_adapter_provider_active_successor_receipt_json_and_digest,
        provider_active_successor_private_integrity_digest,
        validate_external_pool_adapter_provider_active_successor_receipt,
        ExternalPoolAdapterProviderActiveSuccessorMaterial,
        ExternalPoolAdapterProviderActiveSuccessorProcessCustody,
        ExternalPoolAdapterProviderActiveSuccessorReceipt,
        PROVIDER_ACTIVE_SUCCESSOR_CANONICALIZATION, PROVIDER_ACTIVE_SUCCESSOR_DIGEST_ALGORITHM,
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND, PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_SCHEMA,
    },
    store::compute_external_pool_adapter_runtime_bundle::{
        ExternalPoolAdapterProviderActiveSuccessorProcessSealInput,
        ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    },
};

/// Opaque exact row plus the pending process-registry tuple required by the INSERT trigger.
///
/// Dropping it attempts best-effort pending cleanup. Cleanup is not an authority boundary: a
/// pending tuple never authorizes current consumption and is lost on process restart.
pub(in crate::store::compute_external_pool_adapter_provider_active_successor) struct PendingExternalPoolAdapterProviderActiveSuccessorAppend<
    'runtime,
> {
    pub(super) receipt: ExternalPoolAdapterProviderActiveSuccessorReceipt,
    pub(super) receipt_json: String,
    pub(super) process_custody: ExternalPoolAdapterProviderActiveSuccessorProcessCustody,
    pub(super) receipt_integrity_digest: String,
    pub(super) runtime: &'runtime ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    promoted: bool,
}

impl PendingExternalPoolAdapterProviderActiveSuccessorAppend<'_> {
    pub(super) fn receipt(&self) -> &ExternalPoolAdapterProviderActiveSuccessorReceipt {
        &self.receipt
    }

    pub(super) fn receipt_integrity_digest(&self) -> &str {
        &self.receipt_integrity_digest
    }

    pub(super) fn mark_promoted(&mut self) {
        self.promoted = true;
    }
}

impl Drop for PendingExternalPoolAdapterProviderActiveSuccessorAppend<'_> {
    fn drop(&mut self) {
        if !self.promoted {
            let _ = self
                .runtime
                .discard_pending_provider_active_successor_process_seal(
                    PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
                    &self.receipt.active_successor_receipt_id,
                    &self.receipt_integrity_digest,
                );
        }
    }
}

pub(super) fn prepare_pending_append(
    runtime: &ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    active_successor_receipt_id: String,
    successor: ExternalPoolAdapterProviderActiveSuccessorMaterial,
) -> Result<PendingExternalPoolAdapterProviderActiveSuccessorAppend<'_>> {
    if successor.created_at != successor.evidence_checked_at {
        bail!("V274 created_at must equal final evidence_checked_at");
    }
    let mut receipt = ExternalPoolAdapterProviderActiveSuccessorReceipt {
        schema: PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_SCHEMA.into(),
        active_successor_receipt_id,
        receipt_digest: String::new(),
        canonicalization: PROVIDER_ACTIVE_SUCCESSOR_CANONICALIZATION.into(),
        digest_algorithm: PROVIDER_ACTIVE_SUCCESSOR_DIGEST_ALGORITHM.into(),
        successor,
    };
    receipt.receipt_digest =
        canonical_external_pool_adapter_provider_active_successor_receipt_json_and_digest(
            &receipt,
        )?
        .1;
    validate_external_pool_adapter_provider_active_successor_receipt(&receipt)?;
    let (receipt_json, receipt_digest) =
        canonical_external_pool_adapter_provider_active_successor_receipt_json_and_digest(
            &receipt,
        )?;
    if receipt_digest != receipt.receipt_digest {
        bail!("V274 receipt digest changed after canonical materialization");
    }

    let successor = &receipt.successor;
    let process_seal = runtime.seal_provider_active_successor(
        &ExternalPoolAdapterProviderActiveSuccessorProcessSealInput {
            kind: PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
            entity_id: &receipt.active_successor_receipt_id,
            entity_digest: &receipt.receipt_digest,
            activation_root_digest: &successor.activation.activation_root_digest,
            provider_binding_id: &successor.activation.activation_root.provider_binding_id,
            checked_at: &successor.evidence_checked_at,
            expires_at: &successor.runtime_observation.observation_expires_at,
        },
    )?;
    let process_custody = ExternalPoolAdapterProviderActiveSuccessorProcessCustody {
        process_custody_epoch_digest: runtime.custody_epoch_digest().into(),
        process_custody_nonce_digest: process_seal.process_custody_nonce_digest().into(),
        process_custody_seal_digest: process_seal.process_custody_seal_digest().into(),
    };
    let receipt_integrity_digest = provider_active_successor_private_integrity_digest(
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
        &receipt.receipt_digest,
        &process_custody,
    )?;
    runtime.remember_pending_provider_active_successor_process_seal(
        &receipt_integrity_digest,
        &process_seal,
    )?;
    Ok(PendingExternalPoolAdapterProviderActiveSuccessorAppend {
        receipt,
        receipt_json,
        process_custody,
        receipt_integrity_digest,
        runtime,
        promoted: false,
    })
}
