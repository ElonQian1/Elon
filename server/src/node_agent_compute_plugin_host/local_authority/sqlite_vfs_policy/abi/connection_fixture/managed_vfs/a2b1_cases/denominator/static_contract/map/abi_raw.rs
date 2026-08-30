use super::{
    super::model::{DecisionStage, ExclusionProof, Expected, RootOperation},
    builder::MapGraphBuilder,
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
    for mask in [
        "invalid-region",
        "invalid-region-size",
        "invalid-region-and-size",
        "invalid-extend",
        "invalid-region-and-extend",
        "invalid-size-and-extend",
        "invalid-region-size-and-extend",
    ] {
        let id = format!("map.abi.terminal.{output}.{mask}");
        graph.terminal(
            &id,
            Expected::unavailable(RootOperation::Map, "AbiValidation"),
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
