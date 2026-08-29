#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super::super) enum BarrierSelector {
    AdmissionRejected,
    WrapperBefore,
    FenceBefore,
    FenceAfter,
    CompletionBefore,
    CompletionNativeUncertain,
    CompletionAfterSuccessKnown,
    Success,
}

impl BarrierSelector {
    pub(in super::super::super) const ALL: [Self; 8] = [
        Self::AdmissionRejected,
        Self::WrapperBefore,
        Self::FenceBefore,
        Self::FenceAfter,
        Self::CompletionBefore,
        Self::CompletionNativeUncertain,
        Self::CompletionAfterSuccessKnown,
        Self::Success,
    ];

    pub(in super::super::super) const fn report_name(self) -> &'static str {
        match self {
            Self::AdmissionRejected => "admission-rejected",
            Self::WrapperBefore => "wrapper-before",
            Self::FenceBefore => "fence-before",
            Self::FenceAfter => "fence-after",
            Self::CompletionBefore => "completion-before",
            Self::CompletionNativeUncertain => "completion-native-uncertain",
            Self::CompletionAfterSuccessKnown => "completion-after-success-known",
            Self::Success => "success",
        }
    }

    pub(in super::super::super) fn from_report_name(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|selector| selector.report_name() == value)
    }
}

macro_rules! wire_enum {
    ($name:ident { $($variant:ident = $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub(in super::super::super) enum $name { $($variant = $value),+ }

        impl TryFrom<u64> for $name {
            type Error = &'static str;

            fn try_from(value: u64) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(concat!(stringify!($name), " value is unsupported")),
                }
            }
        }
    };
}

wire_enum!(BarrierPhase {
    CallbackAdmission = 0,
    BarrierFence = 1,
    CallbackCompletion = 2,
    Success = 3,
});
wire_enum!(BarrierTiming {
    BeforeCall = 0,
    NativeUncertain = 1,
    AfterSuccessKnown = 2,
    AfterSuccessUncertain = 3,
    Success = 4,
});
wire_enum!(BarrierFailureClass {
    None = 0,
    IoBeforeMutation = 1,
    OutcomeUncertainPoisoned = 2,
    RegistryRejected = 3,
});
wire_enum!(BarrierRegistryRoutePhase {
    Active = 0,
    TerminalQuarantine = 1,
});
wire_enum!(BarrierLogicalRoutePhase {
    Indexed = 0,
    Retained = 1,
});
wire_enum!(BarrierRegistrationPhase { Registered = 0 });
wire_enum!(BarrierDmsCustody {
    Absent = 0,
    Shared = 1,
    Released = 2,
    OutcomeUncertain = 3,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct BarrierActualTarget {
    pub(in super::super::super) scope_is_route_main: bool,
    pub(in super::super::super) registration_id: u64,
    pub(in super::super::super) route_ordinal: u64,
    pub(in super::super::super) runtime_generation: u64,
    pub(in super::super::super) shm_connection_id: u64,
    pub(in super::super::super) role_is_main: bool,
    pub(in super::super::super) callback_is_shm: bool,
    pub(in super::super::super) occurrence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct BarrierActualIdentity {
    pub(in super::super::super) path_is_barrier: bool,
    pub(in super::super::super) topology_is_shared_non_final: bool,
    pub(in super::super::super) unmap_is_not_applicable: bool,
    pub(in super::super::super) node_is_live: bool,
    pub(in super::super::super) variant: u8,
    pub(in super::super::super) pre_shared_mask: u8,
    pub(in super::super::super) pre_exclusive_mask: u8,
    pub(in super::super::super) phase: BarrierPhase,
    pub(in super::super::super) cause_phase_is_none: bool,
    pub(in super::super::super) timing: BarrierTiming,
    pub(in super::super::super) class: BarrierFailureClass,
    pub(in super::super::super) target: BarrierActualTarget,
    pub(in super::super::super) sqlite_outcome_is_void_no_result_code: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct BarrierActualTopology {
    pub(in super::super::super) sqlite_connections: u8,
    pub(in super::super::super) shm_connections: u8,
    pub(in super::super::super) registry_routes: u8,
    pub(in super::super::super) logical_names: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct BarrierActualCustody {
    pub(in super::super::super) node: bool,
    pub(in super::super::super) views: u8,
    pub(in super::super::super) mappings: u8,
    pub(in super::super::super) dms: BarrierDmsCustody,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in super::super::super) struct BarrierActualCounts {
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
    pub(in super::super::super) vfs_unregister_attempt: u8,
    pub(in super::super::super) vfs_unregister_success: u8,
    pub(in super::super::super) fault_observe: u8,
    pub(in super::super::super) fault_trigger: u8,
    pub(in super::super::super) fault_pending: u8,
    pub(in super::super::super) custody_retain: u8,
    pub(in super::super::super) physical_retry: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub(in super::super::super) struct BarrierActual {
    pub(in super::super::super) selector: BarrierSelector,
    pub(in super::super::super) identity: BarrierActualIdentity,
    pub(in super::super::super) mutation_may_have_occurred: bool,
    pub(in super::super::super) lock_outcome_uncertain: bool,
    pub(in super::super::super) domain_terminal: bool,
    pub(in super::super::super) registry_route_phase: BarrierRegistryRoutePhase,
    pub(in super::super::super) logical_route_phase: BarrierLogicalRoutePhase,
    pub(in super::super::super) registration_phase: BarrierRegistrationPhase,
    pub(in super::super::super) later_callback_allowed: bool,
    pub(in super::super::super) pre: BarrierActualTopology,
    pub(in super::super::super) post: BarrierActualTopology,
    pub(in super::super::super) retained: BarrierActualCustody,
    pub(in super::super::super) counts: BarrierActualCounts,
}
