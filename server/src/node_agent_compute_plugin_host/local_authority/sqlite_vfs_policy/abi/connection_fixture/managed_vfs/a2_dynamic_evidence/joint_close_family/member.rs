use std::fmt::Write as _;

use super::super::super::a2b2_cases::{
    CaseKey, JointCloseSelector, ValidatedJointCloseObservation, ValidatedJointCloseReportPayload,
};
use super::super::{
    child::{SanitizedPayloadFamily, ValidatedChildProcessReceipt},
    cleanup::ValidatedParentCleanupReceipt,
    environment::WindowsDynamicEnvironment,
};
use super::{
    digest::{EnvironmentKey, MemberFact},
    opaque_commitment, JointCloseFamilyCohort,
};

/// Linear implementation-candidate record. It cannot render a formal JointClose record.
#[must_use = "a validated JointClose candidate must be reported by the parent runner"]
pub(in super::super::super) struct ValidatedJointCloseCandidateRecord {
    _case_key: CaseKey,
    _validated_payload: ValidatedJointCloseReportPayload,
    report: RedactedJointCloseCandidateReport,
}

/// Borrowed non-formal projection for one isolated candidate case.
pub(in super::super::super) struct JointCloseCandidateReportView<'a> {
    report: &'a RedactedJointCloseCandidateReport,
}

struct RedactedJointCloseCandidateReport {
    case_selector: &'static str,
    git_sha: String,
    target: &'static str,
    actual_payload_commitment: String,
    child_exit_code: i32,
}

/// Linear, non-renderable member receipt retaining all process and cleanup witnesses.
#[must_use = "a JointClose family member must enter the atomic 36-member reducer"]
pub(in super::super::super) struct ValidatedJointCloseFamilyMemberReceipt {
    pub(super) selector: JointCloseSelector,
    pub(super) canonical_name: &'static str,
    registration_id: u64,
    _case_key: CaseKey,
    pub(super) validated_payload: ValidatedJointCloseReportPayload,
    pub(super) environment: WindowsDynamicEnvironment,
    pub(super) child: ValidatedChildProcessReceipt,
    cleanup: ValidatedParentCleanupReceipt,
    cohort_commitment: [u8; 32],
}

impl ValidatedJointCloseCandidateRecord {
    pub(in super::super::super) fn validate(
        observation: ValidatedJointCloseObservation,
        environment: WindowsDynamicEnvironment,
        child: ValidatedChildProcessReceipt,
        cleanup: ValidatedParentCleanupReceipt,
    ) -> Result<Self, &'static str> {
        let selector = observation.selector();
        let registration_id = observation.registration_id();
        let (case_key, validated_payload) = observation.into_evidence_parts();
        validate_member_integrity(
            selector,
            registration_id,
            &validated_payload,
            &environment,
            &child,
            &cleanup,
        )?;
        let report = RedactedJointCloseCandidateReport {
            case_selector: selector.report_name(),
            git_sha: environment.git_sha.clone(),
            target: environment.target,
            actual_payload_commitment: opaque_commitment(&child.redacted_payload_fingerprint()),
            child_exit_code: child.exit_code,
        };
        Ok(Self {
            _case_key: case_key,
            _validated_payload: validated_payload,
            report,
        })
    }

    pub(in super::super::super) fn report(&self) -> JointCloseCandidateReportView<'_> {
        JointCloseCandidateReportView {
            report: &self.report,
        }
    }
}

impl JointCloseCandidateReportView<'_> {
    pub(in super::super::super) fn case_selector(&self) -> &'static str {
        self.report.case_selector
    }

    pub(in super::super::super) fn git_sha(&self) -> &str {
        &self.report.git_sha
    }

    pub(in super::super::super) fn target(&self) -> &'static str {
        self.report.target
    }

    pub(in super::super::super) fn child_exit_code(&self) -> i32 {
        self.report.child_exit_code
    }

    pub(in super::super::super) fn parent_cleanup_deleted(&self) -> bool {
        true
    }

    pub(in super::super::super) fn actual_payload_commitment(&self) -> &str {
        &self.report.actual_payload_commitment
    }
}

impl ValidatedJointCloseFamilyMemberReceipt {
    pub(in super::super::super) fn validate(
        observation: ValidatedJointCloseObservation,
        environment: WindowsDynamicEnvironment,
        child: ValidatedChildProcessReceipt,
        cleanup: ValidatedParentCleanupReceipt,
        cohort: &JointCloseFamilyCohort,
    ) -> Result<Self, &'static str> {
        if !cohort.is_valid() {
            return Err("A2_JOINT_CLOSE_FAMILY_COHORT_INVALID");
        }
        let selector = observation.selector();
        let registration_id = observation.registration_id();
        let (case_key, validated_payload) = observation.into_evidence_parts();
        let receipt = Self {
            selector,
            canonical_name: selector.report_name(),
            registration_id,
            _case_key: case_key,
            validated_payload,
            environment,
            child,
            cleanup,
            cohort_commitment: cohort.commitment,
        };
        receipt.validate_integrity()?;
        Ok(receipt)
    }

    pub(super) fn validate_integrity(&self) -> Result<(), &'static str> {
        validate_member_integrity(
            self.selector,
            self.registration_id,
            &self.validated_payload,
            &self.environment,
            &self.child,
            &self.cleanup,
        )?;
        if self.canonical_name != self.selector.report_name() {
            return Err("A2_JOINT_CLOSE_FAMILY_SELECTOR_ALIAS_INVALID");
        }
        Ok(())
    }

    pub(super) fn fact(&self) -> MemberFact {
        MemberFact {
            selector: self.selector,
            canonical_name: self.canonical_name,
            environment: EnvironmentKey::from(&self.environment),
            child: self.child.fingerprint().0,
            root: self.child.root_commitment.0,
            registration: self.child.registration_commitment.0,
            payload: self.child.payload_commitment.0,
            cohort: self.cohort_commitment,
            family: SanitizedPayloadFamily::JointClose,
            child_exit_code: self.child.exit_code,
        }
    }

    pub(super) fn write_formal_record(&self, text: &mut String, marker: &str) {
        let environment = &self.environment;
        let _ = writeln!(
            text,
            "{marker} case={} commit={} target={} windows_build={} arch={} volume={} filesystem={} bundled_sqlite={} child={} root={} registration={} child_exit={} parent_cleanup=deleted actual={} actual_commitment={}",
            self.canonical_name,
            environment.git_sha,
            environment.target,
            environment.windows_build,
            environment.architecture,
            environment.volume_kind,
            environment.filesystem,
            environment.bundled_sqlite,
            opaque_commitment(&self.child.fingerprint().0),
            opaque_commitment(&self.child.root_commitment.0),
            opaque_commitment(&self.child.registration_commitment.0),
            self.child.exit_code,
            self.validated_payload.exact_payload(),
            opaque_commitment(&self.child.payload_commitment.0),
        );
    }
}

fn validate_member_integrity(
    selector: JointCloseSelector,
    registration_id: u64,
    validated_payload: &ValidatedJointCloseReportPayload,
    environment: &WindowsDynamicEnvironment,
    child: &ValidatedChildProcessReceipt,
    cleanup: &ValidatedParentCleanupReceipt,
) -> Result<(), &'static str> {
    let child_fingerprint = child.fingerprint();
    if environment.child_fingerprint != child_fingerprint
        || cleanup.child_fingerprint != child_fingerprint
    {
        return Err("A2_DYNAMIC_CHILD_RECEIPT_BINDING_MISMATCH");
    }
    if environment.root_commitment != child.root_commitment
        || cleanup.root_commitment != child.root_commitment
    {
        return Err("A2_DYNAMIC_ROOT_RECEIPT_BINDING_MISMATCH");
    }
    if environment.registration_commitment != child.registration_commitment
        || cleanup.registration_commitment != child.registration_commitment
    {
        return Err("A2_DYNAMIC_REGISTRATION_RECEIPT_BINDING_MISMATCH");
    }
    if !child.matches_registration_id(registration_id) {
        return Err("A2_DYNAMIC_REGISTRATION_ID_BINDING_MISMATCH");
    }
    if !child.matches_family(SanitizedPayloadFamily::JointClose) {
        return Err("A2_DYNAMIC_PAYLOAD_FAMILY_BINDING_MISMATCH");
    }
    if !validated_payload.matches_exact(&child.actual_payload)
        || !validated_payload.matches_commitment(&child.payload_commitment)
    {
        return Err("A2_DYNAMIC_ACTUAL_PAYLOAD_BINDING_MISMATCH");
    }
    let mut fields = child.actual_payload.split(',');
    if fields.next() != Some("a2b2jc1") || fields.next() != Some(selector.report_name()) {
        return Err("A2_JOINT_CLOSE_FAMILY_SELECTOR_BINDING_MISMATCH");
    }
    if child.exit_code != 0 {
        return Err("A2_JOINT_CLOSE_FAMILY_CHILD_EXIT_INVALID");
    }
    Ok(())
}
