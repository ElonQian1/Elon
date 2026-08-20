//! Owned, non-authorizing subjects for planned genesis and durable active refresh.

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
    preflight_checked_at: String,
}

impl DurableExternalPoolAdapterActiveNoWorkProbeSubject {
    pub(super) fn new(
        activation_receipt: ExternalPoolAdapterAtomicActivationReceipt,
        activation_root: ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
        active_provider: ComputeProvider,
        transport_target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
        companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        preflight_checked_at: String,
    ) -> Self {
        Self {
            activation_receipt,
            activation_root,
            active_provider,
            transport_target,
            companion,
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
}
