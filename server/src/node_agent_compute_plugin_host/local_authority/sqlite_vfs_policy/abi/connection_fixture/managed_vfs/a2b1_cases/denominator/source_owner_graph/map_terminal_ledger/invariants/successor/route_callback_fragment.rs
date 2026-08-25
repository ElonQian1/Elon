use std::collections::BTreeSet;

use super::super::super::super::super::{
    abi_map_fragment::AbiNullWriteOutcome,
    case_key::Path,
    projection::ExpectedStatus,
    raw_state_fragment::{RawCleanupEffect, RawPostOperationOutcome, INSTALLED_RAW_VALUES},
    route_callback_fragment::{
        ReviewedMapCallbackCompletionFragment, ReviewedMapOperationAdmissionFragment,
        ReviewedMapOperationResultFragment, ReviewedMapOuterFaultIngressFragment,
        ReviewedMapRouteCallbackBranchFragment, ReviewedMapRouteCallbackExpectedFragment,
        ReviewedMapRouteCallbackFragmentCell, ReviewedMapRouteCallbackPendingAxis,
        ReviewedMapRouteCallbackProjectionFragment, ReviewedMapRouteCallbackProvenanceFragment,
        ReviewedMapRoutePreparationFragment, REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS,
    },
    typed_map_fragment::{
        ReviewedTypedMapExitFragment, ReviewedTypedMapOutcomeFragment,
        ReviewedTypedMapOutputFragment, REVIEWED_TYPED_MAP_FRAGMENTS,
    },
};
use super::super::super::{
    model::MapSourceStep,
    reviewed_trace::{ReviewedOpenFrontier, OPEN_FRONTIERS},
};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_exact_six_cell_quotient()?;
    validate_result_and_completion_algebra()?;
    validate_cumulative_pending_provenance()?;
    validate_local_cut_and_typed_projection()?;
    super::route_callback_witnesses::validate(steps)?;
    validate_open_frontiers_preserved()
}

fn validate_cumulative_pending_provenance() -> Result<(), &'static str> {
    for cell in REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS {
        let expected = match (
            cell.branch.operation_admission,
            cell.branch.operation_result,
            cell.expected.projection,
        ) {
            (
                ReviewedMapOperationAdmissionFragment::NotReached,
                ReviewedMapOperationResultFragment::NotRun,
                ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            ) => provenance(
                ReviewedMapRouteCallbackPendingAxis::NotReached,
                ReviewedMapRouteCallbackPendingAxis::NotReached,
                ReviewedMapRouteCallbackPendingAxis::NotReached,
            ),
            (
                ReviewedMapOperationAdmissionFragment::Rejected,
                ReviewedMapOperationResultFragment::NotRun,
                ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            ) => provenance(
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::NotReached,
                ReviewedMapRouteCallbackPendingAxis::Pending,
            ),
            (
                ReviewedMapOperationAdmissionFragment::Accepted,
                ReviewedMapOperationResultFragment::Error,
                ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            ) => provenance(
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
            ),
            (
                ReviewedMapOperationAdmissionFragment::Accepted,
                ReviewedMapOperationResultFragment::Ok,
                ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            )
            | (
                ReviewedMapOperationAdmissionFragment::Accepted,
                ReviewedMapOperationResultFragment::Ok,
                ReviewedMapRouteCallbackProjectionFragment::AdapterProjectionPending,
            ) => provenance(
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
            ),
            _ => return Err("Map route/callback provenance reached an impossible local branch"),
        };
        if cell.expected.provenance != expected {
            return Err("Map route/callback provenance lost a cumulative Pending axis");
        }
    }
    Ok(())
}

fn validate_exact_six_cell_quotient() -> Result<(), &'static str> {
    let expected = expected_cells().into_iter().collect::<BTreeSet<_>>();
    let actual = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual != expected
        || actual.len() != REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS.len()
        || REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS.len() != 6
    {
        return Err("Map route/callback fragment is not the exact six-cell local quotient");
    }
    Ok(())
}

fn validate_result_and_completion_algebra() -> Result<(), &'static str> {
    let route_rejections = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
        .iter()
        .filter(|cell| {
            cell.branch.route_preparation == ReviewedMapRoutePreparationFragment::Rejected
        })
        .count();
    let admission_rejections = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
        .iter()
        .filter(|cell| {
            cell.branch.operation_admission == ReviewedMapOperationAdmissionFragment::Rejected
        })
        .count();
    let admitted = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
        .iter()
        .filter(|cell| {
            cell.branch.operation_admission == ReviewedMapOperationAdmissionFragment::Accepted
        })
        .collect::<Vec<_>>();
    if route_rejections != 1 || admission_rejections != 1 || admitted.len() != 4 {
        return Err("Map route/callback fragment changed its 1 + 1 + 2x2 partition");
    }

    let admitted_product = admitted
        .iter()
        .map(|cell| {
            (
                cell.branch.operation_result,
                cell.branch.callback_completion,
            )
        })
        .collect::<BTreeSet<_>>();
    let expected_product = [
        (
            ReviewedMapOperationResultFragment::Error,
            ReviewedMapCallbackCompletionFragment::Error,
        ),
        (
            ReviewedMapOperationResultFragment::Error,
            ReviewedMapCallbackCompletionFragment::Ok,
        ),
        (
            ReviewedMapOperationResultFragment::Ok,
            ReviewedMapCallbackCompletionFragment::Error,
        ),
        (
            ReviewedMapOperationResultFragment::Ok,
            ReviewedMapCallbackCompletionFragment::Ok,
        ),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if admitted_product != expected_product {
        return Err("Map operation/callback fragment lost one admitted Result pair");
    }

    for cell in REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS {
        let is_admitted =
            cell.branch.operation_admission == ReviewedMapOperationAdmissionFragment::Accepted;
        if cell.expected.callback_completion_attempts != u8::from(is_admitted) {
            return Err("Map callback completion cardinality drifted from admission");
        }
        if cell.branch.operation_result == ReviewedMapOperationResultFragment::Error
            && cell.expected.projection != ReviewedMapRouteCallbackProjectionFragment::TypedFailure
        {
            return Err("Map operation error no longer wins callback completion");
        }
    }
    Ok(())
}

fn validate_local_cut_and_typed_projection() -> Result<(), &'static str> {
    let typed_failure = REVIEWED_TYPED_MAP_FRAGMENTS
        .iter()
        .find(|cell| cell.branch.outcome == ReviewedTypedMapOutcomeFragment::Failure)
        .ok_or("typed Map Failure projection is missing")?;
    if typed_failure.expected.output_at_cut != ReviewedTypedMapOutputFragment::NullRetained
        || typed_failure.expected.sqlite_exit != ReviewedTypedMapExitFragment::ShmMapUnavailable
    {
        return Err("typed Map Failure projection changed beneath route/callback fragment");
    }

    let continuations = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
        .iter()
        .filter(|cell| {
            cell.expected.projection
                == ReviewedMapRouteCallbackProjectionFragment::AdapterProjectionPending
        })
        .collect::<Vec<_>>();
    if continuations.len() != 1
        || continuations[0].branch.operation_result != ReviewedMapOperationResultFragment::Ok
        || continuations[0].branch.callback_completion != ReviewedMapCallbackCompletionFragment::Ok
    {
        return Err("only operation-Ok/completion-Ok may continue to adapter projection");
    }

    for cell in REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS {
        if cell.branch.candidate_path != Path::Map
            || cell.branch.raw_post_operation != RawPostOperationOutcome::AcceptedNormalReturn
            || cell.branch.outer_fault_ingress
                != ReviewedMapOuterFaultIngressFragment::PassedWithLiveInner
            || cell.expected.null_write_at_entry != AbiNullWriteOutcome::NullWritten
            || cell.expected.output_at_cut != ReviewedTypedMapOutputFragment::NullRetained
            || cell.expected.raw_slots_at_entry != INSTALLED_RAW_VALUES
            || cell.expected.raw_slots_at_cut != INSTALLED_RAW_VALUES
            || cell.expected.cleanup != RawCleanupEffect::None
            || cell.expected.pointer_writes != 0
            || cell.expected.expected_status != ExpectedStatus::PendingSourceAndRedTeamReview
        {
            return Err("Map route/callback fragment escaped its normal-return local cut");
        }
    }
    Ok(())
}

fn validate_open_frontiers_preserved() -> Result<(), &'static str> {
    if !OPEN_FRONTIERS.contains(&ReviewedOpenFrontier::TypedMapOperation)
        || !OPEN_FRONTIERS.contains(&ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection)
        || REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS.iter().any(|cell| {
            cell.expected.expected_status != ExpectedStatus::PendingSourceAndRedTeamReview
        })
    {
        return Err("Map route/callback fragment incorrectly closed an existing frontier");
    }
    Ok(())
}

fn expected_cells() -> [ReviewedMapRouteCallbackFragmentCell; 6] {
    [
        cell(
            ReviewedMapRoutePreparationFragment::Rejected,
            ReviewedMapOperationAdmissionFragment::NotReached,
            ReviewedMapOperationResultFragment::NotRun,
            ReviewedMapCallbackCompletionFragment::NotRun,
            0,
            ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            provenance(
                ReviewedMapRouteCallbackPendingAxis::NotReached,
                ReviewedMapRouteCallbackPendingAxis::NotReached,
                ReviewedMapRouteCallbackPendingAxis::NotReached,
            ),
        ),
        cell(
            ReviewedMapRoutePreparationFragment::Accepted,
            ReviewedMapOperationAdmissionFragment::Rejected,
            ReviewedMapOperationResultFragment::NotRun,
            ReviewedMapCallbackCompletionFragment::NotRun,
            0,
            ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            provenance(
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::NotReached,
                ReviewedMapRouteCallbackPendingAxis::Pending,
            ),
        ),
        admitted(
            ReviewedMapOperationResultFragment::Error,
            ReviewedMapCallbackCompletionFragment::Ok,
            ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            provenance(
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
            ),
        ),
        admitted(
            ReviewedMapOperationResultFragment::Error,
            ReviewedMapCallbackCompletionFragment::Error,
            ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            provenance(
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
            ),
        ),
        admitted(
            ReviewedMapOperationResultFragment::Ok,
            ReviewedMapCallbackCompletionFragment::Error,
            ReviewedMapRouteCallbackProjectionFragment::TypedFailure,
            provenance(
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
            ),
        ),
        admitted(
            ReviewedMapOperationResultFragment::Ok,
            ReviewedMapCallbackCompletionFragment::Ok,
            ReviewedMapRouteCallbackProjectionFragment::AdapterProjectionPending,
            provenance(
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
                ReviewedMapRouteCallbackPendingAxis::Pending,
            ),
        ),
    ]
}

fn admitted(
    operation_result: ReviewedMapOperationResultFragment,
    callback_completion: ReviewedMapCallbackCompletionFragment,
    projection: ReviewedMapRouteCallbackProjectionFragment,
    provenance: ReviewedMapRouteCallbackProvenanceFragment,
) -> ReviewedMapRouteCallbackFragmentCell {
    cell(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Accepted,
        operation_result,
        callback_completion,
        1,
        projection,
        provenance,
    )
}

fn cell(
    route_preparation: ReviewedMapRoutePreparationFragment,
    operation_admission: ReviewedMapOperationAdmissionFragment,
    operation_result: ReviewedMapOperationResultFragment,
    callback_completion: ReviewedMapCallbackCompletionFragment,
    callback_completion_attempts: u8,
    projection: ReviewedMapRouteCallbackProjectionFragment,
    provenance: ReviewedMapRouteCallbackProvenanceFragment,
) -> ReviewedMapRouteCallbackFragmentCell {
    ReviewedMapRouteCallbackFragmentCell {
        branch: ReviewedMapRouteCallbackBranchFragment {
            candidate_path: Path::Map,
            raw_post_operation: RawPostOperationOutcome::AcceptedNormalReturn,
            outer_fault_ingress: ReviewedMapOuterFaultIngressFragment::PassedWithLiveInner,
            route_preparation,
            operation_admission,
            operation_result,
            callback_completion,
        },
        expected: ReviewedMapRouteCallbackExpectedFragment {
            null_write_at_entry: AbiNullWriteOutcome::NullWritten,
            output_at_cut: ReviewedTypedMapOutputFragment::NullRetained,
            raw_slots_at_entry: INSTALLED_RAW_VALUES,
            raw_slots_at_cut: INSTALLED_RAW_VALUES,
            cleanup: RawCleanupEffect::None,
            callback_completion_attempts,
            pointer_writes: 0,
            projection,
            provenance,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

fn provenance(
    callback_owner_and_route_custody: ReviewedMapRouteCallbackPendingAxis,
    managed_cause_prestate_and_retention: ReviewedMapRouteCallbackPendingAxis,
    adapter_projection_and_payload_custody: ReviewedMapRouteCallbackPendingAxis,
) -> ReviewedMapRouteCallbackProvenanceFragment {
    ReviewedMapRouteCallbackProvenanceFragment {
        route_promotion_fault_internals_and_custody: ReviewedMapRouteCallbackPendingAxis::Pending,
        callback_owner_and_route_custody,
        managed_cause_prestate_and_retention,
        adapter_projection_control_flow: adapter_projection_and_payload_custody,
        adapter_payload_custody: adapter_projection_and_payload_custody,
    }
}
