//! Source-neutral raw-state fragments for the future Map denominator.
//!
//! The admission table is limited to the eight safe shapes reachable from the single valid,
//! writable ABI continuation. Non-null file storage must be live, aligned, initialized and
//! serialized. Exact methods plus non-null state must identify this module's live envelope.
//! `Occupied` is install-only; forged envelopes, invalid pointers, undefined behavior and
//! aborting panics do not enter these finite fragments.
//!
//! These fragments preserve admission, post-operation and abandonment as separate temporal
//! layers. They do not construct a source step, reviewed successor edge, `SourceBranch`,
//! `Expected`, `CaseKey`, terminal inventory or denominator.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawCustodyRetention {
    /// No raw envelope/opaque-state custody; downstream managed custody is not decided here.
    None,
    InstalledEnvelope,
    OpaqueStatePossible,
    DropUnwindPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RawSlotRetention {
    pub(super) methods_value_retained: bool,
    pub(super) state_value_retained: bool,
    pub(super) custody: RawCustodyRetention,
}

const fn slots(
    methods_value_retained: bool,
    state_value_retained: bool,
    custody: RawCustodyRetention,
) -> RawSlotRetention {
    RawSlotRetention {
        methods_value_retained,
        state_value_retained,
        custody,
    }
}

pub(super) const NO_RAW_VALUES: RawSlotRetention = slots(false, false, RawCustodyRetention::None);
pub(super) const INSTALLED_RAW_VALUES: RawSlotRetention =
    slots(true, true, RawCustodyRetention::InstalledEnvelope);
pub(super) const OPAQUE_STATE_VALUE: RawSlotRetention =
    slots(false, true, RawCustodyRetention::OpaqueStatePossible);
pub(super) const METHODS_VALUE_ONLY: RawSlotRetention =
    slots(true, false, RawCustodyRetention::None);
pub(super) const FOREIGN_METHODS_AND_OPAQUE_STATE: RawSlotRetention =
    slots(true, true, RawCustodyRetention::OpaqueStatePossible);
pub(super) const DROP_UNWIND_CUSTODY_PENDING: RawSlotRetention =
    slots(false, false, RawCustodyRetention::DropUnwindPending);

/// Exact safe raw-state shapes at the protected-call admission cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawAdmissionShape {
    NullFile,
    MethodsNullStateNull,
    MethodsNullStatePresent,
    ForeignMethodsStateNull,
    ForeignMethodsStatePresent,
    ExactMethodsStateNull,
    ExactMethodsInstalledWrongType,
    ExactMethodsInstalledExpectedType,
}

/// The source-neutral branch decision made by the raw-state gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawAdmissionDecision {
    NullFile,
    Uninstalled,
    ForeignMethodsNullTable,
    ForeignMethodsForeignTable,
    StateMissing,
    TypeMismatch,
    ExpectedTypeEntry,
}

/// The protected-call result before Map-specific source-step projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawAdmissionOutcome {
    NullFile,
    Uninstalled,
    ForeignMethods,
    StateMissing,
    TypeMismatch,
    ExpectedTypeEntry,
}

/// Source-neutral abandonment entrypoints shared by admission and post-operation paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawAbandonEndpoint {
    Empty,
    Installed,
    NullFileRejected,
    ForeignMethodsNullTableRejected,
    ForeignMethodsForeignTableRejected,
    StateMissingRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawAdmissionDisposition {
    Abandon(RawAbandonEndpoint),
    TypedOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawAdmissionFragment {
    pub(super) shape: RawAdmissionShape,
    pub(super) decision: RawAdmissionDecision,
    pub(super) outcome: RawAdmissionOutcome,
    pub(super) disposition: RawAdmissionDisposition,
    pub(super) slots: RawSlotRetention,
}

const fn admission(
    shape: RawAdmissionShape,
    decision: RawAdmissionDecision,
    outcome: RawAdmissionOutcome,
    disposition: RawAdmissionDisposition,
    slots: RawSlotRetention,
) -> RawAdmissionFragment {
    RawAdmissionFragment {
        shape,
        decision,
        outcome,
        disposition,
        slots,
    }
}

/// The exact eight-cell raw admission quotient. It has seven abandonment continuations and one
/// typed-operation continuation; neither continuation is a terminal.
pub(super) const RAW_ADMISSION_FRAGMENTS: &[RawAdmissionFragment] = &[
    admission(
        RawAdmissionShape::NullFile,
        RawAdmissionDecision::NullFile,
        RawAdmissionOutcome::NullFile,
        RawAdmissionDisposition::Abandon(RawAbandonEndpoint::NullFileRejected),
        NO_RAW_VALUES,
    ),
    admission(
        RawAdmissionShape::MethodsNullStateNull,
        RawAdmissionDecision::Uninstalled,
        RawAdmissionOutcome::Uninstalled,
        RawAdmissionDisposition::Abandon(RawAbandonEndpoint::Empty),
        NO_RAW_VALUES,
    ),
    admission(
        RawAdmissionShape::MethodsNullStatePresent,
        RawAdmissionDecision::ForeignMethodsNullTable,
        RawAdmissionOutcome::ForeignMethods,
        RawAdmissionDisposition::Abandon(RawAbandonEndpoint::ForeignMethodsNullTableRejected),
        OPAQUE_STATE_VALUE,
    ),
    admission(
        RawAdmissionShape::ForeignMethodsStateNull,
        RawAdmissionDecision::ForeignMethodsForeignTable,
        RawAdmissionOutcome::ForeignMethods,
        RawAdmissionDisposition::Abandon(RawAbandonEndpoint::ForeignMethodsForeignTableRejected),
        METHODS_VALUE_ONLY,
    ),
    admission(
        RawAdmissionShape::ForeignMethodsStatePresent,
        RawAdmissionDecision::ForeignMethodsForeignTable,
        RawAdmissionOutcome::ForeignMethods,
        RawAdmissionDisposition::Abandon(RawAbandonEndpoint::ForeignMethodsForeignTableRejected),
        FOREIGN_METHODS_AND_OPAQUE_STATE,
    ),
    admission(
        RawAdmissionShape::ExactMethodsStateNull,
        RawAdmissionDecision::StateMissing,
        RawAdmissionOutcome::StateMissing,
        RawAdmissionDisposition::Abandon(RawAbandonEndpoint::StateMissingRejected),
        METHODS_VALUE_ONLY,
    ),
    admission(
        RawAdmissionShape::ExactMethodsInstalledWrongType,
        RawAdmissionDecision::TypeMismatch,
        RawAdmissionOutcome::TypeMismatch,
        RawAdmissionDisposition::Abandon(RawAbandonEndpoint::Installed),
        INSTALLED_RAW_VALUES,
    ),
    admission(
        RawAdmissionShape::ExactMethodsInstalledExpectedType,
        RawAdmissionDecision::ExpectedTypeEntry,
        RawAdmissionOutcome::ExpectedTypeEntry,
        RawAdmissionDisposition::TypedOperation,
        INSTALLED_RAW_VALUES,
    ),
];

/// Outcomes observable only after entering the unresolved typed operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawPostOperationOutcome {
    AcceptedNormalReturn,
    CaughtUnwind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawPostOperationDisposition {
    ProtectedCallReturn,
    Abandon(RawAbandonEndpoint),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawPostOperationFragment {
    pub(super) outcome: RawPostOperationOutcome,
    pub(super) disposition: RawPostOperationDisposition,
    pub(super) slots: RawSlotRetention,
}

/// Normal return and caught unwind remain beyond the typed-operation cut.
pub(super) const RAW_POST_OPERATION_FRAGMENTS: &[RawPostOperationFragment] = &[
    RawPostOperationFragment {
        outcome: RawPostOperationOutcome::AcceptedNormalReturn,
        disposition: RawPostOperationDisposition::ProtectedCallReturn,
        slots: INSTALLED_RAW_VALUES,
    },
    RawPostOperationFragment {
        outcome: RawPostOperationOutcome::CaughtUnwind,
        disposition: RawPostOperationDisposition::Abandon(RawAbandonEndpoint::Installed),
        slots: INSTALLED_RAW_VALUES,
    },
];

/// The eight distinct results of invoking the raw abandonment helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawAbandonOutcome {
    Empty,
    InstalledDropCompleted,
    /// Both raw slots were cleared before the installed payload Drop unwound.
    InstalledDropUnwindCaught,
    NullFileRejected,
    ForeignMethodsNullTableRejected,
    ForeignMethodsForeignTableStateNullRejected,
    ForeignMethodsForeignTableStatePresentRejected,
    StateMissingRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawCleanupEffect {
    None,
    ClearSlotsThenDropInstalledEnvelope,
}

/// A cause is temporal: admission causes belong to the prefix, while post-operation causes remain
/// beyond the typed-operation cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawAbandonCauseFragment {
    Admission(RawAdmissionShape),
    PostOperation(RawPostOperationOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RawAbandonFragment {
    pub(super) outcome: RawAbandonOutcome,
    pub(super) causes: &'static [RawAbandonCauseFragment],
    pub(super) endpoint: RawAbandonEndpoint,
    pub(super) cleanup: RawCleanupEffect,
    pub(super) slots_after: RawSlotRetention,
}

const fn abandon(
    outcome: RawAbandonOutcome,
    causes: &'static [RawAbandonCauseFragment],
    endpoint: RawAbandonEndpoint,
    cleanup: RawCleanupEffect,
    slots_after: RawSlotRetention,
) -> RawAbandonFragment {
    RawAbandonFragment {
        outcome,
        causes,
        endpoint,
        cleanup,
        slots_after,
    }
}

/// Exact abandonment results. The installed endpoint has both a pre-typed TypeMismatch cause and
/// a caught-unwind cause beyond the typed-operation cut; this table does not invert that latter
/// association into a prefix successor.
pub(super) const RAW_ABANDON_FRAGMENTS: &[RawAbandonFragment] = &[
    abandon(
        RawAbandonOutcome::Empty,
        &[RawAbandonCauseFragment::Admission(
            RawAdmissionShape::MethodsNullStateNull,
        )],
        RawAbandonEndpoint::Empty,
        RawCleanupEffect::None,
        NO_RAW_VALUES,
    ),
    abandon(
        RawAbandonOutcome::InstalledDropCompleted,
        &[
            RawAbandonCauseFragment::Admission(RawAdmissionShape::ExactMethodsInstalledWrongType),
            RawAbandonCauseFragment::PostOperation(RawPostOperationOutcome::CaughtUnwind),
        ],
        RawAbandonEndpoint::Installed,
        RawCleanupEffect::ClearSlotsThenDropInstalledEnvelope,
        NO_RAW_VALUES,
    ),
    abandon(
        RawAbandonOutcome::InstalledDropUnwindCaught,
        &[
            RawAbandonCauseFragment::Admission(RawAdmissionShape::ExactMethodsInstalledWrongType),
            RawAbandonCauseFragment::PostOperation(RawPostOperationOutcome::CaughtUnwind),
        ],
        RawAbandonEndpoint::Installed,
        RawCleanupEffect::ClearSlotsThenDropInstalledEnvelope,
        DROP_UNWIND_CUSTODY_PENDING,
    ),
    abandon(
        RawAbandonOutcome::NullFileRejected,
        &[RawAbandonCauseFragment::Admission(
            RawAdmissionShape::NullFile,
        )],
        RawAbandonEndpoint::NullFileRejected,
        RawCleanupEffect::None,
        NO_RAW_VALUES,
    ),
    abandon(
        RawAbandonOutcome::ForeignMethodsNullTableRejected,
        &[RawAbandonCauseFragment::Admission(
            RawAdmissionShape::MethodsNullStatePresent,
        )],
        RawAbandonEndpoint::ForeignMethodsNullTableRejected,
        RawCleanupEffect::None,
        OPAQUE_STATE_VALUE,
    ),
    abandon(
        RawAbandonOutcome::ForeignMethodsForeignTableStateNullRejected,
        &[RawAbandonCauseFragment::Admission(
            RawAdmissionShape::ForeignMethodsStateNull,
        )],
        RawAbandonEndpoint::ForeignMethodsForeignTableRejected,
        RawCleanupEffect::None,
        METHODS_VALUE_ONLY,
    ),
    abandon(
        RawAbandonOutcome::ForeignMethodsForeignTableStatePresentRejected,
        &[RawAbandonCauseFragment::Admission(
            RawAdmissionShape::ForeignMethodsStatePresent,
        )],
        RawAbandonEndpoint::ForeignMethodsForeignTableRejected,
        RawCleanupEffect::None,
        FOREIGN_METHODS_AND_OPAQUE_STATE,
    ),
    abandon(
        RawAbandonOutcome::StateMissingRejected,
        &[RawAbandonCauseFragment::Admission(
            RawAdmissionShape::ExactMethodsStateNull,
        )],
        RawAbandonEndpoint::StateMissingRejected,
        RawCleanupEffect::None,
        METHODS_VALUE_ONLY,
    ),
];

/// Explicitly excluded premises. They are not additional raw-state cases or denominator leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RawStateFragmentExclusion {
    OccupiedInstallOnly,
    ForgedEnvelopeOrSlots,
    UndefinedBehaviorPremise,
    AbortingPanic,
}

pub(super) const RAW_STATE_FRAGMENT_EXCLUSIONS: &[RawStateFragmentExclusion] = &[
    RawStateFragmentExclusion::OccupiedInstallOnly,
    RawStateFragmentExclusion::ForgedEnvelopeOrSlots,
    RawStateFragmentExclusion::UndefinedBehaviorPremise,
    RawStateFragmentExclusion::AbortingPanic,
];
