use std::marker::PhantomData;

use anyhow::{bail, Result};
use rusqlite::Transaction;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::store::compute_external_pool_adapter_runtime_bundle::ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject;
use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::{
            canonical_task_protocol_active_carrier_json_and_digest,
            ExternalPoolAdapterTaskProtocolActiveCarrierMaterial,
            TASK_PROTOCOL_ACTIVE_CARRIER_SCHEMA,
        },
        external_pool_adapter_task_protocol_conformance::ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    },
    store::compute_external_pool_adapter_provider_active_successor::CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority,
};

/// Current registering V272 evidence re-bound to an exact planned projected-active target.
/// It is valid only inside the transaction that re-proved both authorities.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) struct PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'tx, 'conn>
{
    run: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    material_json: String,
    digest: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

/// Current projected-active V272 evidence. The Provider-neutral receipt is retained together with
/// the exact V277 historical carrier that made its active reproof possible.
pub(in crate::store) struct CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<
    'tx,
    'conn,
> {
    receipt: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    carrier: CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn>,
    material_json: String,
    digest: String,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

/// Borrowed-carrier V272 leaf used to pin one exact current run while another authority retains
/// ownership of the renewed-route carrier (for example, an active runtime bundle).
pub(in crate::store) struct CurrentExternalPoolAdapterTaskProtocolProjectedActiveLeafAuthority<
    'authority,
    'tx,
    'conn,
> {
    receipt: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    carrier: &'authority CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn>,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl<'tx, 'conn> PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'tx, 'conn> {
    pub(super) fn new(
        _transaction: &'tx Transaction<'conn>,
        no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, 'tx, 'conn>,
        run: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    ) -> Result<Self> {
        let target = no_work.preflight();
        let root = &target.activation_root().activation_root;
        let material = &run.run;
        if target.activation_target_updated_at() > no_work.evidence_checked_at()
            || material.registry_release.registry_release_id != root.registry_release_id
            || material.registry_release.registry_release_digest != root.registry_release_digest
            || material.registry_release.registry_release_material_digest
                != root.registry_release_material_digest
            || material.registry_release.installation_content_digest
                != root.installation_content_digest
            || material.task_protocol_profile_digest != root.task_protocol_profile_digest
        {
            bail!("planned-active V272 carrier differs from registering activation roots");
        }
        let carrier_material = material_for(
            root.provider_binding_id.clone(),
            root.provider_binding_digest.clone(),
            target.activation_root().activation_root_digest.clone(),
            target.target().provider_id.clone(),
            target.target().policy_revision,
            &root.initial_active_provider_digest,
            root.route_adapter_projection_id.clone(),
            &run,
        );
        let (material_json, digest) =
            canonical_task_protocol_active_carrier_json_and_digest(&carrier_material)?;
        Ok(Self {
            run,
            material_json,
            digest,
            transaction: PhantomData,
        })
    }

    pub(in crate::store) fn receipt(
        &self,
    ) -> &ExternalPoolAdapterTaskProtocolConformanceRunReceipt {
        &self.run
    }
    pub(in crate::store) fn material_json(&self) -> &str {
        &self.material_json
    }
    pub(in crate::store) fn digest(&self) -> &str {
        &self.digest
    }

    pub(in crate::store) fn fresh_expires_at_for(
        &self,
        no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, '_, '_>,
    ) -> Result<String> {
        let expires = std::cmp::min(
            no_work.observation().expires_at(),
            self.receipt().run.expires_at.as_str(),
        );
        if expires <= no_work.evidence_checked_at() {
            bail!("planned-active V272/no-work evidence expired before V277 append");
        }
        Ok(expires.into())
    }
}

impl<'tx, 'conn> CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn> {
    pub(super) fn new(
        transaction: &'tx Transaction<'conn>,
        receipt: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
        carrier: CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn>,
        checked_at: String,
    ) -> Result<Self> {
        let activation = carrier.historical_activation();
        let root = &activation.activation_root().activation_root;
        let provider = activation.active_provider();
        let provider_json = serde_json::to_string(provider)?;
        let provider_digest = sha256_hex(provider_json.as_bytes());
        let material = material_for(
            root.provider_binding_id.clone(),
            root.provider_binding_digest.clone(),
            activation.activation_root().activation_root_digest.clone(),
            provider.provider_id.clone(),
            provider.policy_revision,
            &provider_digest,
            root.route_adapter_projection_id.clone(),
            &receipt,
        );
        let (material_json, digest) =
            canonical_task_protocol_active_carrier_json_and_digest(&material)?;
        Ok(Self {
            receipt,
            carrier,
            material_json,
            digest,
            checked_at,
            transaction: PhantomData,
        })
    }

    pub(in crate::store) fn receipt(
        &self,
    ) -> &ExternalPoolAdapterTaskProtocolConformanceRunReceipt {
        &self.receipt
    }
    pub(in crate::store) fn carrier(
        &self,
    ) -> &CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn> {
        &self.carrier
    }
    pub(in crate::store) fn material_json(&self) -> &str {
        &self.material_json
    }
    pub(in crate::store) fn digest(&self) -> &str {
        &self.digest
    }
    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl<'authority, 'tx, 'conn>
    CurrentExternalPoolAdapterTaskProtocolProjectedActiveLeafAuthority<'authority, 'tx, 'conn>
{
    pub(super) fn new(
        transaction: &'tx Transaction<'conn>,
        receipt: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
        carrier: &'authority CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<
            'tx,
            'conn,
        >,
        checked_at: String,
    ) -> Self {
        let _ = transaction;
        Self {
            receipt,
            carrier,
            checked_at,
            transaction: PhantomData,
        }
    }

    pub(in crate::store) fn receipt(
        &self,
    ) -> &ExternalPoolAdapterTaskProtocolConformanceRunReceipt {
        &self.receipt
    }

    pub(in crate::store) fn carrier(
        &self,
    ) -> &CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn> {
        self.carrier
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

#[allow(clippy::too_many_arguments)]
fn material_for(
    provider_binding_id: String,
    provider_binding_digest: String,
    activation_root_digest: String,
    target_active_provider_id: String,
    target_active_provider_policy_revision: i64,
    target_active_provider_digest: &str,
    route_adapter_projection_id: String,
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
) -> ExternalPoolAdapterTaskProtocolActiveCarrierMaterial {
    ExternalPoolAdapterTaskProtocolActiveCarrierMaterial {
        schema: TASK_PROTOCOL_ACTIVE_CARRIER_SCHEMA.into(),
        provider_binding_id,
        provider_binding_digest,
        activation_root_digest,
        target_active_provider_id,
        target_active_provider_policy_revision,
        target_active_provider_digest: target_active_provider_digest.into(),
        route_adapter_projection_id,
        task_protocol_conformance_run_receipt_id: receipt.run_receipt_id.clone(),
        task_protocol_conformance_run_receipt_digest: receipt.run_receipt_digest.clone(),
    }
}

fn sha256_hex(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(value))
}
