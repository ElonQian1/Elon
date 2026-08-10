use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CallbackKind {
    Shm,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Path {
    Barrier,
    Unmap,
    JointClose,
    RegistryLifecycle,
    RegistrationShutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum TopologyKind {
    SharedNonFinal,
    FinalConnection,
    RegistrationOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum UnmapMode {
    NotApplicable,
    Keep,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum NodePrecondition {
    NotApplicable,
    Absent,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Phase {
    RequestValidation,
    CallbackAdmission,
    BarrierFence,
    HeldLockGate,
    ConnectionDetach,
    ViewUnmap,
    MappingClose,
    DmsSharedRelease,
    ShmFileClose,
    DeleteAuthorization,
    ExactSiblingDelete,
    RawStateTake,
    BeginConnectionClose,
    ShmUnmapLift,
    MainLockRelease,
    MainFileClose,
    RegistryWalMainClose,
    CallbackCompletion,
    ConnectionObservation,
    RegistryRouteRemoval,
    LogicalRouteRemoval,
    OutstandingCallbackGate,
    LiveRouteGate,
    QuarantinedCustodyGate,
    RouteIndexObservation,
    VfsUnregister,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Timing {
    Validation,
    BeforeCall,
    NativeRetryable,
    NativeUncertain,
    AfterSuccessKnown,
    AfterSuccessUncertain,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FailureClass {
    None,
    ProtocolViolation,
    BusyNoMutation,
    IoBeforeMutation,
    MutatedButKnown,
    OutcomeUncertainPoisoned,
    RegistryRejected,
    RegistrationRetained,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SqliteOutcome {
    NotApplicable,
    VoidNoResultCode,
    Ok,
    Ioerr,
    IoerrClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RegistryRoutePhase {
    Active,
    Closing,
    AwaitingRetirement,
    Removed,
    TerminalQuarantine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LogicalRoutePhase {
    Indexed,
    Removed,
    Retained,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RegistrationPhase {
    Registered,
    Unregistered,
    RetainedRegistered,
    RetainedAfterUnregister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DmsCustody {
    Absent,
    Shared,
    Released,
    OutcomeUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum TargetScope {
    RouteMain,
    Registration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ExactTarget {
    pub(super) scope: TargetScope,
    pub(super) registration_id: u64,
    pub(super) route_ordinal: u64,
    pub(super) runtime_generation: u64,
    pub(super) shm_connection_id: u64,
    pub(super) role: Option<ManagedSqliteLogicalFileRole>,
    pub(super) callback: Option<CallbackKind>,
    pub(super) occurrence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Topology {
    pub(super) sqlite_connections: u8,
    pub(super) shm_connections: u8,
    pub(super) registry_routes: u8,
    pub(super) logical_names: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Custody {
    pub(super) node: bool,
    pub(super) views: u8,
    pub(super) mappings: u8,
    pub(super) dms: DmsCustody,
    pub(super) shm_file: bool,
    pub(super) main_file: bool,
    pub(super) main_lock_owner: bool,
    pub(super) main_lease: bool,
    pub(super) shm_lease: bool,
    pub(super) callback_leases: u8,
    pub(super) registry_entry: bool,
    pub(super) logical_names: u8,
    pub(super) vfs_table: bool,
    pub(super) vfs_name: bool,
    pub(super) vfs_context: bool,
    pub(super) root_deletable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Counts {
    pub(super) raw_state_take_attempt: u8,
    pub(super) raw_state_take_success: u8,
    pub(super) raw_state_abandon: u8,
    pub(super) methods_clear: u8,
    pub(super) callback_begin: u8,
    pub(super) callback_complete_attempt: u8,
    pub(super) callback_complete_success: u8,
    pub(super) selected_action_attempt: u8,
    pub(super) selected_action_success: u8,
    pub(super) shm_detach: u8,
    pub(super) main_unlock_attempt: u8,
    pub(super) main_unlock_success: u8,
    pub(super) main_file_close_attempt: u8,
    pub(super) main_file_close_success: u8,
    pub(super) registry_close_attempt: u8,
    pub(super) registry_close_success: u8,
    pub(super) connection_observe_attempt: u8,
    pub(super) connection_observe_success: u8,
    pub(super) registry_route_remove_attempt: u8,
    pub(super) registry_route_remove_success: u8,
    pub(super) logical_names_remove_attempt: u8,
    pub(super) logical_names_remove_success: u8,
    pub(super) logical_names_remove: u8,
    pub(super) vfs_unregister_attempt: u8,
    pub(super) vfs_unregister_success: u8,
    pub(super) fault_observe: u8,
    pub(super) fault_trigger: u8,
    pub(super) fault_pending: u8,
    pub(super) custody_retain: u8,
    pub(super) physical_retry: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EvidenceKind {
    StaticContract,
    WindowsDynamic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Case {
    pub(super) path: Path,
    pub(super) topology_kind: TopologyKind,
    pub(super) unmap_mode: UnmapMode,
    pub(super) node_precondition: NodePrecondition,
    pub(super) variant: u8,
    pub(super) pre_shared_mask: u8,
    pub(super) pre_exclusive_mask: u8,
    pub(super) phase: Phase,
    pub(super) cause_phase: Option<Phase>,
    pub(super) timing: Timing,
    pub(super) class: FailureClass,
    pub(super) target: ExactTarget,
    pub(super) sqlite_outcome: SqliteOutcome,
    pub(super) mutation_may_have_occurred: bool,
    pub(super) lock_outcome_uncertain: bool,
    pub(super) domain_terminal: bool,
    pub(super) registry_route_phase: RegistryRoutePhase,
    pub(super) logical_route_phase: LogicalRoutePhase,
    pub(super) registration_phase: RegistrationPhase,
    pub(super) later_callback_allowed: bool,
    pub(super) pre: Topology,
    pub(super) post: Topology,
    pub(super) retained: Custody,
    pub(super) counts: Counts,
    pub(super) evidence: EvidenceKind,
}

pub(super) const TWO: Topology = Topology {
    sqlite_connections: 2,
    shm_connections: 2,
    registry_routes: 2,
    logical_names: 6,
};

pub(super) const ONE: Topology = Topology {
    sqlite_connections: 1,
    shm_connections: 1,
    registry_routes: 1,
    logical_names: 3,
};

pub(super) const EMPTY: Topology = Topology {
    sqlite_connections: 0,
    shm_connections: 0,
    registry_routes: 0,
    logical_names: 0,
};

pub(super) const LIVE_CUSTODY: Custody = Custody {
    node: true,
    views: 1,
    mappings: 1,
    dms: DmsCustody::Shared,
    shm_file: true,
    main_file: true,
    main_lock_owner: true,
    main_lease: true,
    shm_lease: true,
    callback_leases: 0,
    registry_entry: true,
    logical_names: 3,
    vfs_table: true,
    vfs_name: true,
    vfs_context: true,
    root_deletable: false,
};

pub(super) const ZERO_COUNTS: Counts = Counts {
    raw_state_take_attempt: 0,
    raw_state_take_success: 0,
    raw_state_abandon: 0,
    methods_clear: 0,
    callback_begin: 0,
    callback_complete_attempt: 0,
    callback_complete_success: 0,
    selected_action_attempt: 0,
    selected_action_success: 0,
    shm_detach: 0,
    main_unlock_attempt: 0,
    main_unlock_success: 0,
    main_file_close_attempt: 0,
    main_file_close_success: 0,
    registry_close_attempt: 0,
    registry_close_success: 0,
    connection_observe_attempt: 0,
    connection_observe_success: 0,
    registry_route_remove_attempt: 0,
    registry_route_remove_success: 0,
    logical_names_remove_attempt: 0,
    logical_names_remove_success: 0,
    logical_names_remove: 0,
    vfs_unregister_attempt: 0,
    vfs_unregister_success: 0,
    fault_observe: 0,
    fault_trigger: 0,
    fault_pending: 0,
    custody_retain: 0,
    physical_retry: 0,
};

pub(super) const fn target(callback: Option<CallbackKind>) -> ExactTarget {
    ExactTarget {
        scope: TargetScope::RouteMain,
        registration_id: 1,
        route_ordinal: 1,
        runtime_generation: 1,
        shm_connection_id: 1,
        role: Some(ManagedSqliteLogicalFileRole::Main),
        callback,
        occurrence: 1,
    }
}

pub(super) const fn base(
    path: Path,
    topology_kind: TopologyKind,
    phase: Phase,
    callback: Option<CallbackKind>,
) -> Case {
    let topology = match topology_kind {
        TopologyKind::SharedNonFinal => TWO,
        TopologyKind::FinalConnection | TopologyKind::RegistrationOnly => ONE,
    };
    Case {
        path,
        topology_kind,
        unmap_mode: UnmapMode::NotApplicable,
        node_precondition: NodePrecondition::Live,
        variant: 0,
        pre_shared_mask: 0,
        pre_exclusive_mask: 0,
        phase,
        cause_phase: None,
        timing: Timing::Success,
        class: FailureClass::None,
        target: target(callback),
        sqlite_outcome: match path {
            Path::Barrier => SqliteOutcome::VoidNoResultCode,
            Path::RegistrationShutdown => SqliteOutcome::NotApplicable,
            _ => SqliteOutcome::Ok,
        },
        mutation_may_have_occurred: false,
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: RegistryRoutePhase::Active,
        logical_route_phase: LogicalRoutePhase::Indexed,
        registration_phase: RegistrationPhase::Registered,
        later_callback_allowed: true,
        pre: topology,
        post: topology,
        retained: LIVE_CUSTODY,
        counts: ZERO_COUNTS,
        evidence: EvidenceKind::StaticContract,
    }
}

pub(super) fn terminal(mut case: Case, class: FailureClass, mutation: bool) -> Case {
    case.class = class;
    case.sqlite_outcome = failure_outcome(case.path, case.phase);
    case.mutation_may_have_occurred = mutation;
    case.domain_terminal = true;
    if case.registry_route_phase != RegistryRoutePhase::Removed {
        case.registry_route_phase = RegistryRoutePhase::TerminalQuarantine;
    }
    if case.logical_route_phase != LogicalRoutePhase::Removed {
        case.logical_route_phase = LogicalRoutePhase::Retained;
    }
    case.later_callback_allowed = false;
    case.retained.callback_leases = case
        .counts
        .callback_begin
        .saturating_sub(case.counts.callback_complete_success);
    case.counts.custody_retain = 1;
    case
}

pub(super) fn route_terminal(case: Case, class: FailureClass, mutation: bool) -> Case {
    let mut case = terminal(case, class, mutation);
    case.domain_terminal = false;
    case
}

pub(super) fn failure(mut case: Case, timing: Timing, class: FailureClass) -> Case {
    case.timing = timing;
    case.class = class;
    case.sqlite_outcome = failure_outcome(case.path, case.phase);
    if matches!(
        timing,
        Timing::BeforeCall | Timing::AfterSuccessKnown | Timing::AfterSuccessUncertain
    ) {
        case.counts.fault_observe = 1;
        case.counts.fault_trigger = 1;
    }
    case
}

pub(super) fn native_observed(mut case: Case) -> Case {
    case.counts.fault_observe = 1;
    case
}

const fn failure_outcome(path: Path, phase: Phase) -> SqliteOutcome {
    match (path, phase) {
        (Path::Barrier, _) => SqliteOutcome::VoidNoResultCode,
        (Path::RegistrationShutdown, _) | (_, Phase::LogicalRouteRemoval) => {
            SqliteOutcome::NotApplicable
        }
        (Path::JointClose | Path::RegistryLifecycle, _) => SqliteOutcome::IoerrClose,
        (Path::Unmap, _) => SqliteOutcome::Ioerr,
    }
}

pub(super) fn observed_but_pending(mut case: Case) -> Case {
    case.counts.fault_observe = 1;
    case.counts.fault_trigger = 0;
    case.counts.fault_pending = 1;
    case
}

#[allow(dead_code)]
pub(super) const DYNAMIC_EVIDENCE_IS_NOT_THIS_BATCH: EvidenceKind = EvidenceKind::WindowsDynamic;
