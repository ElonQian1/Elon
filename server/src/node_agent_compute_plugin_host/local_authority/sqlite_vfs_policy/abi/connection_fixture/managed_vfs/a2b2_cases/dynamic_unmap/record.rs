use super::{super::case_key::CaseKey, actual::UnmapSelector};

#[derive(PartialEq, Eq)]
#[must_use = "the validated Unmap payload must be bound to parent/child evidence"]
pub(in super::super::super) struct ValidatedUnmapReportPayload {
    exact_payload: String,
}

impl std::fmt::Debug for ValidatedUnmapReportPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ValidatedUnmapReportPayload(<sealed>)")
    }
}

impl ValidatedUnmapReportPayload {
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
#[must_use = "a validated Unmap observation must enter the evidence gate"]
pub(in super::super::super) struct ValidatedUnmapObservation {
    selector: UnmapSelector,
    registration_id: u64,
    case_key: CaseKey,
    report_payload: ValidatedUnmapReportPayload,
}

impl ValidatedUnmapObservation {
    pub(super) fn new(
        selector: UnmapSelector,
        registration_id: u64,
        case_key: CaseKey,
        exact_payload: String,
    ) -> Self {
        Self {
            selector,
            registration_id,
            case_key,
            report_payload: ValidatedUnmapReportPayload::new(exact_payload),
        }
    }

    pub(in super::super::super) const fn selector(&self) -> UnmapSelector {
        self.selector
    }

    pub(in super::super::super) const fn registration_id(&self) -> u64 {
        self.registration_id
    }

    pub(in super::super::super) fn into_evidence_parts(
        self,
    ) -> (CaseKey, ValidatedUnmapReportPayload) {
        (self.case_key, self.report_payload)
    }
}
