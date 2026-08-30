use super::super::super::{
    super::{
        model::{
            CustodyState, DecisionStage, Expected, FailureClass, RootOperation, TerminalDisposition,
        },
        source::{witness, ProductionOwner, SourceWitness},
    },
    builder::Builder,
    outcome,
};

const PREFIX: &str = "lock.raw";

pub(super) fn add_rejected_slots(
    builder: &mut Builder,
    raw: &str,
    shape: &str,
    source: SourceWitness,
    abandon_source: SourceWitness,
    slots: CustodyState,
) {
    let cause = builder.continuation(
        format!("{PREFIX}.{shape}.cause"),
        "raw rejection cause",
        source,
    );
    builder.edge(raw, &cause, DecisionStage::RawAdmission, shape);
    let abandon = add_fallback_boundary(builder, &cause, shape);
    let rejected = builder.continuation(
        format!("{PREFIX}.{shape}.abandon-validation"),
        "raw abandonment validation rejection or no-op",
        abandon_source,
    );
    builder.edge(
        &abandon,
        &rejected,
        DecisionStage::RawAbandon,
        "abandon_validation_rejected_or_noop",
    );
    let mut expected = raw_expected(slots, CustodyState::NotReached);
    expected.disposition = TerminalDisposition::Returned;
    let terminal = builder.terminal(
        format!("{PREFIX}.terminal.{shape}"),
        expected,
        outcome::abi_projection(super::super::super::super::model::SqliteResult::LockUnavailable),
    );
    builder.edge(
        &rejected,
        &terminal,
        DecisionStage::RawAbandon,
        "fallback_returned_with_slots_not_owned",
    );
}

pub(super) fn add_envelope(
    builder: &mut Builder,
    payload_domain: &str,
    label: &str,
    branch: &str,
    trigger: SourceWitness,
    payload_present: bool,
) {
    let cause = builder.continuation(
        format!("{PREFIX}.{label}.cause"),
        "typed admission failure",
        trigger,
    );
    builder.edge(payload_domain, &cause, DecisionStage::RawAdmission, branch);
    let abandon = add_fallback_boundary(builder, &cause, label);
    let clear = builder.continuation(
        format!("{PREFIX}.{label}.raw-slots-cleared"),
        "exact envelope raw-slot clear",
        raw_witness(
            "pub(super) unsafe fn abandon_installed_state",
            "ptr::addr_of_mut!((*file.as_ptr()).base.pMethods).write(ptr::null())",
        ),
    );
    builder.edge(
        &abandon,
        &clear,
        DecisionStage::RawAbandon,
        "exact_envelope_validated_then_slots_cleared",
    );
    let envelope_drop = builder.continuation(
        format!("{PREFIX}.{label}.envelope-drop"),
        "RawSqliteFileStateEnvelope Drop",
        raw_witness(
            "pub(super) unsafe fn abandon_installed_state",
            "drop(Box::from_raw(state.cast::<RawSqliteFileStateEnvelope>()))",
        ),
    );
    builder.edge(
        &clear,
        &envelope_drop,
        DecisionStage::RawAbandon,
        "cleared_state_box_enters_envelope_drop",
    );
    if !payload_present {
        let terminal = builder.terminal(
            format!("{PREFIX}.terminal.{label}.drop-completed"),
            raw_expected(CustodyState::Cleared, CustodyState::Cleared),
            raw_witness(
                "fn drop(&mut self)",
                "if let Some(payload) = self.payload.take()",
            ),
        );
        builder.edge(
            &envelope_drop,
            &terminal,
            DecisionStage::RawAbandon,
            "payload_none_envelope_drop_completed",
        );
        return;
    }
    let dispatch = builder.continuation(
        format!("{PREFIX}.{label}.payload-drop-dispatch"),
        "paired type-erased payload Drop",
        raw_witness(
            "fn drop(&mut self)",
            "unsafe { (self.drop_payload)(payload) };",
        ),
    );
    builder.edge(
        &envelope_drop,
        &dispatch,
        DecisionStage::RawAbandon,
        "payload_some_invokes_paired_drop",
    );
    let typed_drop = builder.continuation(
        format!("{PREFIX}.{label}.typed-payload-drop"),
        "producer-paired concrete payload Drop",
        raw_witness(
            "unsafe fn drop_typed_payload<State>",
            "drop(unsafe { Box::from_raw(payload.cast::<State>().as_ptr()) })",
        ),
    );
    builder.edge(
        &dispatch,
        &typed_drop,
        DecisionStage::RawAbandon,
        "producer_paired_type_erased_drop",
    );
    let outcome = builder.decision(
        format!("{PREFIX}.{label}.drop-outcome"),
        file_witness(
            "unsafe fn abandon_without_unwind",
            "let _ = catch_unwind(AssertUnwindSafe(||",
        ),
    );
    builder.edge(
        &typed_drop,
        &outcome,
        DecisionStage::RawAbandon,
        "typed_drop_runs_inside_unwind_boundary",
    );
    for (branch, payload, unwind) in [
        ("drop-completed", CustodyState::Released, false),
        ("drop-unwind-caught", CustodyState::Quarantined, true),
    ] {
        let mut expected = raw_expected(CustodyState::Cleared, payload);
        if unwind {
            expected.disposition = TerminalDisposition::Quarantined;
        }
        let terminal = builder.terminal(
            format!("{PREFIX}.terminal.{label}.{branch}"),
            expected,
            outcome::abi_projection(
                super::super::super::super::model::SqliteResult::LockUnavailable,
            ),
        );
        builder.edge(&outcome, &terminal, DecisionStage::RawAbandon, branch);
    }
}

fn add_fallback_boundary(builder: &mut Builder, from: &str, label: &str) -> String {
    let fallback = builder.continuation(
        format!("{PREFIX}.{label}.fallback"),
        "outer callback fallback selection",
        file_witness("unsafe fn run_code", "Ok(Err(_)) | Err(_) =>"),
    );
    builder.edge(
        from,
        &fallback,
        DecisionStage::RawAbandon,
        "callback_projects_to_fallback",
    );
    let boundary = builder.continuation(
        format!("{PREFIX}.{label}.abandon-without-unwind"),
        "panic-isolated abandonment boundary",
        file_witness(
            "unsafe fn run_code",
            "unsafe { abandon_without_unwind(file) };",
        ),
    );
    builder.edge(
        &fallback,
        &boundary,
        DecisionStage::RawAbandon,
        "fallback_invokes_abandon_without_unwind",
    );
    let abandon = builder.continuation(
        format!("{PREFIX}.{label}.abandon-installed-state"),
        "raw state abandonment",
        file_witness(
            "unsafe fn abandon_without_unwind",
            "unsafe { raw_state::abandon_installed_state(file) }",
        ),
    );
    builder.edge(
        &boundary,
        &abandon,
        DecisionStage::RawAbandon,
        "catch_unwind_enters_abandon_installed_state",
    );
    abandon
}

fn raw_expected(slots: CustodyState, payload: CustodyState) -> Expected {
    let mut expected = Expected::unavailable(RootOperation::Lock, "RawAdmission");
    expected.disposition = TerminalDisposition::Abandoned;
    expected.failure = FailureClass::ProtocolViolation;
    expected.raw_slots = slots;
    expected.payload = payload;
    expected
}

fn raw_witness(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::AbiRawState, symbol, needle, 1)
}

fn file_witness(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::AbiFileState, symbol, needle, 1)
}
