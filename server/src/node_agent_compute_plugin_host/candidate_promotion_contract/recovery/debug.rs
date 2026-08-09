use std::{error::Error as StdError, fmt};

use super::{
    CandidatePromotionOutcomeUncertainCustody, CandidatePromotionRecoveryAdoption,
    CandidatePromotionRecoveryAdoptionFailure, CandidatePromotionStoreFailure,
    ComputePluginCandidatePromotionExpectation, ComputePluginCandidatePromotionRecoveryKey,
    ComputePluginCandidatePromotionRecoveryOutcome,
};

impl fmt::Debug for ComputePluginCandidatePromotionExpectation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidatePromotionExpectation")
            .field("candidate_generation", &self.candidate_generation)
            .field("process_owner_epoch", &self.process_owner_epoch)
            .field("install_generation_after", &self.install_generation_after)
            .field(
                "activation_generation_after",
                &self.activation_generation_after,
            )
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for ComputePluginCandidatePromotionRecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidatePromotionRecoveryKey")
            .field("install_id", &"<redacted>")
            .field("promotion_id", &"<redacted>")
            .field("plugin_id", &self.plugin_id)
            .field("slot_ref", &self.slot_ref)
            .field("expectation", &self.expectation)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CandidatePromotionOutcomeUncertainCustody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidatePromotionOutcomeUncertainCustody")
            .field("recovery_key", &self.recovery_key)
            .field("handles", &"<retained>")
            .finish()
    }
}

impl fmt::Debug for ComputePluginCandidatePromotionRecoveryOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotCreated => formatter.write_str("NotCreated"),
            Self::Installed(receipts) => {
                formatter.debug_tuple("Installed").field(receipts).finish()
            }
        }
    }
}

impl fmt::Debug for CandidatePromotionRecoveryAdoption<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotCreated(_) => formatter
                .debug_tuple("NotCreated")
                .field(&"<durable-health-custody>")
                .finish(),
            Self::Installed(installed) => {
                formatter.debug_tuple("Installed").field(installed).finish()
            }
        }
    }
}

impl fmt::Display for CandidatePromotionStoreFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidatePromotionStoreFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidatePromotionStoreFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl StdError for CandidatePromotionStoreFailure<'_> {}

impl fmt::Display for CandidatePromotionRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidatePromotionRecoveryAdoptionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidatePromotionRecoveryAdoptionFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .field("observed", &self.observed)
            .finish()
    }
}

impl StdError for CandidatePromotionRecoveryAdoptionFailure<'_> {}
