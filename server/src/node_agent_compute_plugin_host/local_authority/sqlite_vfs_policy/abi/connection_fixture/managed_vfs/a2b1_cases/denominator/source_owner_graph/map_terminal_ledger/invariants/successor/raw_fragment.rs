use std::collections::BTreeSet;

use super::super::super::super::super::{
    abi_map_fragment::AbiNullWriteOutcome,
    case_key::{InitializationPath, Path, PrefixMutation},
    projection::ExpectedStatus,
    raw_map_fragment::{
        ReviewedMapRawCutFragment, ReviewedMapRawDecisionFragment,
        ReviewedMapRawDispositionFragment, ReviewedMapRawExitFragment, ReviewedMapRawFragmentCell,
        ReviewedMapRawTypedOperationFragment, REVIEWED_MAP_RAW_FRAGMENTS,
    },
    raw_state_fragment::{
        RawAbandonCauseFragment, RawAbandonEndpoint, RawAbandonOutcome, RawAdmissionDecision,
        RawAdmissionDisposition, RawAdmissionOutcome, RawAdmissionShape, RawCleanupEffect,
        RawPostOperationDisposition, RawPostOperationOutcome, RawSlotRetention,
        RawStateFragmentExclusion, DROP_UNWIND_CUSTODY_PENDING, FOREIGN_METHODS_AND_OPAQUE_STATE,
        INSTALLED_RAW_VALUES, METHODS_VALUE_ONLY, NO_RAW_VALUES, OPAQUE_STATE_VALUE,
        RAW_ABANDON_FRAGMENTS, RAW_ADMISSION_FRAGMENTS, RAW_POST_OPERATION_FRAGMENTS,
        RAW_STATE_FRAGMENT_EXCLUSIONS,
    },
};
use super::super::super::super::model::SourceEffect;
use super::super::super::reviewed_trace::{
    RawAbandonCauseDisposition, RawStateCase, RawStateTraceDisposition, ReviewedOpenFrontier,
    RAW_ABANDON_OUTCOMES, RAW_STATE_OUTCOMES,
};

type AdmissionExpected = (
    RawAdmissionShape,
    RawAdmissionDecision,
    RawAdmissionOutcome,
    RawAdmissionDisposition,
    RawSlotRetention,
);

type MapCellKey = (RawAdmissionShape, ReviewedMapRawDispositionFragment);

pub(super) fn validate() -> Result<(), &'static str> {
    validate_admission_partition()?;
    validate_post_operation_partition()?;
    validate_abandon_partition()?;
    validate_map_projection()?;
    validate_reviewed_trace_projection()
}

fn validate_admission_partition() -> Result<(), &'static str> {
    let expected = expected_admissions();
    let actual = RAW_ADMISSION_FRAGMENTS
        .iter()
        .map(|record| {
            (
                record.shape,
                record.decision,
                record.outcome,
                record.disposition,
                record.slots,
            )
        })
        .collect::<BTreeSet<_>>();
    if actual != expected.into_iter().collect()
        || actual.len() != RAW_ADMISSION_FRAGMENTS.len()
        || RAW_ADMISSION_FRAGMENTS.len() != 8
    {
        return Err("source-neutral raw admission fragment is not the exact eight-cell quotient");
    }
    Ok(())
}

fn expected_admissions() -> [AdmissionExpected; 8] {
    [
        admission(
            RawAdmissionShape::NullFile,
            RawAdmissionDecision::NullFile,
            RawAdmissionOutcome::NullFile,
            RawAdmissionDisposition::Abandon(RawAbandonEndpoint::NullFileRejected),
            NO_RAW_VALUES,
        ),
        admission(
            RawAdmissionShape::MethodsNullStateNull,
            RawAdmissionDecision::Uninstalled,
            RawAdmissionOutcome::Uninstalled,
            RawAdmissionDisposition::Abandon(RawAbandonEndpoint::Empty),
            NO_RAW_VALUES,
        ),
        admission(
            RawAdmissionShape::MethodsNullStatePresent,
            RawAdmissionDecision::ForeignMethodsNullTable,
            RawAdmissionOutcome::ForeignMethods,
            RawAdmissionDisposition::Abandon(RawAbandonEndpoint::ForeignMethodsNullTableRejected),
            OPAQUE_STATE_VALUE,
        ),
        admission(
            RawAdmissionShape::ForeignMethodsStateNull,
            RawAdmissionDecision::ForeignMethodsForeignTable,
            RawAdmissionOutcome::ForeignMethods,
            RawAdmissionDisposition::Abandon(
                RawAbandonEndpoint::ForeignMethodsForeignTableRejected,
            ),
            METHODS_VALUE_ONLY,
        ),
        admission(
            RawAdmissionShape::ForeignMethodsStatePresent,
            RawAdmissionDecision::ForeignMethodsForeignTable,
            RawAdmissionOutcome::ForeignMethods,
            RawAdmissionDisposition::Abandon(
                RawAbandonEndpoint::ForeignMethodsForeignTableRejected,
            ),
            FOREIGN_METHODS_AND_OPAQUE_STATE,
        ),
        admission(
            RawAdmissionShape::ExactMethodsStateNull,
            RawAdmissionDecision::StateMissing,
            RawAdmissionOutcome::StateMissing,
            RawAdmissionDisposition::Abandon(RawAbandonEndpoint::StateMissingRejected),
            METHODS_VALUE_ONLY,
        ),
        admission(
            RawAdmissionShape::ExactMethodsInstalledWrongType,
            RawAdmissionDecision::TypeMismatch,
            RawAdmissionOutcome::TypeMismatch,
            RawAdmissionDisposition::Abandon(RawAbandonEndpoint::Installed),
            INSTALLED_RAW_VALUES,
        ),
        admission(
            RawAdmissionShape::ExactMethodsInstalledExpectedType,
            RawAdmissionDecision::ExpectedTypeEntry,
            RawAdmissionOutcome::ExpectedTypeEntry,
            RawAdmissionDisposition::TypedOperation,
            INSTALLED_RAW_VALUES,
        ),
    ]
}

const fn admission(
    shape: RawAdmissionShape,
    decision: RawAdmissionDecision,
    outcome: RawAdmissionOutcome,
    disposition: RawAdmissionDisposition,
    slots: RawSlotRetention,
) -> AdmissionExpected {
    (shape, decision, outcome, disposition, slots)
}

fn validate_post_operation_partition() -> Result<(), &'static str> {
    let actual = RAW_POST_OPERATION_FRAGMENTS
        .iter()
        .map(|record| (record.outcome, record.disposition, record.slots))
        .collect::<BTreeSet<_>>();
    let expected = [
        (
            RawPostOperationOutcome::AcceptedNormalReturn,
            RawPostOperationDisposition::ProtectedCallReturn,
            INSTALLED_RAW_VALUES,
        ),
        (
            RawPostOperationOutcome::CaughtUnwind,
            RawPostOperationDisposition::Abandon(RawAbandonEndpoint::Installed),
            INSTALLED_RAW_VALUES,
        ),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if actual != expected || actual.len() != RAW_POST_OPERATION_FRAGMENTS.len() {
        return Err("raw post-operation outcomes leaked into or vanished from the admission cut");
    }
    let exclusions = RAW_STATE_FRAGMENT_EXCLUSIONS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let expected_exclusions = [
        RawStateFragmentExclusion::OccupiedInstallOnly,
        RawStateFragmentExclusion::ForgedEnvelopeOrSlots,
        RawStateFragmentExclusion::UndefinedBehaviorPremise,
        RawStateFragmentExclusion::AbortingPanic,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if exclusions != expected_exclusions || exclusions.len() != RAW_STATE_FRAGMENT_EXCLUSIONS.len()
    {
        return Err("raw fragment safety and install-only exclusions are incomplete");
    }
    Ok(())
}

fn validate_abandon_partition() -> Result<(), &'static str> {
    let outcomes = RAW_ABANDON_FRAGMENTS
        .iter()
        .map(|record| record.outcome)
        .collect::<BTreeSet<_>>();
    let expected = [
        RawAbandonOutcome::Empty,
        RawAbandonOutcome::InstalledDropCompleted,
        RawAbandonOutcome::InstalledDropUnwindCaught,
        RawAbandonOutcome::NullFileRejected,
        RawAbandonOutcome::ForeignMethodsNullTableRejected,
        RawAbandonOutcome::ForeignMethodsForeignTableStateNullRejected,
        RawAbandonOutcome::ForeignMethodsForeignTableStatePresentRejected,
        RawAbandonOutcome::StateMissingRejected,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if outcomes != expected
        || outcomes.len() != RAW_ABANDON_FRAGMENTS.len()
        || RAW_ABANDON_FRAGMENTS.len() != 8
    {
        return Err("source-neutral raw abandonment fragment is not exact");
    }
    for record in RAW_ABANDON_FRAGMENTS {
        let causes = record.causes.iter().copied().collect::<BTreeSet<_>>();
        if causes.len() != record.causes.len() {
            return Err("raw abandonment fragment repeats a temporal cause");
        }
        let has_post_operation_cause = causes.contains(&RawAbandonCauseFragment::PostOperation(
            RawPostOperationOutcome::CaughtUnwind,
        ));
        if has_post_operation_cause
            != matches!(
                record.outcome,
                RawAbandonOutcome::InstalledDropCompleted
                    | RawAbandonOutcome::InstalledDropUnwindCaught
            )
        {
            return Err("caught unwind was flattened into the raw admission partition");
        }
    }
    Ok(())
}

fn validate_map_projection() -> Result<(), &'static str> {
    let actual = REVIEWED_MAP_RAW_FRAGMENTS
        .iter()
        .map(|cell| (cell.branch.admission, cell.branch.disposition))
        .collect::<BTreeSet<_>>();
    let expected = expected_map_cells().into_iter().collect::<BTreeSet<_>>();
    if actual != expected
        || actual.len() != REVIEWED_MAP_RAW_FRAGMENTS.len()
        || REVIEWED_MAP_RAW_FRAGMENTS.len() != 9
    {
        return Err("Map raw fragment is not eight fallback continuations plus one typed entry");
    }
    for cell in REVIEWED_MAP_RAW_FRAGMENTS {
        validate_map_cell(cell)?;
    }
    Ok(())
}

fn expected_map_cells() -> [MapCellKey; 9] {
    [
        map_fallback(
            RawAdmissionShape::NullFile,
            RawAbandonOutcome::NullFileRejected,
        ),
        map_fallback(
            RawAdmissionShape::MethodsNullStateNull,
            RawAbandonOutcome::Empty,
        ),
        map_fallback(
            RawAdmissionShape::MethodsNullStatePresent,
            RawAbandonOutcome::ForeignMethodsNullTableRejected,
        ),
        map_fallback(
            RawAdmissionShape::ForeignMethodsStateNull,
            RawAbandonOutcome::ForeignMethodsForeignTableStateNullRejected,
        ),
        map_fallback(
            RawAdmissionShape::ForeignMethodsStatePresent,
            RawAbandonOutcome::ForeignMethodsForeignTableStatePresentRejected,
        ),
        map_fallback(
            RawAdmissionShape::ExactMethodsStateNull,
            RawAbandonOutcome::StateMissingRejected,
        ),
        map_fallback(
            RawAdmissionShape::ExactMethodsInstalledWrongType,
            RawAbandonOutcome::InstalledDropCompleted,
        ),
        map_fallback(
            RawAdmissionShape::ExactMethodsInstalledWrongType,
            RawAbandonOutcome::InstalledDropUnwindCaught,
        ),
        (
            RawAdmissionShape::ExactMethodsInstalledExpectedType,
            ReviewedMapRawDispositionFragment::ContinuesAtTypedMapOperation,
        ),
    ]
}

const fn map_fallback(shape: RawAdmissionShape, outcome: RawAbandonOutcome) -> MapCellKey {
    (
        shape,
        ReviewedMapRawDispositionFragment::ContinuesAfterAbandon(outcome),
    )
}

fn validate_map_cell(cell: &ReviewedMapRawFragmentCell) -> Result<(), &'static str> {
    let admission = RAW_ADMISSION_FRAGMENTS
        .iter()
        .find(|record| record.shape == cell.branch.admission)
        .ok_or("Map raw cell has no source-neutral admission")?;
    if cell.branch.candidate_path != Path::Map
        || cell.expected.null_write != AbiNullWriteOutcome::NullWritten
        || cell.expected.raw_slots_at_gate != admission.slots
        || cell.expected.prefix_mutation_at_cut != PrefixMutation::NotReached
        || cell.expected.initialization_at_cut != InitializationPath::NotReached
        || cell.expected.expected_status != ExpectedStatus::PendingSourceAndRedTeamReview
    {
        return Err("Map raw fragment changed an inherited ABI or pre-prefix fact");
    }
    match cell.branch.disposition {
        ReviewedMapRawDispositionFragment::ContinuesAtTypedMapOperation => {
            validate_typed_map_cell(cell, admission.disposition)
        }
        ReviewedMapRawDispositionFragment::ContinuesAfterAbandon(outcome) => {
            validate_fallback_map_cell(cell, admission.disposition, outcome)
        }
    }
}

fn validate_typed_map_cell(
    cell: &ReviewedMapRawFragmentCell,
    disposition: RawAdmissionDisposition,
) -> Result<(), &'static str> {
    if disposition != RawAdmissionDisposition::TypedOperation
        || cell.branch.decision != ReviewedMapRawDecisionFragment::ExpectedTypeInstalled
        || cell.branch.cut != ReviewedMapRawCutFragment::TypedMapOperationEntry
        || cell.expected.raw_slots_at_cut != INSTALLED_RAW_VALUES
        || cell.expected.cleanup != RawCleanupEffect::None
        || cell.expected.sqlite_exit != ReviewedMapRawExitFragment::PendingAfterTypedMapOperation
        || cell.expected.typed_operation
            != ReviewedMapRawTypedOperationFragment::PendingAfterTypedMapOperation
    {
        return Err("Map raw expected-type cell closes or misroutes the typed-operation frontier");
    }
    Ok(())
}

fn validate_fallback_map_cell(
    cell: &ReviewedMapRawFragmentCell,
    disposition: RawAdmissionDisposition,
    outcome: RawAbandonOutcome,
) -> Result<(), &'static str> {
    let abandon = RAW_ABANDON_FRAGMENTS
        .iter()
        .find(|record| record.outcome == outcome)
        .ok_or("Map raw fallback cell has no source-neutral abandonment outcome")?;
    let RawAdmissionDisposition::Abandon(endpoint) = disposition else {
        return Err("Map raw typed admission was projected as fallback");
    };
    if endpoint != abandon.endpoint
        || !abandon
            .causes
            .contains(&RawAbandonCauseFragment::Admission(cell.branch.admission))
        || cell.branch.decision != ReviewedMapRawDecisionFragment::AdmissionRejected
        || cell.branch.cut != ReviewedMapRawCutFragment::AfterAbandon
        || cell.expected.raw_slots_at_cut != abandon.slots_after
        || cell.expected.cleanup != abandon.cleanup
        || cell.expected.sqlite_exit
            != ReviewedMapRawExitFragment::KnownUnavailableNullAfterRawFallback
        || cell.expected.typed_operation
            != ReviewedMapRawTypedOperationFragment::NotReachedByRawRejection
    {
        return Err("Map raw fallback cell disagrees with canonical abandonment");
    }
    Ok(())
}

fn validate_reviewed_trace_projection() -> Result<(), &'static str> {
    let prefix_cases = RAW_STATE_OUTCOMES
        .iter()
        .filter_map(|record| match record.trace {
            RawStateTraceDisposition::PrefixSuccessor(_) => Some(record.case),
            RawStateTraceDisposition::BeyondOpenFrontier(_) => None,
        })
        .collect::<BTreeSet<_>>();
    let expected_prefix = RAW_ADMISSION_FRAGMENTS
        .iter()
        .filter_map(|record| trace_case_for_admission(record.shape))
        .collect::<BTreeSet<_>>();
    if prefix_cases != expected_prefix || prefix_cases.len() != 7 {
        return Err("reviewed raw trace is not an exact projection of rejected admissions");
    }
    validate_post_operation_trace()?;
    validate_abandon_trace()
}

fn validate_post_operation_trace() -> Result<(), &'static str> {
    for (outcome, case) in [
        (
            RawPostOperationOutcome::AcceptedNormalReturn,
            RawStateCase::AcceptedAfterTypedOperation,
        ),
        (
            RawPostOperationOutcome::CaughtUnwind,
            RawStateCase::CaughtUnwindFromTypedOperation,
        ),
    ] {
        let canonical = RAW_POST_OPERATION_FRAGMENTS
            .iter()
            .find(|record| record.outcome == outcome)
            .ok_or("canonical raw post-operation outcome is missing")?;
        let reviewed = RAW_STATE_OUTCOMES
            .iter()
            .find(|record| record.case == case)
            .ok_or("reviewed raw post-operation trace is missing")?;
        if canonical.slots != reviewed.slots
            || reviewed.trace
                != RawStateTraceDisposition::BeyondOpenFrontier(
                    ReviewedOpenFrontier::TypedMapOperation,
                )
        {
            return Err("post-operation raw outcome was flattened into the ingress prefix");
        }
    }
    Ok(())
}

fn validate_abandon_trace() -> Result<(), &'static str> {
    for canonical in RAW_ABANDON_FRAGMENTS {
        let reviewed = RAW_ABANDON_OUTCOMES
            .iter()
            .find(|record| record.outcome == canonical.outcome)
            .ok_or("canonical raw abandonment has no reviewed trace projection")?;
        let reviewed_causes = reviewed
            .causes
            .iter()
            .map(|cause| (cause.case, cause.disposition))
            .collect::<BTreeSet<_>>();
        let canonical_causes = canonical
            .causes
            .iter()
            .map(|cause| reviewed_cause(*cause))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if reviewed_causes != canonical_causes
            || reviewed.slots != canonical.slots_after
            || reviewed.effect != source_effect(canonical.cleanup)
        {
            return Err("reviewed raw abandonment drifted from the source-neutral fragment");
        }
    }
    Ok(())
}

fn reviewed_cause(
    cause: RawAbandonCauseFragment,
) -> Result<(RawStateCase, RawAbandonCauseDisposition), &'static str> {
    match cause {
        RawAbandonCauseFragment::Admission(shape) => Ok((
            trace_case_for_admission(shape)
                .ok_or("typed raw admission cannot be an abandonment cause")?,
            RawAbandonCauseDisposition::PrefixSuccessor,
        )),
        RawAbandonCauseFragment::PostOperation(RawPostOperationOutcome::CaughtUnwind) => Ok((
            RawStateCase::CaughtUnwindFromTypedOperation,
            RawAbandonCauseDisposition::BeyondOpenFrontier(ReviewedOpenFrontier::TypedMapOperation),
        )),
        RawAbandonCauseFragment::PostOperation(RawPostOperationOutcome::AcceptedNormalReturn) => {
            Err("normal raw protected-call return cannot cause abandonment")
        }
    }
}

const fn source_effect(cleanup: RawCleanupEffect) -> SourceEffect {
    match cleanup {
        RawCleanupEffect::None => SourceEffect::None,
        RawCleanupEffect::ClearSlotsThenDropInstalledEnvelope => SourceEffect::Cleanup,
    }
}

const fn trace_case_for_admission(shape: RawAdmissionShape) -> Option<RawStateCase> {
    match shape {
        RawAdmissionShape::NullFile => Some(RawStateCase::NullFile),
        RawAdmissionShape::MethodsNullStateNull => Some(RawStateCase::Uninstalled),
        RawAdmissionShape::MethodsNullStatePresent => {
            Some(RawStateCase::ForeignMethodsNullTableStatePresent)
        }
        RawAdmissionShape::ForeignMethodsStateNull => {
            Some(RawStateCase::ForeignMethodsForeignTableStateNull)
        }
        RawAdmissionShape::ForeignMethodsStatePresent => {
            Some(RawStateCase::ForeignMethodsForeignTableStatePresent)
        }
        RawAdmissionShape::ExactMethodsStateNull => {
            Some(RawStateCase::StateMissingInertTableStateNull)
        }
        RawAdmissionShape::ExactMethodsInstalledWrongType => {
            Some(RawStateCase::TypeMismatchInstalled)
        }
        RawAdmissionShape::ExactMethodsInstalledExpectedType => None,
    }
}
