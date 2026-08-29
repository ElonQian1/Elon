use super::super::model::{SourceNodeId, SourceOwnerId};
use super::model::{
    source_anchor, MapBoundaryReviewStatus, MapOpenReviewBoundary, MapPendingBoundaryRecord,
    MapResolvedBoundaryRecord, MapReviewGate, MapReviewGateRecord, MapSourceStepId,
};

/// The review ledger stops at repository-owned typed `Result` seams for platform file I/O,
/// mapping and cleanup. It does not quotient OS error values or claim dynamic equivalence.
pub(super) const DEEPEST_TYPED_BOUNDARY: &str =
    "repository-owned managed-fs typed Result and explicit Windows SHM outcome seams";

pub(super) const REVIEW_GATES: &[MapReviewGateRecord] = &[
    MapReviewGateRecord {
        gate: MapReviewGate::AbiInputShapeSplit,
        witnesses: &[
            MapSourceStepId::AbiNullFirst,
            MapSourceStepId::AbiInputRejected,
            MapSourceStepId::AbiNullOutputRejected,
            MapSourceStepId::AbiRawDispatch,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::AbiPointerPremise,
        witnesses: &[MapSourceStepId::AbiNullFirst],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::RawRejectionVsPanicSplit,
        witnesses: &[
            MapSourceStepId::RawStateAccepted,
            MapSourceStepId::RawStateNullFile,
            MapSourceStepId::RawStateUninstalled,
            MapSourceStepId::RawStateForeignMethodsNullTable,
            MapSourceStepId::RawStateForeignMethodsForeignTable,
            MapSourceStepId::RawStateMissing,
            MapSourceStepId::RawStateTypeMismatch,
            MapSourceStepId::RawStateCaughtPanic,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::RawAbandonOutcomeSplit,
        witnesses: &[
            MapSourceStepId::RawAbandonEmpty,
            MapSourceStepId::RawAbandonStateWitnessRecorded,
            MapSourceStepId::RawAbandonInstalled,
            MapSourceStepId::RawAbandonNullFileRejected,
            MapSourceStepId::RawAbandonForeignMethodsNullTableRejected,
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
            MapSourceStepId::RawAbandonStateMissingRejected,
            MapSourceStepId::RawFallbackProjection,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::RouteAndPromotionExactFixtureExclusion,
        witnesses: &[
            MapSourceStepId::RoutePlanClaimRejected,
            MapSourceStepId::PromotionClaimShmRejected,
            MapSourceStepId::FaultInstallRejected,
            MapSourceStepId::FaultProbeRecordRejected,
            MapSourceStepId::FaultObserverRecordRejected,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::TypedPlatformOutcomeExpansion,
        witnesses: &[
            MapSourceStepId::ExactOpenNativeFailure,
            MapSourceStepId::DmsExclusiveNativeFailure,
            MapSourceStepId::DmsSharedNativeFailure,
            MapSourceStepId::AllocationGranularityFailure,
            MapSourceStepId::MappingCreateNativeFailure,
            MapSourceStepId::ViewMapNativeCleanupOk,
            MapSourceStepId::ViewMapNativeCleanupFailed,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::PrefixMutationAndInitializationCrossProduct,
        witnesses: &[
            MapSourceStepId::FileSizeFaultBefore,
            MapSourceStepId::FileSizeNativeFailure,
            MapSourceStepId::ExistingSizeBudgetRejected,
            MapSourceStepId::MappingCreateFaultBefore,
            MapSourceStepId::MappingCreateNativeFailure,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::SymbolicRegionLoopAndFaultOccurrence,
        witnesses: &[
            MapSourceStepId::RegionLoopContinues,
            MapSourceStepId::MappingArithmeticRejected,
            MapSourceStepId::MappingCreateFaultAfterKnown,
            MapSourceStepId::ViewMapFaultAfterKnown,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::CallbackAndCustodyProjectionClosure,
        witnesses: &[
            MapSourceStepId::PromotionCompletionRejected,
            MapSourceStepId::OperationUnsafeRetain,
            MapSourceStepId::OperationCompletionRejected,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::DynamicTerminalRewriteObservation,
        witnesses: &[
            MapSourceStepId::ExactOpenCloseRewrite,
            MapSourceStepId::DmsTruncateFaultBeforeReleaseFailed,
            MapSourceStepId::ViewMapFaultBeforeCleanupFailed,
            MapSourceStepId::ViewMapNativeCleanupFailed,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::PlatformCfgAndControllerInternals,
        witnesses: &[
            MapSourceStepId::AllocationGranularityFailure,
            MapSourceStepId::RegionOffsetOverflowRejected,
            MapSourceStepId::ViewMapFaultBeforeUncertain,
        ],
    },
    MapReviewGateRecord {
        gate: MapReviewGate::ManagedDefensiveLeafExpansion,
        witnesses: &[
            MapSourceStepId::NodeMissingAfterOpen,
            MapSourceStepId::RegionCustodyMissing,
            MapSourceStepId::LogicalEndOverflowRejected,
            MapSourceStepId::ViewMapNullCustodyRetained,
            MapSourceStepId::ViewMapNullPoisoned,
        ],
    },
];

/// Exact source families that this review ledger deliberately has not expanded into per-leaf
/// terminal records. Their presence is a fail-closed boundary, not an exclusion proof.
pub(super) const OPEN_SOURCE_REVIEW_BOUNDARIES: &[MapOpenReviewBoundary] = &[
    MapOpenReviewBoundary {
        gate: MapReviewGate::AbiPointerPremise,
        anchors: &[
            source_anchor(
                SourceOwnerId::AbiBoundary,
                "A non-null output pointer must be valid and aligned for one pointer write.",
                "A non-null output pointer must be valid and aligned for one pointer write.",
                1,
            ),
            source_anchor(
                SourceOwnerId::AbiIoShm,
                "unsafe extern \"C\" fn map",
                "if output.is_null()",
                1,
            ),
            source_anchor(
                SourceOwnerId::AbiRawState,
                "unsafe fn with_installed_state",
                "installed_envelope(file)?",
                1,
            ),
            source_anchor(
                SourceOwnerId::AbiRawState,
                "unsafe fn installed_envelope",
                "NonNull::new(file.cast::<InertHandleBoundSqliteFile>())",
                1,
            ),
        ],
        note: "output must be callback-owned/non-alias/aligned/writable/live; non-null file must be live/aligned/initialized/serialized and exact methods plus state must identify this module's live envelope; forged, dangling or wrong-layout pointers are UB premises, not finite leaves",
    },
    MapOpenReviewBoundary {
        gate: MapReviewGate::PlatformCfgAndControllerInternals,
        anchors: &[
            source_anchor(
                SourceOwnerId::ManagedFaultOperation,
                "fn begin_test_fault",
                "Err(failure) =>",
                1,
            ),
            source_anchor(
                SourceOwnerId::ManagedFaultOperation,
                "fn begin_test_fault",
                "Err(failure) =>",
                2,
            ),
            source_anchor(
                SourceOwnerId::ManagedFaultApi,
                "fn trigger_after_test_fault",
                "test_fault_internal_failure",
                1,
            ),
            source_anchor(
                SourceOwnerId::ManagedFaultApi,
                "fn trigger_after_test_fault",
                "test_fault_internal_failure",
                2,
            ),
            source_anchor(
                SourceOwnerId::ManagedFaultOperation,
                "fn activate_after_test_fault",
                "Ok(failure) | Err(failure) => failure",
                1,
            ),
        ],
        note: "controller-internal errors remain a phase-by-prefix cross-product Pending review",
    },
    MapOpenReviewBoundary {
        gate: MapReviewGate::PlatformCfgAndControllerInternals,
        anchors: &[
            source_anchor(
                SourceOwnerId::ManagedShmRoot,
                "#[cfg(windows)]",
                "#[path = \"windows_sqlite_shm.rs\"]",
                1,
            ),
            source_anchor(
                SourceOwnerId::ManagedShmRoot,
                "fn allocation_granularity",
                "io::ErrorKind::Unsupported",
                1,
            ),
        ],
        note: "cross-platform typed Result source is reviewed; Windows-only reachability and non-Windows exclusions are unresolved",
    },
    MapOpenReviewBoundary {
        gate: MapReviewGate::PrefixMutationAndInitializationCrossProduct,
        anchors: &[
            source_anchor(
                SourceOwnerId::ManagedInitialization,
                "fn open_node",
                "return Err(self.close_failed_open_file(state, file, failure));",
                1,
            ),
            source_anchor(
                SourceOwnerId::ManagedInitialization,
                "fn open_node",
                "NODE_MANAGED_SQLITE_SHM_DMS_BUSY",
                1,
            ),
            source_anchor(
                SourceOwnerId::ManagedInitialization,
                "fn open_node",
                "ManagedSqliteShmFailurePhase::DmsSharedAcquire",
                3,
            ),
        ],
        note: "DMS acquire native-error and shared-busy cleanup helpers remain unsplit by caller and close outcome",
    },
    MapOpenReviewBoundary {
        gate: MapReviewGate::ManagedDefensiveLeafExpansion,
        anchors: &[
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_SIZE", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_GROW", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_MAP", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_DURING_MAP", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_BUDGET", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_MAPPING_CREATE", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AT_MAP_FAILURE", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AT_NULL_VIEW", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AFTER_VIEW_MAP", 1),
            source_anchor(SourceOwnerId::ManagedMapping, "fn map_connection", "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AFTER_MAP", 1),
        ],
        note: "managed node-presence leaves require per-leaf invariant or exclusion proofs before terminal freeze",
    },
];

pub(super) const PENDING_BOUNDARIES: &[MapPendingBoundaryRecord] = &[
    MapPendingBoundaryRecord {
        node: SourceNodeId::ManagedDmsInitialization,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::ExactOpenFaultBefore,
            MapSourceStepId::DmsExclusiveContended,
            MapSourceStepId::SharedDmsInitialized,
        ],
    },
    MapPendingBoundaryRecord {
        node: SourceNodeId::ManagedMapCoordinator,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::RegionSizeChanged,
            MapSourceStepId::FileSizeNativeFailure,
            MapSourceStepId::ManagedMapped,
        ],
    },
    MapPendingBoundaryRecord {
        node: SourceNodeId::ManagedRegionLoop,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::RegionLoopContinues,
            MapSourceStepId::RegionReuseCandidate,
        ],
    },
    MapPendingBoundaryRecord {
        node: SourceNodeId::ManagedInlineRegionCustody,
        status: MapBoundaryReviewStatus::AnchoredButGraphPending,
        witnesses: &[
            MapSourceStepId::MappingCreateFaultAfterKnown,
            MapSourceStepId::ViewMapNativeCleanupFailed,
            MapSourceStepId::ViewMapFaultAfterKnown,
        ],
    },
    MapPendingBoundaryRecord {
        node: SourceNodeId::WalMainColdNodeWitness,
        status: MapBoundaryReviewStatus::CrossLedgerStateWitnessPending,
        witnesses: &[
            MapSourceStepId::RegionSizeBudgetRejected,
            MapSourceStepId::RegionCountBudgetRejected,
            MapSourceStepId::LogicalEndBudgetRejected,
            MapSourceStepId::AllocationGranularityFailure,
        ],
    },
];

pub(super) const RESOLVED_GRAPH_BOUNDARIES: &[MapResolvedBoundaryRecord] = &[
    MapResolvedBoundaryRecord {
        node: SourceNodeId::AbiMapValidation,
        witnesses: &[
            MapSourceStepId::AbiInputRejected,
            MapSourceStepId::AbiNullOutputRejected,
        ],
    },
    MapResolvedBoundaryRecord {
        node: SourceNodeId::AbiMapRawGate,
        witnesses: &[
            MapSourceStepId::RawStateAccepted,
            MapSourceStepId::RawStateNullFile,
            MapSourceStepId::RawStateUninstalled,
            MapSourceStepId::RawStateForeignMethodsNullTable,
            MapSourceStepId::RawStateForeignMethodsForeignTable,
            MapSourceStepId::RawStateMissing,
            MapSourceStepId::RawStateTypeMismatch,
            MapSourceStepId::RawStateCaughtPanic,
        ],
    },
    MapResolvedBoundaryRecord {
        node: SourceNodeId::AbiMapRawStateAbandon,
        witnesses: &[
            MapSourceStepId::RawAbandonEmpty,
            MapSourceStepId::RawAbandonStateWitnessRecorded,
            MapSourceStepId::RawAbandonInstalled,
            MapSourceStepId::RawAbandonNullFileRejected,
            MapSourceStepId::RawAbandonForeignMethodsNullTableRejected,
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
            MapSourceStepId::RawAbandonStateMissingRejected,
        ],
    },
    MapResolvedBoundaryRecord {
        node: SourceNodeId::ManagedRegionSizeValidation,
        witnesses: &[MapSourceStepId::RegionSizeBudgetRejected],
    },
    MapResolvedBoundaryRecord {
        node: SourceNodeId::ManagedLogicalEndValidation,
        witnesses: &[
            MapSourceStepId::RegionCountBudgetRejected,
            MapSourceStepId::LogicalEndOverflowRejected,
            MapSourceStepId::LogicalEndBudgetRejected,
        ],
    },
    MapResolvedBoundaryRecord {
        node: SourceNodeId::ManagedExistingSizeValidation,
        witnesses: &[MapSourceStepId::ExistingSizeBudgetRejected],
    },
    MapResolvedBoundaryRecord {
        node: SourceNodeId::ManagedMappedTotalValidation,
        witnesses: &[MapSourceStepId::MappingBudgetRejected],
    },
    MapResolvedBoundaryRecord {
        node: SourceNodeId::ManagedFileSize,
        witnesses: &[
            MapSourceStepId::FileSizeFaultBefore,
            MapSourceStepId::FileSizeAfterSelectorRejected,
            MapSourceStepId::FileSizeNativeFailure,
            MapSourceStepId::ObserveNotPresent,
        ],
    },
    MapResolvedBoundaryRecord {
        node: SourceNodeId::ManagedFileGrow,
        witnesses: &[
            MapSourceStepId::FileGrowFaultBefore,
            MapSourceStepId::FileGrowNativeFailure,
            MapSourceStepId::FileGrowFaultAfterKnown,
            MapSourceStepId::FileGrowFaultAfterUncertain,
        ],
    },
];
