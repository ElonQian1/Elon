//! Real Windows child HWND regression. Uses only blank pages and a separate profile.
//! `--legacy` reproduces the old split-write bug and must exit nonzero.
#[path = "../src/webview_layout.rs"]
mod webview_layout;

#[cfg(windows)]
mod native {
    use super::webview_layout;
    use std::sync::atomic::{AtomicI32, Ordering};
    static RESULT: AtomicI32 = AtomicI32::new(2);
    use std::{sync::mpsc, thread, time::Duration};
    use tauri::{
        webview::WebviewBuilder, AppHandle, LogicalPosition, LogicalSize, PhysicalPosition,
        PhysicalSize, Webview, WebviewUrl, Window, WindowBuilder, WindowEvent,
    };
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{GetParent, GetWindowRect},
    };

    fn actual(view: &Webview, host: &Window) -> Result<[i32; 4], String> {
        let origin = host.inner_position().map_err(|e| e.to_string())?;
        let expected_parent = host.hwnd().map_err(|e| e.to_string())?.0 as isize;
        let (send, recv) = mpsc::channel();
        view.with_webview(move |platform| {
            let result = (|| unsafe {
                let mut child = Default::default();
                platform
                    .controller()
                    .ParentWindow(&mut child)
                    .map_err(|e| e.to_string())?;
                if GetParent(child.0 as _) as isize != expected_parent {
                    return Err("child HWND belongs to a different host".to_string());
                }
                let mut rect: RECT = std::mem::zeroed();
                if GetWindowRect(child.0 as _, &mut rect) == 0 {
                    return Err("GetWindowRect failed".to_string());
                }
                Ok([
                    rect.left - origin.x,
                    rect.top - origin.y,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                ])
            })();
            let _ = send.send(result);
        })
        .map_err(|e| e.to_string())?;
        recv.recv_timeout(Duration::from_secs(5))
            .map_err(|e| e.to_string())?
    }

    fn check(view: &Webview, host: &Window, expected: [i32; 4], stage: &str) -> Result<(), String> {
        let mut observed = [0; 4];
        for _ in 0..30 {
            thread::sleep(Duration::from_millis(50));
            observed = actual(view, host)?;
            if observed == expected {
                println!("NATIVE_LAYOUT_PASS={stage} bounds={observed:?}");
                return Ok(());
            }
        }
        Err(format!(
            "{stage}: expected={expected:?}, actual={observed:?}"
        ))
    }

    fn fill(view: &Webview, host: &Window, legacy: bool) -> Result<(), String> {
        if legacy {
            view.set_position(PhysicalPosition::new(0, 0))
                .map_err(|e| e.to_string())?;
            view.set_size(host.inner_size().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())
        } else {
            webview_layout::fill_host(view, host)
        }
    }

    fn check_full(view: &Webview, host: &Window, stage: &str) -> Result<(), String> {
        let size = host.inner_size().map_err(|e| e.to_string())?;
        check(
            view,
            host,
            [0, 0, size.width as i32, size.height as i32],
            stage,
        )
    }

    fn exercise(view: &Webview, main: &Window, popup: &Window, legacy: bool) -> Result<(), String> {
        for cycle in 0..3 {
            view.reparent(popup).map_err(|e| e.to_string())?;
            fill(view, popup, legacy)?;
            check_full(view, popup, &format!("detach-{cycle}"))?;
            popup
                .set_size(LogicalSize::new(820.0 + cycle as f64 * 80.0, 620.0))
                .map_err(|e| e.to_string())?;
            thread::sleep(Duration::from_millis(150));
            fill(view, popup, legacy)?;
            check_full(view, popup, "resize")?;
            popup.maximize().map_err(|e| e.to_string())?;
            thread::sleep(Duration::from_millis(200));
            check_full(view, popup, "maximize")?;
            popup.unmaximize().map_err(|e| e.to_string())?;
            thread::sleep(Duration::from_millis(200));
            check_full(view, popup, "restore")?;
            view.reparent(main).map_err(|e| e.to_string())?;
            for scale in [1.0, 1.25, 1.5, 2.0] {
                // Physical geometry at common DPI scales, measured through native HWNDs.
                let p = LogicalPosition::new(120.0, 80.0).to_physical::<i32>(scale);
                let s = LogicalSize::new(640.0, 440.0).to_physical::<u32>(scale);
                webview_layout::place(view, p, s)?;
                webview_layout::resize_hosted(view, popup.label(), PhysicalSize::new(333, 222))?;
                check(
                    view,
                    main,
                    [p.x, p.y, s.width as i32, s.height as i32],
                    "dock-stale-host-dpi",
                )?;
            }
            webview_layout::park(view, 20_000, 20_000)?;
            let size = view.size().map_err(|e| e.to_string())?;
            check(
                view,
                main,
                [20_000, 20_000, size.width as i32, size.height as i32],
                "park",
            )?;
        }
        Ok(())
    }

    pub fn run() {
        let legacy = std::env::args().any(|arg| arg == "--legacy");
        let profile = std::env::var_os("WEBVIEW_LAYOUT_SMOKE_PROFILE")
            .expect("isolated profile path required");
        tauri::Builder::default()
            .setup(move |app| {
                let main = WindowBuilder::new(app, "geometry-main")
                    .visible(false)
                    .position(30.0, 40.0)
                    .inner_size(1000.0, 720.0)
                    .build()?;
                let popup = WindowBuilder::new(app, "geometry-popup")
                    .visible(false)
                    .position(430.0, 240.0)
                    .inner_size(900.0, 640.0)
                    .build()?;
                let view = main.add_child(
                    WebviewBuilder::new(
                        "geometry-page",
                        WebviewUrl::External("about:blank".parse()?),
                    )
                    .data_directory(profile.into()),
                    LogicalPosition::new(120.0, 80.0),
                    LogicalSize::new(640.0, 440.0),
                )?;
                let resized_view = view.clone();
                let host_label = popup.label().to_string();
                popup.on_window_event(move |event| {
                    let size = match event {
                        WindowEvent::Resized(size) => Some(*size),
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            Some(*new_inner_size)
                        }
                        _ => None,
                    };
                    if let Some(size) = size {
                        let _ = webview_layout::resize_hosted(&resized_view, &host_label, size);
                    }
                });
                let handle: AppHandle = app.handle().clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(200));
                    match exercise(&view, &main, &popup, legacy) {
                        Ok(()) => {
                            println!("NATIVE_LAYOUT_RESULT=passed");
                            RESULT.store(0, Ordering::SeqCst);
                            handle.exit(0);
                        }
                        Err(error) => {
                            eprintln!("NATIVE_LAYOUT_RESULT=failed {error}");
                            RESULT.store(1, Ordering::SeqCst);
                            handle.exit(1);
                        }
                    }
                });
                Ok(())
            })
            .build(tauri::generate_context!())
            .expect("native geometry harness")
            .run_return(|_, _| {});
        std::process::exit(RESULT.load(Ordering::SeqCst));
    }
}

fn main() {
    #[cfg(windows)]
    native::run();
    #[cfg(not(windows))]
    panic!("this regression requires Windows WebView2");
}
