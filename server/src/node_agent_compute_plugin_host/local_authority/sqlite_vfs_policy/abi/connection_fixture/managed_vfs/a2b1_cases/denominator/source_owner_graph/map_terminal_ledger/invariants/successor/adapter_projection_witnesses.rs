use super::super::super::super::super::adapter_projection_fragment::{
    ReviewedMapAdapterDispositionFragment, ReviewedMapAdapterPayloadDispositionFragment,
    ReviewedMapAdapterProjectionFragment, ReviewedMapAdapterReviewState,
    REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS,
};
use super::super::super::model::{
    MapSourceStep, MapSourceStepId, MapValueFlow, MAP_BOTH, MAP_OBSERVE,
};

const NOT_PRESENT_WITNESS_CHAIN: &[MapSourceStepId] = &[
    MapSourceStepId::ObserveNotPresent,
    MapSourceStepId::OperationCompletionAttempt,
    MapSourceStepId::OperationCompletionResultDomain,
    MapSourceStepId::OperationCompletionDelegate,
    MapSourceStepId::OperationCompleted,
    MapSourceStepId::AdapterNotPresent,
    MapSourceStepId::AbiNotPresentProjection,
    MapSourceStepId::RawStateAccepted,
    MapSourceStepId::RawNormalCodeProjection,
];

const MAPPED_WITNESS_CHAIN: &[MapSourceStepId] = &[
    MapSourceStepId::ManagedMapped,
    MapSourceStepId::OperationCompletionAttempt,
    MapSourceStepId::OperationCompletionResultDomain,
    MapSourceStepId::OperationCompletionDelegate,
    MapSourceStepId::OperationCompleted,
    MapSourceStepId::AdapterMapped,
    MapSourceStepId::AbiMappedProjection,
    MapSourceStepId::RawStateAccepted,
    MapSourceStepId::RawNormalCodeProjection,
];

const FAILURE_OUTER_PROJECTION: &[MapSourceStepId] = &[
    MapSourceStepId::AbiFailureProjection,
    MapSourceStepId::RawStateAccepted,
    MapSourceStepId::RawNormalCodeProjection,
];

const MAPPED_VALUE_FLOW: &[(MapSourceStepId, MapValueFlow)] = &[
    (
        MapSourceStepId::ManagedMapped,
        MapValueFlow::TypedPointerCreated,
    ),
    (
        MapSourceStepId::AdapterMapped,
        MapValueFlow::TypedPointerCarried,
    ),
    (
        MapSourceStepId::AbiMappedProjection,
        MapValueFlow::AbiPointerWritten,
    ),
];

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_exact_witness_inventories()?;
    validate_witness_presence(steps)?;
    validate_mapped_value_flow(steps)?;
    validate_fragment_payload_boundary()?;
    validate_operation_scopes(steps)?;
    validate_normal_return_projection(steps)
}

fn validate_fragment_payload_boundary() -> Result<(), &'static str> {
    let dropped = REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS
        .iter()
        .find(|cell| {
            cell.branch.payload_disposition
                == ReviewedMapAdapterPayloadDispositionFragment::SuccessPayloadDroppedBeforeAdapter
        })
        .ok_or("Map adapter fragment lost the operation-Ok/completion-Err branch")?;
    if dropped.expected.provenance.adapter_payload_custody != ReviewedMapAdapterReviewState::Pending
    {
        return Err("Map adapter dropped-success payload custody was incorrectly closed");
    }

    let mapped = REVIEWED_MAP_ADAPTER_PROJECTION_FRAGMENTS
        .iter()
        .find(|cell| cell.expected.projection == ReviewedMapAdapterProjectionFragment::TypedMapped)
        .ok_or("Map adapter fragment lost its mapped value-flow cell")?;
    if mapped.branch.adapter_disposition
        != ReviewedMapAdapterDispositionFragment::MappedAfterDefensiveGuards
        || mapped.expected.provenance.adapter_payload_custody
            != ReviewedMapAdapterReviewState::Pending
    {
        return Err("Map adapter mapped cell is detached from its pending payload witness");
    }
    Ok(())
}

fn validate_exact_witness_inventories() -> Result<(), &'static str> {
    let expected_not_present = [
        MapSourceStepId::ObserveNotPresent,
        MapSourceStepId::OperationCompletionAttempt,
        MapSourceStepId::OperationCompletionResultDomain,
        MapSourceStepId::OperationCompletionDelegate,
        MapSourceStepId::OperationCompleted,
        MapSourceStepId::AdapterNotPresent,
        MapSourceStepId::AbiNotPresentProjection,
        MapSourceStepId::RawStateAccepted,
        MapSourceStepId::RawNormalCodeProjection,
    ];
    let expected_mapped = [
        MapSourceStepId::ManagedMapped,
        MapSourceStepId::OperationCompletionAttempt,
        MapSourceStepId::OperationCompletionResultDomain,
        MapSourceStepId::OperationCompletionDelegate,
        MapSourceStepId::OperationCompleted,
        MapSourceStepId::AdapterMapped,
        MapSourceStepId::AbiMappedProjection,
        MapSourceStepId::RawStateAccepted,
        MapSourceStepId::RawNormalCodeProjection,
    ];
    let expected_failure = [
        MapSourceStepId::AbiFailureProjection,
        MapSourceStepId::RawStateAccepted,
        MapSourceStepId::RawNormalCodeProjection,
    ];
    if NOT_PRESENT_WITNESS_CHAIN != expected_not_present.as_slice()
        || MAPPED_WITNESS_CHAIN != expected_mapped.as_slice()
        || FAILURE_OUTER_PROJECTION != expected_failure.as_slice()
    {
        return Err("Map adapter composed witness inventory changed");
    }
    Ok(())
}

fn validate_witness_presence(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    for chain in [
        NOT_PRESENT_WITNESS_CHAIN,
        MAPPED_WITNESS_CHAIN,
        FAILURE_OUTER_PROJECTION,
    ] {
        for id in chain {
            require_step(steps, *id)?;
        }
    }
    Ok(())
}

fn validate_mapped_value_flow(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    for (id, flow) in MAPPED_VALUE_FLOW {
        if require_step(steps, *id)?.value_flow != *flow {
            return Err("Map adapter mapped pointer changed create/carry/write stage");
        }
    }
    for id in [
        MapSourceStepId::ObserveNotPresent,
        MapSourceStepId::AdapterNotPresent,
        MapSourceStepId::AbiNotPresentProjection,
        MapSourceStepId::AbiFailureProjection,
    ] {
        if require_step(steps, id)?.value_flow != MapValueFlow::None {
            return Err("Map adapter no-pointer outcome unexpectedly carries an ABI pointer");
        }
    }
    Ok(())
}

fn validate_operation_scopes(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    for id in [
        MapSourceStepId::ObserveNotPresent,
        MapSourceStepId::AdapterNotPresent,
        MapSourceStepId::AbiNotPresentProjection,
    ] {
        if require_step(steps, id)?.ops != MAP_OBSERVE {
            return Err("Map adapter NotPresent escaped the Observe-only scope");
        }
    }
    for id in [
        MapSourceStepId::ManagedMapped,
        MapSourceStepId::AdapterMapped,
        MapSourceStepId::AbiMappedProjection,
        MapSourceStepId::AdapterRegionMismatch,
        MapSourceStepId::AdapterLengthMismatch,
        MapSourceStepId::AdapterNullPointer,
    ] {
        if require_step(steps, id)?.ops != MAP_BOTH {
            return Err("Map adapter mapped projection lost Observe/Extend coverage");
        }
    }
    Ok(())
}

fn validate_normal_return_projection(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let accepted = require_step(steps, MapSourceStepId::RawStateAccepted)?;
    let forward = require_step(steps, MapSourceStepId::RawNormalCodeProjection)?;
    if accepted.value_flow != MapValueFlow::None
        || forward.value_flow != MapValueFlow::None
        || accepted.ops != MAP_BOTH
        || forward.ops != MAP_BOTH
    {
        return Err("Map adapter normal-return wrapper witness changed shape");
    }
    Ok(())
}

fn require_step(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
) -> Result<&MapSourceStep, &'static str> {
    steps
        .iter()
        .find(|step| step.id == id)
        .ok_or("Map adapter witness references a missing source-ledger step")
}
