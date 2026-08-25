use std::collections::BTreeSet;

use super::super::super::super::super::{
    abi_map_fragment::AbiNullWriteOutcome,
    adapter_projection_fragment::{
        ReviewedMapAdapterDispositionFragment, ReviewedMapAdapterOperationScopeFragment,
        ReviewedMapAdapterPayloadDispositionFragment,
        ReviewedMapAdapterPayloadGuardDispositionFragment, ReviewedMapAdapterPayloadGuardFragment,
        ReviewedMapAdapterProjectionFragment, ReviewedMapAdapterReviewState,
        REVIEWED_MAP_ADAPTER_PAYLOAD_GUARD_REVIEWS, REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS,
    },
    case_key::Path,
    projection::ExpectedStatus,
    raw_state_fragment::{RawCleanupEffect, RawPostOperationOutcome, INSTALLED_RAW_VALUES},
    route_callback_fragment::{
        ReviewedMapCallbackCompletionFragment, ReviewedMapOperationAdmissionFragment,
        ReviewedMapOperationResultFragment, ReviewedMapRouteCallbackPendingAxis,
        ReviewedMapRouteCallbackProjectionFragment, ReviewedMapRoutePreparationFragment,
        REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS,
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
    validate_exact_reviewed_cell_inventory()?;
    validate_failure_inheritance()?;
    validate_reviewed_success_cells()?;
    validate_local_cut()?;
    validate_provenance_refinement()?;
    validate_exact_payload_guard_reviews()?;
    super::adapter_projection_source_shapes::validate(steps)?;
    super::adapter_projection_witnesses::validate(steps)?;
    validate_open_frontiers_preserved()
}

fn validate_exact_reviewed_cell_inventory() -> Result<(), &'static str> {
    let actual = REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual.len() != REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS.len()
        || REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS.len() != 7
    {
        return Err("Map adapter projection fragment is not an exact seven reviewed-cell set");
    }

    let parent_failures = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
        .iter()
        .filter(|cell| {
            cell.expected.projection == ReviewedMapRouteCallbackProjectionFragment::TypedFailure
        })
        .count();
    let parent_continuations = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS.len() - parent_failures;
    if parent_failures != 5 || parent_continuations != 1 {
        return Err(
            "Map adapter projection parent no longer has five failures and one continuation",
        );
    }

    let inherited_failures = REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS
        .iter()
        .filter(|cell| {
            cell.expected.projection == ReviewedMapAdapterProjectionFragment::TypedFailure
        })
        .count();
    let reviewed_successes = REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS.len() - inherited_failures;
    if inherited_failures != 5 || reviewed_successes != 2 {
        return Err("Map adapter inventory is not five failures plus two reviewed success cells");
    }
    Ok(())
}

fn validate_failure_inheritance() -> Result<(), &'static str> {
    for parent in REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS.iter().filter(|cell| {
        cell.expected.projection == ReviewedMapRouteCallbackProjectionFragment::TypedFailure
    }) {
        let matches = REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS
            .iter()
            .filter(|cell| {
                cell.branch.route_callback == parent.branch
                    && cell.expected.projection
                        == ReviewedMapAdapterProjectionFragment::TypedFailure
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err("one parent route/callback failure does not have one composed child");
        }
        let child = matches[0];
        let adapter_not_reached =
            parent.branch.route_preparation == ReviewedMapRoutePreparationFragment::Rejected;
        let expected_adapter = if adapter_not_reached {
            ReviewedMapAdapterDispositionFragment::NotReached
        } else if parent.branch.operation_admission
            != ReviewedMapOperationAdmissionFragment::Accepted
        {
            ReviewedMapAdapterDispositionFragment::CallbackAdmissionRejectionDropped
        } else if parent.branch.operation_result == ReviewedMapOperationResultFragment::Error {
            ReviewedMapAdapterDispositionFragment::OperationRejectionDropped
        } else {
            ReviewedMapAdapterDispositionFragment::CallbackCompletionRejectionDropped
        };
        let expected_payload = if adapter_not_reached {
            ReviewedMapAdapterPayloadDispositionFragment::NotReached
        } else if parent.branch.operation_result == ReviewedMapOperationResultFragment::Ok
            && parent.branch.callback_completion == ReviewedMapCallbackCompletionFragment::Error
        {
            ReviewedMapAdapterPayloadDispositionFragment::SuccessPayloadDroppedBeforeAdapter
        } else {
            ReviewedMapAdapterPayloadDispositionFragment::NoOperationPayload
        };
        if child.branch.adapter_disposition != expected_adapter
            || child.branch.payload_disposition != expected_payload
            || child.expected.callback_completion_attempts
                != parent.expected.callback_completion_attempts
            || child.expected.output_at_cut != parent.expected.output_at_cut
            || child.expected.pointer_writes != parent.expected.pointer_writes
            || child.expected.sqlite_exit != ReviewedTypedMapExitFragment::ShmMapUnavailable
        {
            return Err(
                "a composed Map failure changed its parent cut or adapter error projection",
            );
        }
    }
    Ok(())
}

fn validate_reviewed_success_cells() -> Result<(), &'static str> {
    let parent = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
        .iter()
        .find(|cell| {
            cell.expected.projection
                == ReviewedMapRouteCallbackProjectionFragment::AdapterProjectionPending
        })
        .ok_or("Map adapter projection parent continuation is missing")?;
    let children = REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS
        .iter()
        .filter(|cell| cell.branch.route_callback == parent.branch)
        .collect::<Vec<_>>();
    if children.len() != 2 {
        return Err("the route/callback continuation lost one of its two reviewed success cells");
    }

    let outcomes = children
        .iter()
        .map(|cell| {
            (
                cell.branch.operation_scope,
                cell.branch.adapter_disposition,
                cell.branch.payload_disposition,
                cell.expected.projection,
                cell.expected.sqlite_exit,
                cell.expected.output_at_cut,
                cell.expected.pointer_writes,
            )
        })
        .collect::<BTreeSet<_>>();
    let expected = [
        (
            ReviewedMapAdapterOperationScopeFragment::ObserveOnly,
            ReviewedMapAdapterDispositionFragment::NotPresent,
            ReviewedMapAdapterPayloadDispositionFragment::NoPointerPayload,
            ReviewedMapAdapterProjectionFragment::TypedNotPresent,
            ReviewedTypedMapExitFragment::SqliteOkNotPresent,
            ReviewedTypedMapOutputFragment::NullRetained,
            0,
        ),
        (
            ReviewedMapAdapterOperationScopeFragment::ObserveOrExtend,
            ReviewedMapAdapterDispositionFragment::MappedAfterDefensiveGuards,
            ReviewedMapAdapterPayloadDispositionFragment::NonOwningPointerCarried,
            ReviewedMapAdapterProjectionFragment::TypedMapped,
            ReviewedTypedMapExitFragment::SqliteOkMapped,
            ReviewedTypedMapOutputFragment::MappedPointerWritten,
            1,
        ),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if outcomes != expected
        || children
            .iter()
            .any(|cell| cell.expected.callback_completion_attempts != 1)
    {
        return Err("Map adapter reviewed success cells changed outcome, payload, exit or write");
    }

    for (projection, typed_outcome) in [
        (
            ReviewedMapAdapterProjectionFragment::TypedNotPresent,
            ReviewedTypedMapOutcomeFragment::NotPresent,
        ),
        (
            ReviewedMapAdapterProjectionFragment::TypedMapped,
            ReviewedTypedMapOutcomeFragment::Mapped,
        ),
    ] {
        let adapter = children
            .iter()
            .find(|cell| cell.expected.projection == projection)
            .ok_or("Map adapter projected outcome is missing")?;
        let typed = REVIEWED_TYPED_MAP_FRAGMENTS
            .iter()
            .find(|cell| cell.branch.outcome == typed_outcome)
            .ok_or("Map typed outer outcome is missing below adapter projection")?;
        if adapter.expected.output_at_cut != typed.expected.output_at_cut
            || adapter.expected.sqlite_exit != typed.expected.sqlite_exit
        {
            return Err("Map adapter success projection drifted from the typed outer fragment");
        }
    }
    Ok(())
}

fn validate_local_cut() -> Result<(), &'static str> {
    let pointer_writers = REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS
        .iter()
        .filter(|cell| cell.expected.pointer_writes == 1)
        .count();
    if pointer_writers != 1 {
        return Err("Map adapter fragment no longer has exactly one ABI pointer writer");
    }
    for cell in REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS {
        if cell.branch.candidate_path != Path::Map
            || cell.branch.route_callback.candidate_path != Path::Map
            || cell.branch.route_callback.raw_post_operation
                != RawPostOperationOutcome::AcceptedNormalReturn
            || cell.expected.null_write_at_entry != AbiNullWriteOutcome::NullWritten
            || cell.expected.raw_slots_at_entry != INSTALLED_RAW_VALUES
            || cell.expected.raw_slots_at_cut != INSTALLED_RAW_VALUES
            || cell.expected.cleanup != RawCleanupEffect::None
            || cell.expected.expected_status != ExpectedStatus::PendingSourceAndRedTeamReview
        {
            return Err("Map adapter projection escaped its normal-return installed-state cut");
        }
    }
    Ok(())
}

fn validate_provenance_refinement() -> Result<(), &'static str> {
    for child in REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS {
        let parent = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
            .iter()
            .find(|cell| cell.branch == child.branch.route_callback)
            .ok_or("Map adapter child lost its route/callback parent")?;
        let route_rejected = child.branch.route_callback.route_preparation
            == ReviewedMapRoutePreparationFragment::Rejected;
        let managed_not_reached = child.branch.route_callback.operation_admission
            != ReviewedMapOperationAdmissionFragment::Accepted;
        let payload_custody_pending = child.branch.adapter_disposition
            == ReviewedMapAdapterDispositionFragment::MappedAfterDefensiveGuards
            || child.branch.payload_disposition
                == ReviewedMapAdapterPayloadDispositionFragment::SuccessPayloadDroppedBeforeAdapter;
        if child
            .expected
            .provenance
            .route_promotion_fault_internals_and_custody
            != ReviewedMapAdapterReviewState::Pending
            || child.expected.provenance.callback_owner_and_route_custody
                != if route_rejected {
                    ReviewedMapAdapterReviewState::NotReached
                } else {
                    ReviewedMapAdapterReviewState::Pending
                }
            || child
                .expected
                .provenance
                .managed_cause_prestate_and_retention
                != if managed_not_reached {
                    ReviewedMapAdapterReviewState::NotReached
                } else {
                    ReviewedMapAdapterReviewState::Pending
                }
            || child.expected.provenance.adapter_projection_control_flow
                != if route_rejected {
                    ReviewedMapAdapterReviewState::NotReached
                } else {
                    ReviewedMapAdapterReviewState::Reviewed
                }
            || child.expected.provenance.adapter_payload_custody
                != if route_rejected {
                    ReviewedMapAdapterReviewState::NotReached
                } else if payload_custody_pending {
                    ReviewedMapAdapterReviewState::Pending
                } else {
                    ReviewedMapAdapterReviewState::Reviewed
                }
        {
            return Err("Map adapter fragment over- or under-stated one provenance axis");
        }
        let parent_adapter = if route_rejected {
            ReviewedMapRouteCallbackPendingAxis::NotReached
        } else {
            ReviewedMapRouteCallbackPendingAxis::Pending
        };
        if parent.expected.provenance.adapter_projection_control_flow != parent_adapter
            || parent.expected.provenance.adapter_payload_custody != parent_adapter
        {
            return Err("Map route/callback parent no longer exposes both adapter Pending axes");
        }
    }
    Ok(())
}

fn validate_exact_payload_guard_reviews() -> Result<(), &'static str> {
    let actual = REVIEWED_MAP_ADAPTER_PAYLOAD_GUARD_REVIEWS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let expected = [
        (
            ReviewedMapAdapterPayloadGuardFragment::RegionMismatch,
            ReviewedMapAdapterPayloadGuardDispositionFragment::PendingSourceReview,
        ),
        (
            ReviewedMapAdapterPayloadGuardFragment::LengthMismatch,
            ReviewedMapAdapterPayloadGuardDispositionFragment::PendingSourceReview,
        ),
        (
            ReviewedMapAdapterPayloadGuardFragment::NullPointer,
            ReviewedMapAdapterPayloadGuardDispositionFragment::ExcludedByNonNullTypeEnvelope,
        ),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let normalized = actual
        .iter()
        .map(|record| (record.guard, record.disposition))
        .collect::<BTreeSet<_>>();
    if normalized != expected || actual.len() != 3 {
        return Err(
            "Map adapter payload guard review is not the exact pending/pending/excluded set",
        );
    }
    Ok(())
}

fn validate_open_frontiers_preserved() -> Result<(), &'static str> {
    if !OPEN_FRONTIERS.contains(&ReviewedOpenFrontier::TypedMapOperation)
        || !OPEN_FRONTIERS.contains(&ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection)
        || REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS
            .iter()
            .any(|cell| {
                cell.expected.expected_status != ExpectedStatus::PendingSourceAndRedTeamReview
            })
    {
        return Err("Map adapter fragment incorrectly closed an existing review frontier");
    }
    Ok(())
}
