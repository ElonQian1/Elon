use std::collections::BTreeSet;

use super::super::super::super::model::SourceEffect;
use super::super::super::{
    model::MapSourceStepId,
    reviewed_trace::{
        RawAbandonCauseDisposition, RawAbandonOutcome, RawSlotRetention, RawStateCase,
        RawStateOutcome, RawStateTraceDisposition, ReviewedOpenFrontier, ReviewedTraceEndpoint,
        RAW_ABANDON_OUTCOMES, RAW_STATE_OUTCOMES,
    },
};
use super::shared::{
    DROP_UNWIND_CUSTODY_PENDING, FOREIGN_METHODS_AND_OPAQUE_STATE, INSTALLED_RAW_VALUES,
    METHODS_VALUE_ONLY, NO_RAW_VALUES, OPAQUE_STATE_VALUE,
};

type RawStateExpected = (
    RawStateCase,
    RawStateOutcome,
    MapSourceStepId,
    RawSlotRetention,
    RawStateTraceDisposition,
);

type AbandonExpected = (
    RawAbandonOutcome,
    Vec<(RawStateCase, RawAbandonCauseDisposition)>,
    MapSourceStepId,
    SourceEffect,
    RawSlotRetention,
);

pub(super) fn validate() -> Result<(), &'static str> {
    validate_raw_state_partition()?;
    validate_abandon_partition()
}

fn validate_raw_state_partition() -> Result<(), &'static str> {
    let expected = [
        (
            RawStateCase::AcceptedAfterTypedOperation,
            RawStateOutcome::AcceptedNormalReturn,
            MapSourceStepId::RawStateAccepted,
            INSTALLED_RAW_VALUES,
            RawStateTraceDisposition::BeyondOpenFrontier(ReviewedOpenFrontier::TypedMapOperation),
        ),
        raw_state_expected(
            RawStateCase::NullFile,
            RawStateOutcome::NullFile,
            MapSourceStepId::RawStateNullFile,
            NO_RAW_VALUES,
            MapSourceStepId::RawAbandonNullFileRejected,
        ),
        raw_state_expected(
            RawStateCase::Uninstalled,
            RawStateOutcome::Uninstalled,
            MapSourceStepId::RawStateUninstalled,
            NO_RAW_VALUES,
            MapSourceStepId::RawAbandonEmpty,
        ),
        raw_state_expected(
            RawStateCase::ForeignMethodsNullTableStatePresent,
            RawStateOutcome::ForeignMethodsNullTable,
            MapSourceStepId::RawStateForeignMethodsNullTable,
            OPAQUE_STATE_VALUE,
            MapSourceStepId::RawAbandonForeignMethodsNullTableRejected,
        ),
        raw_state_expected(
            RawStateCase::ForeignMethodsForeignTableStateNull,
            RawStateOutcome::ForeignMethodsForeignTable,
            MapSourceStepId::RawStateForeignMethodsForeignTable,
            METHODS_VALUE_ONLY,
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
        ),
        raw_state_expected(
            RawStateCase::ForeignMethodsForeignTableStatePresent,
            RawStateOutcome::ForeignMethodsForeignTable,
            MapSourceStepId::RawStateForeignMethodsForeignTable,
            FOREIGN_METHODS_AND_OPAQUE_STATE,
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
        ),
        raw_state_expected(
            RawStateCase::StateMissingInertTableStateNull,
            RawStateOutcome::StateMissing,
            MapSourceStepId::RawStateMissing,
            METHODS_VALUE_ONLY,
            MapSourceStepId::RawAbandonStateMissingRejected,
        ),
        raw_state_expected(
            RawStateCase::TypeMismatchInstalled,
            RawStateOutcome::TypeMismatch,
            MapSourceStepId::RawStateTypeMismatch,
            INSTALLED_RAW_VALUES,
            MapSourceStepId::RawAbandonInstalled,
        ),
        (
            RawStateCase::CaughtUnwindFromTypedOperation,
            RawStateOutcome::CaughtUnwind,
            MapSourceStepId::RawStateCaughtPanic,
            INSTALLED_RAW_VALUES,
            RawStateTraceDisposition::BeyondOpenFrontier(ReviewedOpenFrontier::TypedMapOperation),
        ),
    ];
    let cases = RAW_STATE_OUTCOMES
        .iter()
        .map(|record| record.case)
        .collect::<BTreeSet<_>>();
    if cases != expected.iter().map(|record| record.0).collect()
        || cases.len() != RAW_STATE_OUTCOMES.len()
        || RAW_STATE_OUTCOMES.len() != expected.len()
    {
        return Err("Map raw protected-call case set is not exact");
    }
    let outcomes = RAW_STATE_OUTCOMES
        .iter()
        .map(|record| record.outcome)
        .collect::<BTreeSet<_>>();
    let expected_outcomes = [
        RawStateOutcome::AcceptedNormalReturn,
        RawStateOutcome::NullFile,
        RawStateOutcome::Uninstalled,
        RawStateOutcome::ForeignMethodsNullTable,
        RawStateOutcome::ForeignMethodsForeignTable,
        RawStateOutcome::StateMissing,
        RawStateOutcome::TypeMismatch,
        RawStateOutcome::CaughtUnwind,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if outcomes != expected_outcomes {
        return Err("Map raw rejection, normal-return and caught-unwind outcomes are conflated");
    }
    for expected_record in expected {
        let Some(actual) = RAW_STATE_OUTCOMES
            .iter()
            .find(|record| record.case == expected_record.0)
        else {
            return Err("Map raw protected-call case is missing");
        };
        if (actual.outcome, actual.step, actual.slots, actual.trace)
            != (
                expected_record.1,
                expected_record.2,
                expected_record.3,
                expected_record.4,
            )
        {
            return Err("Map raw protected-call case has a non-exact source or successor shape");
        }
    }
    Ok(())
}

const fn raw_state_expected(
    case: RawStateCase,
    outcome: RawStateOutcome,
    step: MapSourceStepId,
    slots: RawSlotRetention,
    successor: MapSourceStepId,
) -> RawStateExpected {
    (
        case,
        outcome,
        step,
        slots,
        RawStateTraceDisposition::PrefixSuccessor(ReviewedTraceEndpoint::Step(successor)),
    )
}

fn validate_abandon_partition() -> Result<(), &'static str> {
    let expected = [
        abandon_expected(
            RawAbandonOutcome::Empty,
            vec![prefix_cause(RawStateCase::Uninstalled)],
            MapSourceStepId::RawAbandonEmpty,
            SourceEffect::None,
            NO_RAW_VALUES,
        ),
        abandon_expected(
            RawAbandonOutcome::InstalledDropCompleted,
            vec![
                prefix_cause(RawStateCase::TypeMismatchInstalled),
                beyond_frontier_cause(RawStateCase::CaughtUnwindFromTypedOperation),
            ],
            MapSourceStepId::RawAbandonInstalled,
            SourceEffect::Cleanup,
            NO_RAW_VALUES,
        ),
        abandon_expected(
            RawAbandonOutcome::InstalledDropUnwindCaught,
            vec![
                prefix_cause(RawStateCase::TypeMismatchInstalled),
                beyond_frontier_cause(RawStateCase::CaughtUnwindFromTypedOperation),
            ],
            MapSourceStepId::RawAbandonInstalled,
            SourceEffect::Cleanup,
            DROP_UNWIND_CUSTODY_PENDING,
        ),
        abandon_expected(
            RawAbandonOutcome::NullFileRejected,
            vec![prefix_cause(RawStateCase::NullFile)],
            MapSourceStepId::RawAbandonNullFileRejected,
            SourceEffect::None,
            NO_RAW_VALUES,
        ),
        abandon_expected(
            RawAbandonOutcome::ForeignMethodsNullTableRejected,
            vec![prefix_cause(
                RawStateCase::ForeignMethodsNullTableStatePresent,
            )],
            MapSourceStepId::RawAbandonForeignMethodsNullTableRejected,
            SourceEffect::None,
            OPAQUE_STATE_VALUE,
        ),
        abandon_expected(
            RawAbandonOutcome::ForeignMethodsForeignTableStateNullRejected,
            vec![prefix_cause(
                RawStateCase::ForeignMethodsForeignTableStateNull,
            )],
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
            SourceEffect::None,
            METHODS_VALUE_ONLY,
        ),
        abandon_expected(
            RawAbandonOutcome::ForeignMethodsForeignTableStatePresentRejected,
            vec![prefix_cause(
                RawStateCase::ForeignMethodsForeignTableStatePresent,
            )],
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
            SourceEffect::None,
            FOREIGN_METHODS_AND_OPAQUE_STATE,
        ),
        abandon_expected(
            RawAbandonOutcome::StateMissingRejected,
            vec![prefix_cause(RawStateCase::StateMissingInertTableStateNull)],
            MapSourceStepId::RawAbandonStateMissingRejected,
            SourceEffect::None,
            METHODS_VALUE_ONLY,
        ),
    ];
    let outcomes = RAW_ABANDON_OUTCOMES
        .iter()
        .map(|record| record.outcome)
        .collect::<BTreeSet<_>>();
    if outcomes != expected.iter().map(|record| record.0).collect()
        || outcomes.len() != RAW_ABANDON_OUTCOMES.len()
        || RAW_ABANDON_OUTCOMES.len() != expected.len()
    {
        return Err("Map raw abandonment outcome set is not exact");
    }
    for expected_record in expected {
        let Some(actual) = RAW_ABANDON_OUTCOMES
            .iter()
            .find(|record| record.outcome == expected_record.0)
        else {
            return Err("Map raw abandonment outcome is missing");
        };
        let causes = actual
            .causes
            .iter()
            .map(|cause| (cause.case, cause.disposition))
            .collect::<BTreeSet<_>>();
        let expected_causes = expected_record.1.iter().copied().collect::<BTreeSet<_>>();
        if causes != expected_causes
            || causes.len() != actual.causes.len()
            || (
                actual.step,
                actual.effect,
                actual.slots,
                actual.prefix_successor,
            ) != (
                expected_record.2,
                expected_record.3,
                expected_record.4,
                ReviewedTraceEndpoint::Step(MapSourceStepId::RawFallbackProjection),
            )
        {
            return Err("Map raw abandonment cause, effect, slots or successor is not exact");
        }
    }
    Ok(())
}

fn abandon_expected(
    outcome: RawAbandonOutcome,
    causes: Vec<(RawStateCase, RawAbandonCauseDisposition)>,
    step: MapSourceStepId,
    effect: SourceEffect,
    slots: RawSlotRetention,
) -> AbandonExpected {
    (outcome, causes, step, effect, slots)
}

const fn prefix_cause(case: RawStateCase) -> (RawStateCase, RawAbandonCauseDisposition) {
    (case, RawAbandonCauseDisposition::PrefixSuccessor)
}

const fn beyond_frontier_cause(case: RawStateCase) -> (RawStateCase, RawAbandonCauseDisposition) {
    (
        case,
        RawAbandonCauseDisposition::BeyondOpenFrontier(ReviewedOpenFrontier::TypedMapOperation),
    )
}
