use std::collections::BTreeSet;

use super::super::super::super::super::{
    case_key::Path,
    raw_state_fragment::RawPostOperationOutcome,
    route_callback_fragment::{
        ReviewedMapCallbackCompletionFragment, ReviewedMapOperationAdmissionFragment,
        ReviewedMapOperationResultFragment, ReviewedMapOuterFaultIngressFragment,
        ReviewedMapRouteCallbackBranchFragment, ReviewedMapRoutePreparationFragment,
        REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS,
    },
};
use super::super::super::super::model::SourceOwnerId;
use super::super::super::model::{MapSiteId, MapSourceStep, MapSourceStepId};

const ROUTE_REJECTION_CHAIN: &[MapSourceStepId] = &[
    MapSourceStepId::OuterFaultPass,
    MapSourceStepId::RoutePreparationResultGate,
    MapSourceStepId::AbiFailureProjection,
    MapSourceStepId::RawStateAccepted,
    MapSourceStepId::RawNormalCodeProjection,
];

const ADMISSION_REJECTION_CHAIN: &[MapSourceStepId] = &[
    MapSourceStepId::OuterFaultPass,
    MapSourceStepId::RoutePreparationResultGate,
    MapSourceStepId::RouteOperationDispatch,
    MapSourceStepId::BridgeOperationDispatch,
    MapSourceStepId::AdapterOperationDispatch,
    MapSourceStepId::RegistryOperationDispatch,
    MapSourceStepId::OperationAdmissionRejected,
    MapSourceStepId::AbiFailureProjection,
    MapSourceStepId::RawStateAccepted,
    MapSourceStepId::RawNormalCodeProjection,
];

const OPERATION_ERROR_CHAIN: &[MapSourceStepId] = &[
    MapSourceStepId::OuterFaultPass,
    MapSourceStepId::RoutePreparationResultGate,
    MapSourceStepId::RouteOperationDispatch,
    MapSourceStepId::BridgeOperationDispatch,
    MapSourceStepId::AdapterOperationDispatch,
    MapSourceStepId::RegistryOperationDispatch,
    MapSourceStepId::OperationManagedFailure,
    MapSourceStepId::OperationCompletionAttempt,
    MapSourceStepId::OperationCompletionResultDomain,
    MapSourceStepId::OperationCompletionDelegate,
    MapSourceStepId::OperationErrorWinsCompletion,
    MapSourceStepId::AbiFailureProjection,
    MapSourceStepId::RawStateAccepted,
    MapSourceStepId::RawNormalCodeProjection,
];

const COMPLETION_REJECTION_CHAIN: &[MapSourceStepId] = &[
    MapSourceStepId::OuterFaultPass,
    MapSourceStepId::RoutePreparationResultGate,
    MapSourceStepId::RouteOperationDispatch,
    MapSourceStepId::BridgeOperationDispatch,
    MapSourceStepId::AdapterOperationDispatch,
    MapSourceStepId::RegistryOperationDispatch,
    MapSourceStepId::OperationCompletionAttempt,
    MapSourceStepId::OperationCompletionResultDomain,
    MapSourceStepId::OperationCompletionDelegate,
    MapSourceStepId::OperationCompletionRejected,
    MapSourceStepId::AbiFailureProjection,
    MapSourceStepId::RawStateAccepted,
    MapSourceStepId::RawNormalCodeProjection,
];

const ADAPTER_PROJECTION_CHAIN: &[MapSourceStepId] = &[
    MapSourceStepId::OuterFaultPass,
    MapSourceStepId::RoutePreparationResultGate,
    MapSourceStepId::RouteOperationDispatch,
    MapSourceStepId::BridgeOperationDispatch,
    MapSourceStepId::AdapterOperationDispatch,
    MapSourceStepId::RegistryOperationDispatch,
    MapSourceStepId::OperationCompletionAttempt,
    MapSourceStepId::OperationCompletionResultDomain,
    MapSourceStepId::OperationCompletionDelegate,
    MapSourceStepId::OperationCompleted,
];

const EXACT_FIXTURE_EXCLUSIONS: &[MapSourceStepId] = &[
    MapSourceStepId::OperationUnsupportedRole,
    MapSourceStepId::OperationShmDetached,
];

const CONDITIONAL_CUSTODY_STEPS: &[MapSourceStepId] = &[MapSourceStepId::OperationUnsafeRetain];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RouteCallbackWitnessChain {
    branch: ReviewedMapRouteCallbackBranchFragment,
    source_steps: &'static [MapSourceStepId],
}

const ROUTE_CALLBACK_WITNESS_CHAINS: &[RouteCallbackWitnessChain] = &[
    witness(
        ReviewedMapRoutePreparationFragment::Rejected,
        ReviewedMapOperationAdmissionFragment::NotReached,
        ReviewedMapOperationResultFragment::NotRun,
        ReviewedMapCallbackCompletionFragment::NotRun,
        ROUTE_REJECTION_CHAIN,
    ),
    witness(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Rejected,
        ReviewedMapOperationResultFragment::NotRun,
        ReviewedMapCallbackCompletionFragment::NotRun,
        ADMISSION_REJECTION_CHAIN,
    ),
    witness(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Accepted,
        ReviewedMapOperationResultFragment::Error,
        ReviewedMapCallbackCompletionFragment::Ok,
        OPERATION_ERROR_CHAIN,
    ),
    witness(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Accepted,
        ReviewedMapOperationResultFragment::Error,
        ReviewedMapCallbackCompletionFragment::Error,
        OPERATION_ERROR_CHAIN,
    ),
    witness(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Accepted,
        ReviewedMapOperationResultFragment::Ok,
        ReviewedMapCallbackCompletionFragment::Error,
        COMPLETION_REJECTION_CHAIN,
    ),
    witness(
        ReviewedMapRoutePreparationFragment::Accepted,
        ReviewedMapOperationAdmissionFragment::Accepted,
        ReviewedMapOperationResultFragment::Ok,
        ReviewedMapCallbackCompletionFragment::Ok,
        ADAPTER_PROJECTION_CHAIN,
    ),
];

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_exact_branch_witness_partition(steps)?;
    validate_exact_slice_step_partition(steps)?;
    super::route_callback_source_shapes::validate(steps)
}

fn validate_exact_branch_witness_partition(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let fragment_branches = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
        .iter()
        .map(|cell| cell.branch)
        .collect::<BTreeSet<_>>();
    let witness_branches = ROUTE_CALLBACK_WITNESS_CHAINS
        .iter()
        .map(|witness| witness.branch)
        .collect::<BTreeSet<_>>();
    if fragment_branches != witness_branches
        || witness_branches.len() != ROUTE_CALLBACK_WITNESS_CHAINS.len()
        || ROUTE_CALLBACK_WITNESS_CHAINS.len() != 6
    {
        return Err("Map route/callback source witnesses do not cover the exact six branches");
    }

    for witness in ROUTE_CALLBACK_WITNESS_CHAINS {
        if !has_exact_canonical_chain(witness) {
            return Err("Map route/callback branch lost its exact canonical source witness chain");
        }
        let unique = witness
            .source_steps
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if unique.len() != witness.source_steps.len() {
            return Err("Map route/callback witness repeats a source step");
        }
        for id in witness.source_steps {
            require_step(steps, *id)?;
        }

        let expected_attempts = REVIEWED_MAP_ROUTE_CALLBACK_FRAGMENTS
            .iter()
            .find(|cell| cell.branch == witness.branch)
            .ok_or("Map route/callback witness lost its fragment cell")?
            .expected
            .callback_completion_attempts;
        let witnessed_attempts = witness
            .source_steps
            .iter()
            .filter(|id| **id == MapSourceStepId::OperationCompletionAttempt)
            .count();
        if witnessed_attempts != usize::from(expected_attempts) {
            return Err("Map route/callback completion attempts diverged from source witnesses");
        }
    }
    Ok(())
}

fn validate_exact_slice_step_partition(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_operation_callback_owners(steps)?;
    let branch_steps = ROUTE_CALLBACK_WITNESS_CHAINS
        .iter()
        .flat_map(|witness| witness.source_steps.iter().copied())
        .collect::<BTreeSet<_>>();
    let exclusions = EXACT_FIXTURE_EXCLUSIONS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let conditional_custody = CONDITIONAL_CUSTODY_STEPS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if !branch_steps.is_disjoint(&exclusions)
        || !branch_steps.is_disjoint(&conditional_custody)
        || !exclusions.is_disjoint(&conditional_custody)
    {
        return Err("Map route/callback source-step partitions overlap");
    }
    let expected = branch_steps
        .union(&exclusions)
        .copied()
        .collect::<BTreeSet<_>>()
        .union(&conditional_custody)
        .copied()
        .collect::<BTreeSet<_>>();
    let actual = steps
        .iter()
        .filter(|step| is_route_callback_slice_step(step))
        .map(|step| step.id)
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err("Map route/callback ledger slice is not exactly partitioned");
    }
    Ok(())
}

fn validate_operation_callback_owners(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    if steps
        .iter()
        .filter(|step| step.site == MapSiteId::OperationCallback)
        .any(|step| {
            !matches!(
                step.anchor.owner,
                SourceOwnerId::FixtureRouteFile
                    | SourceOwnerId::RegistryTestBridge
                    | SourceOwnerId::RegistryAbiFile
                    | SourceOwnerId::RegistryOperations
                    | SourceOwnerId::RegistryProcessOwner
            )
        })
    {
        return Err("Map route/callback ledger gained an unreviewed callback owner");
    }
    Ok(())
}

fn is_route_callback_slice_step(step: &MapSourceStep) -> bool {
    matches!(
        step.id,
        MapSourceStepId::OuterFaultPass
            | MapSourceStepId::RawStateAccepted
            | MapSourceStepId::RawNormalCodeProjection
            | MapSourceStepId::AbiFailureProjection
    ) || (step.site == MapSiteId::RoutePlan
        && step.anchor.owner == SourceOwnerId::FixtureRouteFile
        && step.anchor.symbol == "fn shm_map")
        || step.site == MapSiteId::OperationCallback
}

fn has_exact_canonical_chain(witness: &RouteCallbackWitnessChain) -> bool {
    match (
        witness.branch.route_preparation,
        witness.branch.operation_admission,
        witness.branch.operation_result,
        witness.branch.callback_completion,
    ) {
        (
            ReviewedMapRoutePreparationFragment::Rejected,
            ReviewedMapOperationAdmissionFragment::NotReached,
            ReviewedMapOperationResultFragment::NotRun,
            ReviewedMapCallbackCompletionFragment::NotRun,
        ) => {
            witness.source_steps
                == &[
                    MapSourceStepId::OuterFaultPass,
                    MapSourceStepId::RoutePreparationResultGate,
                    MapSourceStepId::AbiFailureProjection,
                    MapSourceStepId::RawStateAccepted,
                    MapSourceStepId::RawNormalCodeProjection,
                ]
        }
        (
            ReviewedMapRoutePreparationFragment::Accepted,
            ReviewedMapOperationAdmissionFragment::Rejected,
            ReviewedMapOperationResultFragment::NotRun,
            ReviewedMapCallbackCompletionFragment::NotRun,
        ) => {
            witness.source_steps
                == &[
                    MapSourceStepId::OuterFaultPass,
                    MapSourceStepId::RoutePreparationResultGate,
                    MapSourceStepId::RouteOperationDispatch,
                    MapSourceStepId::BridgeOperationDispatch,
                    MapSourceStepId::AdapterOperationDispatch,
                    MapSourceStepId::RegistryOperationDispatch,
                    MapSourceStepId::OperationAdmissionRejected,
                    MapSourceStepId::AbiFailureProjection,
                    MapSourceStepId::RawStateAccepted,
                    MapSourceStepId::RawNormalCodeProjection,
                ]
        }
        (
            ReviewedMapRoutePreparationFragment::Accepted,
            ReviewedMapOperationAdmissionFragment::Accepted,
            ReviewedMapOperationResultFragment::Error,
            ReviewedMapCallbackCompletionFragment::Ok
            | ReviewedMapCallbackCompletionFragment::Error,
        ) => {
            witness.source_steps
                == &[
                    MapSourceStepId::OuterFaultPass,
                    MapSourceStepId::RoutePreparationResultGate,
                    MapSourceStepId::RouteOperationDispatch,
                    MapSourceStepId::BridgeOperationDispatch,
                    MapSourceStepId::AdapterOperationDispatch,
                    MapSourceStepId::RegistryOperationDispatch,
                    MapSourceStepId::OperationManagedFailure,
                    MapSourceStepId::OperationCompletionAttempt,
                    MapSourceStepId::OperationCompletionResultDomain,
                    MapSourceStepId::OperationCompletionDelegate,
                    MapSourceStepId::OperationErrorWinsCompletion,
                    MapSourceStepId::AbiFailureProjection,
                    MapSourceStepId::RawStateAccepted,
                    MapSourceStepId::RawNormalCodeProjection,
                ]
        }
        (
            ReviewedMapRoutePreparationFragment::Accepted,
            ReviewedMapOperationAdmissionFragment::Accepted,
            ReviewedMapOperationResultFragment::Ok,
            ReviewedMapCallbackCompletionFragment::Error,
        ) => {
            witness.source_steps
                == &[
                    MapSourceStepId::OuterFaultPass,
                    MapSourceStepId::RoutePreparationResultGate,
                    MapSourceStepId::RouteOperationDispatch,
                    MapSourceStepId::BridgeOperationDispatch,
                    MapSourceStepId::AdapterOperationDispatch,
                    MapSourceStepId::RegistryOperationDispatch,
                    MapSourceStepId::OperationCompletionAttempt,
                    MapSourceStepId::OperationCompletionResultDomain,
                    MapSourceStepId::OperationCompletionDelegate,
                    MapSourceStepId::OperationCompletionRejected,
                    MapSourceStepId::AbiFailureProjection,
                    MapSourceStepId::RawStateAccepted,
                    MapSourceStepId::RawNormalCodeProjection,
                ]
        }
        (
            ReviewedMapRoutePreparationFragment::Accepted,
            ReviewedMapOperationAdmissionFragment::Accepted,
            ReviewedMapOperationResultFragment::Ok,
            ReviewedMapCallbackCompletionFragment::Ok,
        ) => {
            witness.source_steps
                == &[
                    MapSourceStepId::OuterFaultPass,
                    MapSourceStepId::RoutePreparationResultGate,
                    MapSourceStepId::RouteOperationDispatch,
                    MapSourceStepId::BridgeOperationDispatch,
                    MapSourceStepId::AdapterOperationDispatch,
                    MapSourceStepId::RegistryOperationDispatch,
                    MapSourceStepId::OperationCompletionAttempt,
                    MapSourceStepId::OperationCompletionResultDomain,
                    MapSourceStepId::OperationCompletionDelegate,
                    MapSourceStepId::OperationCompleted,
                ]
        }
        _ => false,
    }
}

fn require_step(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
) -> Result<&MapSourceStep, &'static str> {
    steps
        .iter()
        .find(|step| step.id == id)
        .ok_or("Map route/callback fragment lost a required source-ledger witness")
}

const fn witness(
    route_preparation: ReviewedMapRoutePreparationFragment,
    operation_admission: ReviewedMapOperationAdmissionFragment,
    operation_result: ReviewedMapOperationResultFragment,
    callback_completion: ReviewedMapCallbackCompletionFragment,
    source_steps: &'static [MapSourceStepId],
) -> RouteCallbackWitnessChain {
    RouteCallbackWitnessChain {
        branch: ReviewedMapRouteCallbackBranchFragment {
            candidate_path: Path::Map,
            raw_post_operation: RawPostOperationOutcome::AcceptedNormalReturn,
            outer_fault_ingress: ReviewedMapOuterFaultIngressFragment::PassedWithLiveInner,
            route_preparation,
            operation_admission,
            operation_result,
            callback_completion,
        },
        source_steps,
    }
}
