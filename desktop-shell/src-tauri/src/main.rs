// desktop-shell/src-tauri/src/main.rs
//
// 一龙桌面工作台原生窗口壳（原型）。
//
// 定位：不重新实现 PC 工作台，只是把已经存在的 `/pc`（pc-frontend）
// 装进一个原生窗口，替代现状——现状是双击“一龙开发平台.exe”
// （elon-pc-node，见 server/src/node_agent_admin_open.rs）时静默启动
// 后台服务，再打开系统默认浏览器标签页。
//
// 已完成：首屏加载遮罩（见 LOADING_OVERLAY_SCRIPT）、系统托盘（替代
// tray-launcher.ps1）、关闭按钮最小化到托盘而不是直接退出、全局快捷键
// 呼出/隐藏窗口、原生通知（首次最小化到托盘时提示一次）、开机自启动
// （见 autostart.rs，Startup 文件夹快捷方式，托盘菜单里用户手动 opt-in）。
//
// 后续阶段再接入：
//   - 复用 node_agent_admin_open::admin_url() 的云端可达探测 + 本地回退
//   - 无边框自定义标题栏 + Mica/Acrylic 背景（两者耦合：Mica 只有在窗口有
//     透明/半透明区域时才可见，当前整窗被远程页面不透明内容铺满，暂不做，
//     避免加了个看不见效果的依赖）
//   - 安装器/快捷方式接入（desktop-shell 作为新增 elon-desktop.exe）

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod autostart;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;

/// 主窗口 label，托盘菜单/左键点击/全局快捷键需要用它找回窗口。
const MAIN_WINDOW_LABEL: &str = "main";

/// 只在本次进程生命周期里提醒一次"已最小化到托盘"，避免每次关闭都打扰用户。
static NOTIFIED_BACKGROUND: AtomicBool = AtomicBool::new(false);

/// 目标工作台地址。
///
/// 原型阶段支持用环境变量覆盖，方便对着本地 `pc-frontend` dev server
/// （例如 `ELON_DESKTOP_URL=http://localhost:5173/pc`）或线上工作台联调；
/// 默认打开线上工作台。
fn target_url() -> String {
    std::env::var("ELON_DESKTOP_URL")
        .unwrap_or_else(|_| "http://43.139.149.158:8080/pc".to_string())
}

/// 呼出/隐藏窗口的全局快捷键。选 Ctrl+Alt+E：避开中文输入法常用的
/// Ctrl+Space / Shift+Space 切换键位，也不和常见系统快捷键冲突。
fn toggle_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyE)
}

/// 显示并聚焦主窗口；找不到窗口就什么都不做。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 全局快捷键行为：窗口可见就隐藏，隐藏/最小化到托盘就唤出，符合"快速呼出"预期。
fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let visible = window.is_visible().unwrap_or(false);
        if visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

/// 首屏加载遮罩：远程页面首次打开偶尔需要数秒到十几秒（服务器在海外/低配主机上冷启动或
/// 首次没有浏览器缓存），纯黑背景下如果没有任何反馈，用户会误以为窗口卡死。
/// 这段脚本在页面自己的内容解析前注入一个墨黑遮罩 + 呼吸态文字，等 `load` 事件
/// 触发（对应 Tauri 的 `PageLoadEvent::Finished`）后自动移除，不改动 pc-frontend 本身。
const LOADING_OVERLAY_SCRIPT: &str = r#"
(function () {
  if (window.__elonDesktopLoadingInjected) { return; }
  window.__elonDesktopLoadingInjected = true;

  function mount() {
    var style = document.createElement('style');
    style.textContent = '@keyframes elonDesktopPulse{0%,100%{opacity:.45}50%{opacity:1}}';

    var overlay = document.createElement('div');
    overlay.id = '__elon_desktop_loading_overlay__';
    overlay.textContent = '正在连接一龙工作台…';
    overlay.style.cssText = [
      'position:fixed', 'inset:0', 'z-index:2147483647',
      'background:#000000', 'color:#D9D9D9',
      'display:flex', 'align-items:center', 'justify-content:center',
      'font-family:\"Microsoft YaHei\",\"Segoe UI\",Arial,sans-serif',
      'font-size:14px',
      'animation:elonDesktopPulse 1.6s ease-in-out infinite'
    ].join(';');

    (document.head || document.documentElement).appendChild(style);
    (document.body || document.documentElement).appendChild(overlay);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', mount, { once: true });
  } else {
    mount();
  }

  window.addEventListener('load', function () {
    var el = document.getElementById('__elon_desktop_loading_overlay__');
    if (el && el.parentNode) {
      el.parentNode.removeChild(el);
    }
  }, { once: true });
})();
"#;

/// 无边框标记：告诉 pc-frontend “你跑在一龙桌面壳的无边框窗口里”，
/// 页面据此渲染自己的 36px 标题栏（拖拽区 + 最小化/最大化/关闭按钮，
/// 通过 window.__TAURI__.window 调用窗口控制）。浏览器里没有这个标记，
/// 页面保持原样，两种宿主互不影响。
const FRAMELESS_FLAG_SCRIPT: &str = "window.__ELON_DESKTOP_FRAMELESS__ = true;";

fn main() {
    tauri::Builder::default()
        // 单实例：用户重复双击客户端图标（或启动器再次 spawn elon-desktop）时，
        // 不再开第二个窗口，而是把已有窗口唤出并聚焦。必须是第一个注册的插件，
        // 这样重复启动的进程在做任何初始化之前就会退出。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed && *shortcut == toggle_shortcut() {
                        toggle_main_window(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let url: tauri::Url = target_url()
                .parse()
                .expect("ELON_DESKTOP_URL 必须是合法 URL");

            println!("[elon-desktop] 目标地址: {url}");

            let window = WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, WebviewUrl::External(url))
                .title("一龙工作台")
                .inner_size(1280.0, 800.0)
                .min_inner_size(960.0, 600.0)
                .center()
                // 无边框：标题栏由 pc-frontend 根据 FRAMELESS_FLAG_SCRIPT 自己渲染，
                // 视觉上与页面纯黑主题无缝，替代系统白色/浅色标题栏。
                .decorations(false)
                // 服务器 43.139.149.158 在本机网络环境下需要绕开系统代理才能直连
                // （项目里 curl/SSH 访问该服务器都要求 --noproxy / ProxyCommand=none）。
                // WebView2 默认走系统代理，这里强制直连，同时保留 wry 默认关闭的
                // Edge 附加组件参数。
                .additional_browser_args(
                    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --no-proxy-server",
                )
                .initialization_script(LOADING_OVERLAY_SCRIPT)
                .initialization_script(FRAMELESS_FLAG_SCRIPT)
                .on_navigation(|url| {
                    println!("[elon-desktop] 导航 -> {url}");
                    true
                })
                .on_page_load(|_window, payload| {
                    println!(
                        "[elon-desktop] 页面事件 {:?} -> {}",
                        payload.event(),
                        payload.url()
                    );
                })
                .build()?;

            // 原型排障阶段自动打开 DevTools，方便直接看控制台报错和网络请求。
            #[cfg(debug_assertions)]
            window.open_devtools();

            app.global_shortcut().register(toggle_shortcut())?;

            // 关闭按钮只隐藏窗口，真正退出走托盘菜单——这样它才像一个“一直在”
            // 的 agent 客户端，而不是一个关掉就没了的网页标签。首次隐藏顺带
            // 弹一次系统通知，告诉用户它还在后台运行，之后不再重复打扰。
            let hide_on_close_window = window.clone();
            let notify_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = hide_on_close_window.hide();
                    if !NOTIFIED_BACKGROUND.swap(true, Ordering::SeqCst) {
                        let result = notify_handle
                            .notification()
                            .builder()
                            .title("一龙工作台")
                            .body("已最小化到系统托盘，仍在后台运行。按 Ctrl+Alt+E 或点击托盘图标重新打开。")
                            .show();
                        if let Err(error) = result {
                            eprintln!("[elon-desktop] 系统通知发送失败: {error:#}");
                        }
                    }
                }
            });

            // 系统托盘：替代旧的 tray-launcher.ps1。左键点开/聚焦主窗口，
            // 右键菜单可以重新打开窗口、切换开机自启动或彻底退出进程。
            let handle = app.handle();
            let show_item =
                MenuItem::with_id(handle, "show", "打开一龙工作台", true, None::<&str>)?;
            let autostart_item = CheckMenuItem::with_id(
                handle,
                "autostart",
                "开机自动启动",
                true,
                autostart::is_enabled(),
                None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(handle, "quit", "退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(handle, &[&show_item, &autostart_item, &quit_item])?;

            let autostart_item_for_event = autostart_item.clone();
            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("缺少默认窗口图标").clone())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .tooltip("一龙工作台")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => show_main_window(app),
                    "autostart" => {
                        // 用户在托盘里主动切换才会触发，符合“开机自启动必须
                        // 明确 opt-in”的产品要求，不会被安装/修复流程默默打开。
                        let result = if autostart::is_enabled() {
                            autostart::disable().map(|_| false)
                        } else {
                            autostart::enable().map(|_| true)
                        };
                        match result {
                            Ok(checked) => {
                                let _ = autostart_item_for_event.set_checked(checked);
                            }
                            Err(error) => {
                                eprintln!("[elon-desktop] 开机自启动切换失败: {error:#}");
                            }
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("一龙桌面壳启动失败");
}
