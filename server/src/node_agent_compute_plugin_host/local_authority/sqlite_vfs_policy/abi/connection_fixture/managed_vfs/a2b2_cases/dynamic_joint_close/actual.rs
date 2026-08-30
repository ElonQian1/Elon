#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super::super) enum JointCloseSelector {
    RawStateTakeRejected,
    BeginConnectionCloseRejected,
    CallbackAdmissionRejected,
    CallbackWrapperBefore,
    ShmViewUnmapBefore,
    ShmViewUnmapNativeUncertain,
    ShmViewUnmapAfterKnown,
    ShmViewUnmapAfterUncertain,
    ShmMappingCloseBefore,
    ShmMappingCloseNativeUncertain,
    ShmMappingCloseAfterKnown,
    ShmMappingCloseAfterUncertain,
    ShmDmsReleaseBefore,
    ShmDmsReleaseNativeUncertain,
    ShmDmsReleaseAfterKnown,
    ShmDmsReleaseAfterUncertain,
    ShmFileCloseBefore,
    ShmFileCloseNativeRetryable,
    ShmFileCloseNativeUncertain,
    ShmFileCloseAfterKnown,
    ShmFileCloseAfterUncertain,
    ShmDetachBefore,
    ShmDetachAfterKnown,
    ShmDetachAfterUncertain,
    MainLockReleaseBefore,
    MainLockReleaseNativeUncertainShared,
    MainLockReleaseNativeUncertainReserved,
    MainLockReleaseAfterKnown,
    MainFileCloseBefore,
    MainFileCloseNativeRetryable,
    MainFileCloseNativeUncertain,
    MainFileCloseAfterKnown,
    PhysicalSuccess,
    RegistryWalMainCloseBefore,
    RegistryWalMainCloseNativeUncertain,
    RegistryWalMainCloseAfterKnown,
}

impl JointCloseSelector {
    pub(in super::super::super) const ALL: [Self; 36] = [
        Self::RawStateTakeRejected,
        Self::BeginConnectionCloseRejected,
        Self::CallbackAdmissionRejected,
        Self::CallbackWrapperBefore,
        Self::ShmViewUnmapBefore,
        Self::ShmViewUnmapNativeUncertain,
        Self::ShmViewUnmapAfterKnown,
        Self::ShmViewUnmapAfterUncertain,
        Self::ShmMappingCloseBefore,
        Self::ShmMappingCloseNativeUncertain,
        Self::ShmMappingCloseAfterKnown,
        Self::ShmMappingCloseAfterUncertain,
        Self::ShmDmsReleaseBefore,
        Self::ShmDmsReleaseNativeUncertain,
        Self::ShmDmsReleaseAfterKnown,
        Self::ShmDmsReleaseAfterUncertain,
        Self::ShmFileCloseBefore,
        Self::ShmFileCloseNativeRetryable,
        Self::ShmFileCloseNativeUncertain,
        Self::ShmFileCloseAfterKnown,
        Self::ShmFileCloseAfterUncertain,
        Self::ShmDetachBefore,
        Self::ShmDetachAfterKnown,
        Self::ShmDetachAfterUncertain,
        Self::MainLockReleaseBefore,
        Self::MainLockReleaseNativeUncertainShared,
        Self::MainLockReleaseNativeUncertainReserved,
        Self::MainLockReleaseAfterKnown,
        Self::MainFileCloseBefore,
        Self::MainFileCloseNativeRetryable,
        Self::MainFileCloseNativeUncertain,
        Self::MainFileCloseAfterKnown,
        Self::PhysicalSuccess,
        Self::RegistryWalMainCloseBefore,
        Self::RegistryWalMainCloseNativeUncertain,
        Self::RegistryWalMainCloseAfterKnown,
    ];

    pub(in super::super::super) const fn report_name(self) -> &'static str {
        match self {
            Self::RawStateTakeRejected => "raw-state-take-rejected",
            Self::BeginConnectionCloseRejected => "begin-connection-close-rejected",
            Self::CallbackAdmissionRejected => "callback-admission-rejected",
            Self::CallbackWrapperBefore => "callback-wrapper-before",
            Self::ShmViewUnmapBefore => "shm-view-unmap-before",
            Self::ShmViewUnmapNativeUncertain => "shm-view-unmap-native-uncertain",
            Self::ShmViewUnmapAfterKnown => "shm-view-unmap-after-known",
            Self::ShmViewUnmapAfterUncertain => "shm-view-unmap-after-uncertain",
            Self::ShmMappingCloseBefore => "shm-mapping-close-before",
            Self::ShmMappingCloseNativeUncertain => "shm-mapping-close-native-uncertain",
            Self::ShmMappingCloseAfterKnown => "shm-mapping-close-after-known",
            Self::ShmMappingCloseAfterUncertain => "shm-mapping-close-after-uncertain",
            Self::ShmDmsReleaseBefore => "shm-dms-release-before",
            Self::ShmDmsReleaseNativeUncertain => "shm-dms-release-native-uncertain",
            Self::ShmDmsReleaseAfterKnown => "shm-dms-release-after-known",
            Self::ShmDmsReleaseAfterUncertain => "shm-dms-release-after-uncertain",
            Self::ShmFileCloseBefore => "shm-file-close-before",
            Self::ShmFileCloseNativeRetryable => "shm-file-close-native-retryable",
            Self::ShmFileCloseNativeUncertain => "shm-file-close-native-uncertain",
            Self::ShmFileCloseAfterKnown => "shm-file-close-after-known",
            Self::ShmFileCloseAfterUncertain => "shm-file-close-after-uncertain",
            Self::ShmDetachBefore => "shm-detach-before",
            Self::ShmDetachAfterKnown => "shm-detach-after-known",
            Self::ShmDetachAfterUncertain => "shm-detach-after-uncertain",
            Self::MainLockReleaseBefore => "main-lock-release-before",
            Self::MainLockReleaseNativeUncertainShared => {
                "main-lock-release-native-uncertain-shared"
            }
            Self::MainLockReleaseNativeUncertainReserved => {
                "main-lock-release-native-uncertain-reserved"
            }
            Self::MainLockReleaseAfterKnown => "main-lock-release-after-known",
            Self::MainFileCloseBefore => "main-file-close-before",
            Self::MainFileCloseNativeRetryable => "main-file-close-native-retryable",
            Self::MainFileCloseNativeUncertain => "main-file-close-native-uncertain",
            Self::MainFileCloseAfterKnown => "main-file-close-after-known",
            Self::PhysicalSuccess => "physical-success",
            Self::RegistryWalMainCloseBefore => "registry-wal-main-close-before",
            Self::RegistryWalMainCloseNativeUncertain => "registry-wal-main-close-native-uncertain",
            Self::RegistryWalMainCloseAfterKnown => "registry-wal-main-close-after-known",
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

wire_enum!(JointClosePath { JointClose = 0 });
wire_enum!(JointCloseTopology { FinalConnection = 0 });
wire_enum!(JointCloseMode { Keep = 0 });
wire_enum!(JointCloseNode { Live = 0 });
wire_enum!(JointCloseMainLockPrestate {
    NotApplicable = 0,
    Shared = 1,
    ReservedShared = 2,
});
wire_enum!(JointCloseMainLockOffsetClass {
    NotApplicable = 0,
    SharedRange = 1,
    ReservedByte = 2,
});
wire_enum!(JointClosePhase {
    RawStateTake = 0,
    BeginConnectionClose = 1,
    CallbackAdmission = 2,
    ShmUnmapLift = 3,
    MainLockRelease = 4,
    MainFileClose = 5,
    RegistryWalMainClose = 6,
    Success = 7,
});
wire_enum!(JointCloseCause {
    None = 0,
    ViewUnmap = 1,
    MappingClose = 2,
    DmsSharedRelease = 3,
    ShmFileClose = 4,
    ConnectionDetach = 5,
});
wire_enum!(JointCloseTiming {
    Validation = 0,
    BeforeCall = 1,
    NativeRetryable = 2,
    NativeUncertain = 3,
    AfterSuccessKnown = 4,
    AfterSuccessUncertain = 5,
    Success = 6,
});
wire_enum!(JointCloseFailureClass {
    None = 0,
    ProtocolViolation = 1,
    IoBeforeMutation = 2,
    MutatedButKnown = 3,
    OutcomeUncertainPoisoned = 4,
    RegistryRejected = 5,
});
wire_enum!(JointCloseTargetScope { RouteMain = 0 });
wire_enum!(JointCloseRole { Main = 0 });
wire_enum!(JointCloseCallback { Close = 0 });
wire_enum!(JointCloseSqliteOutcome { Ok = 0, IoerrClose = 1 });
wire_enum!(JointCloseRegistryRoutePhase {
    Active = 0,
    Closing = 1,
    TerminalQuarantine = 2,
});
wire_enum!(JointCloseLogicalRoutePhase { Indexed = 0, Retained = 1 });
wire_enum!(JointCloseRegistrationPhase { Registered = 0 });
wire_enum!(JointCloseDmsCustody {
    Absent = 0,
    Shared = 1,
    Released = 2,
    OutcomeUncertain = 3,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct JointCloseActualTarget {
    pub(in super::super::super) scope: JointCloseTargetScope,
    pub(in super::super::super) registration_id: u64,
    pub(in super::super::super) route_ordinal: u64,
    pub(in super::super::super) runtime_generation: u64,
    pub(in super::super::super) shm_connection_id: u64,
    pub(in super::super::super) role: JointCloseRole,
    pub(in super::super::super) callback: JointCloseCallback,
    pub(in super::super::super) occurrence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct JointCloseActualIdentity {
    pub(in super::super::super) path: JointClosePath,
    pub(in super::super::super) topology: JointCloseTopology,
    pub(in super::super::super) mode: JointCloseMode,
    pub(in super::super::super) node: JointCloseNode,
    pub(in super::super::super) variant: u8,
    pub(in super::super::super) pre_shared_mask: u8,
    pub(in super::super::super) pre_exclusive_mask: u8,
    pub(in super::super::super) main_lock_prestate: JointCloseMainLockPrestate,
    pub(in super::super::super) main_lock_offset_class: JointCloseMainLockOffsetClass,
    pub(in super::super::super) phase: JointClosePhase,
    pub(in super::super::super) cause: JointCloseCause,
    pub(in super::super::super) timing: JointCloseTiming,
    pub(in super::super::super) class: JointCloseFailureClass,
    pub(in super::super::super) target: JointCloseActualTarget,
    pub(in super::super::super) sqlite_outcome: JointCloseSqliteOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct JointCloseActualTopology {
    pub(in super::super::super) sqlite_connections: u8,
    pub(in super::super::super) shm_connections: u8,
    pub(in super::super::super) registry_routes: u8,
    pub(in super::super::super) logical_names: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct JointCloseActualCustody {
    pub(in super::super::super) node: bool,
    pub(in super::super::super) views: u8,
    pub(in super::super::super) mappings: u8,
    pub(in super::super::super) dms: JointCloseDmsCustody,
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
pub(in super::super::super) struct JointCloseActualCounts {
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
pub(in super::super::super) struct JointCloseActual {
    pub(in super::super::super) selector: JointCloseSelector,
    pub(in super::super::super) identity: JointCloseActualIdentity,
    pub(in super::super::super) mutation_may_have_occurred: bool,
    pub(in super::super::super) lock_outcome_uncertain: bool,
    pub(in super::super::super) domain_terminal: bool,
    pub(in super::super::super) registry_route_phase: JointCloseRegistryRoutePhase,
    pub(in super::super::super) logical_route_phase: JointCloseLogicalRoutePhase,
    pub(in super::super::super) registration_phase: JointCloseRegistrationPhase,
    pub(in super::super::super) later_callback_allowed: bool,
    pub(in super::super::super) pre: JointCloseActualTopology,
    pub(in super::super::super) post: JointCloseActualTopology,
    pub(in super::super::super) retained: JointCloseActualCustody,
    pub(in super::super::super) counts: JointCloseActualCounts,
}
