#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super::super) enum RegistryLifecycleSelector {
    CallbackCompletionBefore,
    CallbackCompletionNativeUncertain,
    CallbackCompletionAfterSuccessKnown,
    ConnectionObservationBefore,
    ConnectionObservationOutstandingSidecar,
    ConnectionObservationAfterSuccessKnown,
    RegistryRouteRemovalBefore,
    RegistryRouteRemovalOwnerNative,
    RegistryRouteRemovalPublishNative,
    RegistryRouteRemovalAfterSuccessKnown,
    LogicalRouteRemovalBefore,
    LogicalRouteRemovalClaimNative,
    LogicalRouteRemovalIndexNative,
    LogicalRouteRemovalAfterSuccessKnown,
    SuccessSharedNonFinal,
    SuccessFinal,
}

impl RegistryLifecycleSelector {
    pub(in super::super::super) const ALL: [Self; 16] = [
        Self::CallbackCompletionBefore,
        Self::CallbackCompletionNativeUncertain,
        Self::CallbackCompletionAfterSuccessKnown,
        Self::ConnectionObservationBefore,
        Self::ConnectionObservationOutstandingSidecar,
        Self::ConnectionObservationAfterSuccessKnown,
        Self::RegistryRouteRemovalBefore,
        Self::RegistryRouteRemovalOwnerNative,
        Self::RegistryRouteRemovalPublishNative,
        Self::RegistryRouteRemovalAfterSuccessKnown,
        Self::LogicalRouteRemovalBefore,
        Self::LogicalRouteRemovalClaimNative,
        Self::LogicalRouteRemovalIndexNative,
        Self::LogicalRouteRemovalAfterSuccessKnown,
        Self::SuccessSharedNonFinal,
        Self::SuccessFinal,
    ];

    pub(in super::super::super) const fn report_name(self) -> &'static str {
        match self {
            Self::CallbackCompletionBefore => "callback-completion-before",
            Self::CallbackCompletionNativeUncertain => "callback-completion-native-uncertain",
            Self::CallbackCompletionAfterSuccessKnown => "callback-completion-after-success-known",
            Self::ConnectionObservationBefore => "connection-observation-before",
            Self::ConnectionObservationOutstandingSidecar => {
                "connection-observation-outstanding-sidecar"
            }
            Self::ConnectionObservationAfterSuccessKnown => {
                "connection-observation-after-success-known"
            }
            Self::RegistryRouteRemovalBefore => "registry-route-removal-before",
            Self::RegistryRouteRemovalOwnerNative => "registry-route-removal-owner-native",
            Self::RegistryRouteRemovalPublishNative => "registry-route-removal-publish-native",
            Self::RegistryRouteRemovalAfterSuccessKnown => {
                "registry-route-removal-after-success-known"
            }
            Self::LogicalRouteRemovalBefore => "logical-route-removal-before",
            Self::LogicalRouteRemovalClaimNative => "logical-route-removal-claim-native",
            Self::LogicalRouteRemovalIndexNative => "logical-route-removal-index-native",
            Self::LogicalRouteRemovalAfterSuccessKnown => {
                "logical-route-removal-after-success-known"
            }
            Self::SuccessSharedNonFinal => "success-shared-nonfinal",
            Self::SuccessFinal => "success-final",
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

wire_enum!(RegistryLifecyclePhase {
    CallbackCompletion = 0,
    ConnectionObservation = 1,
    RegistryRouteRemoval = 2,
    LogicalRouteRemoval = 3,
    Success = 4,
});
wire_enum!(RegistryLifecycleTiming {
    Validation = 0,
    BeforeCall = 1,
    NativeUncertain = 2,
    AfterSuccessKnown = 3,
    Success = 4,
});
wire_enum!(RegistryLifecycleFailureClass {
    None = 0,
    RegistryRejected = 1,
});
wire_enum!(RegistryLifecycleSqliteOutcome {
    Ok = 0,
    IoerrClose = 1,
    NotApplicable = 2,
});
wire_enum!(RegistryLifecycleRegistryRoutePhase {
    Active = 0,
    AwaitingRetirement = 1,
    Removed = 2,
    TerminalQuarantine = 3,
});
wire_enum!(RegistryLifecycleLogicalRoutePhase {
    Indexed = 0,
    Removed = 1,
    Retained = 2,
});
wire_enum!(RegistryLifecycleRegistrationPhase { Registered = 0 });
wire_enum!(RegistryLifecycleDmsCustody {
    Absent = 0,
    Shared = 1,
    Released = 2,
    OutcomeUncertain = 3,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct RegistryLifecycleActualTarget {
    pub(in super::super::super) scope_is_route_main: bool,
    pub(in super::super::super) registration_id: u64,
    pub(in super::super::super) route_ordinal: u64,
    pub(in super::super::super) runtime_generation: u64,
    pub(in super::super::super) shm_connection_id: u64,
    pub(in super::super::super) role_is_main: bool,
    pub(in super::super::super) callback_is_close: bool,
    pub(in super::super::super) occurrence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct RegistryLifecycleActualIdentity {
    pub(in super::super::super) path_is_registry_lifecycle: bool,
    pub(in super::super::super) topology_is_shared_non_final: bool,
    pub(in super::super::super) unmap_is_keep: bool,
    pub(in super::super::super) node_is_live: bool,
    pub(in super::super::super) variant: u8,
    pub(in super::super::super) pre_shared_mask: u8,
    pub(in super::super::super) pre_exclusive_mask: u8,
    pub(in super::super::super) phase: RegistryLifecyclePhase,
    pub(in super::super::super) cause_phase_is_none: bool,
    pub(in super::super::super) timing: RegistryLifecycleTiming,
    pub(in super::super::super) class: RegistryLifecycleFailureClass,
    pub(in super::super::super) target: RegistryLifecycleActualTarget,
    pub(in super::super::super) sqlite_outcome: RegistryLifecycleSqliteOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct RegistryLifecycleActualTopology {
    pub(in super::super::super) sqlite_connections: u8,
    pub(in super::super::super) shm_connections: u8,
    pub(in super::super::super) registry_routes: u8,
    pub(in super::super::super) logical_names: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct RegistryLifecycleActualCustody {
    pub(in super::super::super) node: bool,
    pub(in super::super::super) views: u8,
    pub(in super::super::super) mappings: u8,
    pub(in super::super::super) dms: RegistryLifecycleDmsCustody,
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
pub(in super::super::super) struct RegistryLifecycleActualCounts {
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
pub(in super::super::super) struct RegistryLifecycleActual {
    pub(in super::super::super) selector: RegistryLifecycleSelector,
    pub(in super::super::super) identity: RegistryLifecycleActualIdentity,
    pub(in super::super::super) mutation_may_have_occurred: bool,
    pub(in super::super::super) lock_outcome_uncertain: bool,
    pub(in super::super::super) domain_terminal: bool,
    pub(in super::super::super) registry_route_phase: RegistryLifecycleRegistryRoutePhase,
    pub(in super::super::super) logical_route_phase: RegistryLifecycleLogicalRoutePhase,
    pub(in super::super::super) registration_phase: RegistryLifecycleRegistrationPhase,
    pub(in super::super::super) later_callback_allowed: bool,
    pub(in super::super::super) pre: RegistryLifecycleActualTopology,
    pub(in super::super::super) post: RegistryLifecycleActualTopology,
    pub(in super::super::super) retained: RegistryLifecycleActualCustody,
    pub(in super::super::super) counts: RegistryLifecycleActualCounts,
}
