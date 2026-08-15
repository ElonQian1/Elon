use serde::Serialize;
use std::marker::PhantomData;

use rusqlite::Transaction;

use crate::compute_federation::external_pool_adapter_provider_runtime_readiness::{
    ExternalPoolAdapterProviderRuntimeReadinessCurrentnessSummary,
    ExternalPoolAdapterProviderRuntimeReadinessReceipt,
    ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt,
    ExternalPoolAdapterProviderRuntimeReadinessSafeSummary,
};
use crate::store::compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority;
use crate::store::{
    compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
    compute_external_pool_adapter_runtime_bundle::CurrentExternalPoolAdapterRuntimeBundleAuthority,
    compute_external_pool_adapter_sandbox_reattestation::CurrentExternalPoolAdapterSandboxReattestationAuthority,
    compute_external_pool_adapter_supervisor_session_policy_companion::CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    compute_external_pool_adapter_vulnerability_reattestation::CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
};

pub(crate) struct CreateExternalPoolAdapterProviderRuntimeReadiness {
    pub provider_binding_id: String,
    pub expected_provider_binding_digest: String,
    pub expected_installation_receipt_id: String,
    pub expected_installation_receipt_digest: String,
    pub candidate_id: String,
    pub expected_candidate_digest: String,
    pub profile_id: String,
    pub expected_profile_digest: String,
    pub target_id: String,
    pub expected_target_digest: String,
    pub companion_id: String,
    pub expected_companion_digest: String,
    pub runtime_compatibility_verification_receipt_id: String,
    pub expected_runtime_compatibility_verification_receipt_digest: String,
    pub predecessor_readiness_receipt_id: Option<String>,
    pub expected_predecessor_readiness_receipt_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_by_actor_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

pub(crate) struct RevokeExternalPoolAdapterProviderRuntimeReadiness {
    pub provider_binding_id: String,
    pub candidate_id: String,
    pub profile_id: String,
    pub target_id: String,
    pub companion_id: String,
    pub readiness_receipt_id: String,
    pub expected_readiness_receipt_digest: String,
    pub revoked_by_actor_kind: String,
    pub revoked_by_actor_user_id: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessWriteReceipt {
    pub readiness: ExternalPoolAdapterProviderRuntimeReadinessSafeSummary,
    pub replayed: bool,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessRevocationWriteReceipt {
    pub readiness: ExternalPoolAdapterProviderRuntimeReadinessSafeSummary,
    pub revocation: ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt,
    pub replayed: bool,
}

pub(super) struct StoredProviderRuntimeReadiness {
    pub(super) receipt: ExternalPoolAdapterProviderRuntimeReadinessReceipt,
    pub(super) receipt_json: String,
}

pub(super) struct StoredProviderRuntimeReadinessRevocation {
    pub(super) receipt: ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt,
    pub(super) receipt_json: String,
}

/// Transaction-bound authority. Intentionally non-Clone/non-Debug/non-Serde.
pub(in crate::store) struct CurrentExternalPoolAdapterProviderRuntimeReadinessAuthority<'tx, 'conn>
{
    receipt: ExternalPoolAdapterProviderRuntimeReadinessReceipt,
    bundle: CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn>,
    companion: CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
    credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
    runtime_compatibility:
        CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn>,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentExternalPoolAdapterProviderRuntimeReadinessAuthority<'tx, 'conn> {
    pub(super) fn new(
        _transaction: &'tx Transaction<'conn>,
        receipt: ExternalPoolAdapterProviderRuntimeReadinessReceipt,
        bundle: CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn>,
        companion: CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
        vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
        sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
        credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
        runtime_compatibility: CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<
            'tx,
            'conn,
        >,
        checked_at: String,
    ) -> Self {
        Self {
            receipt,
            bundle,
            companion,
            vulnerability,
            sandbox,
            credential,
            runtime_compatibility,
            checked_at,
            transaction: PhantomData,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterProviderRuntimeReadinessReceipt {
        &self.receipt
    }

    pub(in crate::store) fn runtime_compatibility(
        &self,
    ) -> &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn> {
        &self.runtime_compatibility
    }

    pub(in crate::store) fn bundle(
        &self,
    ) -> &CurrentExternalPoolAdapterRuntimeBundleAuthority<'tx, 'conn> {
        &self.bundle
    }

    pub(in crate::store) fn companion(
        &self,
    ) -> &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority {
        &self.companion
    }

    pub(in crate::store) fn vulnerability(
        &self,
    ) -> &CurrentExternalPoolAdapterVulnerabilityReattestationAuthority {
        &self.vulnerability
    }

    pub(in crate::store) fn sandbox(
        &self,
    ) -> &CurrentExternalPoolAdapterSandboxReattestationAuthority {
        &self.sandbox
    }

    pub(in crate::store) fn credential(
        &self,
    ) -> &CurrentExternalPoolAdapterCredentialReattestationAuthority {
        &self.credential
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

pub(super) type ProviderRuntimeReadinessCurrentness =
    ExternalPoolAdapterProviderRuntimeReadinessCurrentnessSummary;
