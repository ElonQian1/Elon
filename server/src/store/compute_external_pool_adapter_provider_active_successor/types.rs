use std::marker::PhantomData;

use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_provider_active_successor::{
            ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
            ExternalPoolAdapterProviderActiveSuccessorProcessCustody,
            ExternalPoolAdapterProviderActiveSuccessorReceipt,
            ExternalPoolAdapterProviderActiveSuccessorRevocationReceipt,
        },
        provider::ComputeProvider,
    },
    store::{
        compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
        compute_external_pool_adapter_supervisor_session_policy_companion::CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
        compute_provider_registry::ComputeProviderRegistrationReceipt,
    },
};

pub(super) struct StoredExternalPoolAdapterProviderActiveSuccessor {
    pub(super) receipt: ExternalPoolAdapterProviderActiveSuccessorReceipt,
    pub(super) receipt_json: String,
    pub(super) process_custody: ExternalPoolAdapterProviderActiveSuccessorProcessCustody,
    pub(super) receipt_integrity_digest: String,
}

pub(super) struct StoredExternalPoolAdapterProviderActiveSuccessorRevocation {
    pub(super) receipt: ExternalPoolAdapterProviderActiveSuccessorRevocationReceipt,
    pub(super) revocation_json: String,
    pub(super) process_custody: ExternalPoolAdapterProviderActiveSuccessorProcessCustody,
    pub(super) receipt_integrity_digest: String,
}

/// A transaction-bound, non-authorizing source/target/root projection for future V275.
///
/// It intentionally carries no active observation, V272 carrier, process seal, activation witness,
/// route, executor, fence, effect, or readiness and implements neither Clone, Debug nor Serde.
pub(in crate::store) struct PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'tx, 'conn> {
    source: ComputeProviderRegistrationReceipt,
    target: ComputeProvider,
    activation_root: ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
    companion: CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    runtime_compatibility:
        CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn>,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'tx, 'conn> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        _transaction: &'tx Transaction<'conn>,
        source: ComputeProviderRegistrationReceipt,
        target: ComputeProvider,
        activation_root: ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
        companion: CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
        runtime_compatibility: CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<
            'tx,
            'conn,
        >,
        checked_at: String,
    ) -> Self {
        Self {
            source,
            target,
            activation_root,
            companion,
            runtime_compatibility,
            checked_at,
            transaction: PhantomData,
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

    pub(in crate::store) fn companion(
        &self,
    ) -> &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority {
        &self.companion
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    pub(super) fn runtime_compatibility(
        &self,
    ) -> &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn> {
        &self.runtime_compatibility
    }
}
