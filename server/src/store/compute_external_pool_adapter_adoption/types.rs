use serde::Serialize;

use crate::compute_federation::external_pool_adapter_adoption::{
    ExternalPoolAdapterAdoptionReceipt, ExternalPoolAdapterAdoptionTerminalReceipt,
};

pub(crate) struct AdoptExternalPoolAdapter {
    pub application_id: String,
    pub expected_application_digest: String,
    pub admission_id: String,
    pub expected_admission_digest: String,
    pub expected_sandbox_conformance_receipt_digest: String,
    pub credential_verification_receipt_id: String,
    pub expected_credential_verification_receipt_digest: String,
    pub adopted_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeExternalPoolAdapterAdoption {
    pub adoption_receipt_id: String,
    pub expected_adoption_receipt_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterAdoptionSummary {
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub application_id: String,
    pub application_digest: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub adapter_release_version: String,
    pub sandbox_conformance_receipt_id: String,
    pub sandbox_conformance_receipt_digest: String,
    pub credential_verification_receipt_id: String,
    pub credential_verification_receipt_digest: String,
    pub credential_locator_commitment: String,
    pub adopted_by_admin_user_id: String,
    pub adopted_at: String,
    pub adoption_effect: String,
    pub install_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterAdoptionTerminalSummary {
    pub terminal_receipt_id: String,
    pub terminal_receipt_digest: String,
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub revoked_at: String,
    pub adoption_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterAdoptionWriteReceipt {
    pub adoption: ExternalPoolAdapterAdoptionSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<ExternalPoolAdapterAdoptionTerminalSummary>,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterAdoptionCurrentness {
    pub schema: &'static str,
    pub adoption: ExternalPoolAdapterAdoptionSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<ExternalPoolAdapterAdoptionTerminalSummary>,
    pub current_status: String,
    pub sandbox_conformance_status: String,
    pub credential_verification_status: String,
    pub terminal_status: String,
}

pub(super) struct StoredExternalPoolAdapterAdoption {
    pub receipt: ExternalPoolAdapterAdoptionReceipt,
    pub receipt_json: String,
}

pub(super) struct StoredExternalPoolAdapterAdoptionTerminal {
    pub receipt: ExternalPoolAdapterAdoptionTerminalReceipt,
    pub receipt_json: String,
}

pub(in crate::store) struct CurrentExternalPoolAdapterAdoptionAuthority {
    receipt: ExternalPoolAdapterAdoptionReceipt,
    checked_at: String,
}

pub(in crate::store) struct HistoricalExternalPoolAdapterAdoptionAuthority {
    receipt: ExternalPoolAdapterAdoptionReceipt,
}

impl HistoricalExternalPoolAdapterAdoptionAuthority {
    pub(super) fn new(receipt: ExternalPoolAdapterAdoptionReceipt) -> Self {
        Self { receipt }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterAdoptionReceipt {
        &self.receipt
    }
}

impl CurrentExternalPoolAdapterAdoptionAuthority {
    pub(super) fn new(receipt: ExternalPoolAdapterAdoptionReceipt, checked_at: String) -> Self {
        Self {
            receipt,
            checked_at,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterAdoptionReceipt {
        &self.receipt
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl StoredExternalPoolAdapterAdoption {
    pub(super) fn summary(&self) -> ExternalPoolAdapterAdoptionSummary {
        let receipt = &self.receipt;
        let item = &receipt.adoption;
        let binding = &item.binding;
        ExternalPoolAdapterAdoptionSummary {
            adoption_receipt_id: receipt.adoption_receipt_id.clone(),
            adoption_receipt_digest: receipt.adoption_receipt_digest.clone(),
            application_id: binding.application_id.clone(),
            application_digest: binding.application_digest.clone(),
            provider_id: binding.provider_id.clone(),
            provider_policy_revision: binding.provider_policy_revision,
            provider_digest: binding.provider_digest.clone(),
            admission_id: binding.admission_id.clone(),
            admission_digest: binding.admission_digest.clone(),
            adapter_id: binding.adapter_id.clone(),
            adapter_release_version: binding.adapter_release_version.clone(),
            sandbox_conformance_receipt_id: binding.sandbox_conformance_receipt_id.clone(),
            sandbox_conformance_receipt_digest: binding.sandbox_conformance_receipt_digest.clone(),
            credential_verification_receipt_id: binding.credential_verification_receipt_id.clone(),
            credential_verification_receipt_digest: binding
                .credential_verification_receipt_digest
                .clone(),
            credential_locator_commitment: binding.credential_locator_commitment.clone(),
            adopted_by_admin_user_id: item.adopted_by_admin_user_id.clone(),
            adopted_at: item.adopted_at.clone(),
            adoption_effect: item.adoption_effect.clone(),
            install_effect: item.install_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}

impl StoredExternalPoolAdapterAdoptionTerminal {
    pub(super) fn summary(&self) -> ExternalPoolAdapterAdoptionTerminalSummary {
        let receipt = &self.receipt;
        let item = &receipt.terminal;
        ExternalPoolAdapterAdoptionTerminalSummary {
            terminal_receipt_id: receipt.terminal_receipt_id.clone(),
            terminal_receipt_digest: receipt.terminal_receipt_digest.clone(),
            adoption_receipt_id: item.adoption_receipt_id.clone(),
            adoption_receipt_digest: item.adoption_receipt_digest.clone(),
            revoked_by_admin_user_id: item.revoked_by_admin_user_id.clone(),
            reason: item.reason.clone(),
            revoked_at: item.revoked_at.clone(),
            adoption_effect: item.adoption_effect.clone(),
        }
    }
}
