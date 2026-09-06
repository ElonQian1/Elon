use super::{
    cdp::Method,
    emit, gap,
    reads::{bounded, read},
    Context, HostEvent,
};
use serde_json::{json, Value};

const MAX_REQUESTS: usize = 512;
const MAX_SCRIPTS: usize = 256;

pub(super) fn receive(context: &Context, name: &str, value: &Value) {
    {
        let mut state = context.borrow_mut();
        state.synchronize();
        if !state.handle.active() {
            state.reads.clear_waiting();
            return;
        }
    }
    match name {
        "Page.frameNavigated" => frame(context, value),
        "Page.navigatedWithinDocument" => within_document(context, value),
        "Runtime.executionContextCreated" => execution_context(context, value),
        "Runtime.executionContextDestroyed" => {
            if let Some(id) = value.get("executionContextId").and_then(Value::as_i64) {
                context.borrow_mut().contexts.remove(&id);
            }
        }
        "Debugger.scriptParsed" => script(context, value),
        "Network.requestWillBeSent" => request(context, value),
        "Network.responseReceived" => response(context, value),
        "Network.loadingFinished" => finished(context, value),
        "Network.loadingFailed" => failed(context, value),
        _ => {}
    }
    super::reads::drain(context);
}

fn within_document(context: &Context, value: &Value) {
    let Some(url) = text(value, "url", 8192) else {
        return;
    };
    let (handle, generation, business) = {
        let mut state = context.borrow_mut();
        if value.get("frameId").and_then(Value::as_str) != state.frame.as_deref() {
            return;
        }
        let loader = state.loader.clone();
        let contexts = state.contexts.clone();
        let generation = state
            .handle
            .control
            .generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        state.synchronize();
        state.loader = loader;
        state.contexts = contexts;
        state.document_url = url.clone();
        (
            state.handle.clone(),
            generation,
            state.config.allows_document(&url),
        )
    };
    let mut event = HostEvent::new(generation, "navigation", if business { &url } else { "" });
    if !business {
        event.error_code = Some("identity_navigation_not_captured".into());
    }
    (handle.control.sink)(event);
}

fn frame(context: &Context, value: &Value) {
    let frame = &value["frame"];
    if frame.get("parentId").is_some() {
        return;
    }
    let Some(id) = text(frame, "id", 256) else {
        return;
    };
    let url = text(frame, "url", 8192).unwrap_or_default();
    let ready = {
        let mut state = context.borrow_mut();
        state.frame = Some(id);
        state.document_url = url;
        state.loader = text(frame, "loaderId", 256);
        (state.ready && state.config.allows_document(&state.document_url))
            .then(|| HostEvent::new(state.generation, "ready", ""))
    };
    if let Some(event) = ready {
        emit(context, event);
    }
}

fn execution_context(context: &Context, value: &Value) {
    let ctx = &value["context"];
    let Some(id) = ctx.get("id").and_then(Value::as_i64) else {
        return;
    };
    let Some(frame) = text(&ctx["auxData"], "frameId", 256) else {
        return;
    };
    let mut state = context.borrow_mut();
    if state.frame.as_deref() == Some(&frame)
        && state.contexts.len() < 128
        && state.config.allows_document(&state.document_url)
    {
        state.contexts.insert(id, frame);
    }
}

fn request(context: &Context, value: &Value) {
    let Some(id) = text(value, "requestId", 256) else {
        return;
    };
    let Some(url) = text(&value["request"], "url", 8192) else {
        return;
    };
    let resource_type = text(value, "type", 64).unwrap_or_default();
    let document_url = text(value, "documentURL", 8192).unwrap_or_default();
    let Some(method) = text(&value["request"], "method", 16) else {
        return;
    };
    if !method.bytes().all(|v| v.is_ascii_uppercase()) {
        return;
    }
    let (mut event, max_body) = {
        let mut state = context.borrow_mut();
        // A redirect can reuse requestId. Drop its old association before final-origin validation.
        state.requests.remove(&id);
        state.request_bindings.remove(&id);
        if value.get("frameId").and_then(Value::as_str) != state.frame.as_deref()
            || !state.config.allows_document(&document_url)
        {
            return;
        }
        let loader = text(value, "loaderId", 256);
        if state.loader.is_none() && resource_type == "Document" && state.document_url == url {
            state.loader = loader.clone();
        }
        if loader.is_none() || loader != state.loader {
            return;
        }
        let static_resource = matches!(resource_type.as_str(), "Script" | "Document");
        if (static_resource && !state.config.allows_resource(&url))
            || (!static_resource
                && (!matches!(resource_type.as_str(), "XHR" | "Fetch")
                    || !state.config.allows_api(&url)))
        {
            return;
        }
        if state.request_bindings.len() >= MAX_REQUESTS {
            drop(state);
            gap(&context.borrow().handle, "request_index_limit");
            return;
        }
        let mut event = HostEvent::new(
            state.generation,
            if static_resource {
                "resource"
            } else {
                "request"
            },
            &url,
        );
        event.method = Some(method);
        event.resource_type = Some(resource_type);
        state.next_request += 1;
        let observed_id = format!("request-{}-{}", state.generation, state.next_request);
        event.request_id = Some(observed_id.clone());
        state.request_bindings.insert(id.clone(), observed_id);
        event.initiator = initiator(&value["initiator"]);
        (event, state.config.max_body_bytes)
    };
    let post_data = value["request"].get("postData").and_then(Value::as_str);
    if event.kind == "request" {
        if let Some(body) = post_data {
            let (body, truncated) = bounded(body, max_body);
            event.request_body = Some(body);
            event.truncated = truncated;
            if truncated {
                event.error_code = Some("request_body_truncated".into());
            }
        }
    }
    let mut association = event.clone();
    // Core merges later response parts by requestId; do not retain business bodies in this index.
    association.request_body = None;
    association.initiator = None;
    context
        .borrow_mut()
        .requests
        .insert(id.clone(), association);
    emit(context, event.clone());
    if event.kind == "request"
        && post_data.is_none()
        && value["request"].get("hasPostData").and_then(Value::as_bool) == Some(true)
    {
        read(
            context,
            event,
            Method::RequestBody,
            json!({"requestId": id}),
            "postData",
            true,
        );
    }
}

fn response(context: &Context, value: &Value) {
    let Some(id) = text(value, "requestId", 256) else {
        return;
    };
    let Some(url) = text(&value["response"], "url", 8192) else {
        return;
    };
    let mut state = context.borrow_mut();
    let Some(previous) = state.requests.get(&id) else {
        return;
    };
    let allowed = if previous.kind == "resource" {
        state.config.allows_resource(&url)
    } else {
        state.config.allows_api(&url)
    };
    if !allowed || previous.url != url {
        state.requests.remove(&id);
        state.request_bindings.remove(&id);
        return;
    }
    if let Some(event) = state.requests.get_mut(&id) {
        event.status = value["response"]
            .get("status")
            .and_then(Value::as_u64)
            .filter(|v| (100..=599).contains(v))
            .map(|v| v as u16);
        event.mime = text(&value["response"], "mimeType", 128);
    }
}

fn finished(context: &Context, value: &Value) {
    let Some(id) = text(value, "requestId", 256) else {
        return;
    };
    let Some(mut event) = context.borrow_mut().requests.remove(&id) else {
        return;
    };
    if event.status.is_none() {
        event.error_code = Some("response_metadata_missing".into());
        emit(context, event);
        return;
    }
    let mime = event.mime.as_deref().unwrap_or("");
    if !(mime.starts_with("text/")
        || mime.contains("json")
        || mime.contains("javascript")
        || mime.contains("xml"))
    {
        event.error_code = Some("response_body_type_not_supported".into());
        emit(context, event);
        return;
    }
    read(
        context,
        event,
        Method::ResponseBody,
        json!({"requestId": id}),
        "body",
        false,
    );
}

fn failed(context: &Context, value: &Value) {
    let Some(id) = text(value, "requestId", 256) else {
        return;
    };
    let event = context.borrow_mut().requests.remove(&id);
    if let Some(mut event) = event {
        event.error_code = Some("observed_request_failed".into());
        emit(context, event);
    }
}

fn script(context: &Context, value: &Value) {
    let Some(id) = text(value, "scriptId", 256) else {
        return;
    };
    let mut event = {
        let mut state = context.borrow_mut();
        let source_url = text(value, "url", 8192);
        // Anonymous/inline scripts belong to this execution context's document, not a fetched URL.
        let url = source_url
            .clone()
            .unwrap_or_else(|| state.document_url.clone());
        let frame = value
            .get("executionContextId")
            .and_then(Value::as_i64)
            .and_then(|id| state.contexts.get(&id).map(String::as_str))
            .or_else(|| {
                value
                    .pointer("/executionContextAuxData/frameId")
                    .and_then(Value::as_str)
            });
        if frame != state.frame.as_deref()
            || state.frame.is_none()
            || !state.config.allows_document(&state.document_url)
            || !state.config.allows_resource(&url)
            || state.scripts.contains_key(&id)
        {
            return;
        }
        if state.scripts.len() >= MAX_SCRIPTS {
            drop(state);
            gap(&context.borrow().handle, "script_index_limit");
            return;
        }
        state.scripts.insert(id.clone(), ());
        let mut event = HostEvent::new(state.generation, "resource", &url);
        event.script_id = Some(id.clone());
        event.resource_type = Some("Script".into());
        event.mime = Some("application/javascript".into());
        event.initiator = Some(json!({"type":"debugger_script_parsed",
            "sourceUrl":source_url,"hasSourceURL":value.get("hasSourceURL").and_then(Value::as_bool),
            "startLine":value.get("startLine").and_then(Value::as_u64),
            "startColumn":value.get("startColumn").and_then(Value::as_u64)}));
        event
    };
    if value.get("length").and_then(Value::as_u64).unwrap_or(0)
        > context.borrow().config.max_body_bytes as u64
    {
        event.truncated = true;
        event.error_code = Some("script_source_too_large".into());
        emit(context, event);
        return;
    }
    read(
        context,
        event,
        Method::ScriptSource,
        json!({"scriptId": id}),
        "scriptSource",
        false,
    );
}

fn text(value: &Value, key: &str, max: usize) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= max)
        .map(str::to_owned)
}

fn initiator(value: &Value) -> Option<Value> {
    let kind = value.get("type").and_then(Value::as_str)?;
    if !matches!(
        kind,
        "parser" | "script" | "preload" | "preflight" | "other"
    ) {
        return None;
    }
    let frames =
        value
            .pointer("/stack/callFrames")
            .and_then(Value::as_array)
            .map(|items| {
                items.iter().take(12).map(|frame| json!({
            "scriptId": text(frame, "scriptId", 256), "url": text(frame, "url", 8192),
            "lineNumber": frame.get("lineNumber").and_then(Value::as_u64),
            "columnNumber": frame.get("columnNumber").and_then(Value::as_u64)
        })).collect::<Vec<_>>()
            });
    Some(json!({"type":kind,"url":text(value,"url",8192),"callFrames":frames}))
}
