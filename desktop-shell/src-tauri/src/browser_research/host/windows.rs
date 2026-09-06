use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2, ICoreWebView2DevToolsProtocolEventReceiver,
};

use super::{
    handshake::Stage,
    types::{now_ms, Control, HostConfig, HostEvent, HostHandle, HostSink},
};
mod cdp;
mod enable;
mod events;
mod reads;

type Context = Rc<RefCell<Capture>>;
thread_local! { static CAPTURES: RefCell<BTreeMap<String, Context>> = const { RefCell::new(BTreeMap::new()) }; }

struct Capture {
    config: HostConfig,
    handle: HostHandle,
    core: ICoreWebView2,
    generation: u64,
    frame: Option<String>,
    loader: Option<String>,
    document_url: String,
    contexts: BTreeMap<i64, String>,
    requests: BTreeMap<String, HostEvent>,
    request_bindings: BTreeMap<String, String>,
    next_request: u64,
    scripts: BTreeMap<String, ()>,
    receivers: Vec<(ICoreWebView2DevToolsProtocolEventReceiver, i64)>,
    reads: reads::ReadScheduler,
    ready: bool,
}

impl Capture {
    fn synchronize(&mut self) {
        let generation = self.handle.generation();
        if generation != self.generation {
            self.generation = generation;
            self.requests.clear();
            self.contexts.clear();
            self.scripts.clear();
            self.request_bindings.clear();
            self.document_url.clear();
            self.loader = None;
            // Native callbacks cannot be cancelled. Retain their slots until completion,
            // but discard all queued jobs so a new generation never submits old reads.
            self.reads.clear_waiting();
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        for (receiver, token) in &self.receivers {
            unsafe {
                let _ = receiver.remove_DevToolsProtocolEventReceived(*token);
            }
        }
    }
}

pub(super) fn open(
    app: &tauri::AppHandle,
    config: HostConfig,
    sink: HostSink,
) -> Result<HostHandle, String> {
    if app.get_webview(&config.label).is_some() {
        return Err("browser_research_window_exists".into());
    }
    super::super::files::ensure_directory(&config.profile_dir)
        .map_err(|_| "browser_research_profile_unavailable")?;
    let handle = HostHandle {
        label: config.label.clone(),
        control: Arc::new(Control {
            active: AtomicBool::new(true),
            generation: AtomicU64::new(1),
            closed: AtomicBool::new(false),
            expires_at_ms: config.expires_at_ms,
            sink,
            handshake: std::sync::Mutex::default(),
        }),
    };
    let generation = handle.begin_handshake(false);
    if start_deadline_poll(app, &handle).is_err() {
        handle.fail_handshake(generation, "browser_research_deadline_unavailable");
        return Err("browser_research_deadline_unavailable".into());
    }
    let app = app.clone();
    let dispatcher = app.clone();
    let install_handle = handle.clone();
    let (attached_tx, attached_rx) = std::sync::mpsc::sync_channel(1);
    // Creating a window off-thread only queues Tauri's native creation. Keep build
    // and installation in this one UI task so WithWebview cannot overtake it.
    dispatcher
        .run_on_main_thread(move || {
            if !install_handle.handshake_stage(generation, Stage::NativeCreate) {
                let _ = attached_tx.try_send(Err("host_attach_cancelled"));
                return;
            }
            let result = open_on_main(&app, config, &install_handle, generation);
            if let Err(code) = result {
                install_handle.fail_handshake(generation, code);
            }
            let _ = attached_tx.try_send(result);
        })
        .map_err(|_| {
            handle.fail_handshake(generation, "browser_research_host_dispatch_failed");
            "browser_research_host_dispatch_failed".to_string()
        })?;
    // Do not publish a handle before the manager/native view exists: a subsequent
    // status call would legitimately prune it as closed. This wait never runs CDP.
    match attached_rx.recv_timeout(std::time::Duration::from_secs(16)) {
        Ok(Ok(())) if handle.active() => Ok(handle),
        Ok(Ok(())) => Err("host_attach_cancelled".into()),
        Ok(Err(code)) => Err(code.into()),
        Err(_) => {
            handle.fail_handshake(generation, "host_attach_timed_out");
            Err("host_attach_timed_out".into())
        }
    }
}

fn open_on_main(
    app: &tauri::AppHandle,
    config: HostConfig,
    handle: &HostHandle,
    generation: u64,
) -> Result<(), &'static str> {
    if app.get_webview(&config.label).is_some() {
        return Err("browser_research_window_exists");
    }
    let navigation_config = config.clone();
    let navigation_handle = handle.clone();
    let window = WebviewWindowBuilder::new(
        app,
        &config.label,
        WebviewUrl::External(tauri::Url::parse("about:blank").expect("fixed bootstrap URL")),
    )
    .title("网页研究 · 一龙")
    .inner_size(1180.0, 820.0)
    .data_directory(config.profile_dir.clone())
    .incognito(false)
    .on_navigation(move |url| {
        if url.as_str() == "about:blank" {
            return true;
        }
        let allowed = navigation_config.allows_navigation(url.as_str());
        if allowed {
            let generation = navigation_handle
                .control
                .generation
                .fetch_add(1, Ordering::SeqCst)
                + 1;
            let business = navigation_config.allows_document(url.as_str());
            CAPTURES.with(|states| {
                let context = states.borrow().get(&navigation_handle.label).cloned();
                if let Some(context) = context {
                    let mut state = context.borrow_mut();
                    state.synchronize();
                    state.document_url = url.as_str().into();
                }
            });
            let mut event = HostEvent::new(
                generation,
                "navigation",
                if business { url.as_str() } else { "" },
            );
            if !business {
                event.error_code = Some("identity_navigation_not_captured".into());
            }
            (navigation_handle.control.sink)(event);
            navigation_handle.navigation_during_handshake();
        }
        allowed
    })
    // Never silently launch an unscoped popup or move the user's Chrome session.
    .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
    .build()
    .map_err(|_| "browser_research_window_unavailable")?;
    if !handle.handshake_current(generation) {
        return Ok(());
    }
    let closed_handle = handle.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            closed_handle.control.closed.store(true, Ordering::SeqCst);
            closed_handle.control.active.store(false, Ordering::SeqCst);
            let generation = closed_handle
                .control
                .generation
                .fetch_add(1, Ordering::SeqCst)
                + 1;
            CAPTURES.with(|states| {
                states.borrow_mut().remove(&closed_handle.label);
            });
            (closed_handle.control.sink)(HostEvent::new(generation, "closed", ""));
        }
    });
    let install_handle = handle.clone();
    let installed = Arc::new(AtomicBool::new(false));
    let entered = installed.clone();
    window
        .with_webview(move |platform| {
            entered.store(true, Ordering::SeqCst);
            if !install_handle.handshake_stage(generation, Stage::NativeAttached) {
                return;
            }
            let core = unsafe { platform.controller().CoreWebView2() };
            let Ok(core) = core else {
                install_handle.fail_handshake(generation, "host_core_unavailable");
                return;
            };
            let context = Rc::new(RefCell::new(Capture {
                config,
                handle: install_handle.clone(),
                core,
                generation: install_handle.generation(),
                frame: None,
                loader: None,
                document_url: String::new(),
                contexts: BTreeMap::new(),
                requests: BTreeMap::new(),
                scripts: BTreeMap::new(),
                request_bindings: BTreeMap::new(),
                next_request: 0,
                receivers: Vec::new(),
                reads: reads::ReadScheduler::default(),
                ready: false,
            }));
            CAPTURES.with(|states| {
                states
                    .borrow_mut()
                    .insert(install_handle.label.clone(), context.clone());
            });
            if cdp::subscribe(&context).is_err() {
                install_handle.fail_handshake(generation, "host_subscription_unavailable");
                return;
            }
            enable::run(&context, generation, true);
        })
        .map_err(|_| "browser_research_host_dispatch_failed")?;
    // On the main thread Tauri dispatches synchronously; a missing native ID
    // silently skips the closure, so an explicit receipt is essential.
    if !installed.load(Ordering::SeqCst) {
        return Err("host_native_not_attached");
    }
    Ok(())
}

fn start_deadline_poll(app: &tauri::AppHandle, handle: &HostHandle) -> Result<(), String> {
    let weak = Arc::downgrade(&handle.control);
    let app = app.clone();
    let label = handle.label.clone();
    std::thread::Builder::new()
        .name("research-read-deadline".into())
        .stack_size(128 * 1024)
        .spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let Some(control) = weak.upgrade() else {
                break;
            };
            if control.closed.load(Ordering::SeqCst) {
                break;
            }
            HostHandle {
                label: label.clone(),
                control: control.clone(),
            }
            .poll_handshake();
            let expired = now_ms() >= control.expires_at_ms;
            let target = label.clone();
            if app
                .run_on_main_thread(move || {
                    CAPTURES.with(|states| {
                        let context = states.borrow().get(&target).cloned();
                        if let Some(context) = context {
                            reads::drain(&context);
                        }
                    })
                })
                .is_err()
            {
                break;
            }
            if expired {
                break;
            }
        })
        .map(|_| ())
        .map_err(|_| "browser_research_deadline_unavailable".into())
}

pub(super) fn resume(app: &tauri::AppHandle, handle: &HostHandle) -> Result<(), String> {
    if handle.control.closed.load(Ordering::SeqCst) || now_ms() >= handle.control.expires_at_ms {
        return Err("browser_research_session_inactive".into());
    }
    if app.get_webview(&handle.label).is_none() {
        return Err("browser_research_window_unavailable".into());
    }
    let generation = handle.begin_handshake(true);
    let handle = handle.clone();
    let failure_handle = handle.clone();
    app.run_on_main_thread(move || {
        if !handle.handshake_stage(generation, Stage::NativeAttached) {
            return;
        }
        CAPTURES.with(|states| {
            let context = states.borrow().get(&handle.label).cloned();
            if let Some(context) = context {
                context.borrow_mut().synchronize();
                enable::run(&context, generation, false);
            } else {
                handle.fail_handshake(generation, "host_session_unavailable");
            }
        });
    })
    .map_err(|_| {
        failure_handle.fail_handshake(generation, "browser_research_host_dispatch_failed");
        "browser_research_host_dispatch_failed".to_string()
    })
}

fn gap(handle: &HostHandle, code: &str) {
    let mut event = HostEvent::new(handle.generation(), "gap", "");
    event.error_code = Some(code.into());
    (handle.control.sink)(event);
}

fn emit(context: &Context, event: HostEvent) {
    let handle = context.borrow().handle.clone();
    if handle.accepts(event.generation) {
        (handle.control.sink)(event);
    }
}
