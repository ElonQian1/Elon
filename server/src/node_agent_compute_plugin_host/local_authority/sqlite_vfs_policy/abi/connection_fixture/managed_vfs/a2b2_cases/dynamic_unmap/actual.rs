#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super::super) enum UnmapSelector {
    SharedDeleteRequestValidation,
    SharedKeepCallbackAdmission,
    SharedKeepCallbackWrapperBefore,
    SharedKeepHeldSharedLock,
    SharedKeepHeldExclusiveLock,
    SharedKeepDetachBefore,
    SharedKeepDetachAfterKnown,
    SharedKeepDetachAfterUncertain,
    SharedKeepCompletionNativeUncertain,
    SharedKeepSuccess,
    SharedDeleteSuccess,
    FinalKeepViewUnmapBefore,
    FinalKeepViewUnmapNativeUncertain,
    FinalKeepViewUnmapAfterKnown,
    FinalKeepViewUnmapAfterUncertain,
    FinalKeepMappingCloseBefore,
    FinalKeepMappingCloseNativeUncertain,
    FinalKeepMappingCloseAfterKnown,
    FinalKeepMappingCloseAfterUncertain,
    FinalKeepDmsReleaseBefore,
    FinalKeepDmsReleaseNativeUncertain,
    FinalKeepDmsReleaseAfterKnown,
    FinalKeepDmsReleaseAfterUncertain,
    FinalKeepFileCloseBefore,
    FinalKeepFileCloseNativeRetryable,
    FinalKeepFileCloseNativeUncertain,
    FinalKeepFileCloseAfterKnown,
    FinalKeepFileCloseAfterUncertain,
    FinalKeepDetachBefore,
    FinalKeepDetachAfterKnown,
    FinalKeepDetachAfterUncertain,
    FinalKeepCompletionNativeUncertain,
    FinalKeepSuccessLiveNode,
    FinalKeepSuccessNodeAbsent,
    FinalDeleteAuthMainIdentityMissing,
    FinalDeleteAuthMainOrGenerationMismatch,
    FinalDeleteAuthMainNotExclusive,
    FinalDeleteAuthLockStateUncertain,
    FinalDeleteSiblingBefore,
    FinalDeleteSiblingNativeRetryable,
    FinalDeleteSiblingNativeUncertain,
    FinalDeleteSiblingAfterKnown,
    FinalDeleteSiblingAfterUncertain,
    FinalDeleteDetachBefore,
    FinalDeleteDetachAfterKnown,
    FinalDeleteDetachAfterUncertain,
    FinalDeleteCompletionNativeUncertain,
    FinalDeleteSuccessDeleted,
    FinalDeleteSuccessNotFound,
}

impl UnmapSelector {
    pub(in super::super::super) const ALL: [Self; 49] = [
        Self::SharedDeleteRequestValidation,
        Self::SharedKeepCallbackAdmission,
        Self::SharedKeepCallbackWrapperBefore,
        Self::SharedKeepHeldSharedLock,
        Self::SharedKeepHeldExclusiveLock,
        Self::SharedKeepDetachBefore,
        Self::SharedKeepDetachAfterKnown,
        Self::SharedKeepDetachAfterUncertain,
        Self::SharedKeepCompletionNativeUncertain,
        Self::SharedKeepSuccess,
        Self::SharedDeleteSuccess,
        Self::FinalKeepViewUnmapBefore,
        Self::FinalKeepViewUnmapNativeUncertain,
        Self::FinalKeepViewUnmapAfterKnown,
        Self::FinalKeepViewUnmapAfterUncertain,
        Self::FinalKeepMappingCloseBefore,
        Self::FinalKeepMappingCloseNativeUncertain,
        Self::FinalKeepMappingCloseAfterKnown,
        Self::FinalKeepMappingCloseAfterUncertain,
        Self::FinalKeepDmsReleaseBefore,
        Self::FinalKeepDmsReleaseNativeUncertain,
        Self::FinalKeepDmsReleaseAfterKnown,
        Self::FinalKeepDmsReleaseAfterUncertain,
        Self::FinalKeepFileCloseBefore,
        Self::FinalKeepFileCloseNativeRetryable,
        Self::FinalKeepFileCloseNativeUncertain,
        Self::FinalKeepFileCloseAfterKnown,
        Self::FinalKeepFileCloseAfterUncertain,
        Self::FinalKeepDetachBefore,
        Self::FinalKeepDetachAfterKnown,
        Self::FinalKeepDetachAfterUncertain,
        Self::FinalKeepCompletionNativeUncertain,
        Self::FinalKeepSuccessLiveNode,
        Self::FinalKeepSuccessNodeAbsent,
        Self::FinalDeleteAuthMainIdentityMissing,
        Self::FinalDeleteAuthMainOrGenerationMismatch,
        Self::FinalDeleteAuthMainNotExclusive,
        Self::FinalDeleteAuthLockStateUncertain,
        Self::FinalDeleteSiblingBefore,
        Self::FinalDeleteSiblingNativeRetryable,
        Self::FinalDeleteSiblingNativeUncertain,
        Self::FinalDeleteSiblingAfterKnown,
        Self::FinalDeleteSiblingAfterUncertain,
        Self::FinalDeleteDetachBefore,
        Self::FinalDeleteDetachAfterKnown,
        Self::FinalDeleteDetachAfterUncertain,
        Self::FinalDeleteCompletionNativeUncertain,
        Self::FinalDeleteSuccessDeleted,
        Self::FinalDeleteSuccessNotFound,
    ];

    pub(in super::super::super) const fn report_name(self) -> &'static str {
        use UnmapSelector as S;
        match self {
            S::SharedDeleteRequestValidation => "shared-delete-request-validation",
            S::SharedKeepCallbackAdmission => "shared-keep-callback-admission",
            S::SharedKeepCallbackWrapperBefore => "shared-keep-callback-wrapper-before",
            S::SharedKeepHeldSharedLock => "shared-keep-held-shared-lock",
            S::SharedKeepHeldExclusiveLock => "shared-keep-held-exclusive-lock",
            S::SharedKeepDetachBefore => "shared-keep-detach-before",
            S::SharedKeepDetachAfterKnown => "shared-keep-detach-after-known",
            S::SharedKeepDetachAfterUncertain => "shared-keep-detach-after-uncertain",
            S::SharedKeepCompletionNativeUncertain => "shared-keep-completion-native-uncertain",
            S::SharedKeepSuccess => "shared-keep-success",
            S::SharedDeleteSuccess => "shared-delete-success",
            S::FinalKeepViewUnmapBefore => "final-keep-view-unmap-before",
            S::FinalKeepViewUnmapNativeUncertain => "final-keep-view-unmap-native-uncertain",
            S::FinalKeepViewUnmapAfterKnown => "final-keep-view-unmap-after-known",
            S::FinalKeepViewUnmapAfterUncertain => "final-keep-view-unmap-after-uncertain",
            S::FinalKeepMappingCloseBefore => "final-keep-mapping-close-before",
            S::FinalKeepMappingCloseNativeUncertain => "final-keep-mapping-close-native-uncertain",
            S::FinalKeepMappingCloseAfterKnown => "final-keep-mapping-close-after-known",
            S::FinalKeepMappingCloseAfterUncertain => "final-keep-mapping-close-after-uncertain",
            S::FinalKeepDmsReleaseBefore => "final-keep-dms-release-before",
            S::FinalKeepDmsReleaseNativeUncertain => "final-keep-dms-release-native-uncertain",
            S::FinalKeepDmsReleaseAfterKnown => "final-keep-dms-release-after-known",
            S::FinalKeepDmsReleaseAfterUncertain => "final-keep-dms-release-after-uncertain",
            S::FinalKeepFileCloseBefore => "final-keep-file-close-before",
            S::FinalKeepFileCloseNativeRetryable => "final-keep-file-close-native-retryable",
            S::FinalKeepFileCloseNativeUncertain => "final-keep-file-close-native-uncertain",
            S::FinalKeepFileCloseAfterKnown => "final-keep-file-close-after-known",
            S::FinalKeepFileCloseAfterUncertain => "final-keep-file-close-after-uncertain",
            S::FinalKeepDetachBefore => "final-keep-detach-before",
            S::FinalKeepDetachAfterKnown => "final-keep-detach-after-known",
            S::FinalKeepDetachAfterUncertain => "final-keep-detach-after-uncertain",
            S::FinalKeepCompletionNativeUncertain => "final-keep-completion-native-uncertain",
            S::FinalKeepSuccessLiveNode => "final-keep-success-live-node",
            S::FinalKeepSuccessNodeAbsent => "final-keep-success-node-absent",
            S::FinalDeleteAuthMainIdentityMissing => "final-delete-auth-main-identity-missing",
            S::FinalDeleteAuthMainOrGenerationMismatch => {
                "final-delete-auth-main-or-generation-mismatch"
            }
            S::FinalDeleteAuthMainNotExclusive => "final-delete-auth-main-not-exclusive",
            S::FinalDeleteAuthLockStateUncertain => "final-delete-auth-lock-state-uncertain",
            S::FinalDeleteSiblingBefore => "final-delete-sibling-before",
            S::FinalDeleteSiblingNativeRetryable => "final-delete-sibling-native-retryable",
            S::FinalDeleteSiblingNativeUncertain => "final-delete-sibling-native-uncertain",
            S::FinalDeleteSiblingAfterKnown => "final-delete-sibling-after-known",
            S::FinalDeleteSiblingAfterUncertain => "final-delete-sibling-after-uncertain",
            S::FinalDeleteDetachBefore => "final-delete-detach-before",
            S::FinalDeleteDetachAfterKnown => "final-delete-detach-after-known",
            S::FinalDeleteDetachAfterUncertain => "final-delete-detach-after-uncertain",
            S::FinalDeleteCompletionNativeUncertain => "final-delete-completion-native-uncertain",
            S::FinalDeleteSuccessDeleted => "final-delete-success-deleted",
            S::FinalDeleteSuccessNotFound => "final-delete-success-not-found",
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

wire_enum!(UnmapPath { Unmap = 0 });
wire_enum!(UnmapTopology {
    SharedNonFinal = 0,
    FinalConnection = 1,
});
wire_enum!(UnmapMode { Keep = 0, Delete = 1 });
wire_enum!(UnmapNode { Live = 0, Absent = 1 });
wire_enum!(UnmapPhase {
    RequestValidation = 0,
    CallbackAdmission = 1,
    HeldLockGate = 2,
    ConnectionDetach = 3,
    ViewUnmap = 4,
    MappingClose = 5,
    DmsSharedRelease = 6,
    ShmFileClose = 7,
    DeleteAuthorization = 8,
    ExactSiblingDelete = 9,
    CallbackCompletion = 10,
    Success = 11,
});
wire_enum!(UnmapCause { None = 0 });
wire_enum!(UnmapTiming {
    Validation = 0,
    BeforeCall = 1,
    NativeRetryable = 2,
    NativeUncertain = 3,
    AfterSuccessKnown = 4,
    AfterSuccessUncertain = 5,
    Success = 6,
});
wire_enum!(UnmapFailureClass {
    None = 0,
    ProtocolViolation = 1,
    IoBeforeMutation = 2,
    MutatedButKnown = 3,
    OutcomeUncertainPoisoned = 4,
    RegistryRejected = 5,
});
wire_enum!(UnmapTargetScope { RouteMain = 0 });
wire_enum!(UnmapRole { Main = 0 });
wire_enum!(UnmapCallback { Shm = 0 });
wire_enum!(UnmapSqliteOutcome { Ok = 0, Ioerr = 1 });
wire_enum!(UnmapRegistryRoutePhase {
    Active = 0,
    TerminalQuarantine = 1,
});
wire_enum!(UnmapLogicalRoutePhase { Indexed = 0, Retained = 1 });
wire_enum!(UnmapRegistrationPhase { Registered = 0 });
wire_enum!(UnmapDmsCustody {
    Absent = 0,
    Shared = 1,
    Released = 2,
    OutcomeUncertain = 3,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct UnmapActualTarget {
    pub(in super::super::super) scope: UnmapTargetScope,
    pub(in super::super::super) registration_id: u64,
    pub(in super::super::super) route_ordinal: u64,
    pub(in super::super::super) runtime_generation: u64,
    pub(in super::super::super) shm_connection_id: u64,
    pub(in super::super::super) role: UnmapRole,
    pub(in super::super::super) callback: UnmapCallback,
    pub(in super::super::super) occurrence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct UnmapActualIdentity {
    pub(in super::super::super) path: UnmapPath,
    pub(in super::super::super) topology: UnmapTopology,
    pub(in super::super::super) mode: UnmapMode,
    pub(in super::super::super) node: UnmapNode,
    pub(in super::super::super) variant: u8,
    pub(in super::super::super) pre_shared_mask: u8,
    pub(in super::super::super) pre_exclusive_mask: u8,
    pub(in super::super::super) phase: UnmapPhase,
    pub(in super::super::super) cause: UnmapCause,
    pub(in super::super::super) timing: UnmapTiming,
    pub(in super::super::super) class: UnmapFailureClass,
    pub(in super::super::super) target: UnmapActualTarget,
    pub(in super::super::super) sqlite_outcome: UnmapSqliteOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct UnmapActualTopology {
    pub(in super::super::super) sqlite_connections: u8,
    pub(in super::super::super) shm_connections: u8,
    pub(in super::super::super) registry_routes: u8,
    pub(in super::super::super) logical_names: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct UnmapActualCustody {
    pub(in super::super::super) node: bool,
    pub(in super::super::super) views: u8,
    pub(in super::super::super) mappings: u8,
    pub(in super::super::super) dms: UnmapDmsCustody,
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
pub(in super::super::super) struct UnmapActualCounts {
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
pub(in super::super::super) struct UnmapActual {
    pub(in super::super::super) selector: UnmapSelector,
    pub(in super::super::super) identity: UnmapActualIdentity,
    pub(in super::super::super) mutation_may_have_occurred: bool,
    pub(in super::super::super) lock_outcome_uncertain: bool,
    pub(in super::super::super) domain_terminal: bool,
    pub(in super::super::super) registry_route_phase: UnmapRegistryRoutePhase,
    pub(in super::super::super) logical_route_phase: UnmapLogicalRoutePhase,
    pub(in super::super::super) registration_phase: UnmapRegistrationPhase,
    pub(in super::super::super) later_callback_allowed: bool,
    pub(in super::super::super) pre: UnmapActualTopology,
    pub(in super::super::super) post: UnmapActualTopology,
    pub(in super::super::super) retained: UnmapActualCustody,
    pub(in super::super::super) counts: UnmapActualCounts,
}
