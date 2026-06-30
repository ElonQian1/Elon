// server/src/node_client_launcher/process.rs

use anyhow::{bail, Context, Result};
use std::{
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    time::Duration,
};

use super::{
    command as launcher_command, env_file, paths, AGENT_RUNTIME_ARG, CLIENT_EXE_NAME,
    DEFAULT_ADMIN_PORT, DEFAULT_BASE_URL,
};

const ADMIN_HEALTH_TIMEOUT: Duration = Duration::from_millis(900);
const ADMIN_PORT_FALLBACK_LIMIT: u16 = 20;
const ADMIN_HEALTH_READ_LIMIT: usize = 4096;

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
        if !agent_runtime_running(install_dir) {
            spawn_agent_runtime(&client, install_dir, port, &env_values)?;
        }
        if !wait_for_admin_ready(port, Duration::from_secs(15)) {
            spawn_agent_runtime(&client, install_dir, port, &env_values)?;
            if !wait_for_admin_ready(port, Duration::from_secs(10)) {
                bail!("一龙节点本机管理接口启动超时：http://127.0.0.1:{port}/api/status");
            }
        }
    }

    open_pc_web_page(port, &env_values)
}

fn spawn_agent_runtime(
    client: &Path,
    install_dir: &Path,
    port: u16,
    env_values: &std::collections::HashMap<String, String>,
) -> Result<()> {
    let mut cmd = launcher_command::silent_command(client);
    cmd.arg(AGENT_RUNTIME_ARG)
        .current_dir(install_dir)
        .envs(env_values)
        .env("NODE_ADMIN_PORT", port.to_string())
        .env("NODE_AUTO_OPEN_ADMIN", "0");
    launcher_command::spawn_hidden(&mut cmd)
        .with_context(|| format!("无法启动 {}", client.display()))?;
    Ok(())
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
            client = launcher_command::ps_single_quote(CLIENT_EXE_NAME)
        );
        let mut ps = launcher_command::powershell_hidden_command(&script);
        let _ = launcher_command::status_hidden(&mut ps);

        let mut cmd = launcher_command::silent_command("taskkill");
        cmd.args(["/IM", "elon-node-agent.exe", "/F"]);
        let _ = launcher_command::status_hidden(&mut cmd);
    }
    #[cfg(not(windows))]
    {
        let mut cmd = launcher_command::silent_command("pkill");
        cmd.arg("elon-node-agent");
        let _ = launcher_command::status_hidden(&mut cmd);
    }
}

pub(crate) fn launch_installed_client(install_dir: &Path) -> Result<()> {
    let client = paths::client_exe(install_dir);
    if !client.exists() {
        bail!("缺少客户端启动器：{}", client.display());
    }
    // 安装完成后直接启动目标 exe，不再经 PowerShell 二次拉起，降低黑窗和重复启动风险。
    let mut cmd = launcher_command::silent_command(&client);
    cmd.current_dir(install_dir);
    launcher_command::spawn_hidden(&mut cmd)
        .with_context(|| format!("无法启动 {}", client.display()))?;
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
        let mut cmd = launcher_command::open_url_command(url);
        launcher_command::spawn_hidden(&mut cmd)
            .with_context(|| format!("无法打开管理页 {url}"))?;
    }
    #[cfg(not(windows))]
    {
        let mut cmd = launcher_command::silent_command("xdg-open");
        cmd.arg(url);
        launcher_command::spawn_hidden(&mut cmd)
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

fn agent_runtime_running(install_dir: &Path) -> bool {
    #[cfg(windows)]
    {
        let script = agent_runtime_query_script(&paths::client_exe(install_dir));
        let mut command = launcher_command::powershell_hidden_command(&script);
        let Ok(output) = launcher_command::output_hidden(&mut command) else {
            return false;
        };
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("running")
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(windows)]
fn agent_runtime_query_script(client: &Path) -> String {
    format!(
        r#"
$target = [System.IO.Path]::GetFullPath('{client}')
$targets = Get-CimInstance Win32_Process | Where-Object {{
  $line = if ($_.CommandLine) {{ [string]$_.CommandLine }} else {{ '' }}
  $exe = if ($_.ExecutablePath) {{ [string]$_.ExecutablePath }} else {{ '' }}
  $exeMatch = $false
  if ($exe) {{
    try {{
      $exeMatch = [System.IO.Path]::GetFullPath($exe).Equals($target, [StringComparison]::OrdinalIgnoreCase)
    }} catch {{
      $exeMatch = $false
    }}
  }}
  $lineMatch = $line.IndexOf($target, [StringComparison]::OrdinalIgnoreCase) -ge 0
  ($line -match '--agent-runtime') -and ($exeMatch -or $lineMatch)
}}
if ($targets) {{ Write-Output 'running' }}
"#,
        client = launcher_command::ps_single_quote(&client.to_string_lossy())
    )
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
    let request = b"GET /api/status HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(request).is_err() {
        return false;
    }
    let mut response = Vec::new();
    let mut buf = [0u8; 512];
    while response.len() < ADMIN_HEALTH_READ_LIMIT {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let response = String::from_utf8_lossy(&response);
    admin_status_response_healthy(&response)
}

fn admin_status_response_healthy(response: &str) -> bool {
    let status_ok = response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200");
    // 只把本节点的 /api/status 当作健康，避免随机占用 7799 的网页服务误判为已就绪。
    status_ok && response.contains("\"local_admin_token_header\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_query_escapes_local_admin_url() {
        assert_eq!(
            encode_query_component("http://127.0.0.1:7799/?a=1&b=2"),
            "http%3A%2F%2F127.0.0.1%3A7799%2F%3Fa%3D1%26b%3D2"
        );
    }

    #[test]
    fn admin_health_requires_node_status_marker() {
        assert!(admin_status_response_healthy(
            "HTTP/1.1 200 OK\r\n\r\n{\"local_admin_token_header\":\"X-Elon-Local-Admin-Token\"}"
        ));
        assert!(!admin_status_response_healthy(
            "HTTP/1.1 200 OK\r\n\r\n<html>not our service</html>"
        ));
        assert!(!admin_status_response_healthy(
            "HTTP/1.1 404 Not Found\r\n\r\n{\"local_admin_token_header\":\"x\"}"
        ));
    }

    #[cfg(windows)]
    #[test]
    fn runtime_query_matches_current_client_only() {
        let script = agent_runtime_query_script(Path::new(r"C:\ElonNode\一龙PC节点.exe"));

        assert!(script.contains("--agent-runtime"));
        assert!(script.contains(r"C:\ElonNode\一龙PC节点.exe"));
        assert!(!script.contains("elon-node-agent.exe"));
    }
}
