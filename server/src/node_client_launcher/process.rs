use anyhow::{bail, Context, Result};
use std::{
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use super::{
    env_file, paths, AGENT_RUNTIME_ARG, CLIENT_EXE_NAME, DEFAULT_ADMIN_PORT, DEFAULT_BASE_URL,
};

const ADMIN_HEALTH_TIMEOUT: Duration = Duration::from_millis(900);
const ADMIN_PORT_FALLBACK_LIMIT: u16 = 20;

pub(crate) fn start_or_open(install_dir: &Path) -> Result<()> {
    let client = paths::client_exe(install_dir);
    if !client.exists() {
        bail!("缺少客户端主程序：{}", client.display());
    }

    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    let port = env_values
        .get("NODE_ADMIN_PORT")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_ADMIN_PORT);
    let port = select_admin_port(port);

    if !admin_healthy(port, ADMIN_HEALTH_TIMEOUT) {
        let mut cmd = Command::new(&client);
        cmd.arg(AGENT_RUNTIME_ARG)
            .current_dir(install_dir)
            .envs(&env_values)
            .env("NODE_ADMIN_PORT", port.to_string())
            .env("NODE_AUTO_OPEN_ADMIN", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        spawn_hidden(&mut cmd).with_context(|| format!("无法启动 {}", client.display()))?;
        wait_for_admin_ready(port, Duration::from_secs(15));
    }

    open_pc_web_page(port, &env_values)
}

pub(crate) fn stop_agent() {
    #[cfg(windows)]
    {
        let script = format!(
            r#"
$targets = Get-CimInstance Win32_Process | Where-Object {{
  ($_.Name -eq '{client}' -and $_.CommandLine -match '--agent-runtime') -or
  ($_.Name -eq 'elon-node-agent.exe')
}}
foreach ($target in $targets) {{
  Invoke-CimMethod -InputObject $target -MethodName Terminate | Out-Null
}}
"#,
            client = ps_single_quote(CLIENT_EXE_NAME)
        );
        let mut ps = Command::new("powershell");
        ps.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
        let _ = status_hidden(&mut ps);

        let mut cmd = Command::new("taskkill");
        cmd.args(["/IM", "elon-node-agent.exe", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = status_hidden(&mut cmd);
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("pkill");
        cmd.arg("elon-node-agent")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = status_hidden(&mut cmd);
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
            "Start-Sleep -Seconds 2; Start-Process -FilePath '{}'",
            ps_single_quote(&client.to_string_lossy())
        );
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &command,
        ])
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

#[cfg(windows)]
fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
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

fn select_admin_port(preferred: u16) -> u16 {
    if admin_healthy(preferred, ADMIN_HEALTH_TIMEOUT) || !is_port_open(preferred) {
        return preferred;
    }
    for offset in 1..=ADMIN_PORT_FALLBACK_LIMIT {
        let Some(port) = preferred.checked_add(offset) else {
            break;
        };
        if admin_healthy(port, ADMIN_HEALTH_TIMEOUT) || !is_port_open(port) {
            return port;
        }
    }
    preferred
}

fn wait_for_admin_ready(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if admin_healthy(port, ADMIN_HEALTH_TIMEOUT) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    admin_healthy(port, ADMIN_HEALTH_TIMEOUT)
}

fn admin_healthy(port: u16, timeout: Duration) -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, timeout) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let request = b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(request).is_err() {
        return false;
    }
    let mut buf = [0u8; 32];
    match stream.read(&mut buf) {
        Ok(n) if n > 0 => {
            let head = String::from_utf8_lossy(&buf[..n]);
            head.starts_with("HTTP/1.1 200") || head.starts_with("HTTP/1.0 200")
        }
        _ => false,
    }
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

fn status_hidden(command: &mut Command) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.status()
}
