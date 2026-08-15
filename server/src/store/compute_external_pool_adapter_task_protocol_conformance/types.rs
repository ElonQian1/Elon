use std::marker::PhantomData;

use rusqlite::Transaction;
use serde::Serialize;

use crate::{
    compute_federation::external_pool_adapter_task_protocol_conformance::{
        ExternalPoolAdapterTaskProtocolConformanceEffects,
        ExternalPoolAdapterTaskProtocolConformanceReadiness,
        ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt,
        ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    },
    store::{
        compute_external_pool_adapter_registry::CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
        compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
        compute_external_pool_adapter_sandbox_reattestation::CurrentExternalPoolAdapterSandboxReattestationAuthority,
        compute_external_pool_adapter_vulnerability_reattestation::CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    },
};

/// Service-derived request. Provider binding and installation fields are an execution carrier
/// only; they must never be copied into canonical evidence, durable rows, or process-seal input.
pub(crate) struct CreateExternalPoolAdapterTaskProtocolConformanceRun {
    pub registry_release_id: String,
    pub expected_registry_release_digest: String,
    pub sandbox_reattestation_receipt_id: String,
    pub expected_sandbox_reattestation_receipt_digest: String,
    pub runtime_compatibility_verification_receipt_id: String,
    pub expected_runtime_compatibility_verification_receipt_digest: String,
    pub expected_task_protocol_profile_digest: String,
    pub expected_fixture_catalog_digest: String,
    pub provider_binding_id: String,
    pub expected_provider_binding_digest: String,
    pub expected_installation_receipt_id: String,
    pub expected_installation_receipt_digest: String,
    pub predecessor_run_receipt_id: Option<String>,
    pub expected_predecessor_run_receipt_digest: Option<String>,
    pub recorded_by_admin_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

pub(crate) struct RevokeExternalPoolAdapterTaskProtocolConformanceRun {
    pub registry_release_id: String,
    pub run_receipt_id: String,
    pub expected_run_receipt_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt {
    pub run: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRevocationWriteReceipt {
    pub run: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    pub revocation: ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt,
    pub replayed: bool,
}

/// Public diagnostic only. This value deliberately has no Prepared carrier and cannot be
/// converted into the Store-private authority below.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceCurrentness {
    pub schema: String,
    pub run: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    pub currentness_status: String,
    pub head_status: String,
    pub revocation_status: String,
    pub ttl_status: String,
    pub registry_release_status: String,
    pub vulnerability_reattestation_status: String,
    pub sandbox_reattestation_status: String,
    pub sandbox_verifier_key_status: String,
    pub runtime_compatibility_verification_status: String,
    pub task_protocol_profile_status: String,
    pub fixture_catalog_status: String,
    pub canonical_receipt_integrity_status: String,
    pub receipt_integrity_status: String,
    pub process_custody_status: String,
    pub prepared_reproof_status: String,
    pub checked_at: String,
    pub effects: ExternalPoolAdapterTaskProtocolConformanceEffects,
    pub readiness: ExternalPoolAdapterTaskProtocolConformanceReadiness,
}

pub(super) struct StoredTaskProtocolConformanceRun {
    pub(super) receipt: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    pub(super) receipt_json: String,
    pub(super) recorded_by_admin_user_id: String,
    pub(super) idempotency_scope: String,
    pub(super) idempotency_key: String,
    pub(super) confirmation: String,
    pub(super) runtime_custody_epoch_digest: String,
    pub(super) process_hmac_seal: String,
    pub(super) receipt_integrity_digest: String,
}

pub(super) struct StoredTaskProtocolConformanceRevocation {
    pub(super) receipt: ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt,
    pub(super) receipt_json: String,
    pub(super) revoked_by_admin_user_id: String,
    pub(super) idempotency_scope: String,
    pub(super) idempotency_key: String,
    pub(super) confirmation: String,
}

pub(super) struct TaskProtocolConformanceRunPrivateFields<'a> {
    pub(super) recorded_by_admin_user_id: &'a str,
    pub(super) idempotency_scope: &'a str,
    pub(super) idempotency_key: &'a str,
    pub(super) confirmation: &'a str,
    pub(super) runtime_custody_epoch_digest: &'a str,
    pub(super) process_hmac_seal: &'a str,
    pub(super) receipt_integrity_digest: &'a str,
}

pub(super) struct TaskProtocolConformanceRevocationPrivateFields<'a> {
    pub(super) revoked_by_admin_user_id: &'a str,
    pub(super) idempotency_scope: &'a str,
    pub(super) idempotency_key: &'a str,
    pub(super) confirmation: &'a str,
}

/// Same-transaction authority. Intentionally non-Clone/non-Debug/non-Serde. It retains the
/// Provider-specific Prepared carrier privately while its receipt remains Provider-neutral.
pub(in crate::store) struct CurrentExternalPoolAdapterTaskProtocolConformanceAuthority<'tx, 'conn> {
    receipt: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    carrier: CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
    runtime_compatibility:
        CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn>,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentExternalPoolAdapterTaskProtocolConformanceAuthority<'tx, 'conn> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        _transaction: &'tx Transaction<'conn>,
        receipt: ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
        carrier: CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
        vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
        sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
        runtime_compatibility: CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<
            'tx,
            'conn,
        >,
        checked_at: String,
    ) -> Self {
        Self {
            receipt,
            carrier,
            vulnerability,
            sandbox,
            runtime_compatibility,
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
    ) -> &CurrentExternalPoolAdapterRegistryProviderBindingAuthority {
        &self.carrier
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

    pub(in crate::store) fn runtime_compatibility(
        &self,
    ) -> &CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'tx, 'conn> {
        &self.runtime_compatibility
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}
