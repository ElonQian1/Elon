// server/src/node_client_launcher/process.rs

use anyhow::{bail, Context, Result};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::{
    command as launcher_command, env_file, log_file, paths, AGENT_RUNTIME_ARG, CLIENT_EXE_NAME,
    DEFAULT_ADMIN_PORT, DEFAULT_BASE_URL,
};

const ADMIN_HEALTH_TIMEOUT: Duration = Duration::from_secs(4);
const ADMIN_PC_WEB_READY_WAIT: Duration = Duration::from_secs(5);
const ADMIN_LOCAL_READY_WAIT: Duration = Duration::from_secs(15);
const ADMIN_LOCAL_RETRY_WAIT: Duration = Duration::from_secs(10);
const ADMIN_PORT_FALLBACK_LIMIT: u16 = 20;
const ADMIN_HEALTH_READ_LIMIT: usize = 16 * 1024;

pub(crate) fn start_or_open(install_dir: &Path) -> Result<()> {
    let client = paths::client_exe(install_dir);
    if !client.exists() {
        bail!("缺少客户端主程序：{}", client.display());
    }

    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    let port = admin_port_from_env_values(&env_values);
    let runtime_already_running = agent_runtime_running(install_dir);
    let port = select_admin_port_for_runtime(port, runtime_already_running);

    if !admin_healthy(port, ADMIN_HEALTH_TIMEOUT) {
        if !runtime_already_running {
            spawn_agent_runtime(&client, install_dir, port, &env_values)?;
        }

        let open_target = open_target_from_env_values(&env_values);
        let first_wait = if open_target.requires_admin_ready() {
            ADMIN_LOCAL_READY_WAIT
        } else {
            ADMIN_PC_WEB_READY_WAIT
        };

        if !wait_for_admin_ready(port, first_wait) {
            if !agent_runtime_running(install_dir) {
                spawn_agent_runtime(&client, install_dir, port, &env_values)?;
            }

            if open_target.requires_admin_ready()
                && !wait_for_admin_ready(port, ADMIN_LOCAL_RETRY_WAIT)
            {
                bail!("一龙节点本机管理接口启动超时：http://127.0.0.1:{port}/api/status");
            }

            if !open_target.requires_admin_ready() && !admin_healthy(port, ADMIN_HEALTH_TIMEOUT) {
                let runtime_running = agent_runtime_running(install_dir);
                log_file::record_event(
                    install_dir,
                    "launcher_admin_wait_timeout",
                    false,
                    &format!(
                        "admin api still warming; runtime_running={runtime_running}; opening PC workspace anyway: http://127.0.0.1:{port}/api/status"
                    ),
                );
            }
        }
    }

    open_pc_web_page(port, &env_values)
}

pub(crate) fn open_installed_pc_web_page(install_dir: &Path) -> Result<()> {
    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    open_pc_web_page(admin_port_from_env_values(&env_values), &env_values)
}

fn admin_port_from_env_values(env_values: &HashMap<String, String>) -> u16 {
    env_values
        .get("NODE_ADMIN_PORT")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_ADMIN_PORT)
}

fn spawn_agent_runtime(
    client: &Path,
    install_dir: &Path,
    port: u16,
    env_values: &HashMap<String, String>,
) -> Result<()> {
    #[cfg(windows)]
    {
        match spawn_agent_runtime_via_powershell(client, install_dir, port, env_values) {
            Ok(pid) => {
                log_file::record_event(
                    install_dir,
                    "launcher_runtime_spawned",
                    true,
                    &format!("method=powershell; pid={pid}; port={port}"),
                );
                return Ok(());
            }
            Err(error) => {
                log_file::record_event(
                    install_dir,
                    "launcher_runtime_spawn_powershell_failed",
                    false,
                    &format!("{error:#}"),
                );
            }
        }
    }

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

#[cfg(windows)]
fn spawn_agent_runtime_via_powershell(
    client: &Path,
    install_dir: &Path,
    port: u16,
    env_values: &HashMap<String, String>,
) -> Result<u32> {
    let pid_file = runtime_spawn_pid_file(install_dir)?;
    let _ = std::fs::remove_file(&pid_file);
    let script = spawn_agent_runtime_script(client, install_dir, port, env_values, &pid_file);
    let mut ps = launcher_command::powershell_hidden_command(&script);
    let status = launcher_command::status_hidden(&mut ps)
        .context("failed to start node runtime via PowerShell")?;
    if !status.success() {
        bail!("PowerShell Start-Process failed with status {status}");
    }
    let pid_text = std::fs::read_to_string(&pid_file).with_context(|| {
        format!(
            "PowerShell Start-Process did not write {}",
            pid_file.display()
        )
    })?;
    let _ = std::fs::remove_file(&pid_file);
    pid_text
        .trim()
        .parse::<u32>()
        .context("PowerShell Start-Process wrote an invalid runtime pid")
}

#[cfg(windows)]
fn spawn_agent_runtime_script(
    client: &Path,
    install_dir: &Path,
    port: u16,
    env_values: &HashMap<String, String>,
    pid_file: &Path,
) -> String {
    let mut script = String::from("$ErrorActionPreference = 'Stop'\n");
    let mut env_pairs: Vec<_> = env_values.iter().collect();
    env_pairs.sort_by(|left, right| left.0.cmp(right.0));
    for (key, value) in env_pairs {
        push_process_env_assignment(&mut script, key, value);
    }
    push_process_env_assignment(&mut script, "NODE_ADMIN_PORT", &port.to_string());
    push_process_env_assignment(&mut script, "NODE_AUTO_OPEN_ADMIN", "0");
    script.push_str(&format!(
        "$client = '{}'\n",
        launcher_command::ps_single_quote(&client.to_string_lossy())
    ));
    script.push_str(&format!(
        "$installDir = '{}'\n",
        launcher_command::ps_single_quote(&install_dir.to_string_lossy())
    ));
    script.push_str(&format!(
        "$pidFile = '{}'\n",
        launcher_command::ps_single_quote(&pid_file.to_string_lossy())
    ));
    script.push_str("$process = Start-Process -FilePath $client -ArgumentList '--agent-runtime' -WorkingDirectory $installDir -WindowStyle Hidden -PassThru\n");
    script.push_str(
        "if ($null -eq $process) { throw 'Start-Process did not return a process handle' }\n",
    );
    script.push_str(
        "Set-Content -LiteralPath $pidFile -Value ([string]$process.Id) -Encoding ASCII\n",
    );
    script
}

#[cfg(windows)]
fn runtime_spawn_pid_file(install_dir: &Path) -> Result<PathBuf> {
    let logs_dir = paths::internal_dir(install_dir).join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("failed to create {}", logs_dir.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    Ok(logs_dir.join(format!("runtime-spawn-{}-{nonce}.pid", std::process::id())))
}

#[cfg(windows)]
fn push_process_env_assignment(script: &mut String, key: &str, value: &str) {
    script.push_str(&format!(
        "[Environment]::SetEnvironmentVariable('{}', '{}', 'Process')\n",
        launcher_command::ps_single_quote(key),
        launcher_command::ps_single_quote(value)
    ));
}

pub(crate) fn stop_agent() {
    #[cfg(windows)]
    {
        let script = format!(
            r#"
$deadline = (Get-Date).AddSeconds(10)
do {{
  $targets = @(Get-CimInstance Win32_Process | Where-Object {{
    ($_.Name -eq '{client}' -and $_.CommandLine -match '--agent-runtime') -or
    ($_.Name -eq 'elon-node-agent.exe')
  }})
  foreach ($target in $targets) {{
    Invoke-CimMethod -InputObject $target -MethodName Terminate | Out-Null
  }}
  if ($targets.Count -eq 0) {{ break }}
  Start-Sleep -Milliseconds 300
}} while ((Get-Date) -lt $deadline)
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

pub(crate) fn open_pc_web_page(port: u16, env_values: &HashMap<String, String>) -> Result<()> {
    let local_admin_url = format!("http://127.0.0.1:{port}/");
    let url = if open_target_from_env_values(env_values) == OpenTarget::LocalAdmin {
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
        launcher_command::open_url(url).with_context(|| format!("无法打开管理页 {url}"))?;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OpenTarget {
    PcWeb,
    LocalAdmin,
}

impl OpenTarget {
    fn requires_admin_ready(self) -> bool {
        matches!(self, Self::LocalAdmin)
    }
}

fn open_target_from_env_values(env_values: &HashMap<String, String>) -> OpenTarget {
    match env_values
        .get("NODE_AGENT_OPEN_TARGET")
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("local_admin") => OpenTarget::LocalAdmin,
        _ => OpenTarget::PcWeb,
    }
}

fn web_base_url(env_values: &HashMap<String, String>) -> String {
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

fn select_admin_port_for_runtime(preferred: u16, runtime_already_running: bool) -> u16 {
    if runtime_already_running {
        preferred
    } else {
        select_admin_port(preferred)
    }
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
  ($line -match '--agent-runtime') -and $exeMatch
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

    #[test]
    fn admin_health_accepts_large_status_response() {
        let response = format!(
            "HTTP/1.1 200 OK\r\n\r\n{{\"padding\":\"{}\",\"local_admin_token_header\":\"x\"}}",
            "x".repeat(8 * 1024)
        );

        assert!(response.len() < ADMIN_HEALTH_READ_LIMIT);
        assert!(admin_status_response_healthy(&response));
    }

    #[test]
    fn open_target_defaults_to_pc_workspace() {
        let env_values = HashMap::new();

        assert_eq!(open_target_from_env_values(&env_values), OpenTarget::PcWeb);
        assert!(!open_target_from_env_values(&env_values).requires_admin_ready());
    }

    #[test]
    fn admin_port_defaults_and_accepts_configured_value() {
        let env_values = HashMap::new();
        assert_eq!(admin_port_from_env_values(&env_values), DEFAULT_ADMIN_PORT);

        let mut env_values = HashMap::new();
        env_values.insert("NODE_ADMIN_PORT".to_string(), "7801".to_string());
        assert_eq!(admin_port_from_env_values(&env_values), 7801);

        env_values.insert("NODE_ADMIN_PORT".to_string(), "not-a-port".to_string());
        assert_eq!(admin_port_from_env_values(&env_values), DEFAULT_ADMIN_PORT);
    }

    #[test]
    fn local_admin_open_target_requires_ready_admin_api() {
        let mut env_values = HashMap::new();
        env_values.insert(
            "NODE_AGENT_OPEN_TARGET".to_string(),
            " local_ADMIN ".to_string(),
        );

        assert_eq!(
            open_target_from_env_values(&env_values),
            OpenTarget::LocalAdmin
        );
        assert!(open_target_from_env_values(&env_values).requires_admin_ready());
    }

    #[test]
    fn running_runtime_keeps_configured_admin_port() {
        assert_eq!(select_admin_port_for_runtime(7799, true), 7799);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_query_matches_current_client_only() {
        let script = agent_runtime_query_script(Path::new(r"C:\ElonNode\一龙PC节点.exe"));

        assert!(script.contains("--agent-runtime"));
        assert!(script.contains(r"C:\ElonNode\一龙PC节点.exe"));
        assert!(script.contains("and $exeMatch"));
        assert!(!script.contains("lineMatch"));
        assert!(!script.contains("elon-node-agent.exe"));
    }

    #[cfg(windows)]
    #[test]
    fn runtime_spawn_script_uses_start_process_and_overrides_runtime_env() {
        let mut env_values = HashMap::new();
        env_values.insert("NODE_AUTO_OPEN_ADMIN".to_string(), "1".to_string());
        env_values.insert(
            "NODE_AGENT_WEB_BASE_URL".to_string(),
            "http://example.test".to_string(),
        );
        env_values.insert("QUOTED".to_string(), "O'Hara".to_string());

        let script = spawn_agent_runtime_script(
            Path::new(r"C:\ElonNode\client.exe"),
            Path::new(r"C:\ElonNode"),
            7801,
            &env_values,
            Path::new(r"C:\ElonNode\_internal\logs\runtime-spawn.pid"),
        );

        assert!(script.contains("Start-Process -FilePath $client -ArgumentList '--agent-runtime'"));
        assert!(script.contains("Set-Content -LiteralPath $pidFile"));
        assert!(!script.contains("Write-Output $process.Id"));
        assert!(script.contains(
            "[Environment]::SetEnvironmentVariable('NODE_ADMIN_PORT', '7801', 'Process')"
        ));
        assert!(script
            .contains("[Environment]::SetEnvironmentVariable('QUOTED', 'O''Hara', 'Process')"));
        assert!(script.contains(r"$client = 'C:\ElonNode\client.exe'"));
        assert!(script.contains(r"$pidFile = 'C:\ElonNode\_internal\logs\runtime-spawn.pid'"));

        let inherited_auto_open = script
            .find("[Environment]::SetEnvironmentVariable('NODE_AUTO_OPEN_ADMIN', '1', 'Process')")
            .unwrap();
        let launcher_auto_open = script
            .rfind("[Environment]::SetEnvironmentVariable('NODE_AUTO_OPEN_ADMIN', '0', 'Process')")
            .unwrap();
        assert!(launcher_auto_open > inherited_auto_open);
    }
}
