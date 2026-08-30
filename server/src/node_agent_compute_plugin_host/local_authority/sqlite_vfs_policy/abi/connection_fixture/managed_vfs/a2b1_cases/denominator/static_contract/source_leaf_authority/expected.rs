//! V1 observable result schema owned by the source-first ledger.
//!
//! These types intentionally duplicate the public shape of the graph Expected vector.  The
//! authority must construct them independently instead of invoking graph projection helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SqliteResultV1 {
    Ok,
    Busy,
    MapUnavailable,
    LockUnavailable,
}

impl SqliteResultV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Busy => "busy",
            Self::MapUnavailable => "map-unavailable",
            Self::LockUnavailable => "lock-unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TerminalDispositionV1 {
    Returned,
    Abandoned,
    Quarantined,
    CleanupRewritten,
}

impl TerminalDispositionV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::Returned => "returned",
            Self::Abandoned => "abandoned",
            Self::Quarantined => "quarantined",
            Self::CleanupRewritten => "cleanup-rewritten",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FailureClassV1 {
    None,
    ProtocolViolation,
    RegistryRejected,
    BusyNoMutation,
    BusyAfterKnownMutation,
    NotPresent,
    IoBeforeMutation,
    MutatedButKnown,
    OutcomeUncertainPoisoned,
    PlatformUnsupported,
}

impl FailureClassV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ProtocolViolation => "protocol-violation",
            Self::RegistryRejected => "registry-rejected",
            Self::BusyNoMutation => "busy-no-mutation",
            Self::BusyAfterKnownMutation => "busy-after-known-mutation",
            Self::NotPresent => "not-present",
            Self::IoBeforeMutation => "io-before-mutation",
            Self::MutatedButKnown => "mutated-but-known",
            Self::OutcomeUncertainPoisoned => "outcome-uncertain-poisoned",
            Self::PlatformUnsupported => "platform-unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MutationStateV1 {
    None,
    Known,
    Uncertain,
}

impl MutationStateV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Known => "known",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CustodyStateV1 {
    NotReached,
    Unchanged,
    Released,
    Retained,
    Quarantined,
    Cleared,
}

impl CustodyStateV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::NotReached => "not-reached",
            Self::Unchanged => "unchanged",
            Self::Released => "released",
            Self::Retained => "retained",
            Self::Quarantined => "quarantined",
            Self::Cleared => "cleared",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LockEffectV1 {
    NotReached,
    Unchanged,
    Acquired {
        mode: LockModeV1,
        mask: u8,
        native: bool,
    },
    Released {
        mode: LockModeV1,
        mask: u8,
        native: bool,
    },
    OutcomeUncertain {
        mode: LockModeV1,
        mask: u8,
    },
}

impl LockEffectV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::NotReached => "not-reached",
            Self::Unchanged => "unchanged",
            Self::Acquired { .. } => "acquired",
            Self::Released { .. } => "released",
            Self::OutcomeUncertain { .. } => "outcome-uncertain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LockModeV1 {
    Shared,
    Exclusive,
}

impl LockModeV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Exclusive => "exclusive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DmsLockCustodyV1 {
    NotReached,
    UnknownRetained,
    UnobservedRetained,
    ExistingShared,
    AcquiredShared,
    Released,
    ExclusiveKnown,
    ExclusiveOutcomeUncertain,
}

impl DmsLockCustodyV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::NotReached => "not-reached",
            Self::UnknownRetained => "unknown-retained",
            Self::UnobservedRetained => "unobserved-retained",
            Self::ExistingShared => "existing-shared",
            Self::AcquiredShared => "acquired-shared",
            Self::Released => "released",
            Self::ExclusiveKnown => "exclusive-known",
            Self::ExclusiveOutcomeUncertain => "exclusive-outcome-uncertain",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ObservableCountsV1 {
    pub(crate) callback_begin: u16,
    pub(crate) callback_complete: u16,
    pub(crate) native_lock: u16,
    pub(crate) native_unlock: u16,
    pub(crate) file_grow: u16,
    pub(crate) mapping_create: u16,
    pub(crate) view_map: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExpectedV1 {
    pub(crate) sqlite: SqliteResultV1,
    pub(crate) disposition: TerminalDispositionV1,
    pub(crate) phase: String,
    pub(crate) failure: FailureClassV1,
    pub(crate) mutation: MutationStateV1,
    pub(crate) lock_outcome_uncertain: bool,
    pub(crate) lock_effect: LockEffectV1,
    pub(crate) dms_lock: DmsLockCustodyV1,
    pub(crate) raw_slots: CustodyStateV1,
    pub(crate) route: CustodyStateV1,
    pub(crate) callback: CustodyStateV1,
    pub(crate) file: CustodyStateV1,
    pub(crate) mapping: CustodyStateV1,
    pub(crate) view: CustodyStateV1,
    pub(crate) payload: CustodyStateV1,
    pub(crate) counts: ObservableCountsV1,
}
