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

use super::types::{now_ms, Control, HostConfig, HostEvent, HostHandle, HostSink};
mod cdp;
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
        }),
    };
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
        }
        allowed
    })
    // Never silently launch an unscoped popup or move the user's Chrome session.
    .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
    .build()
    .map_err(|_| "browser_research_window_unavailable")?;
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
    window
        .with_webview(move |platform| {
            let core = unsafe { platform.controller().CoreWebView2() };
            let Ok(core) = core else {
                gap(&install_handle, "host_core_unavailable");
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
                gap(&install_handle, "host_subscription_unavailable");
                return;
            }
            cdp::enable(&context, true);
        })
        .map_err(|_| "browser_research_host_dispatch_failed")?;
    if start_deadline_poll(app, &handle).is_err() {
        handle.pause();
        gap(&handle, "browser_research_deadline_unavailable");
    }
    Ok(handle)
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
    let window = app
        .get_webview(&handle.label)
        .ok_or("browser_research_window_unavailable")?;
    let handle = handle.clone();
    window
        .with_webview(move |_| {
            CAPTURES.with(|states| {
                let context = states.borrow().get(&handle.label).cloned();
                if let Some(context) = context {
                    handle.control.generation.fetch_add(1, Ordering::SeqCst);
                    handle.control.active.store(true, Ordering::SeqCst);
                    context.borrow_mut().synchronize();
                    cdp::enable(&context, false);
                } else {
                    gap(&handle, "host_session_unavailable");
                }
            });
        })
        .map_err(|_| "browser_research_host_dispatch_failed".to_string())
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
