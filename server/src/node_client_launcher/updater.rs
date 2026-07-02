// server/src/node_client_launcher/updater.rs

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{path::Path, time::Duration};

use super::{command as launcher_command, env_file, paths, process, DEFAULT_BASE_URL};

const DEFAULT_UPDATE_CONNECT_TIMEOUT_SECS: u64 = 20;
const DEFAULT_UPDATE_DOWNLOAD_TIMEOUT_SECS: u64 = 15 * 60;
const DEFAULT_UPDATE_DOWNLOAD_RETRIES: usize = 3;

#[derive(Debug, Deserialize)]
struct VersionInfo {
    #[serde(default, rename = "gitSha")]
    git_sha: String,
    #[serde(default, rename = "downloadUrl")]
    download_url: String,
    #[serde(default, rename = "windowsClientDownloadUrl")]
    windows_client_download_url: String,
}

pub(crate) fn update_client_if_needed(install_dir: &Path) -> Result<bool> {
    match try_update_client_if_needed(install_dir) {
        Ok(scheduled_restart) => Ok(scheduled_restart),
        Err(error) => {
            super::log_file::record_event(
                install_dir,
                "auto_update_failed",
                false,
                &format!("{error:#}"),
            );
            eprintln!("自动更新检查失败，继续使用本地版本: {error:#}");
            Ok(false)
        }
    }
}

fn try_update_client_if_needed(install_dir: &Path) -> Result<bool> {
    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    if auto_update_disabled(&env_values) {
        return Ok(false);
    }

    let base_url = update_base_url(&env_values);
    let version_url = format!("{}/api/node-agent/version", base_url.trim_end_matches('/'));
    let client = update_http_client()?;
    let remote_text = client
        .get(&version_url)
        .send()
        .with_context(|| format!("无法请求 {version_url}"))?
        .error_for_status()
        .with_context(|| format!("版本接口返回错误 {version_url}"))?
        .text()
        .context("无法读取远程版本内容")?;
    let remote_text = remote_text.trim_start_matches('\u{feff}').to_string();
    let remote: VersionInfo =
        serde_json::from_str(&remote_text).context("远程版本内容不是合法 JSON")?;
    if remote.git_sha.trim().is_empty() {
        return Ok(false);
    }

    let version_file = paths::version_file(install_dir);
    let local_sha = read_local_git_sha(&version_file).unwrap_or_default();
    if local_sha == remote.git_sha {
        return Ok(false);
    }

    let package_url = if remote.windows_client_download_url.trim().is_empty() {
        format!(
            "{}/api/node-agent/download/windows-client",
            base_url.trim_end_matches('/')
        )
    } else {
        remote.windows_client_download_url.clone()
    };
    match try_update_from_client_package(install_dir, &package_url, &remote_text) {
        Ok(updated) => return Ok(updated),
        Err(error) => {
            super::log_file::record_event(
                install_dir,
                "client_package_update_failed",
                false,
                &format!("{error:#}"),
            );
            eprintln!("完整客户端包更新失败，回退到单 exe 更新: {error:#}");
        }
    }

    let download_url = if remote.download_url.trim().is_empty() {
        format!(
            "{}/api/node-agent/download/windows",
            base_url.trim_end_matches('/')
        )
    } else {
        remote.download_url.clone()
    };

    let internal_dir = paths::internal_dir(install_dir);
    std::fs::create_dir_all(&internal_dir)
        .with_context(|| format!("无法创建内部目录 {}", internal_dir.display()))?;
    let tmp_exe = internal_dir.join("一龙开发平台.exe.new");
    let tmp_version = internal_dir.join("node-agent-version.json.new");
    let bytes = download_bytes_with_retries(&client, &download_url, "无法读取下载内容")?;
    if bytes.len() < 1024 * 1024 {
        anyhow::bail!("下载的客户端程序过小，疑似异常响应");
    }
    std::fs::write(&tmp_exe, &bytes).with_context(|| format!("无法写入 {}", tmp_exe.display()))?;
    std::fs::write(&tmp_version, remote_text)
        .with_context(|| format!("无法写入 {}", tmp_version.display()))?;

    process::stop_agent();
    let client = paths::client_exe(install_dir);
    let uninstall = paths::uninstall_exe(install_dir);
    if running_from_path(&client) {
        schedule_self_replace(&tmp_exe, &client, &uninstall, &tmp_version, &version_file)?;
        Ok(true)
    } else {
        replace_client_files(&tmp_exe, &client, &uninstall, &tmp_version, &version_file)?;
        Ok(false)
    }
}

fn auto_update_disabled(env_values: &std::collections::HashMap<String, String>) -> bool {
    env_values
        .get("NODE_AGENT_AUTO_UPDATE")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(false)
}

fn try_update_from_client_package(
    install_dir: &Path,
    package_url: &str,
    remote_text: &str,
) -> Result<bool> {
    let internal_dir = paths::internal_dir(install_dir);
    std::fs::create_dir_all(&internal_dir)
        .with_context(|| format!("无法创建内部目录 {}", internal_dir.display()))?;
    let tmp_zip = internal_dir.join("elon-node-agent-windows.zip.new");
    let tmp_version = internal_dir.join("node-agent-version.json.new");
    let version_file = paths::version_file(install_dir);
    let client = update_http_client()?;
    let bytes = download_bytes_with_retries(&client, package_url, "无法读取完整客户端包内容")?;
    if bytes.len() < 1024 * 1024 {
        anyhow::bail!("下载的完整客户端包过小，疑似异常响应");
    }
    std::fs::write(&tmp_zip, &bytes).with_context(|| format!("无法写入 {}", tmp_zip.display()))?;
    std::fs::write(&tmp_version, remote_text)
        .with_context(|| format!("无法写入 {}", tmp_version.display()))?;

    process::stop_agent();
    let client = paths::client_exe(install_dir);
    if running_from_path(&client) {
        schedule_self_replace_package(&tmp_zip, install_dir, &tmp_version, &version_file)?;
        Ok(true)
    } else {
        replace_client_package(&tmp_zip, install_dir, &tmp_version, &version_file)?;
        Ok(false)
    }
}

fn update_http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(env_u64(
            "NODE_AGENT_UPDATE_CONNECT_TIMEOUT_SECS",
            DEFAULT_UPDATE_CONNECT_TIMEOUT_SECS,
            2,
            120,
        )))
        .timeout(Duration::from_secs(env_u64(
            "NODE_AGENT_UPDATE_DOWNLOAD_TIMEOUT_SECS",
            DEFAULT_UPDATE_DOWNLOAD_TIMEOUT_SECS,
            60,
            60 * 60,
        )))
        .build()
        .context("无法创建更新下载客户端")
}

fn download_bytes_with_retries(
    client: &reqwest::blocking::Client,
    url: &str,
    read_context: &'static str,
) -> Result<Vec<u8>> {
    let attempts = env_usize(
        "NODE_AGENT_UPDATE_DOWNLOAD_RETRIES",
        DEFAULT_UPDATE_DOWNLOAD_RETRIES,
        1,
        5,
    );
    let mut last_error = None;
    for attempt in 1..=attempts {
        match download_bytes_once(client, url, read_context) {
            Ok(bytes) => return Ok(bytes),
            Err(error) => {
                last_error = Some(error);
                if attempt < attempts {
                    std::thread::sleep(Duration::from_millis(700 * attempt as u64));
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("下载失败: {url}")))
}

fn download_bytes_once(
    client: &reqwest::blocking::Client,
    url: &str,
    read_context: &'static str,
) -> Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .with_context(|| format!("无法下载 {url}"))?
        .error_for_status()
        .with_context(|| format!("下载接口返回错误 {url}"))?;
    let bytes = response.bytes().context(read_context)?;
    Ok(bytes.to_vec())
}

fn env_u64(name: &str, default: u64, min: u64, max: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn env_usize(name: &str, default: usize, min: usize, max: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn replace_client_package(
    tmp_zip: &Path,
    install_dir: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> Result<()> {
    #[cfg(windows)]
    {
        let script = package_replace_script(None, tmp_zip, install_dir, tmp_version, version_file);
        let mut cmd = launcher_command::powershell_hidden_command(&script);
        let status =
            launcher_command::status_hidden(&mut cmd).context("无法执行完整客户端包更新")?;
        if !status.success() {
            anyhow::bail!("完整客户端包更新脚本失败");
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (tmp_zip, install_dir, tmp_version, version_file);
        anyhow::bail!("完整客户端包更新只支持 Windows")
    }
}

fn schedule_self_replace_package(
    tmp_zip: &Path,
    install_dir: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = version_file;
        let script = package_self_update_via_repair_script(
            std::process::id(),
            tmp_zip,
            install_dir,
            tmp_version,
        );
        let mut cmd = launcher_command::powershell_hidden_command(&script);
        launcher_command::spawn_hidden(&mut cmd).context("无法安排完整客户端包自更新")?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        replace_client_package(tmp_zip, install_dir, tmp_version, version_file)
    }
}

#[cfg(windows)]
fn package_self_update_via_repair_script(
    pid_to_wait: u32,
    tmp_zip: &Path,
    install_dir: &Path,
    tmp_version: &Path,
) -> String {
    format!(
        r#"
$ErrorActionPreference = 'Stop'
$pidToWait = {pid_to_wait}
$zip = '{tmp_zip}'
$installDir = '{install_dir}'
$tmpVersion = '{tmp_version}'
$extractDir = Join-Path ([System.IO.Path]::GetTempPath()) ('elon-node-agent-update-' + [Guid]::NewGuid().ToString('N'))
$logDir = Join-Path $installDir '_internal\logs'
$logFile = Join-Path $logDir 'client-update.log'
function Write-ElonNodeUpdateLog {{
  param([string]$Message)
  try {{
    New-Item -ItemType Directory -Force -Path $logDir | Out-Null
    Add-Content -LiteralPath $logFile -Encoding UTF8 -Value ((Get-Date).ToString('o') + ' ' + $Message)
  }} catch {{}}
}}
try {{
  Write-ElonNodeUpdateLog "scheduled package repair update from $zip"
  Wait-Process -Id $pidToWait -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
  Expand-Archive -LiteralPath $zip -DestinationPath $extractDir -Force
  $packageClient = Join-Path $extractDir '一龙开发平台.exe'
  if (-not (Test-Path -LiteralPath $packageClient)) {{ $packageClient = Join-Path $extractDir '一龙PC节点.exe' }}
  if (!(Test-Path -LiteralPath $packageClient)) {{ throw '完整客户端包缺少主程序' }}
  $repair = Start-Process -FilePath $packageClient -ArgumentList '--repair' -WorkingDirectory $extractDir -WindowStyle Hidden -PassThru
  if ($null -eq $repair) {{ throw 'Start-Process did not return a repair process handle' }}
  Wait-Process -Id $repair.Id -Timeout 120 -ErrorAction Stop
  $repair.Refresh()
  if (($null -ne $repair.ExitCode) -and ($repair.ExitCode -ne 0)) {{ throw "repair process failed with exit code $($repair.ExitCode)" }}
  Write-ElonNodeUpdateLog "package repair update finished"
}} catch {{
  Write-ElonNodeUpdateLog ("package repair update failed: " + ($_ | Out-String))
  throw
}} finally {{
  Remove-Item -LiteralPath $extractDir -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $zip -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $tmpVersion -Force -ErrorAction SilentlyContinue
}}
"#,
        pid_to_wait = pid_to_wait,
        tmp_zip = launcher_command::ps_single_quote(&tmp_zip.to_string_lossy()),
        install_dir = launcher_command::ps_single_quote(&install_dir.to_string_lossy()),
        tmp_version = launcher_command::ps_single_quote(&tmp_version.to_string_lossy())
    )
}

#[cfg(windows)]
fn package_replace_script(
    pid_to_wait: Option<u32>,
    tmp_zip: &Path,
    install_dir: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> String {
    let wait = pid_to_wait
        .map(|pid| format!("Wait-Process -Id {pid} -ErrorAction SilentlyContinue\n"))
        .unwrap_or_default();
    let restart = pid_to_wait
        .map(|_| restart_agent_runtime_after_update_script("$client", "$installDir"))
        .unwrap_or_default();
    format!(
        r#"
$ErrorActionPreference = 'Stop'
{wait}$zip = '{tmp_zip}'
$installDir = '{install_dir}'
$tmpVersion = '{tmp_version}'
$versionFile = '{version_file}'
$client = Join-Path $installDir '一龙开发平台.exe'
$uninstall = Join-Path $installDir '卸载一龙开发平台.exe'
$extractDir = Join-Path ([System.IO.Path]::GetTempPath()) ('elon-node-agent-update-' + [Guid]::NewGuid().ToString('N'))
{replace_helpers}
New-Item -ItemType Directory -Force -Path $extractDir | Out-Null
try {{
  Expand-Archive -LiteralPath $zip -DestinationPath $extractDir -Force
  # 支持新旧包名称（新：一龙开发平台，旧：一龙PC节点）
  $packageClient = Join-Path $extractDir '一龙开发平台.exe'
  if (-not (Test-Path -LiteralPath $packageClient)) {{ $packageClient = Join-Path $extractDir '一龙PC节点.exe' }}
  $packageUninstall = Join-Path $extractDir '卸载一龙开发平台.exe'
  if (-not (Test-Path -LiteralPath $packageUninstall)) {{ $packageUninstall = Join-Path $extractDir '卸载一龙PC节点.exe' }}
  $packageInternal = Join-Path $extractDir '_internal'
  if (!(Test-Path -LiteralPath $packageClient)) {{ throw '完整客户端包缺少主程序' }}
  if (!(Test-Path -LiteralPath $packageUninstall)) {{ throw '完整客户端包缺少卸载程序' }}
  New-Item -ItemType Directory -Force -Path $installDir | Out-Null
  $targetInternal = Join-Path $installDir '_internal'
  New-Item -ItemType Directory -Force -Path $targetInternal | Out-Null
  Stop-ElonNodeClientProcesses -Client $client
  Copy-ElonNodeFileWithRetry -Source $packageClient -Destination $client
  Copy-ElonNodeFileWithRetry -Source $packageUninstall -Destination $uninstall
  if (Test-Path -LiteralPath $packageInternal) {{
    Copy-Item -Path (Join-Path $packageInternal '*') -Destination $targetInternal -Recurse -Force
  }}
  Move-ElonNodeFileWithRetry -Source $tmpVersion -Destination $versionFile
}} finally {{
  Remove-Item -LiteralPath $extractDir -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $zip -Force -ErrorAction SilentlyContinue
}}
{restart}"#,
        wait = wait,
        tmp_zip = launcher_command::ps_single_quote(&tmp_zip.to_string_lossy()),
        install_dir = launcher_command::ps_single_quote(&install_dir.to_string_lossy()),
        tmp_version = launcher_command::ps_single_quote(&tmp_version.to_string_lossy()),
        version_file = launcher_command::ps_single_quote(&version_file.to_string_lossy()),
        replace_helpers = UPDATE_REPLACE_HELPERS,
        restart = restart
    )
}

fn replace_client_files(
    tmp_exe: &Path,
    client: &Path,
    uninstall: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> Result<()> {
    let _ = std::fs::remove_file(client);
    std::fs::rename(tmp_exe, client).context("替换客户端主程序失败")?;
    std::fs::copy(client, uninstall).context("同步卸载程序失败")?;
    let _ = std::fs::remove_file(version_file);
    std::fs::rename(tmp_version, version_file).context("替换版本信息失败")?;
    Ok(())
}

fn running_from_path(path: &Path) -> bool {
    std::env::current_exe()
        .ok()
        .map(|current| same_path(&current, path))
        .unwrap_or(false)
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn schedule_self_replace(
    tmp_exe: &Path,
    client: &Path,
    uninstall: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> Result<()> {
    #[cfg(windows)]
    {
        let script = self_replace_script(tmp_exe, client, uninstall, tmp_version, version_file);
        // 自替换必须等当前进程退出，脚本保留但入口和重启都强制隐藏窗口。
        let mut cmd = launcher_command::powershell_hidden_command(&script);
        launcher_command::spawn_hidden(&mut cmd).context("无法安排客户端自更新")?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        replace_client_files(tmp_exe, client, uninstall, tmp_version, version_file)
    }
}

#[cfg(windows)]
fn self_replace_script(
    tmp_exe: &Path,
    client: &Path,
    uninstall: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> String {
    format!(
        r#"
$ErrorActionPreference = 'Stop'
$pidToWait = {pid}
$tmpExe = '{tmp_exe}'
$client = '{client}'
$uninstall = '{uninstall}'
$tmpVersion = '{tmp_version}'
$versionFile = '{version_file}'
$installDir = [System.IO.Path]::GetDirectoryName($client)
{replace_helpers}
Wait-Process -Id $pidToWait -ErrorAction SilentlyContinue
Stop-ElonNodeClientProcesses -Client $client
Move-ElonNodeFileWithRetry -Source $tmpExe -Destination $client
Copy-ElonNodeFileWithRetry -Source $client -Destination $uninstall
Move-ElonNodeFileWithRetry -Source $tmpVersion -Destination $versionFile
{restart}
"#,
        pid = std::process::id(),
        tmp_exe = launcher_command::ps_single_quote(&tmp_exe.to_string_lossy()),
        client = launcher_command::ps_single_quote(&client.to_string_lossy()),
        uninstall = launcher_command::ps_single_quote(&uninstall.to_string_lossy()),
        tmp_version = launcher_command::ps_single_quote(&tmp_version.to_string_lossy()),
        version_file = launcher_command::ps_single_quote(&version_file.to_string_lossy()),
        replace_helpers = UPDATE_REPLACE_HELPERS,
        restart = restart_agent_runtime_after_update_script("$client", "$installDir")
    )
}

#[cfg(windows)]
const UPDATE_REPLACE_HELPERS: &str = r#"
function Get-ElonNodeClientProcesses {
  param([Parameter(Mandatory = $true)][string]$Client)
  $fullClient = [System.IO.Path]::GetFullPath($Client)
  Get-CimInstance Win32_Process | Where-Object {
    $matchesClient = $false
    if ($_.ExecutablePath) {
      try {
        $matchesClient = ([System.IO.Path]::GetFullPath($_.ExecutablePath) -ieq $fullClient)
      } catch {}
    }
    $matchesClient -or ($_.Name -eq 'elon-node-agent.exe')
  }
}

function Stop-ElonNodeClientProcesses {
  param(
    [Parameter(Mandatory = $true)][string]$Client,
    [int]$TimeoutSeconds = 20
  )
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ($true) {
    $targets = @(Get-ElonNodeClientProcesses -Client $Client)
    if ($targets.Count -eq 0) { return }
    foreach ($target in $targets) {
      try { Invoke-CimMethod -InputObject $target -MethodName Terminate | Out-Null } catch {}
    }
    if ((Get-Date) -ge $deadline) {
      throw "旧版一龙节点仍在运行，无法完成更新: $Client"
    }
    Start-Sleep -Milliseconds 500
  }
}

function Copy-ElonNodeFileWithRetry {
  param(
    [Parameter(Mandatory = $true)][string]$Source,
    [Parameter(Mandatory = $true)][string]$Destination,
    [int]$Attempts = 30
  )
  $lastError = $null
  for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
    try {
      Copy-Item -LiteralPath $Source -Destination $Destination -Force
      return
    } catch {
      $lastError = $_
      Start-Sleep -Milliseconds ([Math]::Min(250 * $attempt, 2000))
    }
  }
  throw $lastError
}

function Move-ElonNodeFileWithRetry {
  param(
    [Parameter(Mandatory = $true)][string]$Source,
    [Parameter(Mandatory = $true)][string]$Destination,
    [int]$Attempts = 30
  )
  $lastError = $null
  for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
    try {
      Move-Item -LiteralPath $Source -Destination $Destination -Force
      return
    } catch {
      $lastError = $_
      Start-Sleep -Milliseconds ([Math]::Min(250 * $attempt, 2000))
    }
  }
  throw $lastError
}
"#;

#[cfg(windows)]
const UPDATE_RESTART_HELPERS: &str = r#"
function Get-ElonNodeAdminPort {
  param([Parameter(Mandatory = $true)][string]$InstallDir)
  $port = 7799
  $envFile = Join-Path $InstallDir '_internal\node-agent.env'
  if (Test-Path -LiteralPath $envFile) {
    foreach ($line in (Get-Content -LiteralPath $envFile -ErrorAction SilentlyContinue)) {
      if ($line -match '^\s*NODE_ADMIN_PORT\s*=\s*"?([0-9]+)"?\s*$') {
        $port = [int]$Matches[1]
      }
    }
  }
  return $port
}

function Test-ElonNodeAdminHealth {
  param([Parameter(Mandatory = $true)][int]$Port)
  try {
    $resp = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/api/status" -UseBasicParsing -TimeoutSec 2
    return (($resp.StatusCode -eq 200) -and ([string]$resp.Content).Contains('"local_admin_token_header"'))
  } catch {
    return $false
  }
}

function Wait-ElonNodeAdminHealth {
  param(
    [Parameter(Mandatory = $true)][int]$Port,
    [int]$TimeoutSeconds = 25
  )
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    if (Test-ElonNodeAdminHealth -Port $Port) { return $true }
    Start-Sleep -Milliseconds 500
  }
  return (Test-ElonNodeAdminHealth -Port $Port)
}

function Start-ElonNodeRuntimeAndWait {
  param(
    [Parameter(Mandatory = $true)][string]$Client,
    [Parameter(Mandatory = $true)][string]$InstallDir
  )
  $port = Get-ElonNodeAdminPort -InstallDir $InstallDir
  Start-Process -FilePath $Client -ArgumentList '--agent-runtime' -WorkingDirectory $InstallDir -WindowStyle Hidden
  if (-not (Wait-ElonNodeAdminHealth -Port $port -TimeoutSeconds 25)) {
    Start-Process -FilePath $Client -ArgumentList '--agent-runtime' -WorkingDirectory $InstallDir -WindowStyle Hidden
    if (-not (Wait-ElonNodeAdminHealth -Port $port -TimeoutSeconds 10)) {
      throw "新版一龙节点启动后健康检查超时: http://127.0.0.1:$port/api/status"
    }
  }
}
"#;

#[cfg(windows)]
fn restart_agent_runtime_after_update_script(client_expr: &str, install_dir_expr: &str) -> String {
    let mut script = String::from(UPDATE_RESTART_HELPERS);
    script.push_str("Start-ElonNodeRuntimeAndWait -Client ");
    script.push_str(client_expr);
    script.push_str(" -InstallDir ");
    script.push_str(install_dir_expr);
    script.push('\n');
    script
}

fn update_base_url(env_values: &std::collections::HashMap<String, String>) -> String {
    env_values
        .get("NODE_AGENT_UPDATE_BASE_URL")
        .cloned()
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

fn read_local_git_sha(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("gitSha")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn self_replace_script_stops_on_failure_before_restart() {
        use std::path::Path;

        let script = super::self_replace_script(
            Path::new(r"C:\ElonNode\_internal\一龙开发平台.exe.new"),
            Path::new(r"C:\ElonNode\一龙开发平台.exe"),
            Path::new(r"C:\ElonNode\卸载一龙开发平台.exe"),
            Path::new(r"C:\ElonNode\_internal\node-agent-version.json.new"),
            Path::new(r"C:\ElonNode\_internal\node-agent-version.json"),
        );

        assert!(script.contains("$ErrorActionPreference = 'Stop'"));
        assert!(
            script.contains("Start-ElonNodeRuntimeAndWait -Client $client -InstallDir $installDir")
        );
        assert!(script.contains("Stop-ElonNodeClientProcesses -Client $client"));
        assert!(script.contains("Move-ElonNodeFileWithRetry -Source $tmpExe"));
        assert!(script.contains("Copy-ElonNodeFileWithRetry -Source $client"));
        assert!(script.contains("Start-Process -FilePath $Client -ArgumentList '--agent-runtime'"));
        assert!(script.contains("Wait-ElonNodeAdminHealth"));
        assert!(
            script
                .find("Move-ElonNodeFileWithRetry -Source $tmpExe")
                .unwrap()
                < script.find("Start-ElonNodeRuntimeAndWait").unwrap()
        );
        assert!(
            script
                .find("Move-ElonNodeFileWithRetry -Source $tmpVersion")
                .unwrap()
                < script.find("Start-ElonNodeRuntimeAndWait").unwrap()
        );
    }

    #[cfg(windows)]
    #[test]
    fn package_replace_script_updates_full_client_layout() {
        use std::path::Path;

        let script = super::package_replace_script(
            Some(1234),
            Path::new(r"C:\ElonNode\_internal\elon-node-agent-windows.zip.new"),
            Path::new(r"C:\ElonNode"),
            Path::new(r"C:\ElonNode\_internal\node-agent-version.json.new"),
            Path::new(r"C:\ElonNode\_internal\node-agent-version.json"),
        );

        assert!(script.contains("Wait-Process -Id 1234"));
        assert!(script.contains("Expand-Archive -LiteralPath $zip"));
        assert!(script.contains("Stop-ElonNodeClientProcesses -Client $client"));
        assert!(script.contains("Copy-ElonNodeFileWithRetry -Source $packageClient"));
        assert!(script.contains("Copy-ElonNodeFileWithRetry -Source $packageUninstall"));
        assert!(script.contains("Copy-Item -Path (Join-Path $packageInternal '*')"));
        assert!(script.contains("Move-ElonNodeFileWithRetry -Source $tmpVersion"));
        assert!(
            script.contains("Start-ElonNodeRuntimeAndWait -Client $client -InstallDir $installDir")
        );
        assert!(script.contains("Start-Process -FilePath $Client -ArgumentList '--agent-runtime'"));
        assert!(script.contains("Wait-ElonNodeAdminHealth"));
    }

    #[cfg(windows)]
    #[test]
    fn package_self_update_uses_extracted_repair_entrypoint() {
        use std::path::Path;

        let script = super::package_self_update_via_repair_script(
            1234,
            Path::new(r"C:\ElonNode\_internal\elon-node-agent-windows.zip.new"),
            Path::new(r"C:\ElonNode"),
            Path::new(r"C:\ElonNode\_internal\node-agent-version.json.new"),
        );

        assert!(script.contains("Wait-Process -Id $pidToWait"));
        assert!(script.contains("Expand-Archive -LiteralPath $zip"));
        assert!(script.contains("Start-Process -FilePath $packageClient -ArgumentList '--repair'"));
        assert!(script.contains("Wait-Process -Id $repair.Id -Timeout 120"));
        assert!(script.contains("client-update.log"));
        assert!(script.contains("Remove-Item -LiteralPath $tmpVersion"));
        assert!(!script.contains("Copy-ElonNodeFileWithRetry -Source $packageClient"));
        assert!(!script.contains("Start-ElonNodeRuntimeAndWait"));
    }
}
