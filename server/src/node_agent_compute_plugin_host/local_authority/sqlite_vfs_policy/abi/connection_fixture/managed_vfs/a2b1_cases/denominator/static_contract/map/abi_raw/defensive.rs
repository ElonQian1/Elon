use super::super::{
    super::{
        model::{
            CustodyState, DecisionStage, ExclusionProof, Expected, RootOperation,
            TerminalDisposition,
        },
        terminal_descriptor::{
            FaultSeamV1, MapAxesV1, MapOperationV1, MapPrestateV1, OccurrenceV1, PhaseV1,
            RawStateV1, SourceSiteV1, StimulusV1, TimingV1,
        },
    },
    builder::MapGraphBuilder,
    dynamic::DescriptorSeedV1,
    expected, witnesses as w,
};

mod abandon;

const PREFIX: &str = "map.raw";

// The private production field/type domains, rather than any cfg(test) constructor, define these
// defensive callback inputs. Safe producers choose Some + the HandleBound type, while callbacks
// still fail closed for every memory-safe Option/type state admitted by the raw representation.

pub(super) fn build(graph: &mut MapGraphBuilder, raw_input: &str) -> String {
    let entry = graph.decision(
        &format!("{PREFIX}.admission"),
        w::raw(
            "unsafe fn with_installed_state",
            "installed_envelope(file)?",
        ),
    );
    graph.edge(
        raw_input,
        &entry,
        DecisionStage::RawAdmission,
        "protected_raw_admission",
    );
    add_pointer_exclusions(graph, &entry);
    add_validation_rejections(graph, &entry);
    add_envelope_domain(graph, &entry)
}

fn add_pointer_exclusions(graph: &mut MapGraphBuilder, entry: &str) {
    let invalid_file = graph.excluded(
        &format!("{PREFIX}.excluded.invalid-file-pointer"),
        ExclusionProof::SafetyPremise(
            "non-null file must name a live aligned serialized sqlite3_file allocation",
        ),
        w::raw(
            "unsafe fn installed_envelope",
            "NonNull::new(file.cast::<InertHandleBoundSqliteFile>())",
        ),
    );
    graph.edge(
        entry,
        &invalid_file,
        DecisionStage::RawAdmission,
        "file_pointer_violates_C_memory_contract",
    );
    let invalid_envelope = graph.excluded(
        &format!("{PREFIX}.excluded.invalid-envelope-pointer"),
        ExclusionProof::SafetyPremise(
            "exact methods plus non-null state must name a live aligned RawSqliteFileStateEnvelope installed by this module",
        ),
        w::raw(
            "unsafe fn installed_envelope",
            ".cast::<RawSqliteFileStateEnvelope>()",
        ),
    );
    graph.edge(
        entry,
        &invalid_envelope,
        DecisionStage::RawAdmission,
        "state_pointer_violates_envelope_memory_contract",
    );
}

fn add_validation_rejections(graph: &mut MapGraphBuilder, entry: &str) {
    for (shape, raw_state, source, abandon_source, slots) in [
        (
            "null-file",
            RawStateV1::NullFile,
            w::raw(
                "unsafe fn installed_envelope",
                "RawSqliteFileStateRejection::NullFile",
            ),
            w::raw(
                "pub(super) unsafe fn abandon_installed_state",
                "RawSqliteFileStateRejection::NullFile",
            ),
            CustodyState::NotReached,
        ),
        (
            "uninstalled",
            RawStateV1::Uninstalled,
            w::raw(
                "fn validate_installed",
                "RawSqliteFileStateRejection::Uninstalled",
            ),
            w::raw(
                "pub(super) unsafe fn abandon_installed_state",
                "if methods.is_null() && state.is_null()",
            ),
            CustodyState::Cleared,
        ),
        (
            "methods-null-state-present",
            RawStateV1::MethodsNullStatePresent,
            w::raw(
                "fn validate_installed",
                "RawSqliteFileStateRejection::ForeignMethods",
            ),
            w::raw(
                "pub(super) unsafe fn abandon_installed_state",
                "validate_installed(methods, state)?;",
            ),
            CustodyState::Retained,
        ),
        (
            "foreign-methods-state-null",
            RawStateV1::ForeignMethodsStateNull,
            w::raw(
                "fn validate_installed",
                "if !ptr::eq(methods, &INERT_IO_METHODS)",
            ),
            w::raw(
                "pub(super) unsafe fn abandon_installed_state",
                "validate_installed(methods, state)?;",
            ),
            CustodyState::Retained,
        ),
        (
            "foreign-methods-state-present",
            RawStateV1::ForeignMethodsStatePresent,
            w::raw(
                "fn validate_installed",
                "if !ptr::eq(methods, &INERT_IO_METHODS)",
            ),
            w::raw(
                "pub(super) unsafe fn abandon_installed_state",
                "validate_installed(methods, state)?;",
            ),
            CustodyState::Retained,
        ),
        (
            "exact-methods-state-null",
            RawStateV1::ExactMethodsStateNull,
            w::raw(
                "fn validate_installed",
                "RawSqliteFileStateRejection::StateMissing",
            ),
            w::raw(
                "pub(super) unsafe fn abandon_installed_state",
                "validate_installed(methods, state)?;",
            ),
            CustodyState::Retained,
        ),
    ] {
        add_rejected_slots(
            graph,
            entry,
            shape,
            raw_state,
            source,
            abandon_source,
            slots,
        );
    }
}

fn add_rejected_slots(
    graph: &mut MapGraphBuilder,
    entry: &str,
    shape: &str,
    raw_state: RawStateV1,
    source: super::super::super::source::SourceWitness,
    abandon_source: super::super::super::source::SourceWitness,
    slots: CustodyState,
) {
    let cause = graph.decision(&format!("{PREFIX}.{shape}.cause"), source);
    graph.edge(entry, &cause, DecisionStage::RawAdmission, shape);
    let fallback = abandon::add_fallback_boundary(graph, &cause, shape);
    let rejected = graph.decision(
        &format!("{PREFIX}.{shape}.abandon-validation"),
        abandon_source,
    );
    graph.edge(
        &fallback,
        &rejected,
        DecisionStage::RawAbandon,
        "abandon_validation_rejected_or_noop",
    );
    let terminal = format!("{PREFIX}.terminal.{shape}");
    let mut value = expected::raw_fallback(slots, CustodyState::NotReached);
    value.disposition = TerminalDisposition::Returned;
    graph.terminal(
        &terminal,
        value,
        raw_descriptor(raw_state, MapOperationV1::RawAbandon, TimingV1::Cleanup).direct(),
        w::file_state("unsafe fn run_code", "Ok(Err(_)) | Err(_) =>"),
    );
    graph.edge(
        &rejected,
        &terminal,
        DecisionStage::RawAbandon,
        "fallback_returned_with_slots_not_owned",
    );
}

fn add_envelope_domain(graph: &mut MapGraphBuilder, entry: &str) -> String {
    let producer = graph.decision(
        &format!("{PREFIX}.envelope.producer-domain"),
        w::raw(
            "fn new<State: 'static>",
            "drop_payload: drop_typed_payload::<State>",
        ),
    );
    graph.edge(
        entry,
        &producer,
        DecisionStage::RawAdmission,
        "exact_methods_live_envelope",
    );
    let type_domain = graph.decision(
        &format!("{PREFIX}.envelope.type-domain"),
        w::raw(
            "fn is<State: 'static>",
            "self.type_id == TypeId::of::<State>()",
        ),
    );
    graph.edge(
        &producer,
        &type_domain,
        DecisionStage::RawAdmission,
        "generic_installer_type_id_domain",
    );
    add_other_type(graph, &type_domain);
    add_expected_type(graph, &type_domain)
}

fn add_other_type(graph: &mut MapGraphBuilder, type_domain: &str) {
    let payload = payload_domain(graph, "type-mismatch");
    graph.edge(
        type_domain,
        &payload,
        DecisionStage::RawAdmission,
        "defensive_other_installed_state_type",
    );
    abandon::add_envelope(
        graph,
        &payload,
        "type-mismatch.payload-missing",
        "payload_missing",
        w::raw(
            "pub(super) unsafe fn with_installed_state",
            "return Err(RawSqliteFileStateRejection::TypeMismatch);",
        ),
        false,
        RawStateV1::OtherTypePayloadMissing,
        RawStateV1::OtherTypePayloadPresent,
    );
    abandon::add_envelope(
        graph,
        &payload,
        "type-mismatch.payload-present",
        "payload_present",
        w::raw(
            "pub(super) unsafe fn with_installed_state",
            "return Err(RawSqliteFileStateRejection::TypeMismatch);",
        ),
        true,
        RawStateV1::OtherTypePayloadMissing,
        RawStateV1::OtherTypePayloadPresent,
    );
}

fn add_expected_type(graph: &mut MapGraphBuilder, type_domain: &str) -> String {
    let payload = payload_domain(graph, "expected-type");
    graph.edge(
        type_domain,
        &payload,
        DecisionStage::RawAdmission,
        "expected_handle_bound_state_type",
    );
    abandon::add_envelope(
        graph,
        &payload,
        "expected-type.payload-missing",
        "payload_missing",
        w::raw(
            "unsafe fn with_typed<State: 'static, Output>",
            "expect(\"live raw SQLite state envelope must retain its payload\")",
        ),
        false,
        RawStateV1::ExpectedTypePayloadMissing,
        RawStateV1::ExpectedTypePayloadMissing,
    );
    let typed = graph.decision(
        &format!("{PREFIX}.typed-operation"),
        w::raw(
            "pub(super) unsafe fn with_installed_state",
            "envelope.with_typed(operation)",
        ),
    );
    graph.edge(
        &payload,
        &typed,
        DecisionStage::RawAdmission,
        "payload_present",
    );
    add_handle_bound_file_domain(graph, &typed)
}

fn payload_domain(graph: &mut MapGraphBuilder, label: &str) -> String {
    graph.decision(
        &format!("{PREFIX}.{label}.payload-domain"),
        w::raw(
            "struct RawSqliteFileStateEnvelope",
            "payload: Option<NonNull<c_void>>",
        ),
    )
}

fn add_handle_bound_file_domain(graph: &mut MapGraphBuilder, typed: &str) -> String {
    let producer = graph.decision(
        &format!("{PREFIX}.handle-bound-file.producer"),
        w::file_state("fn from_compute_plugin", "file: Some(Box::new(file))"),
    );
    graph.edge(
        typed,
        &producer,
        DecisionStage::Adapter,
        "typed_payload_borrowed",
    );
    let file_domain = graph.decision(
        &format!("{PREFIX}.handle-bound-file.domain"),
        w::file_state(
            "struct HandleBoundSqliteFileState",
            "file: Option<Box<dyn HandleBoundSqliteFileOperations>>",
        ),
    );
    graph.edge(
        &producer,
        &file_domain,
        DecisionStage::Adapter,
        "production_some_plus_defensive_none_domain",
    );
    let file_mut = graph.decision(
        &format!("{PREFIX}.handle-bound-file.file-mut"),
        w::file_state("fn file_mut", "self.file.as_deref_mut().ok_or(())"),
    );
    graph.edge(
        &file_domain,
        &file_mut,
        DecisionStage::Adapter,
        "callback_invokes_file_mut",
    );
    let missing_projection = graph.decision(
        &format!("{PREFIX}.handle-bound-file-missing.adapter-projection"),
        w::abi(
            "unsafe extern \"C\" fn map",
            "Err(()) => result_codes::SHM_MAP_UNAVAILABLE",
        ),
    );
    graph.edge(
        &file_mut,
        &missing_projection,
        DecisionStage::Adapter,
        "file_missing_returns_err",
    );
    let run_code_return = graph.decision(
        &format!("{PREFIX}.handle-bound-file-missing.run-code-return"),
        w::file_state("unsafe fn run_code", "Ok(Ok(code)) => code"),
    );
    graph.edge(
        &missing_projection,
        &run_code_return,
        DecisionStage::AbiProjection,
        "adapter_error_projected_to_fallback_code",
    );
    let terminal = format!("{PREFIX}.terminal.handle-bound-file-missing");
    let mut missing = Expected::unavailable(RootOperation::Map, "Adapter");
    missing.raw_slots = CustodyState::Unchanged;
    missing.payload = CustodyState::Retained;
    missing.file = CustodyState::Cleared;
    graph.terminal(
        &terminal,
        missing,
        raw_descriptor(
            RawStateV1::HandleBoundFileMissing,
            MapOperationV1::AdapterDispatch,
            TimingV1::AtCall,
        )
        .direct(),
        w::file_state("unsafe fn run_code", "Ok(Ok(code)) => code"),
    );
    graph.edge(
        &run_code_return,
        &terminal,
        DecisionStage::AbiProjection,
        "run_code_returns_fallback_code_without_abandon",
    );
    let present = graph.decision(
        &format!("{PREFIX}.handle-bound-file-present"),
        w::abi(
            "unsafe extern \"C\" fn map",
            "|state| match state.shm_map(region, region_size, extend)",
        ),
    );
    graph.edge(
        &file_mut,
        &present,
        DecisionStage::Adapter,
        "handle_bound_file_present",
    );
    present
}

fn raw_descriptor(
    state: RawStateV1,
    operation: MapOperationV1,
    timing: TimingV1,
) -> DescriptorSeedV1 {
    DescriptorSeedV1::new(
        if matches!(operation, MapOperationV1::AdapterDispatch) {
            SourceSiteV1::AdapterDispatch
        } else {
            SourceSiteV1::RawStateAbandon
        },
        StimulusV1::MapRaw(state),
        MapPrestateV1::NotReached,
        operation,
        if matches!(operation, MapOperationV1::AdapterDispatch) {
            PhaseV1::Adapter
        } else {
            PhaseV1::RawAdmission
        },
        timing,
        OccurrenceV1::Natural,
        FaultSeamV1::RawState,
        MapAxesV1::NOT_REACHED,
    )
}
