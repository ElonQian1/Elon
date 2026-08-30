use std::collections::BTreeSet;

use super::source::SourceWitness;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RootOperation {
    Map,
    Lock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DecisionStage {
    AbiValidation,
    RawAdmission,
    RawAbandon,
    Adapter,
    CallbackAdmission,
    ManagedRequest,
    Initialization,
    Coordination,
    NativeCall,
    Cleanup,
    Quarantine,
    CallbackCompletion,
    AbiProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Decision {
    pub(super) stage: DecisionStage,
    pub(super) branch: String,
}

impl Decision {
    pub(super) fn new(stage: DecisionStage, branch: impl Into<String>) -> Self {
        Self {
            stage,
            branch: branch.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SqliteResult {
    Ok,
    Busy,
    MapUnavailable,
    LockUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum TerminalDisposition {
    Returned,
    Abandoned,
    Quarantined,
    CleanupRewritten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FailureClass {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MutationState {
    None,
    Known,
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CustodyState {
    NotReached,
    Unchanged,
    Released,
    Retained,
    Quarantined,
    Cleared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockEffect {
    NotReached,
    Unchanged,
    Acquired {
        mode: LockMode,
        mask: u8,
        native: bool,
    },
    Released {
        mode: LockMode,
        mask: u8,
        native: bool,
    },
    OutcomeUncertain {
        mode: LockMode,
        mask: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LockMode {
    Shared,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DmsLockCustody {
    NotReached,
    UnknownRetained,
    UnobservedRetained,
    ExistingShared,
    AcquiredShared,
    Released,
    ExclusiveKnown,
    ExclusiveOutcomeUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(super) struct ObservableCounts {
    pub(super) callback_begin: u16,
    pub(super) callback_complete: u16,
    pub(super) native_lock: u16,
    pub(super) native_unlock: u16,
    pub(super) file_grow: u16,
    pub(super) mapping_create: u16,
    pub(super) view_map: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Expected {
    pub(super) sqlite: SqliteResult,
    pub(super) disposition: TerminalDisposition,
    pub(super) phase: &'static str,
    pub(super) failure: FailureClass,
    pub(super) mutation: MutationState,
    pub(super) lock_outcome_uncertain: bool,
    pub(super) lock_effect: LockEffect,
    pub(super) dms_lock: DmsLockCustody,
    pub(super) raw_slots: CustodyState,
    pub(super) route: CustodyState,
    pub(super) callback: CustodyState,
    pub(super) file: CustodyState,
    pub(super) mapping: CustodyState,
    pub(super) view: CustodyState,
    pub(super) payload: CustodyState,
    pub(super) counts: ObservableCounts,
}

impl Expected {
    pub(super) const fn unavailable(root: RootOperation, phase: &'static str) -> Self {
        Self {
            sqlite: match root {
                RootOperation::Map => SqliteResult::MapUnavailable,
                RootOperation::Lock => SqliteResult::LockUnavailable,
            },
            disposition: TerminalDisposition::Returned,
            phase,
            failure: FailureClass::ProtocolViolation,
            mutation: MutationState::None,
            lock_outcome_uncertain: false,
            lock_effect: LockEffect::NotReached,
            dms_lock: DmsLockCustody::NotReached,
            raw_slots: CustodyState::NotReached,
            route: CustodyState::NotReached,
            callback: CustodyState::NotReached,
            file: CustodyState::NotReached,
            mapping: CustodyState::NotReached,
            view: CustodyState::NotReached,
            payload: CustodyState::NotReached,
            counts: ObservableCounts {
                callback_begin: 0,
                callback_complete: 0,
                native_lock: 0,
                native_unlock: 0,
                file_grow: 0,
                mapping_create: 0,
                view_map: 0,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ExclusionProof {
    TypeInvariant(&'static str),
    ControlFlow(&'static str),
    SafetyPremise(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NodeKind {
    Decision,
    Continuation {
        expansion_owner: &'static str,
    },
    Terminal {
        leaf_id: String,
        expected: Expected,
    },
    Excluded {
        leaf_id: String,
        proof: ExclusionProof,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ContractNode {
    pub(super) id: String,
    pub(super) kind: NodeKind,
    pub(super) witness: Option<SourceWitness>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ContractEdge {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) decision: Decision,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CaseKey {
    pub(super) root: RootOperation,
    pub(super) leaf_id: String,
    pub(super) decisions: Vec<Decision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StaticCase {
    pub(super) key: CaseKey,
    pub(super) source_branch: Vec<SourceWitness>,
    pub(super) expected: Expected,
}

#[derive(Debug, Clone)]
pub(super) struct ContractGraph {
    pub(super) name: &'static str,
    pub(super) root_operation: RootOperation,
    pub(super) root: String,
    pub(super) nodes: Vec<ContractNode>,
    pub(super) edges: Vec<ContractEdge>,
    pub(super) source_leaf_universe: BTreeSet<String>,
    pub(super) declared_denominator: usize,
    pub(super) legacy_non_denominator_width: usize,
}
