mod abandon;

use super::super::{
    super::{
        model::{CustodyState, DecisionStage, ExclusionProof, Expected, RootOperation},
        source::{witness, ProductionOwner, SourceWitness},
    },
    builder::Builder,
    outcome,
};

const PREFIX: &str = "lock.raw";

// The private production field/type domains, rather than any cfg(test) constructor, define these
// defensive callback inputs. Safe producers choose Some + the HandleBound type, while callbacks
// still fail closed for every memory-safe Option/type state admitted by the raw representation.

pub(super) fn build(builder: &mut Builder, raw: &str) -> String {
    add_pointer_exclusions(builder, raw);
    add_validation_rejections(builder, raw);
    add_envelope_domain(builder, raw)
}

fn add_pointer_exclusions(builder: &mut Builder, raw: &str) {
    for (label, proof, source) in [
        (
            "invalid-file-pointer",
            "non-null file must name a live aligned serialized sqlite3_file allocation",
            raw_witness(
                "unsafe fn installed_envelope",
                "NonNull::new(file.cast::<InertHandleBoundSqliteFile>())",
                1,
            ),
        ),
        (
            "invalid-envelope-pointer",
            "exact methods plus non-null state must name a live aligned RawSqliteFileStateEnvelope installed by this module",
            raw_witness(
                "unsafe fn installed_envelope",
                ".cast::<RawSqliteFileStateEnvelope>()",
                1,
            ),
        ),
    ] {
        let excluded = builder.excluded(
            format!("{PREFIX}.excluded.{label}"),
            ExclusionProof::SafetyPremise(proof),
            source,
        );
        builder.edge(
            raw,
            &excluded,
            DecisionStage::RawAdmission,
            format!("{label}-violates-C-memory-contract"),
        );
    }
}

fn add_validation_rejections(builder: &mut Builder, raw: &str) {
    for (shape, source, abandon_source, slots) in [
        (
            "null-file",
            raw_witness(
                "unsafe fn installed_envelope",
                "RawSqliteFileStateRejection::NullFile",
                1,
            ),
            raw_witness(
                "pub(super) unsafe fn abandon_installed_state",
                "RawSqliteFileStateRejection::NullFile",
                1,
            ),
            CustodyState::NotReached,
        ),
        (
            "uninstalled",
            raw_witness(
                "fn validate_installed",
                "RawSqliteFileStateRejection::Uninstalled",
                1,
            ),
            raw_witness(
                "pub(super) unsafe fn abandon_installed_state",
                "if methods.is_null() && state.is_null()",
                1,
            ),
            CustodyState::Cleared,
        ),
        (
            "methods-null-state-present",
            raw_witness(
                "fn validate_installed",
                "RawSqliteFileStateRejection::ForeignMethods",
                1,
            ),
            abandon_validation_witness(),
            CustodyState::Retained,
        ),
        (
            "foreign-methods-state-null",
            raw_witness(
                "fn validate_installed",
                "RawSqliteFileStateRejection::ForeignMethods",
                2,
            ),
            abandon_validation_witness(),
            CustodyState::Retained,
        ),
        (
            "foreign-methods-state-present",
            raw_witness(
                "fn validate_installed",
                "RawSqliteFileStateRejection::ForeignMethods",
                2,
            ),
            abandon_validation_witness(),
            CustodyState::Retained,
        ),
        (
            "exact-methods-state-null",
            raw_witness(
                "fn validate_installed",
                "RawSqliteFileStateRejection::StateMissing",
                1,
            ),
            abandon_validation_witness(),
            CustodyState::Retained,
        ),
    ] {
        abandon::add_rejected_slots(builder, raw, shape, source, abandon_source, slots);
    }
}

fn add_envelope_domain(builder: &mut Builder, raw: &str) -> String {
    let producer = builder.decision(
        format!("{PREFIX}.envelope.producer-domain"),
        raw_witness(
            "fn new<State: 'static>",
            "drop_payload: drop_typed_payload::<State>",
            1,
        ),
    );
    builder.edge(
        raw,
        &producer,
        DecisionStage::RawAdmission,
        "exact_methods_live_envelope",
    );
    let type_domain = builder.decision(
        format!("{PREFIX}.envelope.type-domain"),
        raw_witness(
            "fn is<State: 'static>",
            "self.type_id == TypeId::of::<State>()",
            1,
        ),
    );
    builder.edge(
        &producer,
        &type_domain,
        DecisionStage::RawAdmission,
        "generic_installer_type_id_domain",
    );
    add_other_type(builder, &type_domain);
    add_expected_type(builder, &type_domain)
}

fn add_other_type(builder: &mut Builder, type_domain: &str) {
    let payload = payload_domain(builder, "type-mismatch");
    builder.edge(
        type_domain,
        &payload,
        DecisionStage::RawAdmission,
        "defensive_other_installed_state_type",
    );
    let mismatch = raw_witness(
        "pub(super) unsafe fn with_installed_state",
        "return Err(RawSqliteFileStateRejection::TypeMismatch);",
        1,
    );
    abandon::add_envelope(
        builder,
        &payload,
        "type-mismatch.payload-missing",
        "payload_missing",
        mismatch,
        false,
    );
    abandon::add_envelope(
        builder,
        &payload,
        "type-mismatch.payload-present",
        "payload_present",
        mismatch,
        true,
    );
}

fn add_expected_type(builder: &mut Builder, type_domain: &str) -> String {
    let payload = payload_domain(builder, "expected-type");
    builder.edge(
        type_domain,
        &payload,
        DecisionStage::RawAdmission,
        "expected_handle_bound_state_type",
    );
    abandon::add_envelope(
        builder,
        &payload,
        "expected-type.payload-missing",
        "payload_missing",
        raw_witness(
            "unsafe fn with_typed<State: 'static, Output>",
            "expect(\"live raw SQLite state envelope must retain its payload\")",
            1,
        ),
        false,
    );
    let typed = builder.decision(
        format!("{PREFIX}.typed-operation"),
        raw_witness(
            "pub(super) unsafe fn with_installed_state",
            "envelope.with_typed(operation)",
            1,
        ),
    );
    builder.edge(
        &payload,
        &typed,
        DecisionStage::RawAdmission,
        "payload_present",
    );
    add_file_domain(builder, &typed)
}

fn payload_domain(builder: &mut Builder, label: &str) -> String {
    builder.decision(
        format!("{PREFIX}.{label}.payload-domain"),
        raw_witness(
            "struct RawSqliteFileStateEnvelope",
            "payload: Option<NonNull<c_void>>",
            1,
        ),
    )
}

fn add_file_domain(builder: &mut Builder, typed: &str) -> String {
    let producer = builder.decision(
        format!("{PREFIX}.handle-bound-file.producer"),
        file_witness("fn from_compute_plugin", "file: Some(Box::new(file))"),
    );
    builder.edge(
        typed,
        &producer,
        DecisionStage::Adapter,
        "typed_payload_borrowed",
    );
    let domain = builder.decision(
        format!("{PREFIX}.handle-bound-file.domain"),
        file_witness(
            "struct HandleBoundSqliteFileState",
            "file: Option<Box<dyn HandleBoundSqliteFileOperations>>",
        ),
    );
    builder.edge(
        &producer,
        &domain,
        DecisionStage::Adapter,
        "production_some_plus_defensive_none_domain",
    );
    let file_mut = builder.decision(
        format!("{PREFIX}.handle-bound-file.file-mut"),
        file_witness("fn file_mut", "self.file.as_deref_mut().ok_or(())"),
    );
    builder.edge(
        &domain,
        &file_mut,
        DecisionStage::Adapter,
        "callback_invokes_file_mut",
    );
    let missing_projection = builder.continuation(
        format!("{PREFIX}.handle-bound-file-missing.adapter-projection"),
        "missing HandleBound file error-code projection",
        io_witness("Err(()) => result_codes::SHM_LOCK_UNAVAILABLE"),
    );
    builder.edge(
        &file_mut,
        &missing_projection,
        DecisionStage::Adapter,
        "file_missing_returns_err",
    );
    let run_code_return = builder.continuation(
        format!("{PREFIX}.handle-bound-file-missing.run-code-return"),
        "run_code ordinary return without abandonment",
        file_witness("unsafe fn run_code", "Ok(Ok(code)) => code"),
    );
    builder.edge(
        &missing_projection,
        &run_code_return,
        DecisionStage::AbiProjection,
        "adapter_error_projected_to_fallback_code",
    );
    let mut missing = Expected::unavailable(RootOperation::Lock, "Adapter");
    missing.raw_slots = CustodyState::Unchanged;
    missing.payload = CustodyState::Retained;
    missing.file = CustodyState::Cleared;
    let terminal = builder.terminal(
        format!("{PREFIX}.terminal.handle-bound-file-missing"),
        missing,
        outcome::abi_projection(super::super::super::model::SqliteResult::LockUnavailable),
    );
    builder.edge(
        &run_code_return,
        &terminal,
        DecisionStage::AbiProjection,
        "run_code_returns_fallback_code_without_abandon",
    );
    let present = builder.decision(
        format!("{PREFIX}.handle-bound-file-present"),
        io_witness("|state| match state.shm_lock(offset, count, action)"),
    );
    builder.edge(
        &file_mut,
        &present,
        DecisionStage::Adapter,
        "handle_bound_file_present",
    );
    present
}

fn abandon_validation_witness() -> SourceWitness {
    raw_witness(
        "pub(super) unsafe fn abandon_installed_state",
        "validate_installed(methods, state)?;",
        1,
    )
}

fn raw_witness(symbol: &'static str, needle: &'static str, occurrence: u8) -> SourceWitness {
    witness(ProductionOwner::AbiRawState, symbol, needle, occurrence)
}

fn file_witness(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::AbiFileState, symbol, needle, 1)
}

fn io_witness(needle: &'static str) -> SourceWitness {
    witness(
        ProductionOwner::AbiIoShm,
        "unsafe extern \"C\" fn lock",
        needle,
        1,
    )
}
