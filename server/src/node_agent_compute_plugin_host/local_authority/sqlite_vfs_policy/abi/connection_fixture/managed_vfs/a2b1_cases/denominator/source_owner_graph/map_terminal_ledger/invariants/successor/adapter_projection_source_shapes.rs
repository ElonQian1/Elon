use super::super::super::super::{
    model::{Epoch, SourceEffect, SourceOwnerId},
    owners,
};
use super::super::super::model::{
    MapExclusionReason, MapExit, MapPendingReason, MapSiteId, MapSourceStep, MapSourceStepId,
    MapStepKind, MapValueFlow, MAP_BOTH, MAP_OBSERVE,
};
use super::super::anchors::source_symbol_span;

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_managed_pointer_shape(steps)?;
    validate_adapter_shapes(steps)?;
    validate_abi_shapes(steps)?;
    validate_exact_source_order()
}

fn validate_managed_pointer_shape(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_shape(
        steps,
        MapSourceStepId::ManagedMapped,
        MapSiteId::RegionSelection,
        SourceOwnerId::ManagedMapping,
        "fn map_connection",
        "Ok(ManagedSqliteShmMapOutcome::Mapped(",
        MAP_BOTH,
        Epoch::WalMainSteady,
        SourceEffect::None,
        MapValueFlow::TypedPointerCreated,
        MapStepKind::Continuation,
    )
}

fn validate_adapter_shapes(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_shape(
        steps,
        MapSourceStepId::AdapterNotPresent,
        MapSiteId::AbiProjection,
        SourceOwnerId::RegistryAbiFile,
        "fn shm_map",
        "ManagedSqliteShmMapOutcome::NotPresent =>",
        MAP_OBSERVE,
        Epoch::WalMainSteady,
        SourceEffect::None,
        MapValueFlow::None,
        MapStepKind::Continuation,
    )?;
    validate_shape(
        steps,
        MapSourceStepId::AdapterMapped,
        MapSiteId::AbiProjection,
        SourceOwnerId::RegistryAbiFile,
        "fn shm_map",
        "ManagedSqliteShmMapOutcome::Mapped(pointer) =>",
        MAP_BOTH,
        Epoch::WalMainSteady,
        SourceEffect::None,
        MapValueFlow::TypedPointerCarried,
        MapStepKind::Continuation,
    )?;
    for (id, needle) in [
        (
            MapSourceStepId::AdapterRegionMismatch,
            "pointer.region() != region",
        ),
        (
            MapSourceStepId::AdapterLengthMismatch,
            "pointer.length() != region_size.get() as usize",
        ),
        (
            MapSourceStepId::AdapterNullPointer,
            "NonNull::new(unsafe { pointer.as_mut_ptr() }.cast()).ok_or(())?",
        ),
    ] {
        let step = require_step(steps, id)?;
        validate_common_shape(
            step,
            MapSiteId::AbiProjection,
            SourceOwnerId::RegistryAbiFile,
            "fn shm_map",
            needle,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapValueFlow::None,
        )?;
        let keeps_parent_disposition = match id {
            MapSourceStepId::AdapterRegionMismatch => matches!(
                step.kind,
                MapStepKind::Pending {
                    reason: MapPendingReason::PrefixMutationSplit,
                    ..
                }
            ),
            MapSourceStepId::AdapterLengthMismatch | MapSourceStepId::AdapterNullPointer => {
                step.kind == MapStepKind::Excluded(MapExclusionReason::DefensiveCorruption)
            }
            _ => false,
        };
        if !keeps_parent_disposition {
            return Err("Map adapter guard changed its shared parent-ledger disposition");
        }
    }
    Ok(())
}

fn validate_abi_shapes(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_shape(
        steps,
        MapSourceStepId::AbiFailureProjection,
        MapSiteId::AbiProjection,
        SourceOwnerId::AbiIoShm,
        "unsafe extern \"C\" fn map",
        "Err(()) => result_codes::SHM_MAP_UNAVAILABLE",
        MAP_BOTH,
        Epoch::AbiInput,
        SourceEffect::None,
        MapValueFlow::None,
        MapStepKind::StructuralJoin,
    )?;
    let not_present = require_step(steps, MapSourceStepId::AbiNotPresentProjection)?;
    if !matches!(not_present.kind, MapStepKind::Terminal(value) if value.exit == MapExit::AbiOkNotPresent)
    {
        return Err("Map adapter NotPresent ABI projection is no longer terminal SQLITE_OK");
    }
    validate_common_shape(
        not_present,
        MapSiteId::AbiProjection,
        SourceOwnerId::AbiIoShm,
        "unsafe extern \"C\" fn map",
        "Ok(HandleBoundSqliteAbiShmMap::NotPresent) => ffi::SQLITE_OK",
        MAP_OBSERVE,
        Epoch::AbiInput,
        SourceEffect::None,
        MapValueFlow::None,
    )?;
    let mapped = require_step(steps, MapSourceStepId::AbiMappedProjection)?;
    if !matches!(mapped.kind, MapStepKind::Terminal(value) if value.exit == MapExit::AbiOkMapped) {
        return Err("Map adapter Mapped ABI projection is no longer terminal SQLITE_OK");
    }
    validate_common_shape(
        mapped,
        MapSiteId::AbiProjection,
        SourceOwnerId::AbiIoShm,
        "unsafe extern \"C\" fn map",
        "output.write(pointer.as_ptr())",
        MAP_BOTH,
        Epoch::AbiInput,
        SourceEffect::OutputPointer,
        MapValueFlow::AbiPointerWritten,
    )
}

fn validate_exact_source_order() -> Result<(), &'static str> {
    require_source_order(
        SourceOwnerId::ManagedTypes,
        "pub(crate) struct ManagedSqliteShmRegionPointer",
        &["pointer: NonNull<u8>", "length: usize", "region: u32"],
    )?;
    require_source_order(
        SourceOwnerId::ManagedTypes,
        "pub(super) fn new(\n        pointer: NonNull<u8>,\n        length: usize,\n        region: u32,\n        runtime_generation: NonZeroU64,",
        &[
            "Self {",
            "pointer,",
            "length,",
            "region,",
        ],
    )?;
    require_source_order(
        SourceOwnerId::ManagedTypes,
        "unsafe fn as_mut_ptr",
        &["self.pointer.as_ptr()"],
    )?;
    require_source_order(SourceOwnerId::ManagedTypes, "fn length", &["self.length"])?;
    require_source_order(SourceOwnerId::ManagedTypes, "fn region", &["self.region"])?;
    require_source_order(
        SourceOwnerId::ManagedMapping,
        "fn map_connection",
        &[
            "let logical_length = usize::try_from(region_size.get())",
            "let logical_pointer = unsafe { NonNull::new_unchecked(base.as_ptr().add(shift)) }",
            "logical_pointer: Some(logical_pointer)",
            "let selected = node.regions.get(region as usize).and_then(|selected| {",
            ".logical_pointer",
            ".map(|pointer| (pointer, selected.logical_length))",
            "let Some((pointer, logical_length)) = selected else",
            "Ok(ManagedSqliteShmMapOutcome::Mapped(",
            "ManagedSqliteShmRegionPointer::new(pointer, logical_length, region, self.generation)",
        ],
    )?;
    require_source_order(
        SourceOwnerId::RegistryAbiFile,
        "fn shm_map",
        &[
            "self.file.shm_map(region, region_size, mode).map_err(drop)?",
            "ManagedSqliteShmMapOutcome::NotPresent =>",
            "ManagedSqliteShmMapOutcome::Mapped(pointer) =>",
            "pointer.region() != region",
            "pointer.length() != region_size.get() as usize",
            "return Err(());",
            "NonNull::new(unsafe { pointer.as_mut_ptr() }.cast()).ok_or(())?",
            "Ok(HandleBoundSqliteAbiShmMap::Mapped(pointer))",
        ],
    )?;
    require_source_order(
        SourceOwnerId::AbiIoShm,
        "unsafe extern \"C\" fn map",
        &[
            "file_state::run_code(",
            "Ok(HandleBoundSqliteAbiShmMap::NotPresent) => ffi::SQLITE_OK",
            "Ok(HandleBoundSqliteAbiShmMap::Mapped(pointer)) =>",
            "output.write(pointer.as_ptr())",
            "Err(()) => result_codes::SHM_MAP_UNAVAILABLE",
        ],
    )?;

    let managed_mapping = owners::source_content(SourceOwnerId::ManagedMapping);
    if managed_mapping
        .matches("ManagedSqliteShmRegionPointer::new(")
        .count()
        != 1
    {
        return Err(
            "reviewed ManagedMapping owner no longer has one lexical pointer constructor call",
        );
    }
    let adapter = source_symbol_span(
        owners::source_content(SourceOwnerId::RegistryAbiFile),
        "fn shm_map",
    )
    .ok_or("Map adapter source symbol span is missing")?;
    if adapter.matches("ManagedSqliteShmMapOutcome::").count() != 2
        || adapter.matches("HandleBoundSqliteAbiShmMap::").count() != 2
        || adapter.matches("return Err(());").count() != 1
    {
        return Err(
            "Map adapter source is no longer the exact two-outcome one-rejection projection",
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_shape(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
    site: MapSiteId,
    owner: SourceOwnerId,
    symbol: &'static str,
    needle: &'static str,
    ops: &'static [super::super::super::super::model::PathOp],
    epoch: Epoch,
    effect: SourceEffect,
    value_flow: MapValueFlow,
    kind: MapStepKind,
) -> Result<(), &'static str> {
    let step = require_step(steps, id)?;
    validate_common_shape(
        step, site, owner, symbol, needle, ops, epoch, effect, value_flow,
    )?;
    if step.kind != kind {
        return Err("Map adapter source witness changed its reviewed disposition");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_common_shape(
    step: &MapSourceStep,
    site: MapSiteId,
    owner: SourceOwnerId,
    symbol: &'static str,
    needle: &'static str,
    ops: &'static [super::super::super::super::model::PathOp],
    epoch: Epoch,
    effect: SourceEffect,
    value_flow: MapValueFlow,
) -> Result<(), &'static str> {
    if step.site != site
        || step.anchor.owner != owner
        || step.anchor.symbol != symbol
        || step.anchor.needle != needle
        || step.anchor.occurrence != 1
        || step.call_context.is_some()
        || step.ops != ops
        || step.epoch != epoch
        || step.effect != effect
        || step.value_flow != value_flow
    {
        return Err("Map adapter source witness changed its exact reviewed shape");
    }
    Ok(())
}

fn require_source_order(
    owner: SourceOwnerId,
    symbol: &'static str,
    needles: &[&'static str],
) -> Result<(), &'static str> {
    let mut tail = source_symbol_span(owners::source_content(owner), symbol)
        .ok_or("Map adapter ordered source symbol span is missing")?;
    for needle in needles {
        let offset = tail
            .find(needle)
            .ok_or("Map adapter source needles are absent or reordered")?;
        tail = tail
            .get(offset + needle.len()..)
            .ok_or("Map adapter ordered source suffix is invalid")?;
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
        .ok_or("Map adapter fragment lost a required source-ledger witness")
}
