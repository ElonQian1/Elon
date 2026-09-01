use std::{
    os::raw::c_int,
    sync::{Mutex, OnceLock},
    thread::ThreadId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum HandleBoundSqliteAbiRawLockRejectionCaseV1 {
    NullFileDirect,
    UninstalledDirect,
    MethodsNullStatePresentDirect,
    ForeignMethodsStateNullDirect,
    ForeignMethodsStatePresentDirect,
    ExactMethodsStateNullDirect,
    OtherTypePayloadMissingDropCompleted,
    OtherTypePayloadPresentDropCompleted,
    OtherTypePayloadPresentDropUnwindCaught,
    ExpectedTypePayloadMissingDropCompleted,
    HandleBoundFileMissingDirect,
}

impl HandleBoundSqliteAbiRawLockRejectionCaseV1 {
    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn tag(self) -> u64 {
        match self {
            Self::NullFileDirect => 1,
            Self::UninstalledDirect => 2,
            Self::MethodsNullStatePresentDirect => 3,
            Self::ForeignMethodsStateNullDirect => 4,
            Self::ForeignMethodsStatePresentDirect => 5,
            Self::ExactMethodsStateNullDirect => 6,
            Self::OtherTypePayloadMissingDropCompleted => 7,
            Self::OtherTypePayloadPresentDropCompleted => 8,
            Self::OtherTypePayloadPresentDropUnwindCaught => 9,
            Self::ExpectedTypePayloadMissingDropCompleted => 10,
            Self::HandleBoundFileMissingDirect => 11,
        }
    }

    pub(super) const fn invocation_file_is_null(self) -> bool {
        matches!(self, Self::NullFileDirect)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) enum HandleBoundSqliteAbiRawLockEvidenceV1 {
    ControlledFaultActual,
}

impl HandleBoundSqliteAbiRawLockEvidenceV1 {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::ControlledFaultActual => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RawValidation {
    NullFile,
    Uninstalled,
    ForeignMethods,
    StateMissing,
    TypeMismatch,
    Accepted,
}

impl RawValidation {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::NullFile => 1,
            Self::Uninstalled => 2,
            Self::ForeignMethods => 3,
            Self::StateMissing => 4,
            Self::TypeMismatch => 5,
            Self::Accepted => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RunCodeOutcome {
    Normal,
    Rejection,
    Unwind,
}

impl RunCodeOutcome {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::Normal => 1,
            Self::Rejection => 2,
            Self::Unwind => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AbandonOutcome {
    Empty,
    NullFileRejected,
    ForeignMethodsRejected,
    StateMissingRejected,
    InstalledDropCompleted,
    InstalledDropUnwindCaught,
}

impl AbandonOutcome {
    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::Empty => 1,
            Self::NullFileRejected => 2,
            Self::ForeignMethodsRejected => 3,
            Self::StateMissingRejected => 4,
            Self::InstalledDropCompleted => 5,
            Self::InstalledDropUnwindCaught => 6,
        }
    }
}

#[derive(Default)]
pub(super) struct EventCounts {
    pub(super) fixture_prepare: u64,
    pub(super) entry: u64,
    pub(super) scalar_admitted: u64,
    pub(super) raw_validation: u64,
    pub(super) type_check: u64,
    pub(super) payload_snapshot: u64,
    pub(super) typed_operation_entry: u64,
    pub(super) handle_file_missing: u64,
    pub(super) abandon_entry: u64,
    pub(super) slots_clear: u64,
    pub(super) envelope_drop: u64,
    pub(super) payload_drop_attempt: u64,
    pub(super) payload_drop_completed: u64,
    pub(super) payload_drop_unwind: u64,
    pub(super) abandon_drop_completed: u64,
    pub(super) abandon_drop_unwind: u64,
    pub(super) returned: u64,
}

pub(super) struct ActiveObservation {
    pub(super) observation_id: u64,
    pub(super) case_v1: HandleBoundSqliteAbiRawLockRejectionCaseV1,
    pub(super) source_file_address: usize,
    pub(super) invocation_file_address: usize,
    pub(super) owner_thread: ThreadId,
    pub(super) slots_before: u64,
    pub(super) slots_prepared: Option<u64>,
    pub(super) slots_after: Option<u64>,
    pub(super) retained_fixture_tag: Option<u64>,
    pub(super) counts: EventCounts,
    pub(super) validation: Option<RawValidation>,
    pub(super) type_matches: Option<bool>,
    pub(super) payload_present: Option<bool>,
    pub(super) run_code_outcome: Option<RunCodeOutcome>,
    pub(super) abandon_outcome: Option<AbandonOutcome>,
    pub(super) result_code: Option<c_int>,
    pub(super) violation: Option<&'static str>,
}

pub(super) struct ObservationLedger {
    pub(super) next_observation_id: u64,
    pub(super) active: Option<ActiveObservation>,
}

impl Default for ObservationLedger {
    fn default() -> Self {
        Self {
            next_observation_id: 1,
            active: None,
        }
    }
}

pub(super) fn ledger() -> &'static Mutex<ObservationLedger> {
    static LEDGER: OnceLock<Mutex<ObservationLedger>> = OnceLock::new();
    LEDGER.get_or_init(|| Mutex::new(ObservationLedger::default()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct HandleBoundSqliteAbiRawLockRejectionReceiptV1 {
    case_v1: HandleBoundSqliteAbiRawLockRejectionCaseV1,
    observation_id: u64,
    result_code: c_int,
    ordered_values: [u64; 32],
}

impl HandleBoundSqliteAbiRawLockRejectionReceiptV1 {
    pub(super) const fn new(
        case_v1: HandleBoundSqliteAbiRawLockRejectionCaseV1,
        observation_id: u64,
        result_code: c_int,
        ordered_values: [u64; 32],
    ) -> Self {
        Self {
            case_v1,
            observation_id,
            result_code,
            ordered_values,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn case_v1(
        self,
    ) -> HandleBoundSqliteAbiRawLockRejectionCaseV1 {
        self.case_v1
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn evidence_v1(
        self,
    ) -> HandleBoundSqliteAbiRawLockEvidenceV1 {
        HandleBoundSqliteAbiRawLockEvidenceV1::ControlledFaultActual
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn observation_id(
        self,
    ) -> u64 {
        self.observation_id
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn result_code(
        self,
    ) -> c_int {
        self.result_code
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn ordered_values(
        self,
    ) -> [u64; 32] {
        self.ordered_values
    }
}
