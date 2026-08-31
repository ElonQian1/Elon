use super::super::{
    source_leaf_authority::{
        digest_case_key, digest_full_record, LeafOutcomeV1, LeafRecordV1, RootOperationV1,
    },
    terminal_descriptor::{
        CallbackV1, CapabilityGapV1, ExecutionRecipeV1, FixtureV1, LockAxesV1, LockCompletionV1,
        LockManagedStimulusV1, LockOperationV1, LockPrestateV1, LockTerminalDescriptorV1,
        MapAxesV1, MapPrestateV1, MapTerminalDescriptorV1, ObserverV1, OccurrenceV1, PhaseV1,
        PrestateV1, ReachabilityV1, RunnerCapabilityV1, SourceSiteV1, StimulusV1,
        TerminalDescriptorV1, TimingV1,
    },
};

use super::{
    canonical::{digest_dynamic_class_key_v1, digest_normalized_descriptor_semantics_v1},
    descriptor_binding::{DescriptorBindingEntryV1, ValidatedDynamicTerminalV1},
    model::{
        DynamicAxesV1, DynamicClassKeyV1, DynamicExpectedV1, DynamicOperationV1,
        DynamicProjectionV1, StaticMemberSealV1, DYNAMIC_PROJECTOR_SCHEMA_V1,
    },
    producer_coherence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionViolationV1 {
    StimulusRoot,
    PrestateRoot,
    SourceSiteRoot,
    CallbackRoot,
    ObserverRoot,
    FixtureRoot,
    RecipeNotExecutable,
    PartialMapProfile,
    MapProfileModeMismatch,
    MapProfilePrestateMismatch,
    MapCompletionNotReached,
    PartialMapLoopAxes,
    MapOrdinalWithoutProfile,
    MapOrdinalRegionsMismatch,
    MapLoopOrdinalOutOfRange,
    MapProducerTupleMismatch,
    MapProducerAxesMismatch,
    MapProducerRecipeMismatch,
    LockCompletionNotReached,
    PartialLockRange,
    LockRequestRejectionDescriptorMismatch,
    InvalidLockRange,
    LockMaskMismatch,
    PartialLockPrestate,
    LockProducerTupleMismatch,
    LockProducerAxesMismatch,
    LockProducerRecipeMismatch,
    TimingOccurrenceMismatch,
    ZeroOccurrence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionErrorV1 {
    ExcludedRecord,
    RootMismatch {
        record: RootOperationV1,
        descriptor: RootOperationV1,
    },
    StaticPhaseMismatch {
        typed: PhaseV1,
    },
    RunnerCapabilityMissing(CapabilityGapV1),
    Invalid(ProjectionViolationV1),
}

pub(crate) fn project_dynamic_class_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Result<DynamicProjectionV1, ProjectionErrorV1> {
    project_validated_dynamic_terminal_v1(record, descriptor)?
        .projection
        .map_err(ProjectionErrorV1::RunnerCapabilityMissing)
}

pub(super) fn project_validated_dynamic_terminal_v1(
    record: &LeafRecordV1,
    descriptor: &TerminalDescriptorV1,
) -> Result<ValidatedDynamicTerminalV1, ProjectionErrorV1> {
    let static_expected = match &record.outcome {
        LeafOutcomeV1::Terminal(expected) => expected,
        LeafOutcomeV1::Excluded(_) => return Err(ProjectionErrorV1::ExcludedRecord),
    };
    let record_root = record.key.identity.root;
    let (root, source_site, stimulus, prestate, operation, phase, timing, occurrence, recipe, axes) =
        match descriptor {
            TerminalDescriptorV1::Map(value) => project_map_descriptor(*value)?,
            TerminalDescriptorV1::Lock(value) => project_lock_descriptor(*value)?,
        };
    if record_root != root {
        return Err(ProjectionErrorV1::RootMismatch {
            record: record_root,
            descriptor: root,
        });
    }
    if static_expected.phase != phase.static_name() {
        return Err(ProjectionErrorV1::StaticPhaseMismatch { typed: phase });
    }
    validate_timing_occurrence(timing, occurrence)?;
    validate_recipe_shape(root, recipe)?;
    producer_coherence::validate(descriptor)?;

    let expected = DynamicExpectedV1 {
        sqlite: static_expected.sqlite,
        disposition: static_expected.disposition,
        phase,
        failure: static_expected.failure,
        mutation: static_expected.mutation,
        lock_outcome_uncertain: static_expected.lock_outcome_uncertain,
        lock_effect: static_expected.lock_effect,
        dms_lock: static_expected.dms_lock,
        raw_slots: static_expected.raw_slots,
        route: static_expected.route,
        callback: static_expected.callback,
        file: static_expected.file,
        mapping: static_expected.mapping,
        view: static_expected.view,
        payload: static_expected.payload,
        counts: static_expected.counts,
    };
    let key = DynamicClassKeyV1 {
        schema_version: DYNAMIC_PROJECTOR_SCHEMA_V1,
        root,
        source_site,
        stimulus,
        prestate,
        operation,
        phase,
        timing,
        occurrence,
        recipe,
        axes,
        expected,
    };
    let member = StaticMemberSealV1 {
        case_key_sha256: digest_case_key(&record.key),
        full_record_sha256: digest_full_record(record),
    };
    let descriptor_binding = DescriptorBindingEntryV1 {
        member,
        descriptor_semantic_sha256: digest_normalized_descriptor_semantics_v1(&key),
    };
    let projection = match recipe.capability {
        RunnerCapabilityV1::Supported => Ok(DynamicProjectionV1 {
            class_key_sha256: digest_dynamic_class_key_v1(&key),
            member,
            key,
        }),
        RunnerCapabilityV1::Missing(gap) => Err(gap),
    };
    Ok(ValidatedDynamicTerminalV1 {
        descriptor_binding,
        semantic_key: key,
        projection,
    })
}

#[allow(clippy::type_complexity)]
fn project_map_descriptor(
    value: MapTerminalDescriptorV1,
) -> Result<
    (
        RootOperationV1,
        super::super::terminal_descriptor::SourceSiteV1,
        StimulusV1,
        PrestateV1,
        DynamicOperationV1,
        PhaseV1,
        TimingV1,
        OccurrenceV1,
        ExecutionRecipeV1,
        DynamicAxesV1,
    ),
    ProjectionErrorV1,
> {
    if !matches!(
        value.stimulus,
        StimulusV1::MapAbi(_)
            | StimulusV1::MapRaw(_)
            | StimulusV1::MapManaged(_)
            | StimulusV1::Initialization(_)
    ) {
        return Err(ProjectionViolationV1::StimulusRoot.into());
    }
    if !matches!(value.prestate, PrestateV1::Map(_)) {
        return Err(ProjectionViolationV1::PrestateRoot.into());
    }
    validate_source_site(RootOperationV1::Map, value.source_site)?;
    validate_map_axes(value)?;
    Ok((
        RootOperationV1::Map,
        value.source_site,
        value.stimulus,
        value.prestate,
        DynamicOperationV1::Map(value.operation),
        value.phase,
        value.timing,
        value.occurrence,
        value.recipe,
        DynamicAxesV1::Map(value.axes),
    ))
}

#[allow(clippy::type_complexity)]
fn project_lock_descriptor(
    value: LockTerminalDescriptorV1,
) -> Result<
    (
        RootOperationV1,
        super::super::terminal_descriptor::SourceSiteV1,
        StimulusV1,
        PrestateV1,
        DynamicOperationV1,
        PhaseV1,
        TimingV1,
        OccurrenceV1,
        ExecutionRecipeV1,
        DynamicAxesV1,
    ),
    ProjectionErrorV1,
> {
    if !matches!(
        value.stimulus,
        StimulusV1::LockAbi(_)
            | StimulusV1::LockRaw(_)
            | StimulusV1::LockManaged(_)
            | StimulusV1::Initialization(_)
    ) {
        return Err(ProjectionViolationV1::StimulusRoot.into());
    }
    if !matches!(value.prestate, PrestateV1::Lock(_)) {
        return Err(ProjectionViolationV1::PrestateRoot.into());
    }
    validate_source_site(RootOperationV1::Lock, value.source_site)?;
    validate_lock_axes(value)?;
    Ok((
        RootOperationV1::Lock,
        value.source_site,
        value.stimulus,
        value.prestate,
        DynamicOperationV1::Lock(value.operation),
        value.phase,
        value.timing,
        value.occurrence,
        value.recipe,
        DynamicAxesV1::Lock(value.axes),
    ))
}

fn validate_source_site(
    root: RootOperationV1,
    source_site: SourceSiteV1,
) -> Result<(), ProjectionErrorV1> {
    let allowed = match root {
        RootOperationV1::Map => !matches!(
            source_site,
            SourceSiteV1::LockAbiBoundary
                | SourceSiteV1::LockLocalState
                | SourceSiteV1::LockNativeAcquire
                | SourceSiteV1::LockNativeRelease
        ),
        RootOperationV1::Lock => !matches!(
            source_site,
            SourceSiteV1::MapAbiBoundary
                | SourceSiteV1::MapFileSize
                | SourceSiteV1::MapFileGrow
                | SourceSiteV1::MapMappingCreate
                | SourceSiteV1::MapViewMap
                | SourceSiteV1::MapMappingClose
        ),
    };
    if allowed {
        Ok(())
    } else {
        Err(ProjectionViolationV1::SourceSiteRoot.into())
    }
}

fn validate_map_axes(value: MapTerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    let axes = value.axes;
    if matches!(axes.completion, ReachabilityV1::NotReached) {
        return Err(ProjectionViolationV1::MapCompletionNotReached.into());
    }
    match (axes.mode, axes.profile) {
        (ReachabilityV1::NotReached, ReachabilityV1::Reached(_)) => {
            return Err(ProjectionViolationV1::PartialMapProfile.into())
        }
        (ReachabilityV1::Reached(mode), ReachabilityV1::Reached(profile))
            if mode != profile.mode =>
        {
            return Err(ProjectionViolationV1::MapProfileModeMismatch.into())
        }
        _ => {}
    }
    if let ReachabilityV1::Reached(profile) = axes.profile {
        let expected = match profile.prestate {
            super::super::terminal_descriptor::MapRegionPrestateV1::Empty => {
                Some((MapPrestateV1::RegionsEmpty, false))
            }
            super::super::terminal_descriptor::MapRegionPrestateV1::NonemptyTargetMissing => {
                Some((MapPrestateV1::TargetMissing, true))
            }
            super::super::terminal_descriptor::MapRegionPrestateV1::Reuse => {
                Some((MapPrestateV1::TargetMapped, true))
            }
            super::super::terminal_descriptor::MapRegionPrestateV1::ObserveNotPresent => None,
        };
        if expected
            != Some((
                match value.prestate {
                    PrestateV1::Map(prestate) => prestate,
                    PrestateV1::Lock(_) => return Err(ProjectionViolationV1::PrestateRoot.into()),
                },
                profile.preexisting_mapping,
            ))
        {
            return Err(ProjectionViolationV1::MapProfilePrestateMismatch.into());
        }
    }
    match (axes.ordinal, axes.regions_to_create) {
        (ReachabilityV1::NotReached, ReachabilityV1::NotReached) => {}
        (ReachabilityV1::Reached(_), ReachabilityV1::NotReached)
        | (ReachabilityV1::NotReached, ReachabilityV1::Reached(_)) => {
            return Err(ProjectionViolationV1::PartialMapLoopAxes.into())
        }
        (ReachabilityV1::Reached(ordinal), ReachabilityV1::Reached(regions)) => {
            if matches!(axes.profile, ReachabilityV1::NotReached) {
                return Err(ProjectionViolationV1::MapOrdinalWithoutProfile.into());
            }
            if ordinal != regions {
                return Err(ProjectionViolationV1::MapOrdinalRegionsMismatch.into());
            }
            if !(1..=256).contains(&ordinal) {
                return Err(ProjectionViolationV1::MapLoopOrdinalOutOfRange.into());
            }
        }
    }
    Ok(())
}

fn validate_lock_axes(value: LockTerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    let axes = value.axes;
    if matches!(axes.completion, ReachabilityV1::NotReached) {
        return Err(ProjectionViolationV1::LockCompletionNotReached.into());
    }
    match (axes.action, axes.first, axes.count, axes.mask) {
        (
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
        ) => {}
        (
            ReachabilityV1::Reached(_),
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
        ) => validate_lock_request_rejection(value)?,
        (
            ReachabilityV1::Reached(_),
            ReachabilityV1::Reached(first),
            ReachabilityV1::Reached(count),
            ReachabilityV1::Reached(mask),
        ) => {
            let Some(end) = first.checked_add(count) else {
                return Err(ProjectionViolationV1::InvalidLockRange.into());
            };
            if count == 0 || end > 8 {
                return Err(ProjectionViolationV1::InvalidLockRange.into());
            }
            let expected = ((1_u16 << end) - (1_u16 << first)) as u8;
            if mask != expected {
                return Err(ProjectionViolationV1::LockMaskMismatch.into());
            }
        }
        _ => return Err(ProjectionViolationV1::PartialLockRange.into()),
    }
    let held = [
        axes.held_shared_mask,
        axes.held_exclusive_mask,
        axes.sibling_shared_mask,
        axes.sibling_exclusive_mask,
    ];
    let reached = held
        .iter()
        .filter(|value| matches!(value, ReachabilityV1::Reached(_)))
        .count();
    if reached != 0 && reached != held.len() {
        return Err(ProjectionViolationV1::PartialLockPrestate.into());
    }
    Ok(())
}

fn validate_lock_request_rejection(
    value: LockTerminalDescriptorV1,
) -> Result<(), ProjectionErrorV1> {
    let axes = value.axes;
    let exact_request_rejection = matches!(
        value.stimulus,
        StimulusV1::LockManaged(
            LockManagedStimulusV1::RangeOverflow
                | LockManagedStimulusV1::EndPastEight
                | LockManagedStimulusV1::SharedMultiSlot
        )
    ) && value.source_site == SourceSiteV1::ManagedRequestValidation
        && value.prestate == PrestateV1::Lock(LockPrestateV1::NotReached)
        && value.operation == LockOperationV1::ManagedRequest
        && value.phase == PhaseV1::RequestValidation
        && value.timing == TimingV1::BeforeCall
        && value.occurrence == OccurrenceV1::Natural
        && matches!(axes.initialization, ReachabilityV1::NotReached)
        && matches!(axes.held_shared_mask, ReachabilityV1::NotReached)
        && matches!(axes.held_exclusive_mask, ReachabilityV1::NotReached)
        && matches!(axes.sibling_shared_mask, ReachabilityV1::NotReached)
        && matches!(axes.sibling_exclusive_mask, ReachabilityV1::NotReached)
        && axes.completion == ReachabilityV1::Reached(LockCompletionV1::Direct);
    if exact_request_rejection {
        Ok(())
    } else {
        Err(ProjectionViolationV1::LockRequestRejectionDescriptorMismatch.into())
    }
}

fn validate_timing_occurrence(
    timing: TimingV1,
    occurrence: OccurrenceV1,
) -> Result<(), ProjectionErrorV1> {
    if matches!(occurrence, OccurrenceV1::Exact(0)) {
        return Err(ProjectionViolationV1::ZeroOccurrence.into());
    }
    if matches!(timing, TimingV1::NotReached) != matches!(occurrence, OccurrenceV1::NotReached) {
        Err(ProjectionViolationV1::TimingOccurrenceMismatch.into())
    } else {
        Ok(())
    }
}

fn validate_recipe_shape(
    root: RootOperationV1,
    recipe: ExecutionRecipeV1,
) -> Result<(), ProjectionErrorV1> {
    if matches!(recipe.fixture, FixtureV1::NotReached)
        || matches!(recipe.callback, CallbackV1::NotReached)
        || matches!(
            recipe.fault_seam,
            super::super::terminal_descriptor::FaultSeamV1::NotReached
        )
        || matches!(
            recipe.observer,
            super::super::terminal_descriptor::ObserverV1::NotReached
        )
        || matches!(
            recipe.cleanup,
            super::super::terminal_descriptor::CleanupV1::NotReached
        )
    {
        return Err(ProjectionViolationV1::RecipeNotExecutable.into());
    }
    match (root, recipe.callback) {
        (RootOperationV1::Map, CallbackV1::XShmMap)
        | (RootOperationV1::Lock, CallbackV1::XShmLock) => {}
        _ => return Err(ProjectionViolationV1::CallbackRoot.into()),
    }
    match (root, recipe.fixture) {
        (_, FixtureV1::AbiRawOnly)
        | (RootOperationV1::Map, FixtureV1::ManagedWalMainSingleConnection)
        | (RootOperationV1::Lock, FixtureV1::ManagedWalMainSingleConnection)
        | (RootOperationV1::Lock, FixtureV1::ManagedWalMainTwoConnections) => {}
        _ => return Err(ProjectionViolationV1::FixtureRoot.into()),
    }
    match (root, recipe.observer) {
        (
            RootOperationV1::Map,
            ObserverV1::MapCallbackAndSnapshot | ObserverV1::CustodyAndCleanup,
        )
        | (
            RootOperationV1::Lock,
            ObserverV1::LockCallbackAndSnapshot | ObserverV1::CustodyAndCleanup,
        ) => {}
        _ => return Err(ProjectionViolationV1::ObserverRoot.into()),
    }
    Ok(())
}

impl From<ProjectionViolationV1> for ProjectionErrorV1 {
    fn from(value: ProjectionViolationV1) -> Self {
        Self::Invalid(value)
    }
}
