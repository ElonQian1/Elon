use super::super::terminal_descriptor::*;

macro_rules! tags {
    ($name:ident, $ty:ty, $($variant:path => $value:expr),+ $(,)?) => {
        pub(super) fn $name(value: $ty) -> u16 { match value { $($variant => $value),+ } }
    };
}

tags!(presence_tag, PresenceV1, PresenceV1::Absent => 1, PresenceV1::Present => 2);
tags!(validity_tag, ValidityV1, ValidityV1::Invalid => 1, ValidityV1::Valid => 2);
tags!(raw_state_tag, RawStateV1,
    RawStateV1::NullFile => 1, RawStateV1::Uninstalled => 2,
    RawStateV1::MethodsNullStatePresent => 3, RawStateV1::ForeignMethodsStateNull => 4,
    RawStateV1::ForeignMethodsStatePresent => 5, RawStateV1::ExactMethodsStateNull => 6,
    RawStateV1::OtherTypePayloadMissing => 7, RawStateV1::OtherTypePayloadPresent => 8,
    RawStateV1::ExpectedTypePayloadMissing => 9, RawStateV1::HandleBoundFileMissing => 10,
    RawStateV1::DropCompleted => 11, RawStateV1::DropUnwindCaught => 12);
tags!(map_stimulus_tag, MapManagedStimulusV1,
    MapManagedStimulusV1::CallbackRouteUnknownPriorQuarantine => 1,
    MapManagedStimulusV1::CallbackCounterOverflow => 2,
    MapManagedStimulusV1::CallbackUnsupportedFileRole => 3,
    MapManagedStimulusV1::CallbackShmDetached => 4,
    MapManagedStimulusV1::RegionSizeBudget => 5, MapManagedStimulusV1::RegionCountBudget => 6,
    MapManagedStimulusV1::LogicalSizeBudget => 7, MapManagedStimulusV1::AllocationGranularity => 8,
    MapManagedStimulusV1::StoredPoison => 9, MapManagedStimulusV1::Initialization => 10,
    MapManagedStimulusV1::RegionSize => 11, MapManagedStimulusV1::FileSize => 12,
    MapManagedStimulusV1::FileGrow => 13, MapManagedStimulusV1::MappingCreate => 14,
    MapManagedStimulusV1::ViewMap => 15, MapManagedStimulusV1::MappingClose => 16,
    MapManagedStimulusV1::RegionLoop => 17, MapManagedStimulusV1::Success => 18);
tags!(lock_stimulus_tag, LockManagedStimulusV1,
    LockManagedStimulusV1::Callback => 1, LockManagedStimulusV1::AdmissionRouteUnknown => 2,
    LockManagedStimulusV1::AdmissionCounterOverflow => 3, LockManagedStimulusV1::RangeOverflow => 4,
    LockManagedStimulusV1::EndPastEight => 5, LockManagedStimulusV1::SharedMultiSlot => 6,
    LockManagedStimulusV1::UnsupportedFileRole => 7, LockManagedStimulusV1::ShmDetached => 8,
    LockManagedStimulusV1::StoredPoison => 9, LockManagedStimulusV1::Initialization => 10,
    LockManagedStimulusV1::LocalState => 11, LockManagedStimulusV1::NativeAcquire => 12,
    LockManagedStimulusV1::NativeRelease => 13, LockManagedStimulusV1::Success => 14);
tags!(lock_prestate_tag, LockPrestateV1,
    LockPrestateV1::NotReached => 0, LockPrestateV1::NoHeldLocks => 1,
    LockPrestateV1::OwnOverlap => 2, LockPrestateV1::OwnSharedHeld => 3,
    LockPrestateV1::OwnExclusiveHeld => 4, LockPrestateV1::ExclusiveRangeMismatch => 5,
    LockPrestateV1::SiblingExclusiveContention => 6, LockPrestateV1::SiblingAnyContention => 7,
    LockPrestateV1::SiblingSharedCoalesced => 8, LockPrestateV1::StoredPoison => 9);
tags!(initialization_fault_tag, InitializationFaultSiteV1,
    InitializationFaultSiteV1::ParentValidationBeforeOpen => 1,
    InitializationFaultSiteV1::ParentHandle => 2, InitializationFaultSiteV1::PlatformOpen => 3,
    InitializationFaultSiteV1::OpenCompletionValidation => 4,
    InitializationFaultSiteV1::OpenFileValidation => 5,
    InitializationFaultSiteV1::ParentValidationAfterOpen => 6,
    InitializationFaultSiteV1::DmsExclusiveAcquire => 7, InitializationFaultSiteV1::DmsTruncate => 8,
    InitializationFaultSiteV1::DmsExclusiveRelease => 9,
    InitializationFaultSiteV1::DmsSharedAcquire => 10);
tags!(initialization_path_tag, InitializationPathV1,
    InitializationPathV1::NotReached => 0, InitializationPathV1::Opening => 1,
    InitializationPathV1::Created => 2, InitializationPathV1::Existing => 3,
    InitializationPathV1::CreatedFirst => 4, InitializationPathV1::ExistingFirst => 5,
    InitializationPathV1::CreatedJoiner => 6, InitializationPathV1::ExistingJoiner => 7);
tags!(map_stored_poison_tag, MapStoredPoisonPrestateV1,
    MapStoredPoisonPrestateV1::NoNode => 1, MapStoredPoisonPrestateV1::LiveNodeRegionsEmpty => 2,
    MapStoredPoisonPrestateV1::LiveNodeCompleteRegions => 3,
    MapStoredPoisonPrestateV1::NodeAbsentFileQuarantinedNoRegions => 4,
    MapStoredPoisonPrestateV1::NodeAbsentFileQuarantinedRegionsReleased => 5,
    MapStoredPoisonPrestateV1::NodeAbsentFileReleasedNoRegions => 6,
    MapStoredPoisonPrestateV1::NodeAbsentFileAndRegionsReleased => 7,
    MapStoredPoisonPrestateV1::LiveNodeMappingOnlyViewNotCreated => 8,
    MapStoredPoisonPrestateV1::LiveNodeMappingOnlyViewReleased => 9,
    MapStoredPoisonPrestateV1::LiveNodeMappingOnlyWithRetainedView => 10,
    MapStoredPoisonPrestateV1::LiveNodeViewUnmapPartialRetained => 11,
    MapStoredPoisonPrestateV1::LiveNodeRegionsReleased => 12);
tags!(map_mode_tag, MapModeV1, MapModeV1::Observe => 1, MapModeV1::Extend => 2);
tags!(initialization_tag, InitializationProfileV1,
    InitializationProfileV1::NodeLive => 1, InitializationProfileV1::CreatedFirstShared => 2,
    InitializationProfileV1::CreatedJoinerShared => 3,
    InitializationProfileV1::ExistingFirstShared => 4,
    InitializationProfileV1::ExistingJoinerShared => 5);
tags!(region_prestate_tag, MapRegionPrestateV1,
    MapRegionPrestateV1::Empty => 1, MapRegionPrestateV1::NonemptyTargetMissing => 2,
    MapRegionPrestateV1::Reuse => 3, MapRegionPrestateV1::ObserveNotPresent => 4);
tags!(region_size_tag, MapRegionSizeArmV1,
    MapRegionSizeArmV1::NotReached => 0, MapRegionSizeArmV1::Same => 1,
    MapRegionSizeArmV1::UnsetAssigned => 2, MapRegionSizeArmV1::Changed => 3);
tags!(file_path_tag, MapFilePathV1,
    MapFilePathV1::NotReached => 0, MapFilePathV1::SizeSufficient => 1,
    MapFilePathV1::GrowAttempted => 2, MapFilePathV1::GrowSucceeded => 3,
    MapFilePathV1::ObserveNotPresent => 4);
tags!(lock_action_tag, LockActionV1,
    LockActionV1::LockShared => 1, LockActionV1::LockExclusive => 2,
    LockActionV1::UnlockShared => 3, LockActionV1::UnlockExclusive => 4);
tags!(map_operation_tag, MapOperationV1,
    MapOperationV1::AbiValidation => 1, MapOperationV1::RawAdmission => 2,
    MapOperationV1::RawAbandon => 3, MapOperationV1::AdapterDispatch => 4,
    MapOperationV1::CallbackAdmission => 5, MapOperationV1::ManagedRequest => 6,
    MapOperationV1::Initialization => 7, MapOperationV1::FileSize => 8,
    MapOperationV1::FileGrow => 9, MapOperationV1::MappingCreate => 10,
    MapOperationV1::ViewMap => 11, MapOperationV1::MappingClose => 12,
    MapOperationV1::SuccessProjection => 13, MapOperationV1::CallbackCompletion => 14,
    MapOperationV1::Quarantine => 15, MapOperationV1::AbiProjection => 16);
tags!(lock_operation_tag, LockOperationV1,
    LockOperationV1::AbiValidation => 1, LockOperationV1::RawAdmission => 2,
    LockOperationV1::RawAbandon => 3, LockOperationV1::AdapterDispatch => 4,
    LockOperationV1::CallbackAdmission => 5, LockOperationV1::ManagedRequest => 6,
    LockOperationV1::Initialization => 7, LockOperationV1::LocalAcquire => 8,
    LockOperationV1::LocalRelease => 9, LockOperationV1::NativeAcquire => 10,
    LockOperationV1::NativeRelease => 11, LockOperationV1::CallbackCompletion => 12,
    LockOperationV1::Quarantine => 13, LockOperationV1::AbiProjection => 14);
tags!(fixture_tag, FixtureV1,
    FixtureV1::NotReached => 0, FixtureV1::AbiRawOnly => 1,
    FixtureV1::ManagedWalMainSingleConnection => 2, FixtureV1::ManagedWalMainTwoConnections => 3);
tags!(callback_tag, CallbackV1,
    CallbackV1::NotReached => 0, CallbackV1::XShmMap => 1, CallbackV1::XShmLock => 2);
tags!(fault_seam_tag, FaultSeamV1,
    FaultSeamV1::NotReached => 0, FaultSeamV1::Natural => 1,
    FaultSeamV1::AbiBoundary => 2, FaultSeamV1::RawState => 3,
    FaultSeamV1::RegistryAdmission => 4, FaultSeamV1::ManagedRequest => 5,
    FaultSeamV1::Initialization => 6, FaultSeamV1::NativeOperation => 7,
    FaultSeamV1::CallbackCompletion => 8, FaultSeamV1::Cleanup => 9);
tags!(observer_tag, ObserverV1,
    ObserverV1::NotReached => 0, ObserverV1::MapCallbackAndSnapshot => 1,
    ObserverV1::LockCallbackAndSnapshot => 2, ObserverV1::CustodyAndCleanup => 3);
tags!(cleanup_tag, CleanupV1,
    CleanupV1::NotReached => 0, CleanupV1::ParentOwnedRoot => 1,
    CleanupV1::RetainUnsafeCustodyThenParentCleanup => 2);
tags!(gap_tag, CapabilityGapV1,
    CapabilityGapV1::QuotientRunnerNotIntegrated => 1,
    CapabilityGapV1::CallbackAfterSuccessUnavailable => 2,
    CapabilityGapV1::LockObservationIncomplete => 3,
    CapabilityGapV1::TerminalRecipeMissing => 4);
tags!(map_completion_tag, MapCompletionV1,
    MapCompletionV1::Direct => 1, MapCompletionV1::Completed => 2,
    MapCompletionV1::RouteUnknown => 3,
    MapCompletionV1::UnsafeRetentionSucceededThenRouteUnknown => 4,
    MapCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown => 5,
    MapCompletionV1::RawDropCompleted => 6, MapCompletionV1::RawDropUnwindCaught => 7);
tags!(lock_completion_tag, LockCompletionV1,
    LockCompletionV1::Direct => 1, LockCompletionV1::Completed => 2,
    LockCompletionV1::RouteUnknown => 3,
    LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown => 4,
    LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown => 5,
    LockCompletionV1::RawDropCompleted => 6, LockCompletionV1::RawDropUnwindCaught => 7);
