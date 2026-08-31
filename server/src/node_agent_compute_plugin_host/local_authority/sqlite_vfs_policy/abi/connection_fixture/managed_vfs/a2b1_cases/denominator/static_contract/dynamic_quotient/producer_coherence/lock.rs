use super::super::super::terminal_descriptor::{
    CallbackV1, CleanupV1, FaultSeamV1, FixtureV1, LockActionV1, LockAxesV1, LockCompletionV1,
    LockManagedStimulusV1, LockOperationV1, LockPrestateV1, LockTerminalDescriptorV1, ObserverV1,
    PhaseV1, PrestateV1, RawStateV1, ReachabilityV1, SourceSiteV1, StimulusV1, TimingV1,
    ValidityV1,
};
use super::super::projector::{ProjectionErrorV1, ProjectionViolationV1};
use super::{
    invalid, valid_initialization_tuple, valid_lock_capability, valid_stored_poison_phase,
};

pub(super) fn validate(value: LockTerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    validate_recipe(value)?;
    super::lock_axes::validate(value)?;
    if !valid_tuple(value) {
        return Err(invalid(ProjectionViolationV1::LockProducerTupleMismatch));
    }
    if !valid_completion(value) {
        return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch));
    }
    Ok(())
}

fn validate_recipe(value: LockTerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    let PrestateV1::Lock(prestate) = value.prestate else {
        return Err(invalid(ProjectionViolationV1::LockProducerTupleMismatch));
    };
    let ReachabilityV1::Reached(completion) = value.axes.completion else {
        return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch));
    };
    let expected_fixture = if matches!(value.axes.action, ReachabilityV1::NotReached) {
        FixtureV1::AbiRawOnly
    } else if matches!(
        prestate,
        LockPrestateV1::SiblingExclusiveContention
            | LockPrestateV1::SiblingAnyContention
            | LockPrestateV1::SiblingSharedCoalesced
    ) {
        FixtureV1::ManagedWalMainTwoConnections
    } else {
        FixtureV1::ManagedWalMainSingleConnection
    };
    let observer = if value.source_site == SourceSiteV1::RawStateAbandon {
        ObserverV1::CustodyAndCleanup
    } else {
        ObserverV1::LockCallbackAndSnapshot
    };
    let cleanup = match completion {
        LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown
        | LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown => {
            CleanupV1::RetainUnsafeCustodyThenParentCleanup
        }
        LockCompletionV1::Direct
        | LockCompletionV1::Completed
        | LockCompletionV1::RouteUnknown
        | LockCompletionV1::RawDropCompleted
        | LockCompletionV1::RawDropUnwindCaught => CleanupV1::ParentOwnedRoot,
    };
    if value.recipe.fixture == expected_fixture
        && value.recipe.callback == CallbackV1::XShmLock
        && value.recipe.observer == observer
        && value.recipe.cleanup == cleanup
        && valid_lock_capability(value.recipe)
    {
        Ok(())
    } else {
        Err(invalid(ProjectionViolationV1::LockProducerRecipeMismatch))
    }
}

fn valid_tuple(value: LockTerminalDescriptorV1) -> bool {
    let PrestateV1::Lock(prestate) = value.prestate else {
        return false;
    };
    match value.stimulus {
        StimulusV1::LockAbi(scalar) => {
            value.source_site == SourceSiteV1::LockAbiBoundary
                && prestate == LockPrestateV1::NotReached
                && value.operation == LockOperationV1::AbiValidation
                && value.phase == PhaseV1::AbiValidation
                && value.timing == TimingV1::BeforeCall
                && value.recipe.fault_seam == FaultSeamV1::AbiBoundary
                && early_axes(value.axes)
                && (scalar.offset == ValidityV1::Invalid
                    || scalar.count == ValidityV1::Invalid
                    || scalar.flags == ValidityV1::Invalid)
        }
        StimulusV1::LockRaw(raw) => valid_raw_tuple(value, prestate, raw),
        StimulusV1::Initialization(stimulus) => {
            prestate == LockPrestateV1::NoHeldLocks
                && value.operation == LockOperationV1::Initialization
                && value.recipe.fault_seam == FaultSeamV1::Initialization
                && acquire_action(value.axes.action)
                && managed_range(value.axes)
                && valid_initialization_tuple(
                    value.source_site,
                    stimulus,
                    value.phase,
                    value.timing,
                )
        }
        StimulusV1::LockManaged(stimulus) => valid_managed_tuple(value, prestate, stimulus),
        StimulusV1::MapAbi(_) | StimulusV1::MapRaw(_) | StimulusV1::MapManaged(_) => false,
    }
}

fn valid_raw_tuple(
    value: LockTerminalDescriptorV1,
    prestate: LockPrestateV1,
    raw: RawStateV1,
) -> bool {
    let common = prestate == LockPrestateV1::NotReached
        && value.recipe.fault_seam == FaultSeamV1::RawState
        && early_axes(value.axes);
    common
        && ((raw == RawStateV1::HandleBoundFileMissing
            && value.source_site == SourceSiteV1::AdapterDispatch
            && value.operation == LockOperationV1::AdapterDispatch
            && value.phase == PhaseV1::Adapter
            && value.timing == TimingV1::BeforeCall)
            || (raw != RawStateV1::HandleBoundFileMissing
                && !matches!(
                    raw,
                    RawStateV1::DropCompleted | RawStateV1::DropUnwindCaught
                )
                && value.source_site == SourceSiteV1::RawStateAbandon
                && value.operation == LockOperationV1::RawAbandon
                && value.phase == PhaseV1::RawAdmission
                && value.timing == TimingV1::Cleanup))
}

fn valid_managed_tuple(
    value: LockTerminalDescriptorV1,
    prestate: LockPrestateV1,
    stimulus: LockManagedStimulusV1,
) -> bool {
    use LockManagedStimulusV1 as S;
    match stimulus {
        S::RangeOverflow | S::EndPastEight | S::SharedMultiSlot => {
            let shared_multi = stimulus != S::SharedMultiSlot
                || matches!(
                    value.axes.action,
                    ReachabilityV1::Reached(LockActionV1::LockShared | LockActionV1::UnlockShared)
                );
            value.source_site == SourceSiteV1::ManagedRequestValidation
                && prestate == LockPrestateV1::NotReached
                && value.operation == LockOperationV1::ManagedRequest
                && value.phase == PhaseV1::RequestValidation
                && value.timing == TimingV1::BeforeCall
                && value.recipe.fault_seam == FaultSeamV1::ManagedRequest
                && request_axes(value.axes)
                && shared_multi
        }
        S::AdmissionRouteUnknown | S::AdmissionCounterOverflow => managed_exact(
            value,
            prestate,
            SourceSiteV1::RegistryCallbackAdmission,
            LockPrestateV1::NotReached,
            LockOperationV1::CallbackAdmission,
            PhaseV1::CallbackAdmission,
            TimingV1::BeforeCall,
            FaultSeamV1::RegistryAdmission,
        ),
        S::UnsupportedFileRole | S::ShmDetached => managed_exact(
            value,
            prestate,
            SourceSiteV1::AdapterDispatch,
            LockPrestateV1::NotReached,
            LockOperationV1::CallbackAdmission,
            PhaseV1::CallbackAdmission,
            TimingV1::BeforeCall,
            FaultSeamV1::RegistryAdmission,
        ),
        S::StoredPoison => {
            value.source_site == SourceSiteV1::CoordinatorState
                && prestate == LockPrestateV1::StoredPoison
                && value.operation == LockOperationV1::Quarantine
                && valid_stored_poison_phase(value.phase)
                && value.timing == TimingV1::BeforeCall
                && value.recipe.fault_seam == FaultSeamV1::Natural
                && managed_range(value.axes)
        }
        S::LocalState => valid_local_tuple(value, prestate),
        S::NativeAcquire => {
            value.source_site == SourceSiteV1::LockNativeAcquire
                && prestate == LockPrestateV1::NoHeldLocks
                && value.operation == LockOperationV1::NativeAcquire
                && acquire_action(value.axes.action)
                && managed_range(value.axes)
                && value.recipe.fault_seam == FaultSeamV1::NativeOperation
                && ((value.phase == PhaseV1::LockAcquire && value.timing == TimingV1::AtCall)
                    || (value.phase == PhaseV1::Success && value.timing == TimingV1::AfterSuccess))
        }
        S::NativeRelease => {
            value.source_site == SourceSiteV1::LockNativeRelease
                && value.operation == LockOperationV1::NativeRelease
                && release_action(value.axes.action)
                && release_prestate(value.axes.action, prestate)
                && managed_range(value.axes)
                && value.recipe.fault_seam == FaultSeamV1::NativeOperation
                && ((value.phase == PhaseV1::LockRelease && value.timing == TimingV1::AtCall)
                    || (value.phase == PhaseV1::Success && value.timing == TimingV1::AfterSuccess))
        }
        S::Callback | S::Initialization | S::Success => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn managed_exact(
    value: LockTerminalDescriptorV1,
    prestate: LockPrestateV1,
    source: SourceSiteV1,
    expected_prestate: LockPrestateV1,
    operation: LockOperationV1,
    phase: PhaseV1,
    timing: TimingV1,
    seam: FaultSeamV1,
) -> bool {
    value.source_site == source
        && prestate == expected_prestate
        && value.operation == operation
        && value.phase == phase
        && value.timing == timing
        && value.recipe.fault_seam == seam
        && managed_range(value.axes)
}

fn valid_local_tuple(value: LockTerminalDescriptorV1, prestate: LockPrestateV1) -> bool {
    if value.source_site != SourceSiteV1::LockLocalState
        || value.timing != TimingV1::Natural
        || value.recipe.fault_seam != FaultSeamV1::Natural
        || !managed_range(value.axes)
    {
        return false;
    }
    let action = value.axes.action;
    match (value.operation, prestate, value.phase) {
        (LockOperationV1::LocalAcquire, LockPrestateV1::OwnOverlap, PhaseV1::RequestValidation) => {
            acquire_action(action)
        }
        (
            LockOperationV1::LocalAcquire,
            LockPrestateV1::SiblingExclusiveContention,
            PhaseV1::LockAcquire,
        ) => action == ReachabilityV1::Reached(LockActionV1::LockShared),
        (
            LockOperationV1::LocalAcquire,
            LockPrestateV1::SiblingAnyContention,
            PhaseV1::LockAcquire,
        ) => action == ReachabilityV1::Reached(LockActionV1::LockExclusive),
        (
            LockOperationV1::LocalAcquire,
            LockPrestateV1::SiblingSharedCoalesced,
            PhaseV1::Success,
        ) => action == ReachabilityV1::Reached(LockActionV1::LockShared),
        (
            LockOperationV1::LocalRelease,
            LockPrestateV1::NoHeldLocks,
            PhaseV1::RequestValidation,
        ) => release_action(action),
        (
            LockOperationV1::LocalRelease,
            LockPrestateV1::SiblingSharedCoalesced,
            PhaseV1::Success,
        ) => action == ReachabilityV1::Reached(LockActionV1::UnlockShared),
        (
            LockOperationV1::LocalRelease,
            LockPrestateV1::ExclusiveRangeMismatch,
            PhaseV1::RequestValidation,
        ) => action == ReachabilityV1::Reached(LockActionV1::UnlockExclusive),
        _ => false,
    }
}

fn early_axes(axes: LockAxesV1) -> bool {
    matches!(axes.action, ReachabilityV1::NotReached)
        && matches!(axes.first, ReachabilityV1::NotReached)
        && matches!(axes.count, ReachabilityV1::NotReached)
        && matches!(axes.mask, ReachabilityV1::NotReached)
}

fn request_axes(axes: LockAxesV1) -> bool {
    matches!(axes.action, ReachabilityV1::Reached(_))
        && matches!(axes.first, ReachabilityV1::NotReached)
        && matches!(axes.count, ReachabilityV1::NotReached)
        && matches!(axes.mask, ReachabilityV1::NotReached)
}

fn managed_range(axes: LockAxesV1) -> bool {
    matches!(axes.action, ReachabilityV1::Reached(_))
        && matches!(axes.first, ReachabilityV1::Reached(_))
        && matches!(axes.count, ReachabilityV1::Reached(_))
        && matches!(axes.mask, ReachabilityV1::Reached(_))
}

fn acquire_action(action: ReachabilityV1<LockActionV1>) -> bool {
    matches!(
        action,
        ReachabilityV1::Reached(LockActionV1::LockShared | LockActionV1::LockExclusive)
    )
}

fn release_action(action: ReachabilityV1<LockActionV1>) -> bool {
    matches!(
        action,
        ReachabilityV1::Reached(LockActionV1::UnlockShared | LockActionV1::UnlockExclusive)
    )
}

fn release_prestate(action: ReachabilityV1<LockActionV1>, prestate: LockPrestateV1) -> bool {
    matches!(
        (action, prestate),
        (
            ReachabilityV1::Reached(LockActionV1::UnlockShared),
            LockPrestateV1::OwnSharedHeld
        ) | (
            ReachabilityV1::Reached(LockActionV1::UnlockExclusive),
            LockPrestateV1::OwnExclusiveHeld
        )
    )
}

fn valid_completion(value: LockTerminalDescriptorV1) -> bool {
    let ReachabilityV1::Reached(completion) = value.axes.completion else {
        return false;
    };
    let safe = matches!(
        completion,
        LockCompletionV1::Completed | LockCompletionV1::RouteUnknown
    );
    let unsafe_path = matches!(
        completion,
        LockCompletionV1::UnsafeRetentionSucceededThenRouteUnknown
            | LockCompletionV1::UnsafeRetentionRouteUnknownThenRouteUnknown
    );
    match value.stimulus {
        StimulusV1::LockAbi(_) => completion == LockCompletionV1::Direct,
        StimulusV1::LockRaw(raw) => match raw {
            RawStateV1::NullFile
            | RawStateV1::Uninstalled
            | RawStateV1::MethodsNullStatePresent
            | RawStateV1::ForeignMethodsStateNull
            | RawStateV1::ForeignMethodsStatePresent
            | RawStateV1::ExactMethodsStateNull
            | RawStateV1::HandleBoundFileMissing => completion == LockCompletionV1::Direct,
            RawStateV1::OtherTypePayloadMissing | RawStateV1::ExpectedTypePayloadMissing => {
                completion == LockCompletionV1::RawDropCompleted
            }
            RawStateV1::OtherTypePayloadPresent => matches!(
                completion,
                LockCompletionV1::RawDropCompleted | LockCompletionV1::RawDropUnwindCaught
            ),
            RawStateV1::DropCompleted | RawStateV1::DropUnwindCaught => false,
        },
        StimulusV1::Initialization(stimulus) => {
            let may_be_safe = !stimulus.cleanup_rewrite && matches!(
                (stimulus.fault_site, stimulus.path),
                (super::super::super::terminal_descriptor::InitializationFaultSiteV1::DmsExclusiveAcquire,
                    super::super::super::terminal_descriptor::InitializationPathV1::Existing)
                    | (super::super::super::terminal_descriptor::InitializationFaultSiteV1::DmsSharedAcquire,
                        super::super::super::terminal_descriptor::InitializationPathV1::ExistingJoiner));
            unsafe_path || (may_be_safe && safe)
        }
        StimulusV1::LockManaged(stimulus) => match stimulus {
            LockManagedStimulusV1::RangeOverflow
            | LockManagedStimulusV1::EndPastEight
            | LockManagedStimulusV1::SharedMultiSlot
            | LockManagedStimulusV1::AdmissionRouteUnknown
            | LockManagedStimulusV1::AdmissionCounterOverflow => {
                completion == LockCompletionV1::Direct
            }
            LockManagedStimulusV1::UnsupportedFileRole
            | LockManagedStimulusV1::ShmDetached
            | LockManagedStimulusV1::LocalState => safe,
            LockManagedStimulusV1::StoredPoison => unsafe_path,
            LockManagedStimulusV1::NativeAcquire => {
                if value.phase == PhaseV1::Success {
                    safe
                } else {
                    safe || unsafe_path
                }
            }
            LockManagedStimulusV1::NativeRelease => {
                if value.phase == PhaseV1::Success {
                    safe
                } else {
                    unsafe_path
                }
            }
            LockManagedStimulusV1::Callback
            | LockManagedStimulusV1::Initialization
            | LockManagedStimulusV1::Success => false,
        },
        StimulusV1::MapAbi(_) | StimulusV1::MapRaw(_) | StimulusV1::MapManaged(_) => false,
    }
}
