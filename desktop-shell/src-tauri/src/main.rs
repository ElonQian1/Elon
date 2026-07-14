// desktop-shell/src-tauri/src/main.rs
//
// 一龙桌面工作台原生窗口壳（原型）。
//
// 定位：不重新实现 PC 工作台，只是把已经存在的 `/pc`（pc-frontend）
// 装进一个原生窗口，替代现状——现状是双击“一龙开发平台.exe”
// （elon-pc-node，见 server/src/node_agent_admin_open.rs）时静默启动
// 后台服务，再打开系统默认浏览器标签页。
//
// 原型阶段只验证核心可行性：原生窗口能否正常加载 /pc 并完成登录、
// 会话、实时刷新等既有能力。后续阶段再接入：
//   - 复用 node_agent_admin_open::admin_url() 的云端可达探测 + 本地回退
//   - 系统托盘（替代 tray-launcher.ps1）
//   - 无边框自定义标题栏 + Mica/Acrylic 背景
//   - 全局快捷键、原生通知
//   - 安装器/快捷方式接入（desktop-shell 作为新增 elon-desktop.exe）

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use tauri::{WebviewUrl, WebviewWindowBuilder};

/// 目标工作台地址。
///
/// 原型阶段支持用环境变量覆盖，方便对着本地 `pc-frontend` dev server
/// （例如 `ELON_DESKTOP_URL=http://localhost:5173/pc`）或线上工作台联调；
/// 默认打开线上工作台。
fn target_url() -> String {
    std::env::var("ELON_DESKTOP_URL").unwrap_or_else(|_| "http://43.139.149.158:8080/pc".to_string())
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

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let url: tauri::Url = target_url()
                .parse()
                .expect("ELON_DESKTOP_URL 必须是合法 URL");

            println!("[elon-desktop] 目标地址: {url}");

            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("一龙工作台")
                .inner_size(1280.0, 800.0)
                .min_inner_size(960.0, 600.0)
                .center()
                // 服务器 43.139.149.158 在本机网络环境下需要绕开系统代理才能直连
                // （项目里 curl/SSH 访问该服务器都要求 --noproxy / ProxyCommand=none）。
                // WebView2 默认走系统代理，这里强制直连，同时保留 wry 默认关闭的
                // Edge 附加组件参数。
                .additional_browser_args(
                    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --no-proxy-server",
                )
                .initialization_script(LOADING_OVERLAY_SCRIPT)
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("一龙桌面壳启动失败");
}
