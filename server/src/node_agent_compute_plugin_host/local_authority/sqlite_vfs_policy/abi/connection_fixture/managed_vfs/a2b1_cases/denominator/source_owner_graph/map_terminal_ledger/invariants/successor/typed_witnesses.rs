use std::collections::BTreeSet;

use super::super::super::super::super::typed_map_fragment::ReviewedTypedMapOutcomeFragment;
use super::super::super::super::model::{SourceEffect, SourceOwnerId};
use super::super::super::model::{
    MapExit, MapPendingReason, MapSiteId, MapSourceStep, MapSourceStepId, MapStepKind,
    MapValueFlow, SourceAnchor, MAP_BOTH, MAP_OBSERVE,
};
use super::route_callback_source_shapes::require_source_order;

const NORMAL_WITNESS_CHAINS: &[(ReviewedTypedMapOutcomeFragment, [MapSourceStepId; 3])] = &[
    (
        ReviewedTypedMapOutcomeFragment::NotPresent,
        [
            MapSourceStepId::AbiNotPresentProjection,
            MapSourceStepId::RawStateAccepted,
            MapSourceStepId::RawNormalCodeProjection,
        ],
    ),
    (
        ReviewedTypedMapOutcomeFragment::Mapped,
        [
            MapSourceStepId::AbiMappedProjection,
            MapSourceStepId::RawStateAccepted,
            MapSourceStepId::RawNormalCodeProjection,
        ],
    ),
    (
        ReviewedTypedMapOutcomeFragment::Failure,
        [
            MapSourceStepId::AbiFailureProjection,
            MapSourceStepId::RawStateAccepted,
            MapSourceStepId::RawNormalCodeProjection,
        ],
    ),
];

const UNWIND_WITNESS_CHAIN: &[MapSourceStepId] = &[
    MapSourceStepId::RawStateCaughtPanic,
    MapSourceStepId::RawAbandonUnwindFence,
    MapSourceStepId::RawAbandonStateWitnessRecorded,
    MapSourceStepId::RawAbandonInstalled,
    MapSourceStepId::RawFallbackProjection,
];

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_exact_abi_outer_set(steps)?;
    validate_normal_return_witnesses(steps)?;
    validate_unwind_witnesses(steps)?;
    validate_abi_result_step(
        require_step(steps, MapSourceStepId::AbiFailureProjection)?,
        MapSourceStepId::AbiFailureProjection,
    )?;
    validate_abi_result_step(
        require_step(steps, MapSourceStepId::AbiNotPresentProjection)?,
        MapSourceStepId::AbiNotPresentProjection,
    )?;
    validate_abi_result_step(
        require_step(steps, MapSourceStepId::AbiMappedProjection)?,
        MapSourceStepId::AbiMappedProjection,
    )?;
    validate_ordered_witness_chains(steps)
}

fn validate_exact_abi_outer_set(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let actual = steps
        .iter()
        .filter(|step| {
            step.site == MapSiteId::AbiProjection
                && step.anchor.owner == SourceOwnerId::AbiIoShm
                && step.anchor.symbol == "unsafe extern \"C\" fn map"
        })
        .map(|step| step.id)
        .collect::<BTreeSet<_>>();
    let expected = [
        MapSourceStepId::AbiFailureProjection,
        MapSourceStepId::AbiNotPresentProjection,
        MapSourceStepId::AbiMappedProjection,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err("typed Map fragment is not exact-linked to the three ABI result arms");
    }
    Ok(())
}

fn validate_normal_return_witnesses(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let accepted = require_step(steps, MapSourceStepId::RawStateAccepted)?;
    if accepted.site != MapSiteId::RawGate
        || accepted.anchor.owner != SourceOwnerId::AbiRawState
        || accepted.anchor.symbol != "unsafe fn with_installed_state"
        || accepted.anchor.needle != "Ok(unsafe { envelope.with_typed(operation) })"
        || accepted.anchor.occurrence != 1
        || accepted.ops != MAP_BOTH
        || accepted.effect != SourceEffect::None
        || accepted.value_flow != MapValueFlow::None
        || accepted.kind != MapStepKind::Continuation
    {
        return Err("typed Map normal-return raw witness changed shape");
    }
    validate_structural_witness(
        steps,
        MapSourceStepId::RawNormalCodeProjection,
        SourceOwnerId::AbiFileState,
        "unsafe fn run_code",
        "Ok(Ok(code)) => code",
        1,
        map_run_code_context(),
    )
}

fn validate_unwind_witnesses(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    require_source_order(
        SourceOwnerId::AbiRawState,
        "unsafe fn abandon_installed_state",
        &[
            "validate_installed(methods, state)?;",
            ".record_state_abandon();",
            "base.pMethods).write(ptr::null());",
            "drop(Box::from_raw",
        ],
    )?;

    let caught = require_step(steps, MapSourceStepId::RawStateCaughtPanic)?;
    if caught.site != MapSiteId::RawGate
        || caught.anchor.owner != SourceOwnerId::AbiFileState
        || caught.anchor.symbol != "unsafe fn run_code"
        || caught.anchor.needle != "| Err(_) =>"
        || caught.anchor.occurrence != 1
        || caught.ops != MAP_BOTH
        || caught.effect != SourceEffect::None
        || caught.value_flow != MapValueFlow::None
        || !matches!(
            caught.kind,
            MapStepKind::Pending {
                reason: MapPendingReason::RawFallbackCustodyProjection,
                ..
            }
        )
        || caught.call_context != Some(map_run_code_context())
    {
        return Err("typed Map caught-unwind raw witness changed shape");
    }

    validate_structural_witness(
        steps,
        MapSourceStepId::RawAbandonUnwindFence,
        SourceOwnerId::AbiFileState,
        "unsafe fn abandon_without_unwind",
        "let _ = catch_unwind(AssertUnwindSafe(||",
        1,
        run_code_abandon_context(),
    )?;

    let state_witness = require_step(steps, MapSourceStepId::RawAbandonStateWitnessRecorded)?;
    if state_witness.site != MapSiteId::RawAbandon
        || state_witness.anchor.owner != SourceOwnerId::AbiRawCloseWitness
        || state_witness.anchor.symbol != "fn record_state_abandon"
        || state_witness.anchor.needle != "self.record("
        || state_witness.anchor.occurrence != 1
        || state_witness.ops != MAP_BOTH
        || state_witness.effect != SourceEffect::None
        || state_witness.value_flow != MapValueFlow::None
        || state_witness.kind != MapStepKind::StructuralJoin
        || state_witness.call_context != Some(raw_abandon_witness_context())
    {
        return Err("typed Map unwind state-abandon witness changed shape");
    }

    let abandon = require_step(steps, MapSourceStepId::RawAbandonInstalled)?;
    if abandon.site != MapSiteId::RawAbandon
        || abandon.anchor.owner != SourceOwnerId::AbiRawState
        || abandon.anchor.symbol != "unsafe fn abandon_installed_state"
        || abandon.anchor.needle != "drop(Box::from_raw"
        || abandon.anchor.occurrence != 1
        || abandon.ops != MAP_BOTH
        || abandon.effect != SourceEffect::Cleanup
        || abandon.value_flow != MapValueFlow::None
        || !matches!(
            abandon.kind,
            MapStepKind::Pending {
                reason: MapPendingReason::RawFallbackCustodyProjection,
                ..
            }
        )
    {
        return Err("typed Map unwind abandonment witness changed shape");
    }

    validate_structural_witness(
        steps,
        MapSourceStepId::RawFallbackProjection,
        SourceOwnerId::AbiFileState,
        "unsafe fn run_code",
        "fallback",
        2,
        map_run_code_context(),
    )
}

fn validate_structural_witness(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
    owner: SourceOwnerId,
    symbol: &'static str,
    needle: &'static str,
    occurrence: u8,
    context: SourceAnchor,
) -> Result<(), &'static str> {
    let step = require_step(steps, id)?;
    if step.site != MapSiteId::AbiProjection
        || step.anchor
            != (SourceAnchor {
                owner,
                symbol,
                needle,
                occurrence,
            })
        || step.ops != MAP_BOTH
        || step.effect != SourceEffect::None
        || step.value_flow != MapValueFlow::None
        || step.kind != MapStepKind::StructuralJoin
        || step.call_context != Some(context)
    {
        return Err("typed Map outer wrapper projection witness changed shape");
    }
    Ok(())
}

fn validate_ordered_witness_chains(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let expected_normal = [
        (
            ReviewedTypedMapOutcomeFragment::NotPresent,
            [
                MapSourceStepId::AbiNotPresentProjection,
                MapSourceStepId::RawStateAccepted,
                MapSourceStepId::RawNormalCodeProjection,
            ],
        ),
        (
            ReviewedTypedMapOutcomeFragment::Mapped,
            [
                MapSourceStepId::AbiMappedProjection,
                MapSourceStepId::RawStateAccepted,
                MapSourceStepId::RawNormalCodeProjection,
            ],
        ),
        (
            ReviewedTypedMapOutcomeFragment::Failure,
            [
                MapSourceStepId::AbiFailureProjection,
                MapSourceStepId::RawStateAccepted,
                MapSourceStepId::RawNormalCodeProjection,
            ],
        ),
    ];
    let expected_unwind = [
        MapSourceStepId::RawStateCaughtPanic,
        MapSourceStepId::RawAbandonUnwindFence,
        MapSourceStepId::RawAbandonStateWitnessRecorded,
        MapSourceStepId::RawAbandonInstalled,
        MapSourceStepId::RawFallbackProjection,
    ];
    if NORMAL_WITNESS_CHAINS != expected_normal.as_slice()
        || UNWIND_WITNESS_CHAIN != expected_unwind.as_slice()
    {
        return Err("typed Map post-frontier source witness order changed");
    }
    for (_, chain) in NORMAL_WITNESS_CHAINS {
        for id in chain {
            require_step(steps, *id)?;
        }
    }
    for id in UNWIND_WITNESS_CHAIN {
        require_step(steps, *id)?;
    }
    Ok(())
}

fn validate_abi_result_step(
    step: &MapSourceStep,
    expected_id: MapSourceStepId,
) -> Result<(), &'static str> {
    if step.site != MapSiteId::AbiProjection
        || step.anchor.owner != SourceOwnerId::AbiIoShm
        || step.anchor.symbol != "unsafe extern \"C\" fn map"
        || step.effect
            != if expected_id == MapSourceStepId::AbiMappedProjection {
                SourceEffect::OutputPointer
            } else {
                SourceEffect::None
            }
    {
        return Err("typed Map ABI result witness changed its common source shape");
    }
    match expected_id {
        MapSourceStepId::AbiFailureProjection => {
            if step.anchor.needle != "Err(()) => result_codes::SHM_MAP_UNAVAILABLE"
                || step.anchor.occurrence != 1
                || step.ops != MAP_BOTH
                || step.value_flow != MapValueFlow::None
                || step.kind != MapStepKind::StructuralJoin
            {
                return Err("typed Map failure projection changed shape");
            }
        }
        MapSourceStepId::AbiNotPresentProjection => {
            if step.anchor.needle != "Ok(HandleBoundSqliteAbiShmMap::NotPresent) => ffi::SQLITE_OK"
                || step.anchor.occurrence != 1
                || step.ops != MAP_OBSERVE
                || step.value_flow != MapValueFlow::None
                || !matches!(step.kind, MapStepKind::Terminal(value) if value.exit == MapExit::AbiOkNotPresent)
            {
                return Err("typed Map NotPresent projection changed shape");
            }
        }
        MapSourceStepId::AbiMappedProjection => {
            if step.anchor.needle != "output.write(pointer.as_ptr())"
                || step.anchor.occurrence != 1
                || step.ops != MAP_BOTH
                || step.value_flow != MapValueFlow::AbiPointerWritten
                || !matches!(step.kind, MapStepKind::Terminal(value) if value.exit == MapExit::AbiOkMapped)
            {
                return Err("typed Map mapped projection changed shape");
            }
        }
        _ => return Err("typed Map ABI result validator received an unrelated witness"),
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
        .ok_or("typed Map fragment lost a required source-ledger witness")
}

fn map_run_code_context() -> SourceAnchor {
    SourceAnchor {
        owner: SourceOwnerId::AbiIoShm,
        symbol: "unsafe extern \"C\" fn map",
        needle: "file_state::run_code(",
        occurrence: 1,
    }
}

fn run_code_abandon_context() -> SourceAnchor {
    SourceAnchor {
        owner: SourceOwnerId::AbiFileState,
        symbol: "unsafe fn run_code",
        needle: "abandon_without_unwind(file)",
        occurrence: 1,
    }
}

fn raw_abandon_witness_context() -> SourceAnchor {
    SourceAnchor {
        owner: SourceOwnerId::AbiRawState,
        symbol: "unsafe fn abandon_installed_state",
        needle: ".record_state_abandon();",
        occurrence: 1,
    }
}
