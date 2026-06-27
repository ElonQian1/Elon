// server/src/node_agent_admin_open.rs

#[cfg(windows)]
use std::time::Duration;

pub fn admin_port_from_env() -> u16 {
    std::env::var("NODE_ADMIN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7799)
}

/// 打开云端 PC 工作台。用户登录后云端页面会自动探测本机节点并完成绑定。
/// 回退到本地管理页可通过 NODE_OPEN_LOCAL=1 切换。
#[cfg(windows)]
fn admin_url(port: u16) -> String {
    if std::env::var("NODE_OPEN_LOCAL").map(|v| v == "1").unwrap_or(false) {
        return format!("http://127.0.0.1:{port}/");
    }
    let cloud_base = std::env::var("NODE_CLOUD_URL")
        .unwrap_or_else(|_| "ws://43.139.149.158:8080/agent/ws".to_string());
    let http_base = if let Some(rest) = cloud_base.strip_prefix("wss://") {
        format!("https://{}", rest.split('/').next().unwrap_or("43.139.149.158:8080"))
    } else if let Some(rest) = cloud_base.strip_prefix("ws://") {
        format!("http://{}", rest.split('/').next().unwrap_or("43.139.149.158:8080"))
    } else {
        "http://43.139.149.158:8080".to_string()
    };
    // 直接打开 PC 工作台；云端页面会在后台静默探测 localhost:7799
    format!("{http_base}/pc")
}

#[cfg(windows)]
pub fn maybe_open_admin_page(port: u16) {
    if !auto_open_enabled() {
        return;
    }
    std::thread::spawn(move || {
        wait_for_admin_port(port);
        let url = admin_url(port);
        let mut cmd = std::process::Command::new("explorer.exe");
        cmd
            // 避免 cmd /C start 额外拉起 shell，减少旧系统上闪黑窗的概率。
            .arg(&url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Err(err) = spawn_hidden(&mut cmd) {
            tracing::warn!(%url, error = %err, "无法自动打开 node-agent 管理页");
        }
    });
}

#[cfg(not(windows))]
pub fn maybe_open_admin_page(_port: u16) {}

#[cfg(windows)]
fn wait_for_admin_port(port: u16) {
    for _ in 0..40 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[cfg(windows)]
fn auto_open_enabled() -> bool {
    std::env::var("NODE_AUTO_OPEN_ADMIN")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(true)
}

#[cfg(windows)]
fn spawn_hidden(command: &mut std::process::Command) -> std::io::Result<std::process::Child> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command
        .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .spawn()
}
