use super::{super::case_key::CaseKey, actual::BarrierSelector};

#[derive(PartialEq, Eq)]
#[must_use = "the validated canonical Barrier payload must be bound to parent/child evidence"]
pub(in super::super::super) struct ValidatedBarrierReportPayload {
    exact_payload: String,
}

impl std::fmt::Debug for ValidatedBarrierReportPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ValidatedBarrierReportPayload(<sealed>)")
    }
}

impl ValidatedBarrierReportPayload {
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
#[must_use = "a validated Barrier observation must be consumed by the dynamic evidence gate"]
pub(in super::super::super) struct ValidatedBarrierObservation {
    selector: BarrierSelector,
    registration_id: u64,
    case_key: CaseKey,
    report_payload: ValidatedBarrierReportPayload,
}

impl ValidatedBarrierObservation {
    pub(super) fn new(
        selector: BarrierSelector,
        registration_id: u64,
        case_key: CaseKey,
        exact_payload: String,
    ) -> Self {
        Self {
            selector,
            registration_id,
            case_key,
            report_payload: ValidatedBarrierReportPayload::new(exact_payload),
        }
    }

    pub(in super::super::super) const fn selector(&self) -> BarrierSelector {
        self.selector
    }

    pub(in super::super::super) const fn registration_id(&self) -> u64 {
        self.registration_id
    }

    pub(in super::super::super) fn into_evidence_parts(
        self,
    ) -> (CaseKey, ValidatedBarrierReportPayload) {
        (self.case_key, self.report_payload)
    }
}
