//! Closed typed identity for the eleven q11 Lock raw-state rejection programs.

use super::super::super::super::super::terminal_descriptor::{
    LockAxesV1, LockCompletionV1, LockOperationV1, ObserverV1, PhaseV1, RawStateV1,
    ReachabilityV1, SourceSiteV1, TimingV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum LockRawStateRejectionCaseV1 {
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

impl LockRawStateRejectionCaseV1 {
    pub(super) const ALL_V1: [Self; 11] = [
        Self::NullFileDirect,
        Self::UninstalledDirect,
        Self::MethodsNullStatePresentDirect,
        Self::ForeignMethodsStateNullDirect,
        Self::ForeignMethodsStatePresentDirect,
        Self::ExactMethodsStateNullDirect,
        Self::OtherTypePayloadMissingDropCompleted,
        Self::OtherTypePayloadPresentDropCompleted,
        Self::OtherTypePayloadPresentDropUnwindCaught,
        Self::ExpectedTypePayloadMissingDropCompleted,
        Self::HandleBoundFileMissingDirect,
    ];

    pub(super) const fn from_typed_v1(
        raw_state: RawStateV1,
        completion: LockCompletionV1,
    ) -> Option<Self> {
        match (raw_state, completion) {
            (RawStateV1::NullFile, LockCompletionV1::Direct) => Some(Self::NullFileDirect),
            (RawStateV1::Uninstalled, LockCompletionV1::Direct) => Some(Self::UninstalledDirect),
            (RawStateV1::MethodsNullStatePresent, LockCompletionV1::Direct) => {
                Some(Self::MethodsNullStatePresentDirect)
            }
            (RawStateV1::ForeignMethodsStateNull, LockCompletionV1::Direct) => {
                Some(Self::ForeignMethodsStateNullDirect)
            }
            (RawStateV1::ForeignMethodsStatePresent, LockCompletionV1::Direct) => {
                Some(Self::ForeignMethodsStatePresentDirect)
            }
            (RawStateV1::ExactMethodsStateNull, LockCompletionV1::Direct) => {
                Some(Self::ExactMethodsStateNullDirect)
            }
            (RawStateV1::OtherTypePayloadMissing, LockCompletionV1::RawDropCompleted) => {
                Some(Self::OtherTypePayloadMissingDropCompleted)
            }
            (RawStateV1::OtherTypePayloadPresent, LockCompletionV1::RawDropCompleted) => {
                Some(Self::OtherTypePayloadPresentDropCompleted)
            }
            (RawStateV1::OtherTypePayloadPresent, LockCompletionV1::RawDropUnwindCaught) => {
                Some(Self::OtherTypePayloadPresentDropUnwindCaught)
            }
            (RawStateV1::ExpectedTypePayloadMissing, LockCompletionV1::RawDropCompleted) => {
                Some(Self::ExpectedTypePayloadMissingDropCompleted)
            }
            (RawStateV1::HandleBoundFileMissing, LockCompletionV1::Direct) => {
                Some(Self::HandleBoundFileMissingDirect)
            }
            _ => None,
        }
    }

    pub(super) const fn raw_state_v1(self) -> RawStateV1 {
        match self {
            Self::NullFileDirect => RawStateV1::NullFile,
            Self::UninstalledDirect => RawStateV1::Uninstalled,
            Self::MethodsNullStatePresentDirect => RawStateV1::MethodsNullStatePresent,
            Self::ForeignMethodsStateNullDirect => RawStateV1::ForeignMethodsStateNull,
            Self::ForeignMethodsStatePresentDirect => RawStateV1::ForeignMethodsStatePresent,
            Self::ExactMethodsStateNullDirect => RawStateV1::ExactMethodsStateNull,
            Self::OtherTypePayloadMissingDropCompleted => RawStateV1::OtherTypePayloadMissing,
            Self::OtherTypePayloadPresentDropCompleted
            | Self::OtherTypePayloadPresentDropUnwindCaught => RawStateV1::OtherTypePayloadPresent,
            Self::ExpectedTypePayloadMissingDropCompleted => RawStateV1::ExpectedTypePayloadMissing,
            Self::HandleBoundFileMissingDirect => RawStateV1::HandleBoundFileMissing,
        }
    }

    pub(super) const fn completion_v1(self) -> LockCompletionV1 {
        match self {
            Self::NullFileDirect
            | Self::UninstalledDirect
            | Self::MethodsNullStatePresentDirect
            | Self::ForeignMethodsStateNullDirect
            | Self::ForeignMethodsStatePresentDirect
            | Self::ExactMethodsStateNullDirect
            | Self::HandleBoundFileMissingDirect => LockCompletionV1::Direct,
            Self::OtherTypePayloadMissingDropCompleted
            | Self::OtherTypePayloadPresentDropCompleted
            | Self::ExpectedTypePayloadMissingDropCompleted => LockCompletionV1::RawDropCompleted,
            Self::OtherTypePayloadPresentDropUnwindCaught => LockCompletionV1::RawDropUnwindCaught,
        }
    }

    pub(super) const fn source_site_v1(self) -> SourceSiteV1 {
        match self {
            Self::HandleBoundFileMissingDirect => SourceSiteV1::AdapterDispatch,
            _ => SourceSiteV1::RawStateAbandon,
        }
    }

    pub(super) const fn operation_v1(self) -> LockOperationV1 {
        match self {
            Self::HandleBoundFileMissingDirect => LockOperationV1::AdapterDispatch,
            _ => LockOperationV1::RawAbandon,
        }
    }

    pub(super) const fn phase_v1(self) -> PhaseV1 {
        match self {
            Self::HandleBoundFileMissingDirect => PhaseV1::Adapter,
            _ => PhaseV1::RawAdmission,
        }
    }

    pub(super) const fn timing_v1(self) -> TimingV1 {
        match self {
            Self::HandleBoundFileMissingDirect => TimingV1::BeforeCall,
            _ => TimingV1::Cleanup,
        }
    }

    pub(super) const fn observer_v1(self) -> ObserverV1 {
        match self {
            Self::HandleBoundFileMissingDirect => ObserverV1::LockCallbackAndSnapshot,
            _ => ObserverV1::CustodyAndCleanup,
        }
    }

    pub(super) const fn axes_v1(self) -> LockAxesV1 {
        LockAxesV1 {
            completion: ReachabilityV1::Reached(self.completion_v1()),
            ..LockAxesV1::NOT_REACHED
        }
    }

    pub(super) const fn implementation_tag_v1(self) -> u8 {
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
}
