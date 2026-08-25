use std::collections::BTreeSet;

use super::super::super::super::super::{
    abi_map_fragment::AbiNullWriteOutcome,
    case_key::Path,
    projection::ExpectedStatus,
    raw_state_fragment::{
        RawAbandonCauseFragment, RawAbandonEndpoint, RawAbandonOutcome, RawCleanupEffect,
        RawPostOperationDisposition, RawPostOperationOutcome, RawSlotRetention,
        DROP_UNWIND_CUSTODY_PENDING, INSTALLED_RAW_VALUES, NO_RAW_VALUES, RAW_ABANDON_FRAGMENTS,
        RAW_POST_OPERATION_FRAGMENTS,
    },
    typed_map_fragment::{
        ReviewedTypedMapBranchFragment, ReviewedTypedMapDispositionFragment,
        ReviewedTypedMapExitFragment, ReviewedTypedMapExpectedFragment,
        ReviewedTypedMapFragmentCell, ReviewedTypedMapOutcomeFragment,
        ReviewedTypedMapOutputFragment, ReviewedTypedMapProvenanceFragment,
        REVIEWED_TYPED_MAP_FRAGMENTS,
    },
};
use super::super::super::{
    model::MapSourceStep,
    reviewed_trace::{ReviewedOpenFrontier, OPEN_FRONTIERS},
};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_exact_outer_quotient()?;
    validate_canonical_raw_projection()?;
    super::typed_witnesses::validate(steps)?;
    validate_frontier_preserved()
}

fn validate_exact_outer_quotient() -> Result<(), &'static str> {
    let expected = expected_cells().into_iter().collect::<BTreeSet<_>>();
    let actual = REVIEWED_TYPED_MAP_FRAGMENTS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual != expected
        || actual.len() != REVIEWED_TYPED_MAP_FRAGMENTS.len()
        || REVIEWED_TYPED_MAP_FRAGMENTS.len() != 5
    {
        return Err("typed Map outer-result fragment is not the exact five-cell projection");
    }

    let outcomes = REVIEWED_TYPED_MAP_FRAGMENTS
        .iter()
        .map(|cell| cell.branch.outcome)
        .collect::<BTreeSet<_>>();
    if outcomes
        != [
            ReviewedTypedMapOutcomeFragment::NotPresent,
            ReviewedTypedMapOutcomeFragment::Mapped,
            ReviewedTypedMapOutcomeFragment::Failure,
            ReviewedTypedMapOutcomeFragment::CaughtUnwind,
        ]
        .into_iter()
        .collect()
    {
        return Err("typed Map fragment lost one of its four outer outcome classes");
    }

    let normal = REVIEWED_TYPED_MAP_FRAGMENTS
        .iter()
        .filter(|cell| cell.branch.disposition == ReviewedTypedMapDispositionFragment::NormalReturn)
        .count();
    let caught = REVIEWED_TYPED_MAP_FRAGMENTS.len() - normal;
    let pointer_writes = REVIEWED_TYPED_MAP_FRAGMENTS
        .iter()
        .filter(|cell| {
            cell.expected.output_at_cut == ReviewedTypedMapOutputFragment::MappedPointerWritten
        })
        .count();
    if normal != 3 || caught != 2 || pointer_writes != 1 {
        return Err("typed Map fragment changed its normal/unwind or pointer-write cardinality");
    }
    Ok(())
}

fn expected_cells() -> [ReviewedTypedMapFragmentCell; 5] {
    [
        normal(
            ReviewedTypedMapOutcomeFragment::NotPresent,
            ReviewedTypedMapOutputFragment::NullRetained,
            ReviewedTypedMapExitFragment::SqliteOkNotPresent,
        ),
        normal(
            ReviewedTypedMapOutcomeFragment::Mapped,
            ReviewedTypedMapOutputFragment::MappedPointerWritten,
            ReviewedTypedMapExitFragment::SqliteOkMapped,
        ),
        normal(
            ReviewedTypedMapOutcomeFragment::Failure,
            ReviewedTypedMapOutputFragment::NullRetained,
            ReviewedTypedMapExitFragment::ShmMapUnavailable,
        ),
        unwind(RawAbandonOutcome::InstalledDropCompleted, NO_RAW_VALUES),
        unwind(
            RawAbandonOutcome::InstalledDropUnwindCaught,
            DROP_UNWIND_CUSTODY_PENDING,
        ),
    ]
}

fn normal(
    outcome: ReviewedTypedMapOutcomeFragment,
    output_at_cut: ReviewedTypedMapOutputFragment,
    sqlite_exit: ReviewedTypedMapExitFragment,
) -> ReviewedTypedMapFragmentCell {
    cell(
        outcome,
        RawPostOperationOutcome::AcceptedNormalReturn,
        ReviewedTypedMapDispositionFragment::NormalReturn,
        output_at_cut,
        INSTALLED_RAW_VALUES,
        RawCleanupEffect::None,
        sqlite_exit,
    )
}

fn unwind(
    outcome: RawAbandonOutcome,
    raw_slots_at_cut: RawSlotRetention,
) -> ReviewedTypedMapFragmentCell {
    cell(
        ReviewedTypedMapOutcomeFragment::CaughtUnwind,
        RawPostOperationOutcome::CaughtUnwind,
        ReviewedTypedMapDispositionFragment::FallbackAfterCaughtUnwind(outcome),
        ReviewedTypedMapOutputFragment::NullRetained,
        raw_slots_at_cut,
        RawCleanupEffect::ClearSlotsThenDropInstalledEnvelope,
        ReviewedTypedMapExitFragment::ShmMapUnavailable,
    )
}

fn cell(
    outcome: ReviewedTypedMapOutcomeFragment,
    raw_post_operation: RawPostOperationOutcome,
    disposition: ReviewedTypedMapDispositionFragment,
    output_at_cut: ReviewedTypedMapOutputFragment,
    raw_slots_at_cut: RawSlotRetention,
    cleanup: RawCleanupEffect,
    sqlite_exit: ReviewedTypedMapExitFragment,
) -> ReviewedTypedMapFragmentCell {
    ReviewedTypedMapFragmentCell {
        branch: ReviewedTypedMapBranchFragment {
            candidate_path: Path::Map,
            outcome,
            raw_post_operation,
            disposition,
        },
        expected: ReviewedTypedMapExpectedFragment {
            null_write_at_entry: AbiNullWriteOutcome::NullWritten,
            output_at_cut,
            raw_slots_at_entry: INSTALLED_RAW_VALUES,
            raw_slots_at_cut,
            cleanup,
            sqlite_exit,
            provenance: ReviewedTypedMapProvenanceFragment::PendingRouteManagedPrestateAndCustody,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

fn validate_canonical_raw_projection() -> Result<(), &'static str> {
    for cell in REVIEWED_TYPED_MAP_FRAGMENTS {
        let Some(post_operation) = RAW_POST_OPERATION_FRAGMENTS
            .iter()
            .find(|record| record.outcome == cell.branch.raw_post_operation)
        else {
            return Err("typed Map fragment references a missing canonical post-operation outcome");
        };
        if post_operation.slots != cell.expected.raw_slots_at_entry {
            return Err("typed Map fragment changed canonical raw slots at operation return");
        }
        match cell.branch.disposition {
            ReviewedTypedMapDispositionFragment::NormalReturn => {
                if post_operation.disposition != RawPostOperationDisposition::ProtectedCallReturn
                    || cell.expected.raw_slots_at_cut != INSTALLED_RAW_VALUES
                    || cell.expected.cleanup != RawCleanupEffect::None
                {
                    return Err(
                        "typed Map normal return does not preserve the installed raw state",
                    );
                }
            }
            ReviewedTypedMapDispositionFragment::FallbackAfterCaughtUnwind(outcome) => {
                if post_operation.disposition
                    != RawPostOperationDisposition::Abandon(RawAbandonEndpoint::Installed)
                {
                    return Err("typed Map unwind no longer enters installed-state abandonment");
                }
                let Some(abandonment) = RAW_ABANDON_FRAGMENTS
                    .iter()
                    .find(|record| record.outcome == outcome)
                else {
                    return Err("typed Map unwind references a missing canonical abandonment");
                };
                if abandonment.endpoint != RawAbandonEndpoint::Installed
                    || abandonment.cleanup != cell.expected.cleanup
                    || abandonment.slots_after != cell.expected.raw_slots_at_cut
                    || !abandonment
                        .causes
                        .contains(&RawAbandonCauseFragment::PostOperation(
                            RawPostOperationOutcome::CaughtUnwind,
                        ))
                {
                    return Err("typed Map unwind drifted from canonical Drop cleanup and custody");
                }
            }
        }
    }
    Ok(())
}

fn validate_frontier_preserved() -> Result<(), &'static str> {
    if !OPEN_FRONTIERS.contains(&ReviewedOpenFrontier::TypedMapOperation)
        || REVIEWED_TYPED_MAP_FRAGMENTS.iter().any(|cell| {
            cell.expected.provenance
                != ReviewedTypedMapProvenanceFragment::PendingRouteManagedPrestateAndCustody
                || cell.expected.expected_status != ExpectedStatus::PendingSourceAndRedTeamReview
        })
    {
        return Err("typed Map local fragment incorrectly closed managed provenance or review");
    }
    Ok(())
}
