//! Store-derived V277 genesis route and durable receipt construction.

use anyhow::{ensure, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::{
            derive_external_pool_projected_v211_adapter_binding,
            derive_external_pool_stable_executor, ExternalPoolAdapterAtomicActivationReceipt,
            ExternalPoolProjectedV211AdapterBinding, ExternalPoolStableExecutorBinding,
            ExternalPoolStableExecutorIdMaterial,
        },
        route_authority::AuthorizedComputeRouteAuthorization,
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::PreparedExternalPoolAdapterCredentialProjectedActiveTransition,
        compute_external_pool_adapter_runtime_bundle::ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
        compute_external_pool_adapter_task_protocol_conformance::PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier,
    },
};

mod material;
mod route;

/// Owned canonical material. It is not authority: only the private transaction kernel can consume
/// the sealed route together with the transaction-bound typed inputs retained by the caller.
pub(in crate::store) struct BuiltExternalPoolAdapterAtomicActivationGenesis {
    route: AuthorizedComputeRouteAuthorization,
    receipt: ExternalPoolAdapterAtomicActivationReceipt,
    target_digest: String,
}

impl BuiltExternalPoolAdapterAtomicActivationGenesis {
    pub(in crate::store) fn route(&self) -> &AuthorizedComputeRouteAuthorization {
        &self.route
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterAtomicActivationReceipt {
        &self.receipt
    }

    pub(in crate::store) fn target_digest(&self) -> &str {
        &self.target_digest
    }
}

pub(in crate::store) fn build_external_pool_adapter_atomic_activation_genesis_on<
    'proof,
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    no_work: &'proof ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, 'tx, 'conn>,
    transition: &PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'proof, 'tx, 'conn>,
    task_protocol: &PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'tx, 'conn>,
) -> Result<BuiltExternalPoolAdapterAtomicActivationGenesis> {
    let planned = no_work.preflight();
    let root = &planned.activation_root().activation_root;
    ensure!(
        transition.planned().activation_root() == planned.activation_root()
            && transition.planned().source().provider == planned.source().provider
            && transition.planned().source().provider_digest == planned.source().provider_digest
            && transition.planned().target() == planned.target(),
        "V277 genesis builder received a different planned transition"
    );
    let target_json = serde_json::to_string(planned.target())?;
    let target_digest = sha256_hex(target_json.as_bytes());
    ensure!(
        target_json == root.initial_active_provider_json
            && target_digest == root.initial_active_provider_digest,
        "V277 genesis target differs from the frozen V274 root"
    );
    let (_, projected) = derive_external_pool_projected_v211_adapter_binding(
        planned.target(),
        &root.route_adapter_projection_id,
    )?;
    let stable = derive_external_pool_stable_executor(
        ExternalPoolStableExecutorIdMaterial {
            provider_binding_id: root.provider_binding_id.clone(),
            provider_binding_digest: root.provider_binding_digest.clone(),
            activation_root_digest: planned.activation_root().activation_root_digest.clone(),
            route_adapter_projection_id: root.route_adapter_projection_id.clone(),
            service_actor_id: root.service_actor_id.clone(),
            task_production_carrier_policy_digest: root
                .task_production_carrier_policy_digest
                .clone(),
        },
        root.logical_projection_compatibility_digest.clone(),
        projected.projected_v211_adapter_binding_digest.clone(),
        root.lane_subject_digest.clone(),
    )?;
    let route = route::build_genesis_route_on(
        transaction,
        no_work,
        transition,
        task_protocol,
        &stable,
        &projected,
    )?;
    let receipt = material::build_genesis_receipt(
        no_work,
        transition,
        task_protocol,
        &stable,
        &projected,
        &route,
        target_json,
        target_digest.clone(),
    )?;
    Ok(BuiltExternalPoolAdapterAtomicActivationGenesis {
        route,
        receipt,
        target_digest,
    })
}

fn sha256_hex(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(value))
}
