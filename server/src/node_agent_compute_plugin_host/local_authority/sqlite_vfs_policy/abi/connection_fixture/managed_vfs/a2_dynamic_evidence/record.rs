use std::fmt;

use super::super::a2b2_cases::{
    CaseKey, ValidatedBarrierObservation, ValidatedBarrierReportPayload,
    ValidatedRegistrationShutdownObservation, ValidatedRegistrationShutdownReportPayload,
    ValidatedRegistryLifecycleObservation, ValidatedRegistryLifecycleReportPayload,
};
use super::{
    child::{
        SanitizedActualPayloadCommitment, SanitizedPayloadFamily, ValidatedChildProcessReceipt,
    },
    cleanup::ValidatedParentCleanupReceipt,
    environment::WindowsDynamicEnvironment,
};

/// Linear Windows dynamic record. It intentionally implements neither `Clone` nor serde traits.
#[must_use = "a validated Windows dynamic record must be reported by the parent runner"]
pub(in super::super) struct ValidatedWindowsDynamicRecord {
    _case_key: CaseKey,
    _validated_payload: ValidatedDynamicPayload,
    report: RedactedWindowsDynamicReport,
}

enum ValidatedDynamicObservation {
    RegistrationShutdown {
        selector: &'static str,
        registration_id: u64,
        case_key: CaseKey,
        payload: ValidatedDynamicPayload,
    },
    Barrier {
        selector: &'static str,
        registration_id: u64,
        case_key: CaseKey,
        payload: ValidatedDynamicPayload,
    },
    RegistryLifecycle {
        selector: &'static str,
        registration_id: u64,
        case_key: CaseKey,
        payload: ValidatedDynamicPayload,
    },
}

enum ValidatedDynamicPayload {
    RegistrationShutdown(ValidatedRegistrationShutdownReportPayload),
    Barrier(ValidatedBarrierReportPayload),
    RegistryLifecycle(ValidatedRegistryLifecycleReportPayload),
}

struct RedactedWindowsDynamicReport {
    case_selector: &'static str,
    git_sha: String,
    target: &'static str,
    windows_build: String,
    architecture: &'static str,
    volume_kind: &'static str,
    filesystem: String,
    bundled_sqlite: String,
    child_identity_fingerprint: String,
    root_identity_fingerprint: String,
    registration_identity_fingerprint: String,
    actual_payload: String,
    actual_payload_commitment: String,
    child_exit_code: i32,
}

/// Borrowed report projection. The canonical actual contains only the frozen test registration
/// counter and allow-listed enums/counts; it exposes no PID, nonce, path, pointer, handle, Secret
/// or reusable custody.
pub(in super::super) struct WindowsDynamicReportView<'a> {
    report: &'a RedactedWindowsDynamicReport,
}

impl ValidatedWindowsDynamicRecord {
    /// Consumes the parent-local observation and all process/root/cleanup witnesses exactly once.
    pub(in super::super) fn validate(
        observation: ValidatedRegistrationShutdownObservation,
        environment: WindowsDynamicEnvironment,
        child: ValidatedChildProcessReceipt,
        cleanup: ValidatedParentCleanupReceipt,
    ) -> Result<Self, &'static str> {
        Self::validate_observation(
            ValidatedDynamicObservation::registration_shutdown(observation),
            environment,
            child,
            cleanup,
        )
    }

    pub(in super::super) fn validate_barrier(
        observation: ValidatedBarrierObservation,
        environment: WindowsDynamicEnvironment,
        child: ValidatedChildProcessReceipt,
        cleanup: ValidatedParentCleanupReceipt,
    ) -> Result<Self, &'static str> {
        Self::validate_observation(
            ValidatedDynamicObservation::barrier(observation),
            environment,
            child,
            cleanup,
        )
    }

    pub(in super::super) fn validate_registry_lifecycle(
        observation: ValidatedRegistryLifecycleObservation,
        environment: WindowsDynamicEnvironment,
        child: ValidatedChildProcessReceipt,
        cleanup: ValidatedParentCleanupReceipt,
    ) -> Result<Self, &'static str> {
        Self::validate_observation(
            ValidatedDynamicObservation::registry_lifecycle(observation),
            environment,
            child,
            cleanup,
        )
    }

    fn validate_observation(
        observation: ValidatedDynamicObservation,
        environment: WindowsDynamicEnvironment,
        child: ValidatedChildProcessReceipt,
        cleanup: ValidatedParentCleanupReceipt,
    ) -> Result<Self, &'static str> {
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
        if !child.matches_registration_id(observation.registration_id()) {
            return Err("A2_DYNAMIC_REGISTRATION_ID_BINDING_MISMATCH");
        }
        if !child.matches_family(observation.family()) {
            return Err("A2_DYNAMIC_PAYLOAD_FAMILY_BINDING_MISMATCH");
        }

        let (case_selector, case_key, validated_payload) = observation.into_evidence_parts();
        if !validated_payload.matches_exact(&child.actual_payload)
            || !validated_payload.matches_commitment(&child.payload_commitment)
        {
            return Err("A2_DYNAMIC_ACTUAL_PAYLOAD_BINDING_MISMATCH");
        }
        let redacted_payload_fingerprint = child.redacted_payload_fingerprint();
        let report = RedactedWindowsDynamicReport {
            case_selector,
            git_sha: environment.git_sha,
            target: environment.target,
            windows_build: environment.windows_build,
            architecture: environment.architecture,
            volume_kind: environment.volume_kind,
            filesystem: environment.filesystem,
            bundled_sqlite: environment.bundled_sqlite,
            child_identity_fingerprint: opaque_commitment(&child_fingerprint.0),
            root_identity_fingerprint: opaque_commitment(&child.root_commitment.0),
            registration_identity_fingerprint: opaque_commitment(&child.registration_commitment.0),
            actual_payload: validated_payload.exact_payload().to_owned(),
            actual_payload_commitment: opaque_commitment(&redacted_payload_fingerprint),
            child_exit_code: child.exit_code,
        };
        Ok(Self {
            _case_key: case_key,
            _validated_payload: validated_payload,
            report,
        })
    }

    pub(in super::super) fn report(&self) -> WindowsDynamicReportView<'_> {
        WindowsDynamicReportView {
            report: &self.report,
        }
    }
}

impl ValidatedDynamicObservation {
    fn registration_shutdown(observation: ValidatedRegistrationShutdownObservation) -> Self {
        let selector = observation.selector().report_name();
        let registration_id = observation.registration_id();
        let (case_key, payload) = observation.into_evidence_parts();
        Self::RegistrationShutdown {
            selector,
            registration_id,
            case_key,
            payload: ValidatedDynamicPayload::RegistrationShutdown(payload),
        }
    }

    fn barrier(observation: ValidatedBarrierObservation) -> Self {
        let selector = observation.selector().report_name();
        let registration_id = observation.registration_id();
        let (case_key, payload) = observation.into_evidence_parts();
        Self::Barrier {
            selector,
            registration_id,
            case_key,
            payload: ValidatedDynamicPayload::Barrier(payload),
        }
    }

    fn registry_lifecycle(observation: ValidatedRegistryLifecycleObservation) -> Self {
        let selector = observation.selector().report_name();
        let registration_id = observation.registration_id();
        let (case_key, payload) = observation.into_evidence_parts();
        Self::RegistryLifecycle {
            selector,
            registration_id,
            case_key,
            payload: ValidatedDynamicPayload::RegistryLifecycle(payload),
        }
    }

    fn registration_id(&self) -> u64 {
        match self {
            Self::RegistrationShutdown {
                registration_id, ..
            }
            | Self::Barrier {
                registration_id, ..
            }
            | Self::RegistryLifecycle {
                registration_id, ..
            } => *registration_id,
        }
    }

    fn family(&self) -> SanitizedPayloadFamily {
        match self {
            Self::RegistrationShutdown { .. } => SanitizedPayloadFamily::RegistrationShutdown,
            Self::Barrier { .. } => SanitizedPayloadFamily::Barrier,
            Self::RegistryLifecycle { .. } => SanitizedPayloadFamily::RegistryLifecycle,
        }
    }

    fn into_evidence_parts(self) -> (&'static str, CaseKey, ValidatedDynamicPayload) {
        match self {
            Self::RegistrationShutdown {
                selector,
                case_key,
                payload,
                ..
            }
            | Self::Barrier {
                selector,
                case_key,
                payload,
                ..
            }
            | Self::RegistryLifecycle {
                selector,
                case_key,
                payload,
                ..
            } => (selector, case_key, payload),
        }
    }
}

impl ValidatedDynamicPayload {
    fn matches_exact(&self, candidate: &str) -> bool {
        match self {
            Self::RegistrationShutdown(payload) => payload.matches_exact(candidate),
            Self::Barrier(payload) => payload.matches_exact(candidate),
            Self::RegistryLifecycle(payload) => payload.matches_exact(candidate),
        }
    }

    fn matches_commitment(&self, commitment: &SanitizedActualPayloadCommitment) -> bool {
        match self {
            Self::RegistrationShutdown(payload) => payload.matches_commitment(commitment),
            Self::Barrier(payload) => payload.matches_commitment(commitment),
            Self::RegistryLifecycle(payload) => payload.matches_commitment(commitment),
        }
    }

    fn exact_payload(&self) -> &str {
        match self {
            Self::RegistrationShutdown(payload) => payload.exact_payload(),
            Self::Barrier(payload) => payload.exact_payload(),
            Self::RegistryLifecycle(payload) => payload.exact_payload(),
        }
    }
}

impl WindowsDynamicReportView<'_> {
    pub(in super::super) fn case_selector(&self) -> &'static str {
        self.report.case_selector
    }

    pub(in super::super) fn git_sha(&self) -> &str {
        &self.report.git_sha
    }

    pub(in super::super) fn target(&self) -> &'static str {
        self.report.target
    }

    pub(in super::super) fn windows_build(&self) -> &str {
        &self.report.windows_build
    }

    pub(in super::super) fn architecture(&self) -> &'static str {
        self.report.architecture
    }

    pub(in super::super) fn volume_kind(&self) -> &'static str {
        self.report.volume_kind
    }

    pub(in super::super) fn filesystem(&self) -> &str {
        &self.report.filesystem
    }

    pub(in super::super) fn bundled_sqlite(&self) -> &str {
        &self.report.bundled_sqlite
    }

    pub(in super::super) fn child_identity_fingerprint(&self) -> &str {
        &self.report.child_identity_fingerprint
    }

    pub(in super::super) fn root_identity_fingerprint(&self) -> &str {
        &self.report.root_identity_fingerprint
    }

    pub(in super::super) fn registration_identity_fingerprint(&self) -> &str {
        &self.report.registration_identity_fingerprint
    }

    pub(in super::super) fn child_exit_code(&self) -> i32 {
        self.report.child_exit_code
    }

    pub(in super::super) fn parent_cleanup_deleted(&self) -> bool {
        true
    }

    pub(in super::super) fn actual_payload_commitment(&self) -> &str {
        &self.report.actual_payload_commitment
    }

    pub(in super::super) fn actual_payload(&self) -> &str {
        &self.report.actual_payload
    }
}

impl fmt::Display for WindowsDynamicReportView<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "A2_WINDOWS_DYNAMIC_V2 case={} commit={} target={} windows_build={} arch={} volume={} filesystem={} bundled_sqlite={} child={} root={} registration={} child_exit={} parent_cleanup=deleted actual={} actual_commitment={}",
            self.case_selector(),
            self.git_sha(),
            self.target(),
            self.windows_build(),
            self.architecture(),
            self.volume_kind(),
            self.filesystem(),
            self.bundled_sqlite(),
            self.child_identity_fingerprint(),
            self.root_identity_fingerprint(),
            self.registration_identity_fingerprint(),
            self.child_exit_code(),
            self.actual_payload(),
            self.actual_payload_commitment(),
        )
    }
}

fn opaque_commitment(value: &[u8; 32]) -> String {
    format!("sha256:{}", hex::encode(value))
}
