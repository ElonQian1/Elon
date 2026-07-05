use axum::{http::StatusCode, Json};
#[cfg(windows)]
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};
#[cfg(windows)]
use std::process::{Command, Stdio};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
#[derive(Debug, Clone)]
pub(crate) struct AutostartInfo {
    pub(crate) source: String,
    pub(crate) command: Option<String>,
    pub(crate) legacy_detected: bool,
}

pub(crate) async fn push_update_from_server(
    cloud_http_url: &str,
    download_url_override: Option<&str>,
) -> Result<String, String> {
    // 先试已安装的更新程序
    #[cfg(windows)]
    if spawn_client_action(ClientAction::Update).is_ok() {
        record_maintenance_event(
            "push_update",
            true,
            "Win 端正在更新升级，通信临时中断，会自动恢复。via_installer",
        );
        return Ok("Win 端正在更新升级，通信临时中断，会自动恢复。".to_string());
    }
    // 没有安装程序时：直接下载新版 exe，写旁路 bat 脚本替换并重启
    let url = download_url_override
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "{}/api/node-agent/download/windows",
                cloud_http_url.trim_end_matches('/')
            )
        });
    let current_exe = std::env::current_exe().map_err(|e| format!("无法定位当前 exe: {e}"))?;
    let download_path = current_exe.with_extension("new.exe");
    let bat_path = current_exe.with_extension("update.bat");
    // 异步下载
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_default();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("下载新版失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("下载服务器返回 {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取下载内容失败: {e}"))?;
    tokio::fs::write(&download_path, &bytes)
        .await
        .map_err(|e| format!("写入新版 exe 失败: {e}"))?;
    // 写一个 bat 脚本：等待当前进程退出后替换并重启
    let cur_str = current_exe.to_string_lossy();
    let new_str = download_path.to_string_lossy();
    let bat_content = format!(
        "@echo off\r\ntimeout /t 2 /nobreak >nul\r\nmove /y \"{new_str}\" \"{cur_str}\"\r\nstart \"\" \"{cur_str}\" --agent-runtime\r\ndel \"%~f0\"\r\n"
    );
    tokio::fs::write(&bat_path, bat_content.as_bytes())
        .await
        .map_err(|e| format!("写入更新脚本失败: {e}"))?;
    // 启动 bat 然后退出当前进程
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", &bat_path.to_string_lossy()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000 | 0x0000_0200); // CREATE_NO_WINDOW | DETACHED
        cmd.spawn().map_err(|e| format!("启动更新脚本失败: {e}"))?;
    }
    record_maintenance_event(
        "push_update",
        true,
        "Win 端正在更新升级，通信临时中断，会自动恢复。via_download_replace",
    );
    // 异步退出（让当前响应先发出）
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        std::process::exit(0);
    });
    Ok(format!(
        "Win 端正在更新升级，通信临时中断，会自动恢复。下载大小: {} KB",
        bytes.len() / 1024
    ))
}
pub(crate) fn spawn_client_action(action: ClientAction) -> Result<(), String> {
    #[cfg(windows)]
    {
        let installed = installed_paths()?;
        let current_exe =
            std::env::current_exe().map_err(|error| format!("无法定位当前客户端程序: {error}"))?;
        let (program, arg) = match action {
            ClientAction::Repair => (
                if installed.client_exe.exists() {
                    installed.client_exe.clone()
                } else {
                    current_exe
                },
                "--repair",
            ),
            ClientAction::Update => (installed.client_exe.clone(), "--update"),
            ClientAction::Uninstall => (installed.uninstall_exe.clone(), "--uninstall"),
        };
        if !program.exists() {
            return Err(format!("缺少客户端程序: {}", program.display()));
        }
        let current_dir = if installed.install_dir.exists() {
            installed.install_dir.clone()
        } else {
            program
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        };
        let mut command = Command::new(&program);
        command.arg(arg).current_dir(current_dir);
        apply_hidden_window(&mut command);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("无法启动 {}: {error}", program.display()))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = action;
        Err("当前平台不支持 Windows 客户端维护动作。".to_string())
    }
}

#[cfg(windows)]
pub(crate) fn query_autostart_info() -> Result<AutostartInfo, String> {
    let script = autostart_query_script();
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]);
    command.arg(script);
    apply_hidden_window(&mut command);
    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("无法读取开机自启动设置: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "无法读取开机自启动设置: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let encoded = String::from_utf8_lossy(&output.stdout).trim().to_string();
    decode_autostart_info(&encoded)
}

#[cfg(windows)]
pub(crate) fn command_targets_path(command: &str, expected_path: &str) -> bool {
    let actual = command.trim().trim_matches('"').to_ascii_lowercase();
    let expected = expected_path.trim().trim_matches('"').to_ascii_lowercase();
    !expected.is_empty() && actual.contains(&expected)
}

#[cfg(windows)]
pub(crate) fn autostart_query_script() -> String {
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$result = [ordered]@{{
  source = 'none'
  command = $null
  task_name = '{task_name}'
  run_value_name = '{run_value_name}'
  legacy_detected = $false
}}
function Set-TaskCommand($Task, $Source) {{
  $actions = @($Task.Actions)
  if ($actions.Count -eq 0) {{ return }}
  $action = $actions[0]
  $execute = [string]$action.Execute
  $arguments = [string]$action.Arguments
  if ([string]::IsNullOrWhiteSpace($execute)) {{ return }}
  if ([string]::IsNullOrWhiteSpace($arguments)) {{
    $result.command = '"' + $execute + '"'
  }} else {{
    $result.command = '"' + $execute + '" ' + $arguments
  }}
  $result.source = $Source
}}
if (Get-Command Get-ScheduledTask -ErrorAction SilentlyContinue) {{
  $task = Get-ScheduledTask -TaskName '{task_name}' -ErrorAction SilentlyContinue
  if ($null -ne $task) {{
    Set-TaskCommand $task 'scheduled_task'
  }}
  foreach ($legacyTaskName in @({legacy_task_names})) {{
    $legacyTask = Get-ScheduledTask -TaskName $legacyTaskName -ErrorAction SilentlyContinue
    if ($null -ne $legacyTask) {{
      $result.legacy_detected = $true
      if ($result.source -eq 'none') {{
        Set-TaskCommand $legacyTask 'legacy_scheduled_task'
      }}
    }}
  }}
}}
$keyPath = 'Software\Microsoft\Windows\CurrentVersion\Run'
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($keyPath)
if ($null -ne $key) {{
  try {{
    foreach ($name in @({run_value_names})) {{
      $value = $key.GetValue($name, $null)
      if ($null -ne $value) {{
        if ($name -ne '{run_value_name}') {{
          $result.legacy_detected = $true
        }} elseif ($result.source -eq 'scheduled_task') {{
          $result.legacy_detected = $true
        }}
        if ($result.source -eq 'none') {{
          $result.source = if ($name -eq '{run_value_name}') {{ 'hkcu_run' }} else {{ 'legacy_hkcu_run' }}
          $result.command = [string]$value
        }}
      }}
    }}
  }} finally {{
    $key.Dispose()
  }}
}}
"#,
        task_name = ps_single_quote(crate::node_client_launcher::AUTOSTART_TASK_NAME),
        run_value_name = ps_single_quote(crate::node_client_launcher::AUTOSTART_RUN_VALUE_NAME),
        legacy_task_names =
            ps_string_array(crate::node_client_launcher::AUTOSTART_LEGACY_TASK_NAMES),
        run_value_names = ps_string_array(
            &[
                &[crate::node_client_launcher::AUTOSTART_RUN_VALUE_NAME],
                crate::node_client_launcher::AUTOSTART_LEGACY_RUN_VALUE_NAMES,
            ]
            .concat(),
        ),
    );
    format!(
        r#"{script}
$json = $result | ConvertTo-Json -Compress
$bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
[Console]::Out.Write([Convert]::ToBase64String($bytes))
"#
    )
}

#[cfg(windows)]
pub(crate) fn decode_autostart_info(encoded: &str) -> Result<AutostartInfo, String> {
    let encoded = encoded.trim();
    if encoded.is_empty() {
        return Ok(AutostartInfo {
            source: "none".to_string(),
            command: None,
            legacy_detected: false,
        });
    }
    let bytes = B64
        .decode(encoded)
        .map_err(|error| format!("开机自启动设置不是合法 base64: {error}"))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("开机自启动设置不是合法 JSON: {error}"))?;
    Ok(AutostartInfo {
        source: value
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        command: value
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_string),
        legacy_detected: value
            .get("legacy_detected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

#[cfg(windows)]
pub(crate) fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn ps_string_array(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("'{}'", ps_single_quote(value)))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn maintenance_paths() -> MaintenancePaths {
    let state_file = crate::state_path();
    let config_dir = state_file
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let logs_dir = config_dir.join("logs");
    MaintenancePaths {
        diagnostics_dir: config_dir.join("diagnostics"),
        task_journal_dir: state_file.with_file_name("task-journal"),
        launcher_logs_dir: launcher_logs_dir(),
        launcher_log_file: launcher_log_file(),
        maintenance_log_file: logs_dir.join("client-maintenance.jsonl"),
        logs_dir,
        state_file,
        config_dir,
    }
}

pub(crate) fn record_maintenance_event(action: &str, ok: bool, detail: &str) {
    let paths = maintenance_paths();
    let _ = fs::create_dir_all(&paths.logs_dir);
    let line = json!({
        "at_ms": now_ms(),
        "action": action,
        "ok": ok,
        "detail": truncate_chars(detail, 500)
    })
    .to_string();
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.maintenance_log_file)
    {
        let _ = writeln!(file, "{line}");
    }
}

pub(crate) fn recent_maintenance_events(path: &Path, limit: usize) -> Value {
    if limit == 0 {
        return Value::Array(Vec::new());
    }
    let Ok(text) = fs::read_to_string(path) else {
        return Value::Array(Vec::new());
    };
    let mut events = Vec::new();
    for line in text.lines().rev() {
        if events.len() >= limit {
            break;
        }
        let Ok(raw) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let action = raw.get("action").and_then(Value::as_str).unwrap_or("");
        if action.trim().is_empty() {
            continue;
        }
        events.push(json!({
            "at_ms": raw.get("at_ms").filter(|value| value.is_number()).cloned().unwrap_or(Value::Null),
            "action": truncate_chars(action, 80),
            "ok": raw.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "detail": truncate_chars(raw.get("detail").and_then(Value::as_str).unwrap_or(""), 180),
        }));
    }
    Value::Array(events)
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    trimmed.chars().take(max_chars).collect()
}

pub(crate) fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(windows)]
pub(crate) fn installed_paths() -> Result<InstalledPaths, String> {
    let install_dir =
        install_dir_from_local_app_data(std::env::var("LOCALAPPDATA").ok().as_deref())
            .ok_or_else(|| "无法读取 LOCALAPPDATA，不能确定安装目录。".to_string())?;
    Ok(InstalledPaths {
        client_exe: install_dir.join(crate::node_client_launcher::CLIENT_EXE_NAME),
        uninstall_exe: install_dir.join(crate::node_client_launcher::UNINSTALL_EXE_NAME),
        install_dir,
    })
}

#[cfg(any(windows, test))]
pub(crate) fn install_dir_from_local_app_data(local_app_data: Option<&str>) -> Option<PathBuf> {
    local_app_data
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| PathBuf::from(value).join("ElonNode"))
}

#[cfg(windows)]
pub(crate) fn apply_hidden_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
}

pub(crate) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub(crate) fn optional_path_to_value(path: Option<&Path>) -> Value {
    path.map(path_to_string)
        .map(Value::String)
        .unwrap_or(Value::Null)
}

pub(crate) fn error_response(status: StatusCode, error: String) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "ok": false, "error": error })))
}

pub(crate) fn launcher_logs_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        install_dir_from_local_app_data(std::env::var("LOCALAPPDATA").ok().as_deref())
            .map(|install_dir| install_dir.join("_internal").join("logs"))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub(crate) fn launcher_log_file() -> Option<PathBuf> {
    launcher_logs_dir().map(|dir| dir.join("client-launcher.jsonl"))
}

pub(crate) struct MaintenancePaths {
    pub(crate) state_file: PathBuf,
    pub(crate) config_dir: PathBuf,
    pub(crate) task_journal_dir: PathBuf,
    pub(crate) logs_dir: PathBuf,
    pub(crate) maintenance_log_file: PathBuf,
    pub(crate) launcher_logs_dir: Option<PathBuf>,
    pub(crate) launcher_log_file: Option<PathBuf>,
    pub(crate) diagnostics_dir: PathBuf,
}

pub(crate) struct MaintenanceTarget {
    pub(crate) path: PathBuf,
    pub(crate) select_file: bool,
    pub(crate) ensure_dir: bool,
    pub(crate) must_exist: bool,
}

#[cfg(windows)]
pub(crate) struct InstalledPaths {
    pub(crate) install_dir: PathBuf,
    pub(crate) client_exe: PathBuf,
    pub(crate) uninstall_exe: PathBuf,
}

pub(crate) enum ClientAction {
    Repair,
    Update,
    Uninstall,
}
