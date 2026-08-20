//! Exact one-row persistence, same-transaction readback, and same-connection postcommit promotion.

use anyhow::{bail, ensure, Result};
use rusqlite::{named_params, Connection, Transaction};

use crate::compute_federation::external_pool_adapter_provider_active_successor::{
    ExternalPoolAdapterProviderActiveSuccessorReceipt,
    PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
};
use crate::store::compute_external_pool_adapter_runtime_bundle::ExternalPoolAdapterAtomicActivationPendingPlanGuard;

use super::super::read::receipt_by_id_on;
use super::material::PendingExternalPoolAdapterProviderActiveSuccessorAppend;

/// Process-authorized durable receipt. Construction requires postcommit exact readback followed by
/// successful promotion of the exact pending purpose seal.
pub(in crate::store) struct CommittedExternalPoolAdapterProviderActiveSuccessorAppend {
    receipt: ExternalPoolAdapterProviderActiveSuccessorReceipt,
}

impl CommittedExternalPoolAdapterProviderActiveSuccessorAppend {
    pub(super) fn new(receipt: ExternalPoolAdapterProviderActiveSuccessorReceipt) -> Self {
        Self { receipt }
    }

    pub(super) fn receipt(&self) -> &ExternalPoolAdapterProviderActiveSuccessorReceipt {
        &self.receipt
    }
}

pub(super) fn insert_and_readback_pending_append_on(
    transaction: &Transaction<'_>,
    pending: &PendingExternalPoolAdapterProviderActiveSuccessorAppend<'_>,
) -> Result<()> {
    let changed = transaction.execute(
        INSERT_RECEIPT,
        named_params! {
            ":receipt_json": &pending.receipt_json,
            ":process_custody_epoch_digest": &pending.process_custody.process_custody_epoch_digest,
            ":process_custody_nonce_digest": &pending.process_custody.process_custody_nonce_digest,
            ":process_custody_seal_digest": &pending.process_custody.process_custody_seal_digest,
            ":receipt_integrity_digest": &pending.receipt_integrity_digest,
        },
    )?;
    if changed != 1 {
        bail!("V274 append did not insert exactly one receipt row");
    }
    require_exact_readback_on(transaction, pending)
}

/// Must be called with the same connection immediately after its owning transaction commits.
pub(in crate::store::compute_external_pool_adapter_provider_active_successor) fn postcommit_external_pool_adapter_provider_active_successor_readback_on(
    connection: &Connection,
    plan_guard: &ExternalPoolAdapterAtomicActivationPendingPlanGuard,
    mut pending: PendingExternalPoolAdapterProviderActiveSuccessorAppend<'_>,
) -> Result<CommittedExternalPoolAdapterProviderActiveSuccessorAppend> {
    ensure!(
        connection.is_autocommit(),
        "V274 seal promotion requires a committed autocommit connection"
    );
    plan_guard.ensure_same_connection(connection)?;
    plan_guard.ensure_fully_consumed()?;
    require_exact_readback_on(connection, &pending)?;
    let promoted = pending
        .runtime
        .promote_provider_active_successor_process_seal(
            connection,
            plan_guard,
            PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND,
            &pending.receipt.active_successor_receipt_id,
            &pending.receipt_integrity_digest,
        )?;
    if !promoted {
        bail!("V274 postcommit readback lost its exact pending purpose seal");
    }
    pending.mark_promoted();
    Ok(CommittedExternalPoolAdapterProviderActiveSuccessorAppend {
        receipt: pending.receipt.clone(),
    })
}

pub(super) fn require_exact_readback_on(
    connection: &Connection,
    pending: &PendingExternalPoolAdapterProviderActiveSuccessorAppend<'_>,
) -> Result<()> {
    let stored = receipt_by_id_on(connection, &pending.receipt.active_successor_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("V274 receipt disappeared during exact readback"))?;
    if stored.receipt != pending.receipt
        || stored.receipt_json != pending.receipt_json
        || stored.process_custody != pending.process_custody
        || stored.receipt_integrity_digest != pending.receipt_integrity_digest
    {
        bail!("V274 receipt readback differs from the pending exact tuple");
    }
    Ok(())
}

const INSERT_RECEIPT: &str = r#"
INSERT INTO compute_external_pool_adapter_provider_active_successor_receipts (
  active_successor_receipt_id, active_successor_receipt_schema, receipt_digest, receipt_json,
  canonicalization, digest_algorithm, provider_binding_id, activation_root_digest,
  successor_sequence, predecessor_active_successor_receipt_id,
  predecessor_active_successor_receipt_digest, activation_root_json, provider_id,
  provider_owner_account_id, source_registering_provider_id,
  source_registering_provider_policy_revision, source_registering_provider_json,
  source_registering_provider_digest, initial_active_provider_id,
  initial_active_provider_policy_revision, initial_active_provider_json,
  initial_active_provider_digest, provider_binding_digest, registry_release_id,
  registry_release_digest, registry_release_material_digest, installation_receipt_id,
  installation_receipt_digest, installation_content_digest, candidate_id, candidate_digest,
  delegation_id, delegation_digest, service_actor_id, logical_adapter_id,
  logical_adapter_binding_digest, logical_projection_compatibility_digest,
  route_adapter_projection_id, profile_id, profile_digest, launch_policy_digest, target_id,
  target_digest, target_policy_digest, companion_id, companion_digest,
  supervisor_session_policy_digest, entrypoint_capsule_policy_digest, launch_image_sha256,
  task_protocol_profile_digest, lane_subject_digest, task_production_carrier_policy_digest,
  evidence_provider_id, evidence_provider_policy_revision, evidence_provider_json,
  evidence_provider_digest, reattestation_receipt_id, reattestation_receipt_digest,
  credential_observed_provider_id, credential_observed_provider_policy_revision,
  credential_observed_provider_json, credential_observed_provider_digest,
  runtime_observation_id, runtime_observation_digest, runtime_observed_provider_id,
  runtime_observed_provider_policy_revision, runtime_observed_provider_json,
  runtime_observed_provider_digest, observation_started_at, observation_completed_at,
  observation_expires_at, task_protocol_conformance_run_receipt_id,
  task_protocol_conformance_run_receipt_digest, task_protocol_conformance_expires_at,
  process_custody_epoch_digest, process_custody_nonce_digest, process_custody_seal_digest,
  activation_witness_id, activation_witness_digest, activation_target_updated_at,
  evidence_checked_at, created_at, effects_json, readiness_json, receipt_integrity_digest
) VALUES (
  json_extract(:receipt_json,'$.active_successor_receipt_id'),
  json_extract(:receipt_json,'$.schema'),
  json_extract(:receipt_json,'$.receipt_digest'),
  :receipt_json,
  json_extract(:receipt_json,'$.canonicalization'),
  json_extract(:receipt_json,'$.digest_algorithm'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.provider_binding_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root_digest'),
  json_extract(:receipt_json,'$.successor.lineage.successor_sequence'),
  json_extract(:receipt_json,'$.successor.lineage.predecessor_active_successor_receipt_id'),
  json_extract(:receipt_json,'$.successor.lineage.predecessor_active_successor_receipt_digest'),
  json_extract(:receipt_json,'$.successor.activation'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.provider_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.provider_owner_account_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.source_registering_provider_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.source_registering_provider_policy_revision'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.source_registering_provider_json'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.source_registering_provider_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.initial_active_provider_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.initial_active_provider_policy_revision'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.initial_active_provider_json'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.initial_active_provider_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.provider_binding_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.registry_release_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.registry_release_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.registry_release_material_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.installation_receipt_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.installation_receipt_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.installation_content_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.candidate_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.candidate_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.delegation_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.delegation_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.service_actor_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.logical_adapter_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.logical_adapter_binding_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.logical_projection_compatibility_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.route_adapter_projection_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.profile_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.profile_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.launch_policy_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.target_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.target_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.target_policy_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.companion_id'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.companion_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.supervisor_session_policy_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.entrypoint_capsule_policy_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.launch_image_sha256'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.task_protocol_profile_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.lane_subject_digest'),
  json_extract(:receipt_json,'$.successor.activation.activation_root.task_production_carrier_policy_digest'),
  json_extract(:receipt_json,'$.successor.evidence_provider.provider_id'),
  json_extract(:receipt_json,'$.successor.evidence_provider.provider_policy_revision'),
  json_extract(:receipt_json,'$.successor.evidence_provider.provider_json'),
  json_extract(:receipt_json,'$.successor.evidence_provider.provider_digest'),
  json_extract(:receipt_json,'$.successor.credential_evidence.reattestation_receipt_id'),
  json_extract(:receipt_json,'$.successor.credential_evidence.reattestation_receipt_digest'),
  json_extract(:receipt_json,'$.successor.credential_evidence.observed_provider.provider_id'),
  json_extract(:receipt_json,'$.successor.credential_evidence.observed_provider.provider_policy_revision'),
  json_extract(:receipt_json,'$.successor.credential_evidence.observed_provider.provider_json'),
  json_extract(:receipt_json,'$.successor.credential_evidence.observed_provider.provider_digest'),
  json_extract(:receipt_json,'$.successor.runtime_observation.runtime_observation_id'),
  json_extract(:receipt_json,'$.successor.runtime_observation.runtime_observation_digest'),
  json_extract(:receipt_json,'$.successor.runtime_observation.observed_provider.provider_id'),
  json_extract(:receipt_json,'$.successor.runtime_observation.observed_provider.provider_policy_revision'),
  json_extract(:receipt_json,'$.successor.runtime_observation.observed_provider.provider_json'),
  json_extract(:receipt_json,'$.successor.runtime_observation.observed_provider.provider_digest'),
  json_extract(:receipt_json,'$.successor.runtime_observation.observation_started_at'),
  json_extract(:receipt_json,'$.successor.runtime_observation.observation_completed_at'),
  json_extract(:receipt_json,'$.successor.runtime_observation.observation_expires_at'),
  json_extract(:receipt_json,'$.successor.task_protocol_evidence.task_protocol_conformance_run_receipt_id'),
  json_extract(:receipt_json,'$.successor.task_protocol_evidence.task_protocol_conformance_run_receipt_digest'),
  json_extract(:receipt_json,'$.successor.task_protocol_evidence.task_protocol_conformance_expires_at'),
  :process_custody_epoch_digest,
  :process_custody_nonce_digest,
  :process_custody_seal_digest,
  json_extract(:receipt_json,'$.successor.activation_witness.activation_witness_id'),
  json_extract(:receipt_json,'$.successor.activation_witness.activation_witness_digest'),
  json_extract(:receipt_json,'$.successor.activation_target_updated_at'),
  json_extract(:receipt_json,'$.successor.evidence_checked_at'),
  json_extract(:receipt_json,'$.successor.created_at'),
  json_extract(:receipt_json,'$.successor.effects'),
  json_extract(:receipt_json,'$.successor.readiness'),
  :receipt_integrity_digest
)
"#;
