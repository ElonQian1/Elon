use super::{events, gap, Context};
use ::windows::core::{HSTRING, PCWSTR, PWSTR};
use serde_json::Value;
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
