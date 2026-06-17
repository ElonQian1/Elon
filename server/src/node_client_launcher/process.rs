use anyhow::{bail, Context, Result};
use std::{
    net::TcpStream,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use super::{env_file, paths, DEFAULT_ADMIN_PORT, DEFAULT_BASE_URL};

pub(crate) fn start_or_open(install_dir: &Path) -> Result<()> {
    let internal_dir = paths::internal_dir(install_dir);
    let agent = paths::agent_exe(install_dir);
    if !agent.exists() {
        bail!("缺少内部节点程序：{}", agent.display());
    }

    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    let port = env_values
        .get("NODE_ADMIN_PORT")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_ADMIN_PORT);

    if !is_port_open(port) {
        let mut cmd = Command::new(&agent);
        cmd.current_dir(&internal_dir)
            .envs(&env_values)
            .env("NODE_AUTO_OPEN_ADMIN", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        spawn_hidden(&mut cmd).with_context(|| format!("无法启动 {}", agent.display()))?;
        wait_for_port(port, Duration::from_secs(15));
    }

    open_pc_web_page(port, &env_values)
}

pub(crate) fn stop_agent() {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/IM", "elon-node-agent.exe", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("pkill")
            .arg("elon-node-agent")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

pub(crate) fn launch_installed_client(install_dir: &Path) -> Result<()> {
    let client = paths::client_exe(install_dir);
    if !client.exists() {
        bail!("缺少客户端启动器：{}", client.display());
    }
    #[cfg(windows)]
    {
        let command = format!(
            "timeout /t 2 /nobreak >nul & start \"\" \"{}\"",
            client.display()
        );
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", &command])
            .current_dir(install_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        spawn_hidden(&mut cmd).with_context(|| format!("无法启动 {}", client.display()))?;
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new(&client);
        cmd.current_dir(install_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        spawn_hidden(&mut cmd).with_context(|| format!("无法启动 {}", client.display()))?;
    }
    Ok(())
}

pub(crate) fn open_pc_web_page(
    port: u16,
    env_values: &std::collections::HashMap<String, String>,
) -> Result<()> {
    let local_admin_url = format!("http://127.0.0.1:{port}/");
    let open_target = env_values
        .get("NODE_AGENT_OPEN_TARGET")
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "pc_web".to_string());
    let url = if open_target == "local_admin" {
        local_admin_url
    } else {
        let base_url = web_base_url(env_values);
        format!(
            "{}/pc?node_admin={}",
            base_url.trim_end_matches('/'),
            encode_query_component(&local_admin_url)
        )
    };
    open_url(&url)
}

fn open_url(url: &str) -> Result<()> {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", url])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        spawn_hidden(&mut cmd).with_context(|| format!("无法打开管理页 {url}"))?;
    }
    #[cfg(not(windows))]
    {
        Command::new("xdg-open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("无法打开管理页 {url}"))?;
    }
    Ok(())
}

fn web_base_url(env_values: &std::collections::HashMap<String, String>) -> String {
    env_values
        .get("NODE_AGENT_WEB_BASE_URL")
        .or_else(|| env_values.get("NODE_AGENT_UPDATE_BASE_URL"))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_BASE_URL)
        .to_string()
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub(crate) fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if is_port_open(port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

pub(crate) fn wait_for_port_closed(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if !is_port_open(port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    !is_port_open(port)
}

pub(crate) fn is_port_open(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

fn spawn_hidden(command: &mut Command) -> std::io::Result<std::process::Child> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn()
}
