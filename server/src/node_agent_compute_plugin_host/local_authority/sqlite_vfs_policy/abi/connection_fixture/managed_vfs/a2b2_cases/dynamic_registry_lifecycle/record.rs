use super::{super::case_key::CaseKey, actual::RegistryLifecycleSelector};

#[derive(PartialEq, Eq)]
#[must_use = "the validated RegistryLifecycle payload must be bound to parent/child evidence"]
pub(in super::super::super) struct ValidatedRegistryLifecycleReportPayload {
    exact_payload: String,
}

impl std::fmt::Debug for ValidatedRegistryLifecycleReportPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ValidatedRegistryLifecycleReportPayload(<sealed>)")
    }
}

impl ValidatedRegistryLifecycleReportPayload {
    pub(super) fn new(exact_payload: String) -> Self {
        Self { exact_payload }
    }

    pub(in super::super::super) fn matches_exact(&self, candidate: &str) -> bool {
        self.exact_payload == candidate
    }

    pub(in super::super::super) fn matches_commitment(
        &self,
        commitment: &super::super::super::a2_dynamic_evidence::SanitizedActualPayloadCommitment,
    ) -> bool {
        commitment.matches_payload(&self.exact_payload)
    }

    pub(in super::super::super) fn exact_payload(&self) -> &str {
        &self.exact_payload
    }
}

#[derive(Debug, PartialEq, Eq)]
#[must_use = "a validated RegistryLifecycle observation must enter the evidence gate"]
pub(in super::super::super) struct ValidatedRegistryLifecycleObservation {
    selector: RegistryLifecycleSelector,
    registration_id: u64,
    case_key: CaseKey,
    report_payload: ValidatedRegistryLifecycleReportPayload,
}

impl ValidatedRegistryLifecycleObservation {
    pub(super) fn new(
        selector: RegistryLifecycleSelector,
        registration_id: u64,
        case_key: CaseKey,
        exact_payload: String,
    ) -> Self {
        Self {
            selector,
            registration_id,
            case_key,
            report_payload: ValidatedRegistryLifecycleReportPayload::new(exact_payload),
        }
    }

    pub(in super::super::super) const fn selector(&self) -> RegistryLifecycleSelector {
        self.selector
    }

    pub(in super::super::super) const fn registration_id(&self) -> u64 {
        self.registration_id
    }

    pub(in super::super::super) fn into_evidence_parts(
        self,
    ) -> (CaseKey, ValidatedRegistryLifecycleReportPayload) {
        (self.case_key, self.report_payload)
    }
}
