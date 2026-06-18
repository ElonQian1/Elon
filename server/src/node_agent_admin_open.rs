#[cfg(windows)]
use std::time::Duration;

pub fn admin_port_from_env() -> u16 {
    std::env::var("NODE_ADMIN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7799)
}

#[cfg(windows)]
fn admin_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/")
}

#[cfg(windows)]
pub fn maybe_open_admin_page(port: u16) {
    if !auto_open_enabled() {
        return;
    }
    std::thread::spawn(move || {
        wait_for_admin_port(port);
        let url = admin_url(port);
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C")
            .arg("start")
            .arg("")
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
    command.creation_flags(CREATE_NO_WINDOW).spawn()
}
