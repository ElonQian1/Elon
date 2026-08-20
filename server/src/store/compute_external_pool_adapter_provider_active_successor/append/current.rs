//! Process-current V274 head composed from the V277 carrier plus fresh projected-active V272.

use std::marker::PhantomData;

use anyhow::{bail, Result};
use rusqlite::Transaction;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_provider_active_successor::{
        ExternalPoolAdapterProviderActiveSuccessorReceipt,
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
    },
    store::{
        compute_external_pool_adapter_runtime_bundle::ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
        compute_external_pool_adapter_task_protocol_conformance::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority,
    },
};

use super::super::{
    read::{head_by_binding_and_root_on, revocation_by_target_on},
    types::StoredExternalPoolAdapterProviderActiveSuccessor,
};

/// Transaction-bound current head. It retains fresh projected-active V272 (and its layer-two
/// carrier) plus the committed process seal; neither a diagnostic view nor an old row constructs it.
pub(in crate::store) struct CurrentExternalPoolAdapterProviderActiveSuccessorAuthority<'tx, 'conn> {
    stored: StoredExternalPoolAdapterProviderActiveSuccessor,
    task_protocol: CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn>,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentExternalPoolAdapterProviderActiveSuccessorAuthority<'tx, 'conn> {
    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterProviderActiveSuccessorReceipt {
        &self.stored.receipt
    }

    pub(in crate::store) fn task_protocol(
        &self,
    ) -> &CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn> {
        &self.task_protocol
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

pub(in crate::store) fn require_current_external_pool_adapter_provider_active_successor_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    task_protocol: CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn>,
    runtime: &ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    checked_at: &str,
) -> Result<CurrentExternalPoolAdapterProviderActiveSuccessorAuthority<'tx, 'conn>> {
    let carrier = task_protocol.carrier();
    if carrier.checked_at() != checked_at {
        bail!("V274 current carrier and consumer time anchors differ");
    }
    if task_protocol.checked_at() != checked_at {
        bail!("V274 current V272 carrier and consumer time anchors differ");
    }
    let historical = carrier.historical_activation();
    let v277 = historical.receipt();
    let activation = historical.activation_root();
    let root = &activation.activation_root;
    let stored = head_by_binding_and_root_on(
        transaction,
        &root.provider_binding_id,
        &activation.activation_root_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("V274 current authority lacks a durable head"))?;
    if revocation_by_target_on(transaction, &stored.receipt.active_successor_receipt_id)?.is_some()
    {
        bail!("V274 durable head is revoked");
    }
    let successor = &stored.receipt.successor;
    let current_credential = carrier.credential().receipt();
    let active_provider_json = serde_json::to_string(historical.active_provider())?;
    let active_provider_digest = hex::encode(Sha256::digest(active_provider_json.as_bytes()));
    if &successor.activation != activation
        || successor.activation_witness.activation_witness_id != v277.activation_receipt_id
        || successor.activation_witness.activation_witness_digest != v277.activation_receipt_digest
        || successor.evidence_provider.provider_id != historical.active_provider().provider_id
        || successor.evidence_provider.provider_policy_revision
            != historical.active_provider().policy_revision
        || successor.evidence_provider.provider_json != active_provider_json
        || successor.evidence_provider.provider_digest != active_provider_digest
        || successor.credential_evidence.reattestation_receipt_id
            != current_credential.reattestation_receipt_id
        || successor.credential_evidence.reattestation_receipt_digest
            != current_credential.reattestation_receipt_digest
        || successor
            .task_protocol_evidence
            .task_protocol_conformance_run_receipt_id
            != task_protocol.receipt().run_receipt_id
        || successor
            .task_protocol_evidence
            .task_protocol_conformance_run_receipt_digest
            != task_protocol.receipt().run_receipt_digest
        || successor
            .task_protocol_evidence
            .task_protocol_conformance_expires_at
            != task_protocol.receipt().run.expires_at
        || successor.evidence_checked_at.as_str() > checked_at
        || checked_at
            >= successor
                .runtime_observation
                .observation_expires_at
                .as_str()
    {
        bail!("V274 durable head is not exact for the live V277 active carrier");
    }
    let process_current = runtime.attests_committed_provider_active_successor_process_seal(
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
        &stored.receipt.active_successor_receipt_id,
        &stored.receipt.receipt_digest,
        &stored.process_custody.process_custody_epoch_digest,
        &stored.process_custody.process_custody_nonce_digest,
        &stored.process_custody.process_custody_seal_digest,
        &stored.receipt_integrity_digest,
        &successor.runtime_observation.observation_expires_at,
    )?;
    if !process_current {
        bail!("V274 durable head lacks its exact committed process seal");
    }
    Ok(CurrentExternalPoolAdapterProviderActiveSuccessorAuthority {
        stored,
        task_protocol,
        checked_at: checked_at.into(),
        transaction: PhantomData,
    })
}
