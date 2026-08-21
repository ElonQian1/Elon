#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super::super) enum RegistrationShutdownSelector {
    OutstandingCallbackGate,
    LiveRouteGate,
    QuarantinedCustodyGate,
    RouteIndexObservation,
    VfsUnregisterBeforeCall,
    /// Deterministic injected pre-native retryable observation; SQLite unregister is not called.
    VfsUnregisterNativeRetryable,
    VfsUnregisterAfterSuccessKnown,
    Success,
}

impl RegistrationShutdownSelector {
    pub(in super::super::super) const ALL: [Self; 8] = [
        Self::OutstandingCallbackGate,
        Self::LiveRouteGate,
        Self::QuarantinedCustodyGate,
        Self::RouteIndexObservation,
        Self::VfsUnregisterBeforeCall,
        Self::VfsUnregisterNativeRetryable,
        Self::VfsUnregisterAfterSuccessKnown,
        Self::Success,
    ];

    pub(in super::super::super) const fn report_name(self) -> &'static str {
        match self {
            Self::OutstandingCallbackGate => "outstanding-callback-gate",
            Self::LiveRouteGate => "live-route-gate",
            Self::QuarantinedCustodyGate => "quarantined-custody-gate",
            Self::RouteIndexObservation => "route-index-observation",
            Self::VfsUnregisterBeforeCall => "vfs-unregister-before-call",
            Self::VfsUnregisterNativeRetryable => {
                "vfs-unregister-injected-pre-native-retryable-observation"
            }
            Self::VfsUnregisterAfterSuccessKnown => "vfs-unregister-after-success-known",
            Self::Success => "success",
        }
    }

    pub(in super::super::super) fn from_report_name(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|selector| selector.report_name() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(in super::super::super) enum RegistrationShutdownPhase {
    OutstandingCallbackGate = 0,
    LiveRouteGate = 1,
    QuarantinedCustodyGate = 2,
    RouteIndexObservation = 3,
    VfsUnregister = 4,
    Success = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(in super::super::super) enum RegistrationShutdownTiming {
    Validation = 0,
    BeforeCall = 1,
    NativeRetryable = 2,
    NativeUncertain = 3,
    AfterSuccessKnown = 4,
    Success = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(in super::super::super) enum RegistrationShutdownFailureClass {
    None = 0,
    RegistrationRetained = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(in super::super::super) enum RegistrationShutdownRegistryRoutePhase {
    Active = 0,
    Closing = 1,
    AwaitingRetirement = 2,
    Removed = 3,
    TerminalQuarantine = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(in super::super::super) enum RegistrationShutdownLogicalRoutePhase {
    Indexed = 0,
    Removed = 1,
    Retained = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(in super::super::super) enum RegistrationShutdownRegistrationPhase {
    Registered = 0,
    Unregistered = 1,
    RetainedRegistered = 2,
    RetainedAfterUnregister = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(in super::super::super) enum RegistrationShutdownDmsCustody {
    Absent = 0,
    Shared = 1,
    Released = 2,
    OutcomeUncertain = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Registration-scoped target semantics. The fresh exact-test child observes the real
/// registration identity; route/runtime/SHM identifiers remain inapplicable to shutdown itself.
pub(in super::super::super) struct RegistrationShutdownActualTarget {
    pub(in super::super::super) scope_is_registration: bool,
    pub(in super::super::super) registration_id: u64,
    pub(in super::super::super) route_ordinal_is_not_applicable: bool,
    pub(in super::super::super) runtime_generation_is_not_applicable: bool,
    pub(in super::super::super) shm_connection_id_is_not_applicable: bool,
    pub(in super::super::super) role_is_none: bool,
    pub(in super::super::super) callback_is_none: bool,
    pub(in super::super::super) occurrence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct RegistrationShutdownActualIdentity {
    pub(in super::super::super) path_is_registration_shutdown: bool,
    pub(in super::super::super) topology_is_registration_only: bool,
    pub(in super::super::super) unmap_is_not_applicable: bool,
    pub(in super::super::super) node_is_not_applicable: bool,
    pub(in super::super::super) variant: u8,
    pub(in super::super::super) pre_shared_mask: u8,
    pub(in super::super::super) pre_exclusive_mask: u8,
    pub(in super::super::super) phase: RegistrationShutdownPhase,
    pub(in super::super::super) cause_phase_is_none: bool,
    pub(in super::super::super) timing: RegistrationShutdownTiming,
    pub(in super::super::super) class: RegistrationShutdownFailureClass,
    pub(in super::super::super) target: RegistrationShutdownActualTarget,
    pub(in super::super::super) sqlite_outcome_is_not_applicable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct RegistrationShutdownActualTopology {
    pub(in super::super::super) sqlite_connections: u8,
    pub(in super::super::super) shm_connections: u8,
    pub(in super::super::super) registry_routes: u8,
    pub(in super::super::super) logical_names: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct RegistrationShutdownActualCustody {
    pub(in super::super::super) node: bool,
    pub(in super::super::super) views: u8,
    pub(in super::super::super) mappings: u8,
    pub(in super::super::super) dms: RegistrationShutdownDmsCustody,
    pub(in super::super::super) shm_file: bool,
    pub(in super::super::super) main_file: bool,
    pub(in super::super::super) main_lock_owner: bool,
    pub(in super::super::super) main_lease: bool,
    pub(in super::super::super) shm_lease: bool,
    pub(in super::super::super) callback_leases: u8,
    pub(in super::super::super) registry_entry: bool,
    pub(in super::super::super) logical_names: u8,
    pub(in super::super::super) vfs_table: bool,
    pub(in super::super::super) vfs_name: bool,
    pub(in super::super::super) vfs_context: bool,
    pub(in super::super::super) root_deletable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct RegistrationShutdownActualCounts {
    pub(in super::super::super) raw_state_take_attempt: u8,
    pub(in super::super::super) raw_state_take_success: u8,
    pub(in super::super::super) raw_state_abandon: u8,
    pub(in super::super::super) methods_clear: u8,
    pub(in super::super::super) callback_begin: u8,
    pub(in super::super::super) callback_complete_attempt: u8,
    pub(in super::super::super) callback_complete_success: u8,
    pub(in super::super::super) selected_action_attempt: u8,
    pub(in super::super::super) selected_action_success: u8,
    pub(in super::super::super) shm_detach: u8,
    pub(in super::super::super) main_unlock_attempt: u8,
    pub(in super::super::super) main_unlock_success: u8,
    pub(in super::super::super) main_file_close_attempt: u8,
    pub(in super::super::super) main_file_close_success: u8,
    pub(in super::super::super) registry_close_attempt: u8,
    pub(in super::super::super) registry_close_success: u8,
    pub(in super::super::super) connection_observe_attempt: u8,
    pub(in super::super::super) connection_observe_success: u8,
    pub(in super::super::super) registry_route_remove_attempt: u8,
    pub(in super::super::super) registry_route_remove_success: u8,
    pub(in super::super::super) logical_names_remove_attempt: u8,
    pub(in super::super::super) logical_names_remove_success: u8,
    pub(in super::super::super) logical_names_remove: u8,
    /// Entries into the unregister action seam, not necessarily calls to SQLite unregister.
    pub(in super::super::super) vfs_unregister_attempt: u8,
    pub(in super::super::super) vfs_unregister_success: u8,
    pub(in super::super::super) fault_observe: u8,
    pub(in super::super::super) fault_trigger: u8,
    pub(in super::super::super) fault_pending: u8,
    pub(in super::super::super) custody_retain: u8,
    pub(in super::super::super) physical_retry: u8,
}

#[derive(Debug, PartialEq, Eq)]
/// Independently observed raw case state. It deliberately has no `EvidenceKind`; validation keeps
/// the frozen case `StaticContract`, while a separate process/cleanup gate owns any later record.
pub(in super::super::super) struct RegistrationShutdownActual {
    pub(in super::super::super) selector: RegistrationShutdownSelector,
    pub(in super::super::super) identity: RegistrationShutdownActualIdentity,
    pub(in super::super::super) mutation_may_have_occurred: bool,
    pub(in super::super::super) lock_outcome_uncertain: bool,
    pub(in super::super::super) domain_terminal: bool,
    pub(in super::super::super) registry_route_phase: RegistrationShutdownRegistryRoutePhase,
    pub(in super::super::super) logical_route_phase: RegistrationShutdownLogicalRoutePhase,
    pub(in super::super::super) registration_phase: RegistrationShutdownRegistrationPhase,
    pub(in super::super::super) later_callback_allowed: bool,
    pub(in super::super::super) pre: RegistrationShutdownActualTopology,
    pub(in super::super::super) post: RegistrationShutdownActualTopology,
    pub(in super::super::super) retained: RegistrationShutdownActualCustody,
    pub(in super::super::super) counts: RegistrationShutdownActualCounts,
}
