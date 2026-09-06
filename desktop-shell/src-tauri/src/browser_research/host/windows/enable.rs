//! The finite, fixed sequence needed to observe; no caller-selectable CDP methods.
use super::super::handshake::Stage;
use super::{
    cdp::{self, Method},
    gap, Context, HostEvent,
};
use ::windows::core::HSTRING;
use serde_json::{json, Value};

const STEPS: [(Method, Stage, &str); 8] = [
    (
        Method::PageEnable,
        Stage::PageEnable,
        "page_domain_unavailable",
    ),
    (
        Method::FrameTree,
        Stage::FrameTree,
        "frame_tree_unavailable",
    ),
    (
        Method::RuntimeDisable,
        Stage::RuntimeReset,
        "runtime_reset_unavailable",
    ),
    (
        Method::RuntimeEnable,
        Stage::RuntimeEnable,
        "runtime_domain_unavailable",
    ),
    (
        Method::DebuggerDisable,
        Stage::DebuggerReset,
        "debugger_reset_unavailable",
    ),
    (
        Method::NetworkEnable,
        Stage::NetworkEnable,
        "network_domain_unavailable",
    ),
    (
        Method::SkipPauses,
        Stage::DebuggerSafeMode,
        "debugger_safe_mode_unavailable",
    ),
    (
        Method::DebuggerEnable,
        Stage::DebuggerEnable,
        "debugger_domain_unavailable",
    ),
];

pub(super) fn run(context: &Context, generation: u64, navigate: bool) {
    context.borrow_mut().ready = false;
    step(context, generation, navigate, 0);
}

fn step(context: &Context, generation: u64, navigate: bool, index: usize) {
    let handle = context.borrow().handle.clone();
    if !handle.handshake_current(generation) {
        return;
    }
    let Some(&(method, stage, failure)) = STEPS.get(index) else {
        complete(context, generation, navigate);
        return;
    };
    if !handle.handshake_stage(generation, stage) {
        return;
    }
    let size = context.borrow().config.max_body_bytes;
    let parameters = match method {
        Method::NetworkEnable => json!({"maxTotalBufferSize":size * 8,
            "maxResourceBufferSize":size,"maxPostDataSize":size}),
        Method::DebuggerEnable => json!({"maxScriptsCacheSize":size * 8}),
        Method::SkipPauses => json!({"skip":true}),
        _ => json!({}),
    };
    if !cdp::call(context, method, parameters, move |context, result| {
        let handle = context.borrow().handle.clone();
        // A paused, timed-out or replaced attempt cannot enable domains or navigate later.
        if !handle.handshake_current(generation) {
            return;
        }
        let reset = matches!(method, Method::RuntimeDisable | Method::DebuggerDisable);
        if result.is_err() && !reset {
            handle.fail_handshake(generation, failure);
            return;
        }
        if matches!(method, Method::FrameTree) {
            if let Ok(tree) = result {
                frame_tree(context, generation, navigate, &tree);
            }
        }
        step(context, generation, navigate, index + 1);
    }) {
        handle.fail_handshake(generation, "host_cdp_dispatch_failed");
    }
}

fn frame_tree(context: &Context, generation: u64, navigate: bool, tree: &Value) {
    let mut state = context.borrow_mut();
    state.frame = tree
        .pointer("/frameTree/frame/id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    state.document_url = tree
        .pointer("/frameTree/frame/url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .into();
    state.loader = tree
        .pointer("/frameTree/frame/loaderId")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if !navigate {
        let business = state.config.allows_document(&state.document_url);
        let mut event = HostEvent::new(
            generation,
            "navigation",
            if business { &state.document_url } else { "" },
        );
        if !business {
            event.error_code = Some("identity_navigation_not_captured".into());
        }
        (state.handle.control.sink)(event);
    }
}

fn complete(context: &Context, generation: u64, navigate: bool) {
    let (handle, core, url) = {
        let state = context.borrow();
        (
            state.handle.clone(),
            state.core.clone(),
            state.config.start_url.clone(),
        )
    };
    if !handle.finish_handshake(generation) {
        return;
    }
    context.borrow_mut().ready = true;
    gap(
        &handle,
        "coverage_top_frame_text_only_no_workers_websockets",
    );
    if navigate
        && handle.accepts(generation)
        && unsafe { core.Navigate(&HSTRING::from(url)) }.is_err()
    {
        // The CDP handshake completed, but failure to load is still terminal for this attempt.
        handle
            .control
            .active
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let mut event = HostEvent::new(generation, "failed", "");
        event.error_code = Some("host_initial_navigation_failed".into());
        (handle.control.sink)(event);
    }
}
