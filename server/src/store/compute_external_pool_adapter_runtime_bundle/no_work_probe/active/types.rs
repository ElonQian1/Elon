//! Owned, non-authorizing subjects for planned genesis and durable active refresh.

use std::marker::PhantomData;

use rusqlite::Transaction;

use crate::store::{
    compute_external_pool_adapter_runtime_bundle::{
        secret_delivery::{
            CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
            ExternalPoolAdapterEphemeralSecretDeliveryBinding,
        },
        CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority,
    },
    compute_external_pool_adapter_task_protocol_conformance::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority,
};
use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::ExternalPoolAdapterAtomicActivationReceipt,
        external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
        external_pool_adapter_supervisor_session_policy_companion::ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
        provider::ComputeProvider,
    },
    store::compute_provider_registry::ComputeProviderRegistrationReceipt,
};
use elon_external_pool_adapter_session_core::ExternalPoolAdapterNoWorkProbeHostReceipt;

/// Owned genesis intent frozen before external I/O. It is neither current nor durable authority.
/// Construction is restricted to the typed V274 target preflight.
pub(in crate::store) struct PlannedExternalPoolAdapterActiveNoWorkProbeSubject {
    source: ComputeProviderRegistrationReceipt,
    target: ComputeProvider,
    activation_root: ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
    transport_target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
    companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
    runtime_compatibility_verification_receipt_id: String,
    runtime_compatibility_verification_receipt_digest: String,
    activation_target_updated_at: String,
}

impl PlannedExternalPoolAdapterActiveNoWorkProbeSubject {
    pub(super) fn new(
        source: ComputeProviderRegistrationReceipt,
        target: ComputeProvider,
        activation_root: ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
        transport_target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
        companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        runtime_compatibility_verification_receipt_id: String,
        runtime_compatibility_verification_receipt_digest: String,
        activation_target_updated_at: String,
    ) -> Self {
        Self {
            source,
            target,
            activation_root,
            transport_target,
            companion,
            runtime_compatibility_verification_receipt_id,
            runtime_compatibility_verification_receipt_digest,
            activation_target_updated_at,
        }
    }

    pub(in crate::store) fn source(&self) -> &ComputeProviderRegistrationReceipt {
        &self.source
    }

    pub(in crate::store) fn target(&self) -> &ComputeProvider {
        &self.target
    }

    pub(in crate::store) fn activation_root(
        &self,
    ) -> &ExternalPoolAdapterProviderActiveSuccessorActivationRoot {
        &self.activation_root
    }

    pub(in crate::store) fn transport_target(
        &self,
    ) -> &ExternalPoolAdapterUpstreamTransportTargetReceipt {
        &self.transport_target
    }

    pub(in crate::store) fn companion(
        &self,
    ) -> &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt {
        &self.companion
    }

    pub(in crate::store) fn activation_target_updated_at(&self) -> &str {
        &self.activation_target_updated_at
    }

    pub(in crate::store) fn runtime_compatibility_verification_receipt_id(&self) -> &str {
        &self.runtime_compatibility_verification_receipt_id
    }

    pub(in crate::store) fn runtime_compatibility_verification_receipt_digest(&self) -> &str {
        &self.runtime_compatibility_verification_receipt_digest
    }
}

/// Owned durable-active intent frozen before external I/O from the layer-two S1 carrier.
///
/// The original carrier is transaction-bound and is deliberately not retained. A final callback
/// must obtain a new carrier and compare every field before it may authorize a V274 append.
pub(super) struct DurableExternalPoolAdapterActiveNoWorkProbeSubject {
    activation_receipt: ExternalPoolAdapterAtomicActivationReceipt,
    activation_root: ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
    active_provider: ComputeProvider,
    transport_target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
    companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
    route_renewal_receipt_id: String,
    route_renewal_receipt_digest: String,
    route_effective_expires_at: String,
    credential_reattestation_receipt_id: String,
    credential_reattestation_receipt_digest: String,
    runtime_compatibility_verification_receipt_id: String,
    runtime_compatibility_verification_receipt_digest: String,
    task_protocol_run_receipt_id: String,
    task_protocol_run_receipt_digest: String,
    preflight_checked_at: String,
}

impl DurableExternalPoolAdapterActiveNoWorkProbeSubject {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        activation_receipt: ExternalPoolAdapterAtomicActivationReceipt,
        activation_root: ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
        active_provider: ComputeProvider,
        transport_target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
        companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        route_renewal_receipt_id: String,
        route_renewal_receipt_digest: String,
        route_effective_expires_at: String,
        credential_reattestation_receipt_id: String,
        credential_reattestation_receipt_digest: String,
        runtime_compatibility_verification_receipt_id: String,
        runtime_compatibility_verification_receipt_digest: String,
        task_protocol_run_receipt_id: String,
        task_protocol_run_receipt_digest: String,
        preflight_checked_at: String,
    ) -> Self {
        Self {
            activation_receipt,
            activation_root,
            active_provider,
            transport_target,
            companion,
            route_renewal_receipt_id,
            route_renewal_receipt_digest,
            route_effective_expires_at,
            credential_reattestation_receipt_id,
            credential_reattestation_receipt_digest,
            runtime_compatibility_verification_receipt_id,
            runtime_compatibility_verification_receipt_digest,
            task_protocol_run_receipt_id,
            task_protocol_run_receipt_digest,
            preflight_checked_at,
        }
    }

    pub(in crate::store) fn activation_receipt(
        &self,
    ) -> &ExternalPoolAdapterAtomicActivationReceipt {
        &self.activation_receipt
    }

    pub(in crate::store) fn activation_root(
        &self,
    ) -> &ExternalPoolAdapterProviderActiveSuccessorActivationRoot {
        &self.activation_root
    }

    pub(in crate::store) fn active_provider(&self) -> &ComputeProvider {
        &self.active_provider
    }

    pub(in crate::store) fn transport_target(
        &self,
    ) -> &ExternalPoolAdapterUpstreamTransportTargetReceipt {
        &self.transport_target
    }

    pub(in crate::store) fn companion(
        &self,
    ) -> &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt {
        &self.companion
    }

    pub(in crate::store) fn preflight_checked_at(&self) -> &str {
        &self.preflight_checked_at
    }

    pub(in crate::store) fn route_renewal_receipt_id(&self) -> &str {
        &self.route_renewal_receipt_id
    }

    pub(in crate::store) fn route_renewal_receipt_digest(&self) -> &str {
        &self.route_renewal_receipt_digest
    }

    pub(in crate::store) fn route_effective_expires_at(&self) -> &str {
        &self.route_effective_expires_at
    }

    pub(in crate::store) fn credential_reattestation_receipt_id(&self) -> &str {
        &self.credential_reattestation_receipt_id
    }

    pub(in crate::store) fn credential_reattestation_receipt_digest(&self) -> &str {
        &self.credential_reattestation_receipt_digest
    }

    pub(in crate::store) fn runtime_compatibility_verification_receipt_id(&self) -> &str {
        &self.runtime_compatibility_verification_receipt_id
    }

    pub(in crate::store) fn runtime_compatibility_verification_receipt_digest(&self) -> &str {
        &self.runtime_compatibility_verification_receipt_digest
    }

    pub(in crate::store) fn task_protocol_run_receipt_id(&self) -> &str {
        &self.task_protocol_run_receipt_id
    }

    pub(in crate::store) fn task_protocol_run_receipt_digest(&self) -> &str {
        &self.task_protocol_run_receipt_digest
    }
}

/// Final #5/#6 transaction proof of authenticated no-work for one renewed-route active subject.
pub(in crate::store) struct CurrentExternalPoolAdapterProjectedActiveNoWorkObservationAuthority<
    'authority,
    'tx,
    'conn,
> {
    receipt: &'authority ExternalPoolAdapterNoWorkProbeHostReceipt,
    binding: &'authority ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    selected_address: std::net::SocketAddr,
    bundle: &'authority CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'tx, 'conn>,
    task_protocol:
        &'authority CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn>,
    cleaned: &'authority CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
    runtime_bundle_identity_commitment: &'authority str,
    post_cleanup_observation_commitment: String,
    checked_at: String,
    expires_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'authority, 'tx, 'conn>
    CurrentExternalPoolAdapterProjectedActiveNoWorkObservationAuthority<'authority, 'tx, 'conn>
{
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        receipt: &'authority ExternalPoolAdapterNoWorkProbeHostReceipt,
        binding: &'authority ExternalPoolAdapterEphemeralSecretDeliveryBinding,
        selected_address: std::net::SocketAddr,
        bundle: &'authority CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<
            'tx,
            'conn,
        >,
        task_protocol: &'authority CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<
            'tx,
            'conn,
        >,
        cleaned: &'authority CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
        runtime_bundle_identity_commitment: &'authority str,
        post_cleanup_observation_commitment: String,
        checked_at: String,
        expires_at: String,
    ) -> Self {
        Self {
            receipt,
            binding,
            selected_address,
            bundle,
            task_protocol,
            cleaned,
            runtime_bundle_identity_commitment,
            post_cleanup_observation_commitment,
            checked_at,
            expires_at,
            transaction: PhantomData,
        }
    }

    pub(in crate::store) fn no_work_observed(&self) -> bool {
        self.cleaned.authenticated_shutdown_completed()
            && self.cleaned.pidfd_reaped()
            && self.cleaned.cgroup_cleaned()
            && self.cleaned.scratch_cleaned()
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterNoWorkProbeHostReceipt {
        self.receipt
    }

    pub(in crate::store) fn binding(&self) -> &ExternalPoolAdapterEphemeralSecretDeliveryBinding {
        self.binding
    }

    pub(in crate::store) fn selected_address(&self) -> std::net::SocketAddr {
        self.selected_address
    }

    pub(in crate::store) fn bundle(
        &self,
    ) -> &CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'tx, 'conn> {
        self.bundle
    }

    pub(in crate::store) fn task_protocol(
        &self,
    ) -> &CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn> {
        self.task_protocol
    }

    pub(in crate::store) fn post_cleanup_observation_commitment(&self) -> &str {
        &self.post_cleanup_observation_commitment
    }

    pub(in crate::store) fn runtime_bundle_identity_commitment(&self) -> &str {
        self.runtime_bundle_identity_commitment
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    pub(in crate::store) fn probe_checked_at(&self) -> &str {
        self.cleaned.delivery_checked_at()
    }

    pub(in crate::store) fn expires_at(&self) -> &str {
        &self.expires_at
    }
}
