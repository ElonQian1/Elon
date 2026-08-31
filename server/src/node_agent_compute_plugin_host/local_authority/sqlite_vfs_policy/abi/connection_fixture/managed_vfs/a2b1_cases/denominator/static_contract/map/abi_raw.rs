use super::{
    super::{
        model::{DecisionStage, ExclusionProof, Expected, RootOperation},
        terminal_descriptor::{
            FaultSeamV1, MapAbiScalarV1, MapAxesV1, MapOperationV1, MapPrestateV1, OccurrenceV1,
            PhaseV1, PresenceV1, SourceSiteV1, StimulusV1, TimingV1, ValidityV1,
        },
    },
    builder::MapGraphBuilder,
    dynamic::DescriptorSeedV1,
    witnesses as w,
};

mod defensive;

pub(super) const ROOT: &str = "map.abi.output-null-initialization";

pub(super) struct ModeEntries {
    pub(super) observe: String,
    pub(super) extend: String,
}

pub(super) fn build(graph: &mut MapGraphBuilder) -> ModeEntries {
    graph.decision(
        ROOT,
        w::boundary("unsafe { output.write(ptr::null_mut()) };"),
    );
    let invalid_pointer = graph.excluded(
        "map.abi.excluded.invalid-output-pointer",
        ExclusionProof::SafetyPremise(
            "a non-null SQLite output slot must be live, aligned, writable and non-aliasing",
        ),
        w::boundary("unsafe { output.write(ptr::null_mut()) };"),
    );
    graph.edge(
        ROOT,
        &invalid_pointer,
        DecisionStage::AbiValidation,
        "non_null_output_violates_C_memory_contract",
    );

    let absent = graph.decision(
        "map.abi.scalar.output-absent",
        w::abi(
            "unsafe extern \"C\" fn map",
            "let (Ok(region), Some(region_size), Some(extend))",
        ),
    );
    let present = graph.decision(
        "map.abi.scalar.output-present",
        w::abi(
            "unsafe extern \"C\" fn map",
            "let (Ok(region), Some(region_size), Some(extend))",
        ),
    );
    graph.edge(
        ROOT,
        &absent,
        DecisionStage::AbiValidation,
        "output_slot_absent",
    );
    graph.edge(
        ROOT,
        &present,
        DecisionStage::AbiValidation,
        "output_slot_present_valid",
    );
    add_scalar_cells(graph, &absent, "absent", false);
    let raw = add_scalar_cells(graph, &present, "present", true)
        .expect("present valid scalar cell continues");
    let typed = defensive::build(graph, &raw);
    let mode = graph.decision("map.typed.adapter-mode", w::adapter("let mode = if extend"));
    graph.edge(
        &typed,
        &mode,
        DecisionStage::Adapter,
        "expected_type_operation_enters_adapter",
    );
    let observe = graph.decision(
        "map.observe.typed-entry",
        w::adapter("ManagedSqliteShmMapMode::Observe"),
    );
    let extend = graph.decision(
        "map.extend.typed-entry",
        w::adapter("ManagedSqliteShmMapMode::Extend"),
    );
    graph.edge(&mode, &observe, DecisionStage::Adapter, "observe");
    graph.edge(&mode, &extend, DecisionStage::Adapter, "extend");
    ModeEntries { observe, extend }
}

fn add_scalar_cells(
    graph: &mut MapGraphBuilder,
    from: &str,
    output: &str,
    can_continue: bool,
) -> Option<String> {
    for (mask, region, region_size, extend) in [
        (
            "invalid-region",
            ValidityV1::Invalid,
            ValidityV1::Valid,
            ValidityV1::Valid,
        ),
        (
            "invalid-region-size",
            ValidityV1::Valid,
            ValidityV1::Invalid,
            ValidityV1::Valid,
        ),
        (
            "invalid-region-and-size",
            ValidityV1::Invalid,
            ValidityV1::Invalid,
            ValidityV1::Valid,
        ),
        (
            "invalid-extend",
            ValidityV1::Valid,
            ValidityV1::Valid,
            ValidityV1::Invalid,
        ),
        (
            "invalid-region-and-extend",
            ValidityV1::Invalid,
            ValidityV1::Valid,
            ValidityV1::Invalid,
        ),
        (
            "invalid-size-and-extend",
            ValidityV1::Valid,
            ValidityV1::Invalid,
            ValidityV1::Invalid,
        ),
        (
            "invalid-region-size-and-extend",
            ValidityV1::Invalid,
            ValidityV1::Invalid,
            ValidityV1::Invalid,
        ),
    ] {
        let id = format!("map.abi.terminal.{output}.{mask}");
        graph.terminal(
            &id,
            Expected::unavailable(RootOperation::Map, "AbiValidation"),
            abi_scalar_descriptor(
                if can_continue {
                    PresenceV1::Present
                } else {
                    PresenceV1::Absent
                },
                region,
                region_size,
                extend,
            ),
            w::abi(
                "unsafe extern \"C\" fn map",
                "return result_codes::SHM_MAP_UNAVAILABLE;",
            ),
        );
        graph.edge(from, &id, DecisionStage::AbiValidation, mask);
    }
    if !can_continue {
        let id = "map.abi.terminal.output-absent.valid-scalars";
        graph.terminal(
            id,
            Expected::unavailable(RootOperation::Map, "AbiValidation"),
            abi_scalar_descriptor(
                PresenceV1::Absent,
                ValidityV1::Valid,
                ValidityV1::Valid,
                ValidityV1::Valid,
            ),
            w::abi("unsafe extern \"C\" fn map", "if output.is_null()"),
        );
        graph.edge(from, id, DecisionStage::AbiValidation, "valid_scalars");
        return None;
    }
    let raw = graph.decision(
        "map.raw.valid-input-entry",
        w::file_state(
            "unsafe fn run_code",
            "raw_state::with_installed_state(file, operation)",
        ),
    );
    graph.edge(from, &raw, DecisionStage::AbiValidation, "valid_scalars");
    Some(raw)
}

fn abi_scalar_descriptor(
    output: PresenceV1,
    region: ValidityV1,
    region_size: ValidityV1,
    extend: ValidityV1,
) -> super::super::terminal_descriptor::TerminalDescriptorV1 {
    DescriptorSeedV1::new(
        SourceSiteV1::MapAbiBoundary,
        StimulusV1::MapAbi(MapAbiScalarV1 {
            output,
            region,
            region_size,
            extend,
        }),
        MapPrestateV1::NotReached,
        MapOperationV1::AbiValidation,
        PhaseV1::AbiValidation,
        TimingV1::BeforeCall,
        OccurrenceV1::Natural,
        FaultSeamV1::AbiBoundary,
        MapAxesV1::NOT_REACHED,
    )
    .direct()
}
