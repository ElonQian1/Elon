use serde::Serialize;

use crate::{
    compute_federation::external_pool_adapter_release_lifecycle::{
        ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
        ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding,
    },
    store::compute_external_pool_adapter_release::ExternalPoolAdapterReleaseArtifactSourceAdmission,
};

pub(crate) use crate::compute_federation::external_pool_adapter_release_lifecycle::{
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION,
};

pub(super) const CURRENTNESS_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_release_admission_currentness.v1";

pub(crate) struct CreateExternalPoolAdapterReleaseAdmissionTerminal {
    pub admission_id: String,
    pub expected_admission_digest: String,
    pub terminal_status: String,
    pub successor_admission_id: Option<String>,
    pub expected_successor_admission_digest: Option<String>,
    pub actor_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt {
    pub terminal_receipt: ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
    pub replayed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt {
    pub schema: &'static str,
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub admission_status: &'static str,
    pub current_status: String,
    pub applied_at: String,
    pub terminal_receipt_id: Option<String>,
    pub terminal_receipt_digest: Option<String>,
    pub terminal_occurred_at: Option<String>,
    pub successor_admission: Option<ComputeExternalPoolAdapterReleaseSuccessorAdmissionBinding>,
}

/// Sealed point-in-time proof that the exact immutable admission has no terminal.
///
/// It is deliberately non-Clone and non-Serde. Consumers must reacquire it at their own
/// transaction linearization point; retaining this value does not remove that obligation.
pub(in crate::store) struct CurrentExternalPoolAdapterReleaseAdmissionAuthority {
    admission: ExternalPoolAdapterReleaseArtifactSourceAdmission,
    applied_at: String,
}

impl CurrentExternalPoolAdapterReleaseAdmissionAuthority {
    pub(super) fn new(
        admission: ExternalPoolAdapterReleaseArtifactSourceAdmission,
        applied_at: String,
    ) -> Self {
        Self {
            admission,
            applied_at,
        }
    }

    pub(in crate::store) fn admission(&self) -> &ExternalPoolAdapterReleaseArtifactSourceAdmission {
        &self.admission
    }

    pub(in crate::store) fn admission_id(&self) -> &str {
        &self.admission.admission_id
    }

    pub(in crate::store) fn admission_digest(&self) -> &str {
        &self.admission.admission_digest
    }

    pub(in crate::store) fn adapter_id(&self) -> &str {
        &self.admission.adapter_id
    }

    pub(in crate::store) fn release_version(&self) -> &str {
        &self.admission.release_version
    }

    pub(in crate::store) fn declared_implementation_sha256(&self) -> &str {
        &self.admission.declared_implementation_sha256
    }

    pub(in crate::store) fn supported_capabilities(
        &self,
    ) -> &[crate::compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseCapability]
    {
        &self.admission.supported_capabilities
    }

    pub(in crate::store) fn capability_set_digest(&self) -> &str {
        &self.admission.capability_set_digest
    }

    pub(in crate::store) fn expected_credential_verifier(
        &self,
    ) -> &crate::compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseVerifierIntent
    {
        &self.admission.expected_credential_verifier
    }

    pub(in crate::store) fn applied_at(&self) -> &str {
        &self.applied_at
    }
}

pub(super) struct AuditedAdmission {
    pub admission: ExternalPoolAdapterReleaseArtifactSourceAdmission,
    pub applied_at: String,
}

pub(super) struct StoredTerminalReceipt {
    pub terminal_receipt: ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
    pub terminal_receipt_json: String,
}

impl StoredTerminalReceipt {
    pub(super) fn into_write_receipt(
        self,
        replayed: bool,
    ) -> ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt {
        ExternalPoolAdapterReleaseAdmissionTerminalWriteReceipt {
            terminal_receipt: self.terminal_receipt,
            replayed,
        }
    }
}
