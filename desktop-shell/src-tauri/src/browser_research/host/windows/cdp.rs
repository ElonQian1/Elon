use super::{events, gap, Context};
use ::windows::core::{HSTRING, PCWSTR, PWSTR};
use serde_json::{json, Value};
use std::rc::Rc;
use webview2_com::{
    CallDevToolsProtocolMethodCompletedHandler, CoTaskMemPWSTR,
    DevToolsProtocolEventReceivedEventHandler,
};

const EVENTS: [&str; 9] = [
    "Page.frameNavigated",
    "Page.navigatedWithinDocument",
    "Runtime.executionContextCreated",
    "Runtime.executionContextDestroyed",
    "Debugger.scriptParsed",
    "Network.requestWillBeSent",
    "Network.responseReceived",
    "Network.loadingFinished",
    "Network.loadingFailed",
];

// This private enum is the entire CDP surface. It cannot be deserialized from MCP input.
#[derive(Clone, Copy)]
pub(super) enum Method {
    PageEnable,
    FrameTree,
    RuntimeDisable,
    RuntimeEnable,
    DebuggerDisable,
    DebuggerEnable,
    SkipPauses,
    NetworkEnable,
    ScriptSource,
    ResponseBody,
    RequestBody,
}
impl Method {
    fn name(self) -> &'static str {
        match self {
            Self::PageEnable => "Page.enable",
            Self::FrameTree => "Page.getFrameTree",
            Self::RuntimeDisable => "Runtime.disable",
            Self::RuntimeEnable => "Runtime.enable",
            Self::DebuggerDisable => "Debugger.disable",
            Self::DebuggerEnable => "Debugger.enable",
            Self::NetworkEnable => "Network.enable",
            Self::SkipPauses => "Debugger.setSkipAllPauses",
            Self::ScriptSource => "Debugger.getScriptSource",
            Self::ResponseBody => "Network.getResponseBody",
            Self::RequestBody => "Network.getRequestPostData",
        }
    }
}

pub(super) fn subscribe(context: &Context) -> Result<(), ()> {
    for name in EVENTS {
        let weak = Rc::downgrade(context);
        let handler =
            DevToolsProtocolEventReceivedEventHandler::create(Box::new(move |_, args| {
                let Some(context) = weak.upgrade() else {
                    return Ok(());
                };
                let Some(args) = args else {
                    return Ok(());
                };
                let mut pointer = PWSTR::null();
                unsafe {
                    args.ParameterObjectAsJson(&mut pointer)?;
                }
                let allocated = CoTaskMemPWSTR::from(pointer);
                let text = allocated.to_string();
                // Network event headers are not copied into HostEvent or persisted.
                if text.len() <= 3 * 1024 * 1024 {
                    if let Ok(value) = serde_json::from_str::<Value>(&text) {
                        events::receive(&context, name, &value);
                    }
                } else {
                    gap(&context.borrow().handle, "host_event_too_large");
                }
                Ok(())
            }));
        let core = context.borrow().core.clone();
        let receiver = unsafe { core.GetDevToolsProtocolEventReceiver(&HSTRING::from(name)) }
            .map_err(|_| ())?;
        let mut token = 0;
        unsafe { receiver.add_DevToolsProtocolEventReceived(&handler, &mut token) }
            .map_err(|_| ())?;
        context.borrow_mut().receivers.push((receiver, token));
    }
    Ok(())
}

pub(super) fn call(
    context: &Context,
    method: Method,
    parameters: Value,
    callback: impl FnOnce(&Context, Result<Value, ()>) + 'static,
) -> bool {
    let core = context.borrow().core.clone();
    let weak = Rc::downgrade(context);
    let completed =
        CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |result, response| {
            if let Some(context) = weak.upgrade() {
                let cap = context.borrow().config.max_body_bytes.saturating_mul(6) + 65536;
                let parsed = if result.is_ok() && response.len() <= cap {
                    serde_json::from_str::<Value>(&response)
                        .ok()
                        .filter(|v| v.get("error").is_none())
                        .ok_or(())
                } else {
                    Err(())
                };
                callback(&context, parsed);
            }
            Ok(())
        }));
    let name = HSTRING::from(method.name());
    let parameters = HSTRING::from(parameters.to_string());
    let result = unsafe {
        core.CallDevToolsProtocolMethod(
            PCWSTR(name.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            &completed,
        )
    };
    if result.is_err() {
        gap(&context.borrow().handle, "host_cdp_dispatch_failed");
    }
    result.is_ok()
}

pub(super) fn enable(context: &Context, navigate: bool) {
    let initial_generation = context.borrow().handle.generation();
    // Install observation before loading the target document. Acknowledgements gate readiness.
    call(
        context,
        Method::PageEnable,
        json!({}),
        move |context, result| {
            if result.is_err() {
                gap(&context.borrow().handle, "page_domain_unavailable");
                return;
            }
            call(
                context,
                Method::FrameTree,
                json!({}),
                move |context, result| {
                    let Ok(tree) = result else {
                        gap(&context.borrow().handle, "frame_tree_unavailable");
                        return;
                    };
                    {
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
                    }
                    if !navigate {
                        let state = context.borrow();
                        let business = state.config.allows_document(&state.document_url);
                        let mut event = super::HostEvent::new(
                            state.handle.generation(),
                            "navigation",
                            if business { &state.document_url } else { "" },
                        );
                        if !business {
                            event.error_code = Some("identity_navigation_not_captured".into());
                        }
                        (state.handle.control.sink)(event);
                    }
                    call(
                        context,
                        Method::RuntimeDisable,
                        json!({}),
                        move |context, _| {
                            call(
                                context,
                                Method::RuntimeEnable,
                                json!({}),
                                move |context, result| {
                                    if result.is_err() {
                                        gap(&context.borrow().handle, "runtime_domain_unavailable");
                                        return;
                                    }
                                    // Re-enabling debugger re-announces scripts already loaded in the current document.
                                    call(
                                        context,
                                        Method::DebuggerDisable,
                                        json!({}),
                                        move |context, _| {
                                            let size = context.borrow().config.max_body_bytes;
                                            call(
                                                context,
                                                Method::NetworkEnable,
                                                json!({"maxTotalBufferSize": size * 8,
                        "maxResourceBufferSize": size, "maxPostDataSize": size}),
                                                move |context, result| {
                                                    if result.is_err() {
                                                        gap(
                                                            &context.borrow().handle,
                                                            "network_domain_unavailable",
                                                        );
                                                        return;
                                                    }
                                                    call(
                                                        context,
                                                        Method::SkipPauses,
                                                        json!({"skip":true}),
                                                        move |context, result| {
                                                            if result.is_err() {
                                                                gap(&context.borrow().handle, "debugger_safe_mode_unavailable");
                                                                return;
                                                            }
                                                            call(
                                                                context,
                                                                Method::DebuggerEnable,
                                                                json!({"maxScriptsCacheSize": size * 8}),
                                                                move |context, result| {
                                                                    if result.is_err() {
                                                                        gap(&context.borrow().handle, "debugger_domain_unavailable");
                                                                        return;
                                                                    }
                                                                    let (handle, core, url) = {
                                                                        let mut state =
                                                                            context.borrow_mut();
                                                                        state.ready = true;
                                                                        (
                                                                            state.handle.clone(),
                                                                            state.core.clone(),
                                                                            state
                                                                                .config
                                                                                .start_url
                                                                                .clone(),
                                                                        )
                                                                    };
                                                                    if !handle.active() {
                                                                        return;
                                                                    }
                                                                    let mut event =
                                                                        super::HostEvent::new(
                                                                            handle.generation(),
                                                                            "ready",
                                                                            "",
                                                                        );
                                                                    event.resource_type = Some("top_frame_cdp_network_and_script".into());
                                                                    (handle.control.sink)(event);
                                                                    gap(&handle, "coverage_top_frame_text_only_no_workers_websockets");
                                                                    if navigate
                                                                        && handle.generation()
                                                                            == initial_generation
                                                                    {
                                                                        if unsafe {
                                                                            core.Navigate(
                                                                                &HSTRING::from(url),
                                                                            )
                                                                        }
                                                                        .is_err()
                                                                        {
                                                                            gap(&handle, "host_initial_navigation_failed");
                                                                        }
                                                                    }
                                                                },
                                                            );
                                                        },
                                                    );
                                                },
                                            );
                                        },
                                    );
                                },
                            );
                        },
                    );
                },
            );
        },
    );
}
