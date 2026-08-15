use serde::Serialize;

use crate::compute_federation::{
    external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseReceipt,
    external_pool_adapter_runtime_compatibility_verification::{
        ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
        ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt,
        ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
        ExternalPoolAdapterRuntimeCompatibilitySignatureChallenge,
        ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
    },
};
use crate::store::compute_external_pool_adapter_sandbox_verifier_key::CurrentExternalPoolAdapterSandboxVerifierKeyAuthority;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityChallengeWriteReceipt {
    pub challenge: ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    pub replayed: bool,
}

pub(in crate::store) struct ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt {
    pub run_observation: ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
    pub signature_challenge: ExternalPoolAdapterRuntimeCompatibilitySignatureChallenge,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityVerificationWriteReceipt {
    pub verification: ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityVerificationRevocationWriteReceipt {
    pub verification: ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
    pub revocation: ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt,
    pub replayed: bool,
}

pub(super) struct StoredRuntimeCompatibilityChallenge {
    pub(super) receipt: ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    pub(super) receipt_json: String,
}

pub(super) struct StoredRuntimeCompatibilityRunObservation {
    pub(super) receipt: ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
    pub(super) receipt_json: String,
}

pub(super) struct StoredRuntimeCompatibilityVerification {
    pub(super) receipt: ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
    pub(super) receipt_json: String,
}

pub(super) struct StoredRuntimeCompatibilityRevocation {
    pub(super) receipt: ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt,
    pub(super) receipt_json: String,
}

/// Same-connection current authority. It is intentionally non-Clone/non-Debug/non-Serde.
pub(in crate::store) struct CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority {
    verification: ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
    run_observation: ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
    release: ExternalPoolAdapterRegistryReleaseReceipt,
    verifier_key: CurrentExternalPoolAdapterSandboxVerifierKeyAuthority,
    checked_at: String,
}

impl CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority {
    pub(super) fn new(
        verification: ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
        run_observation: ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
        release: ExternalPoolAdapterRegistryReleaseReceipt,
        verifier_key: CurrentExternalPoolAdapterSandboxVerifierKeyAuthority,
        checked_at: String,
    ) -> Self {
        Self {
            verification,
            run_observation,
            release,
            verifier_key,
            checked_at,
        }
    }

    pub(in crate::store) fn verification(
        &self,
    ) -> &ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt {
        &self.verification
    }

    pub(in crate::store) fn run_observation(
        &self,
    ) -> &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt {
        &self.run_observation
    }

    pub(in crate::store) fn release(&self) -> &ExternalPoolAdapterRegistryReleaseReceipt {
        &self.release
    }

    pub(in crate::store) fn verifier_key(
        &self,
    ) -> &CurrentExternalPoolAdapterSandboxVerifierKeyAuthority {
        &self.verifier_key
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}
