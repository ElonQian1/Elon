use std::marker::PhantomData;

use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_artifact_package::ExternalPoolAdapterArtifactPackageReceipt,
        external_pool_adapter_atomic_activation::{
            ExternalPoolAdapterAtomicActivationReceipt,
            ExternalPoolAdapterAtomicActivationRouteClosure,
        },
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_provider_active_successor::{
            ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
            ExternalPoolAdapterProviderActiveSuccessorReceipt,
        },
        external_pool_adapter_runtime_launch_profile::ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        external_pool_adapter_supervisor_session_policy_companion::ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
        provider::ComputeProvider,
    },
    store::{
        compute_external_pool_adapter_artifact_source::ExternalPoolAdapterArtifactSourceAuthority,
        compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
        compute_external_pool_adapter_registry::{
            CurrentExternalPoolAdapterRegistryReleaseAuthority,
            HistoricalExternalPoolAdapterRegistryProviderBindingAuthority,
            HistoricalExternalPoolAdapterRegistryReleaseAuthority,
        },
        compute_external_pool_adapter_release_lifecycle::CurrentExternalPoolAdapterReleaseAdmissionAuthority,
        compute_external_pool_adapter_route_renewal::CurrentExternalPoolAdapterRenewedRouteAuthority,
        compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
        compute_external_pool_provider_activation_candidate::HistoricalExternalPoolProviderActivationCandidateAuthority,
    },
};

/// Durable V277 authority plus its immutable genesis witness and exact live active projection.
/// It deliberately contains no current V253 authority and performs no current V274-head lookup.
pub(in crate::store) struct HistoricalExternalPoolAdapterAtomicActivationAuthority {
    receipt: ExternalPoolAdapterAtomicActivationReceipt,
    genesis: ExternalPoolAdapterProviderActiveSuccessorReceipt,
    active_provider: ComputeProvider,
}

impl HistoricalExternalPoolAdapterAtomicActivationAuthority {
    pub(super) fn new(
        receipt: ExternalPoolAdapterAtomicActivationReceipt,
        genesis: ExternalPoolAdapterProviderActiveSuccessorReceipt,
        active_provider: ComputeProvider,
    ) -> Self {
        Self {
            receipt,
            genesis,
            active_provider,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterAtomicActivationReceipt {
        &self.receipt
    }

    pub(in crate::store) fn genesis(&self) -> &ExternalPoolAdapterProviderActiveSuccessorReceipt {
        &self.genesis
    }

    pub(in crate::store) fn activation_root(
        &self,
    ) -> &ExternalPoolAdapterProviderActiveSuccessorActivationRoot {
        &self.genesis.successor.activation
    }

    pub(in crate::store) fn active_provider(&self) -> &ComputeProvider {
        &self.active_provider
    }

    pub(in crate::store) fn route_closure(
        &self,
    ) -> &ExternalPoolAdapterAtomicActivationRouteClosure {
        &self.receipt.activation.route_closure
    }
}

/// Purpose-specific active execution carrier. Historical roots remain immutable while release,
/// policy, V253/V268 and prepared-content authority are re-proved at one transaction timestamp.
pub(in crate::store) struct CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<
    'tx,
    'conn,
> {
    historical_activation: HistoricalExternalPoolAdapterAtomicActivationAuthority,
    renewed_route: CurrentExternalPoolAdapterRenewedRouteAuthority<'tx, 'conn>,
    registry_binding: HistoricalExternalPoolAdapterRegistryProviderBindingAuthority,
    registry_release: HistoricalExternalPoolAdapterRegistryReleaseAuthority,
    current_release: CurrentExternalPoolAdapterRegistryReleaseAuthority,
    candidate: HistoricalExternalPoolProviderActivationCandidateAuthority,
    profile: ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
    companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
    credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
    runtime_compatibility:
        CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn>,
    release_admission: CurrentExternalPoolAdapterReleaseAdmissionAuthority,
    package: ExternalPoolAdapterArtifactPackageReceipt,
    source: ExternalPoolAdapterArtifactSourceAuthority,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'tx, 'conn> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        _transaction: &'tx Transaction<'conn>,
        historical_activation: HistoricalExternalPoolAdapterAtomicActivationAuthority,
        renewed_route: CurrentExternalPoolAdapterRenewedRouteAuthority<'tx, 'conn>,
        registry_binding: HistoricalExternalPoolAdapterRegistryProviderBindingAuthority,
        registry_release: HistoricalExternalPoolAdapterRegistryReleaseAuthority,
        current_release: CurrentExternalPoolAdapterRegistryReleaseAuthority,
        candidate: HistoricalExternalPoolProviderActivationCandidateAuthority,
        profile: ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
        companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
        runtime_compatibility: CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<
            'tx,
            'conn,
        >,
        release_admission: CurrentExternalPoolAdapterReleaseAdmissionAuthority,
        package: ExternalPoolAdapterArtifactPackageReceipt,
        source: ExternalPoolAdapterArtifactSourceAuthority,
        prepared: PreparedExternalPoolAdapterInstallation,
        checked_at: String,
    ) -> Self {
        Self {
            historical_activation,
            renewed_route,
            registry_binding,
            registry_release,
            current_release,
            candidate,
            profile,
            target,
            companion,
            credential,
            runtime_compatibility,
            release_admission,
            package,
            source,
            prepared,
            checked_at,
            transaction: PhantomData,
        }
    }

    pub(in crate::store) fn historical_activation(
        &self,
    ) -> &HistoricalExternalPoolAdapterAtomicActivationAuthority {
        &self.historical_activation
    }
    pub(in crate::store) fn renewed_route(
        &self,
    ) -> &CurrentExternalPoolAdapterRenewedRouteAuthority<'tx, 'conn> {
        &self.renewed_route
    }
    pub(in crate::store) fn registry_binding(
        &self,
    ) -> &HistoricalExternalPoolAdapterRegistryProviderBindingAuthority {
        &self.registry_binding
    }
    pub(in crate::store) fn registry_release(
        &self,
    ) -> &HistoricalExternalPoolAdapterRegistryReleaseAuthority {
        &self.registry_release
    }
    pub(in crate::store) fn current_release(
        &self,
    ) -> &CurrentExternalPoolAdapterRegistryReleaseAuthority {
        &self.current_release
    }
    pub(in crate::store) fn candidate(
        &self,
    ) -> &HistoricalExternalPoolProviderActivationCandidateAuthority {
        &self.candidate
    }
    pub(in crate::store) fn profile(&self) -> &ExternalPoolAdapterRuntimeLaunchProfileReceipt {
        &self.profile
    }
    pub(in crate::store) fn target(&self) -> &ExternalPoolAdapterUpstreamTransportTargetReceipt {
        &self.target
    }
    pub(in crate::store) fn companion(
        &self,
    ) -> &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt {
        &self.companion
    }
    pub(in crate::store) fn credential(
        &self,
    ) -> &CurrentExternalPoolAdapterCredentialReattestationAuthority {
        &self.credential
    }
    pub(in crate::store) fn runtime_compatibility(
        &self,
    ) -> &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn> {
        &self.runtime_compatibility
    }
    pub(in crate::store) fn release_admission(
        &self,
    ) -> &CurrentExternalPoolAdapterReleaseAdmissionAuthority {
        &self.release_admission
    }
    pub(in crate::store) fn package(&self) -> &ExternalPoolAdapterArtifactPackageReceipt {
        &self.package
    }
    pub(in crate::store) fn source(&self) -> &ExternalPoolAdapterArtifactSourceAuthority {
        &self.source
    }
    pub(in crate::store) fn prepared(&self) -> &PreparedExternalPoolAdapterInstallation {
        &self.prepared
    }
    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    /// Consumes the purpose-specific carrier after the caller has derived its sealed domain roots.
    pub(in crate::store) fn into_prepared(self) -> PreparedExternalPoolAdapterInstallation {
        self.prepared
    }
}

pub(super) struct StoredExternalPoolAdapterAtomicActivation {
    pub(super) receipt: ExternalPoolAdapterAtomicActivationReceipt,
    pub(super) scalar_values: Vec<rusqlite::types::Value>,
}
