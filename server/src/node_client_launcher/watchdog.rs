use anyhow::{bail, Context, Result};
use std::{
    path::Path,
    time::{Duration, Instant},
};

use super::{command as launcher_command, env_file, log_file, paths, process, WATCHDOG_ARG};

const DEFAULT_INTERVAL_SECS: u64 = 15;
const MIN_INTERVAL_SECS: u64 = 5;
const MAX_INTERVAL_SECS: u64 = 300;
const RESTART_AFTER_FAILURES: u32 = 3;
const ADMIN_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const ADMIN_START_WAIT: Duration = Duration::from_secs(8);
const ADMIN_RESTART_WAIT: Duration = Duration::from_secs(12);
const PORT_CLOSE_WAIT: Duration = Duration::from_secs(8);

pub(crate) fn ensure_running(install_dir: &Path) -> Result<()> {
    #[cfg(not(windows))]
    {
        let _ = install_dir;
        return Ok(());
    }

    #[cfg(windows)]
    {
        if watchdog_running(install_dir) {
            return Ok(());
        }
        let client = paths::client_exe(install_dir);
        if !client.exists() {
            bail!("缺少客户端守护程序：{}", client.display());
        }
        let mut command = launcher_command::silent_command(&client);
        command
            .arg(WATCHDOG_ARG)
            .current_dir(install_dir)
            .env("NODE_AUTO_OPEN_ADMIN", "0");
        let child = launcher_command::spawn_hidden(&mut command)
            .with_context(|| format!("无法启动 Win 端守护进程 {}", client.display()))?;
        log_file::record_event(
            install_dir,
            "watchdog_spawned",
            true,
            &format!("pid={}; arg={WATCHDOG_ARG}", child.id()),
        );
        Ok(())
    }
}

pub(crate) fn stop_running(install_dir: &Path) {
    #[cfg(not(windows))]
    {
        let _ = install_dir;
    }

    #[cfg(windows)]
    {
        let script = watchdog_stop_script(&paths::client_exe(install_dir), std::process::id());
        let mut command = launcher_command::powershell_hidden_command(&script);
        let _ = launcher_command::status_hidden(&mut command);
    }
}

pub(crate) fn run_loop(install_dir: &Path) -> Result<()> {
    let client = paths::client_exe(install_dir);
    if !client.exists() {
        bail!("缺少客户端守护程序：{}", client.display());
    }

    let interval = watchdog_interval();
    let mut state = WatchdogState::default();
    log_file::record_event(
        install_dir,
        "watchdog_started",
        true,
        &format!("interval_secs={}", interval.as_secs()),
    );

    loop {
        let started = Instant::now();
        if let Err(error) = check_once(install_dir, &client, &mut state) {
            log_file::record_event(
                install_dir,
                "watchdog_check_failed",
                false,
                &format!("{error:#}"),
            );
        }
        let elapsed = started.elapsed();
        if elapsed < interval {
            std::thread::sleep(interval - elapsed);
        }
    }
}

fn check_once(install_dir: &Path, client: &Path, state: &mut WatchdogState) -> Result<()> {
    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    let port = process::admin_port_from_env_values(&env_values);
    if process::admin_healthy(port, ADMIN_PROBE_TIMEOUT) {
        if state.consecutive_admin_failures > 0 {
            log_file::record_event(
                install_dir,
                "watchdog_admin_recovered",
                true,
                &format!("port={port}; failures={}", state.consecutive_admin_failures),
            );
        }
        state.consecutive_admin_failures = 0;
        return Ok(());
    }

    let runtime_running = process::agent_runtime_running(install_dir);
    if !runtime_running {
        state.consecutive_admin_failures = 0;
        log_file::record_event(
            install_dir,
            "watchdog_runtime_missing",
            false,
            &format!("starting runtime on port {port}"),
        );
        process::spawn_agent_runtime(client, install_dir, port, &env_values)?;
        if !process::wait_for_admin_ready(port, ADMIN_START_WAIT) {
            state.consecutive_admin_failures = 1;
            log_file::record_event(
                install_dir,
                "watchdog_runtime_start_unhealthy",
                false,
                &format!("admin api did not become healthy on port {port}"),
            );
        }
        return Ok(());
    }

    state.consecutive_admin_failures += 1;
    log_file::record_event(
        install_dir,
        "watchdog_admin_unhealthy",
        false,
        &format!(
            "port={port}; consecutive_failures={}; restart_after={RESTART_AFTER_FAILURES}",
            state.consecutive_admin_failures
        ),
    );

    if should_restart(state.consecutive_admin_failures, RESTART_AFTER_FAILURES) {
        log_file::record_event(
            install_dir,
            "watchdog_restart_runtime",
            false,
            &format!("admin api stuck on port {port}; restarting runtime"),
        );
        process::stop_agent();
        let _ = process::wait_for_port_closed(port, PORT_CLOSE_WAIT);
        process::spawn_agent_runtime(client, install_dir, port, &env_values)?;
        if process::wait_for_admin_ready(port, ADMIN_RESTART_WAIT) {
            log_file::record_event(
                install_dir,
                "watchdog_restart_recovered",
                true,
                &format!("port={port}"),
            );
            state.consecutive_admin_failures = 0;
        } else {
            state.consecutive_admin_failures = 1;
            log_file::record_event(
                install_dir,
                "watchdog_restart_still_unhealthy",
                false,
                &format!("port={port}"),
            );
        }
    }

    Ok(())
}

#[derive(Default)]
struct WatchdogState {
    consecutive_admin_failures: u32,
}

pub(super) fn should_restart(consecutive_failures: u32, threshold: u32) -> bool {
    threshold > 0 && consecutive_failures >= threshold
}

fn watchdog_interval() -> Duration {
    watchdog_interval_from(std::env::var("NODE_WATCHDOG_INTERVAL_SECS").ok().as_deref())
}

pub(super) fn watchdog_interval_from(value: Option<&str>) -> Duration {
    let seconds = value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECS)
        .clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS);
    Duration::from_secs(seconds)
}

#[cfg(windows)]
fn watchdog_running(install_dir: &Path) -> bool {
    let script = watchdog_query_script(&paths::client_exe(install_dir), std::process::id());
    let mut command = launcher_command::powershell_hidden_command(&script);
    let Ok(output) = launcher_command::output_hidden(&mut command) else {
        return false;
    };
    output.status.success() && String::from_utf8_lossy(&output.stdout).contains("running")
}

#[cfg(windows)]
pub(super) fn watchdog_query_script(client: &Path, current_pid: u32) -> String {
    format!(
        r#"
$target = [System.IO.Path]::GetFullPath('{client}')
$currentPid = {current_pid}
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
  ([uint32]$_.ProcessId -ne [uint32]$currentPid) -and ($line -match '--watchdog') -and $exeMatch
}}
if ($targets) {{ Write-Output 'running' }}
"#,
        client = launcher_command::ps_single_quote(&client.to_string_lossy()),
        current_pid = current_pid
    )
}

#[cfg(windows)]
pub(super) fn watchdog_stop_script(client: &Path, current_pid: u32) -> String {
    format!(
        r#"
$target = [System.IO.Path]::GetFullPath('{client}')
$currentPid = {current_pid}
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
  ([uint32]$_.ProcessId -ne [uint32]$currentPid) -and ($line -match '--watchdog') -and $exeMatch
}}
foreach ($targetProcess in $targets) {{
  Invoke-CimMethod -InputObject $targetProcess -MethodName Terminate | Out-Null
}}
"#,
        client = launcher_command::ps_single_quote(&client.to_string_lossy()),
        current_pid = current_pid
    )
}
