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

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let url: tauri::Url = target_url()
                .parse()
                .expect("ELON_DESKTOP_URL 必须是合法 URL");

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("一龙工作台")
                .inner_size(1280.0, 800.0)
                .min_inner_size(960.0, 600.0)
                .center()
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("一龙桌面壳启动失败");
}
