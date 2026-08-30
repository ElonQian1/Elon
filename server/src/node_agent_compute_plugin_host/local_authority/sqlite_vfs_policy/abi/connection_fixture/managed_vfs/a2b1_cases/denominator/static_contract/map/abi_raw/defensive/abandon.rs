use super::super::super::super::source::SourceWitness;
use super::super::super::{
    super::model::{CustodyState, DecisionStage, Expected, TerminalDisposition},
    builder::MapGraphBuilder,
    expected, witnesses as w,
};

const PREFIX: &str = "map.raw";

pub(super) fn add_fallback_boundary(
    graph: &mut MapGraphBuilder,
    from: &str,
    label: &str,
) -> String {
    let fallback = graph.decision(
        &format!("{PREFIX}.{label}.fallback"),
        w::file_state("unsafe fn run_code", "Ok(Err(_)) | Err(_) =>"),
    );
    graph.edge(
        from,
        &fallback,
        DecisionStage::RawAbandon,
        "callback_projects_to_fallback",
    );
    let boundary = graph.decision(
        &format!("{PREFIX}.{label}.abandon-without-unwind"),
        w::file_state(
            "unsafe fn run_code",
            "unsafe { abandon_without_unwind(file) };",
        ),
    );
    graph.edge(
        &fallback,
        &boundary,
        DecisionStage::RawAbandon,
        "fallback_invokes_abandon_without_unwind",
    );
    let abandon = graph.decision(
        &format!("{PREFIX}.{label}.abandon-installed-state"),
        w::file_state(
            "unsafe fn abandon_without_unwind",
            "unsafe { raw_state::abandon_installed_state(file) }",
        ),
    );
    graph.edge(
        &boundary,
        &abandon,
        DecisionStage::RawAbandon,
        "catch_unwind_enters_abandon_installed_state",
    );
    abandon
}

pub(super) fn add_envelope(
    graph: &mut MapGraphBuilder,
    payload_domain: &str,
    label: &str,
    branch: &str,
    trigger: SourceWitness,
    payload_present: bool,
) {
    let cause = graph.decision(&format!("{PREFIX}.{label}.cause"), trigger);
    graph.edge(payload_domain, &cause, DecisionStage::RawAdmission, branch);
    let abandon = add_fallback_boundary(graph, &cause, label);
    let clear = graph.decision(
        &format!("{PREFIX}.{label}.raw-slots-cleared"),
        w::raw(
            "pub(super) unsafe fn abandon_installed_state",
            "ptr::addr_of_mut!((*file.as_ptr()).base.pMethods).write(ptr::null())",
        ),
    );
    graph.edge(
        &abandon,
        &clear,
        DecisionStage::RawAbandon,
        "exact_envelope_validated_then_slots_cleared",
    );
    let envelope_drop = graph.decision(
        &format!("{PREFIX}.{label}.envelope-drop"),
        w::raw(
            "pub(super) unsafe fn abandon_installed_state",
            "drop(Box::from_raw(state.cast::<RawSqliteFileStateEnvelope>()))",
        ),
    );
    graph.edge(
        &clear,
        &envelope_drop,
        DecisionStage::RawAbandon,
        "cleared_state_box_enters_envelope_drop",
    );
    if !payload_present {
        let terminal = format!("{PREFIX}.terminal.{label}.drop-completed");
        graph.terminal(
            &terminal,
            raw_expected(CustodyState::Cleared, false),
            w::raw(
                "fn drop(&mut self)",
                "if let Some(payload) = self.payload.take()",
            ),
        );
        graph.edge(
            &envelope_drop,
            &terminal,
            DecisionStage::RawAbandon,
            "payload_none_envelope_drop_completed",
        );
        return;
    }
    let dispatch = graph.decision(
        &format!("{PREFIX}.{label}.payload-drop-dispatch"),
        w::raw(
            "fn drop(&mut self)",
            "unsafe { (self.drop_payload)(payload) };",
        ),
    );
    graph.edge(
        &envelope_drop,
        &dispatch,
        DecisionStage::RawAbandon,
        "payload_some_invokes_paired_drop",
    );
    let typed_drop = graph.decision(
        &format!("{PREFIX}.{label}.typed-payload-drop"),
        w::raw(
            "unsafe fn drop_typed_payload<State>",
            "drop(unsafe { Box::from_raw(payload.cast::<State>().as_ptr()) })",
        ),
    );
    graph.edge(
        &dispatch,
        &typed_drop,
        DecisionStage::RawAbandon,
        "producer_paired_type_erased_drop",
    );
    let outcome = graph.decision(
        &format!("{PREFIX}.{label}.drop-outcome"),
        w::file_state(
            "unsafe fn abandon_without_unwind",
            "let _ = catch_unwind(AssertUnwindSafe(||",
        ),
    );
    graph.edge(
        &typed_drop,
        &outcome,
        DecisionStage::RawAbandon,
        "typed_drop_runs_inside_unwind_boundary",
    );
    for (branch, payload, unwind) in [
        ("drop-completed", CustodyState::Released, false),
        ("drop-unwind-caught", CustodyState::Quarantined, true),
    ] {
        let terminal = format!("{PREFIX}.terminal.{label}.{branch}");
        graph.terminal(
            &terminal,
            raw_expected(payload, unwind),
            w::file_state("unsafe fn run_code", "Ok(Err(_)) | Err(_) =>"),
        );
        graph.edge(&outcome, &terminal, DecisionStage::RawAbandon, branch);
    }
}

fn raw_expected(payload: CustodyState, unwind: bool) -> Expected {
    let mut value = expected::raw_fallback(CustodyState::Cleared, payload);
    if unwind {
        value.disposition = TerminalDisposition::Quarantined;
    }
    value
}
