// server/src/node_client_launcher/windows_integration.rs

#[cfg(windows)]
use anyhow::Context;
use anyhow::Result;
use std::path::Path;

#[cfg(windows)]
use super::{command as launcher_command, paths, APP_NAME, WATCHDOG_ARG};

pub(crate) const RUN_VALUE_NAME: &str = "ElonNodeAgent";
pub(crate) const TASK_NAME: &str = "ElonNodeAgent";
pub(crate) const LEGACY_TASK_NAMES: &[&str] = &["ElonNodeAgentTray"];
const PROTOCOL_SCHEME: &str = "elon-node";
pub(crate) const LEGACY_RUN_VALUE_NAMES: &[&str] = &[
    "ElonNodeAgentTray",
    "ElonNodeClient",
    "一龙PC节点",
    "elon-node-agent",
];

#[derive(Clone, Copy)]
#[cfg(any(windows, test))]
enum ShortcutTarget {
    Client,
    Uninstall,
}

#[derive(Clone, Copy)]
#[cfg(any(windows, test))]
struct ShortcutSpec {
    file_name: &'static str,
    target: ShortcutTarget,
    arguments: &'static str,
    description: &'static str,
}

#[cfg(any(windows, test))]
const START_MENU_SHORTCUTS: &[ShortcutSpec] = &[
    ShortcutSpec {
        file_name: "一龙开发平台.lnk",
        target: ShortcutTarget::Client,
        arguments: "",
        description: "打开一龙开发平台",
    },
    ShortcutSpec {
        file_name: "打开运行日志.lnk",
        target: ShortcutTarget::Client,
        arguments: "--open-logs",
        description: "打开一龙开发平台运行日志",
    },
    ShortcutSpec {
        file_name: "导出诊断.lnk",
        target: ShortcutTarget::Client,
        arguments: "--export-diagnostics",
        description: "导出一龙开发平台脱敏诊断文件",
    },
    ShortcutSpec {
        file_name: "检查更新.lnk",
        target: ShortcutTarget::Client,
        arguments: "--check-update",
        description: "检查并安装一龙开发平台更新",
    },
    ShortcutSpec {
        file_name: "修复客户端.lnk",
        target: ShortcutTarget::Client,
        arguments: "--repair",
        description: "修复一龙开发平台主程序、卸载程序、开始菜单和网页唤起入口",
    },
    ShortcutSpec {
        file_name: "卸载一龙开发平台.lnk",
        target: ShortcutTarget::Uninstall,
        arguments: "",
        description: "卸载一龙开发平台客户端",
    },
];

pub(crate) fn enable_autostart(install_dir: &Path) -> Result<()> {
    #[cfg(not(windows))]
    let _ = install_dir;
    #[cfg(windows)]
    {
        let client = paths::client_exe(install_dir);
        create_autostart_task(&client)?;
        remove_known_run_values();
        remove_legacy_startup_shortcuts();
        remove_legacy_scheduled_tasks();
    }
    Ok(())
}

pub(crate) fn repair_existing_autostart(install_dir: &Path) -> Result<()> {
    #[cfg(not(windows))]
    let _ = install_dir;
    #[cfg(windows)]
    {
        if autostart_marker_present()? {
            enable_autostart(install_dir)?;
        }
    }
    Ok(())
}

pub(crate) fn create_start_menu_shortcuts(install_dir: &Path) -> Result<()> {
    #[cfg(not(windows))]
    let _ = install_dir;
    #[cfg(windows)]
    {
        let client = paths::client_exe(install_dir);
        let uninstall = paths::uninstall_exe(install_dir);
        let workdir = install_dir.to_path_buf();
        let specs = start_menu_shortcut_ps_items();
        let script = format!(
            r#"
$programs = [Environment]::GetFolderPath('Programs')
if ([string]::IsNullOrWhiteSpace($programs)) {{
  $programs = Join-Path ([Environment]::GetFolderPath('StartMenu')) 'Programs'
}}
$folder = Join-Path $programs '{}'
New-Item -ItemType Directory -Force -Path $folder | Out-Null
$client = '{}'
$uninstall = '{}'
$workdir = '{}'
$shell = New-Object -ComObject WScript.Shell
$items = @(
{}
)
foreach ($item in $items) {{
  $shortcut = Join-Path $folder $item.Name
  $link = $shell.CreateShortcut($shortcut)
  $link.TargetPath = if ($item.Target -eq 'uninstall') {{ $uninstall }} else {{ $client }}
  $link.Arguments = $item.Arguments
  $link.WorkingDirectory = $workdir
  $link.IconLocation = $link.TargetPath
  $link.Description = $item.Description
  $link.Save()
}}
"#,
            launcher_command::ps_single_quote(APP_NAME),
            launcher_command::ps_single_quote(&client.to_string_lossy()),
            launcher_command::ps_single_quote(&uninstall.to_string_lossy()),
            launcher_command::ps_single_quote(&workdir.to_string_lossy()),
            specs
        );
        let mut cmd = launcher_command::powershell_hidden_command(&script);
        let status = launcher_command::status_hidden(&mut cmd).context("无法创建开始菜单入口")?;
        if !status.success() {
            anyhow::bail!("创建开始菜单入口失败");
        }
    }
    Ok(())
}

pub(crate) fn remove_start_menu_shortcuts() {
    #[cfg(windows)]
    {
        let script = format!(
            r#"
$programs = [Environment]::GetFolderPath('Programs')
if ([string]::IsNullOrWhiteSpace($programs)) {{
  $programs = Join-Path ([Environment]::GetFolderPath('StartMenu')) 'Programs'
}}
$folder = Join-Path $programs '{}'
Remove-Item -LiteralPath $folder -Recurse -Force -ErrorAction SilentlyContinue
"#,
            launcher_command::ps_single_quote(APP_NAME)
        );
        let mut cmd = launcher_command::powershell_hidden_command(&script);
        let _ = launcher_command::status_hidden(&mut cmd);
    }
}

pub(crate) fn register_url_protocol(install_dir: &Path) -> Result<()> {
    #[cfg(not(windows))]
    let _ = install_dir;
    #[cfg(windows)]
    {
        let client = paths::client_exe(install_dir);
        let key = format!(r"HKCU\Software\Classes\{}", PROTOCOL_SCHEME);
        let icon_key = format!(r"{}\DefaultIcon", key);
        let command_key = format!(r"{}\shell\open\command", key);
        let display_name = format!("URL:{}", APP_NAME);
        let icon_value = format!("\"{}\",0", client.display());
        let command_value = format!("\"{}\" \"%1\"", client.display());

        reg_add(
            &["add", &key, "/ve", "/d", &display_name, "/f"],
            "无法注册网页唤起入口",
        )?;
        reg_add(
            &[
                "add",
                &key,
                "/v",
                "URL Protocol",
                "/t",
                "REG_SZ",
                "/d",
                "",
                "/f",
            ],
            "无法注册网页唤起协议标记",
        )?;
        reg_add(
            &["add", &icon_key, "/ve", "/d", &icon_value, "/f"],
            "无法注册网页唤起图标",
        )?;
        reg_add(
            &["add", &command_key, "/ve", "/d", &command_value, "/f"],
            "无法注册网页唤起命令",
        )?;
    }
    Ok(())
}

pub(crate) fn remove_url_protocol() {
    #[cfg(windows)]
    {
        let key = format!(r"HKCU\Software\Classes\{}", PROTOCOL_SCHEME);
        let mut cmd = launcher_command::silent_command("reg");
        cmd.args(["delete", &key, "/f"]);
        let _ = launcher_command::status_hidden(&mut cmd);
    }
}

pub(crate) fn disable_autostart() {
    #[cfg(windows)]
    {
        remove_scheduled_task(TASK_NAME);
        remove_legacy_scheduled_tasks();
        remove_known_run_values();
        remove_legacy_startup_shortcuts();
    }
}

pub(crate) fn create_desktop_shortcut(install_dir: &Path) -> Result<()> {
    #[cfg(not(windows))]
    let _ = install_dir;
    #[cfg(windows)]
    {
        let target = paths::client_exe(install_dir);
        let workdir = install_dir.to_path_buf();
        let script = format!(
            r#"
$desktop = [Environment]::GetFolderPath('Desktop')
if (-not (Test-Path -LiteralPath $desktop)) {{
  New-Item -ItemType Directory -Force -Path $desktop | Out-Null
}}
$shortcut = Join-Path $desktop '{}.lnk'
$target = '{}'
$workdir = '{}'
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($shortcut)
$link.TargetPath = $target
$link.WorkingDirectory = $workdir
$link.IconLocation = $target
$link.Save()
"#,
            launcher_command::ps_single_quote(APP_NAME),
            launcher_command::ps_single_quote(&target.to_string_lossy()),
            launcher_command::ps_single_quote(&workdir.to_string_lossy())
        );
        let mut cmd = launcher_command::powershell_hidden_command(&script);
        let status = launcher_command::status_hidden(&mut cmd).context("无法创建桌面快捷方式")?;
        if !status.success() {
            anyhow::bail!("创建桌面快捷方式失败");
        }
    }
    Ok(())
}

pub(crate) fn remove_desktop_shortcut() {
    #[cfg(windows)]
    {
        let script = format!(
            r#"
$desktop = [Environment]::GetFolderPath('Desktop')
$shortcut = Join-Path $desktop '{}.lnk'
Remove-Item -LiteralPath $shortcut -Force -ErrorAction SilentlyContinue
"#,
            launcher_command::ps_single_quote(APP_NAME)
        );
        let mut cmd = launcher_command::powershell_hidden_command(&script);
        let _ = launcher_command::status_hidden(&mut cmd);
    }
}

#[cfg(windows)]
fn start_menu_shortcut_ps_items() -> String {
    START_MENU_SHORTCUTS
        .iter()
        .map(|spec| {
            format!(
                "  @{{ Name='{}'; Target='{}'; Arguments='{}'; Description='{}' }}",
                launcher_command::ps_single_quote(spec.file_name),
                match spec.target {
                    ShortcutTarget::Client => "client",
                    ShortcutTarget::Uninstall => "uninstall",
                },
                launcher_command::ps_single_quote(spec.arguments),
                launcher_command::ps_single_quote(spec.description)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

#[cfg(windows)]
fn create_autostart_task(client: &Path) -> Result<()> {
    let mut cmd = launcher_command::silent_command("schtasks");
    let task_run = autostart_task_run_command(client);
    cmd.args([
        "/Create", "/TN", TASK_NAME, "/SC", "ONLOGON", "/TR", &task_run, "/RL", "LIMITED", "/F",
    ]);
    let status =
        launcher_command::status_hidden(&mut cmd).context("无法注册当前用户开机计划任务")?;
    if !status.success() {
        anyhow::bail!("注册当前用户开机计划任务失败");
    }
    Ok(())
}

#[cfg(any(windows, test))]
fn autostart_task_run_command(client: &Path) -> String {
    format!("\"{}\" {}", client.display(), WATCHDOG_ARG)
}

#[cfg(windows)]
fn autostart_marker_present() -> Result<bool> {
    let script = autostart_marker_probe_script();
    let mut cmd = launcher_command::powershell_hidden_command(&script);
    let output = launcher_command::output_hidden(&mut cmd).context("无法读取已有开机自启设置")?;
    if !output.status.success() {
        anyhow::bail!("读取已有开机自启设置失败");
    }
    Ok(String::from_utf8_lossy(&output.stdout).contains("present"))
}

#[cfg(any(windows, test))]
fn autostart_marker_probe_script() -> String {
    format!(
        r#"
$ErrorActionPreference = 'Stop'
if (Get-Command Get-ScheduledTask -ErrorAction SilentlyContinue) {{
  foreach ($taskName in @({task_names})) {{
    if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {{
      [Console]::Out.Write('present')
      exit 0
    }}
  }}
}}
$keyPath = 'Software\Microsoft\Windows\CurrentVersion\Run'
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($keyPath)
if ($null -ne $key) {{
  try {{
    foreach ($name in @({run_value_names})) {{
      if ($null -ne $key.GetValue($name, $null)) {{
        [Console]::Out.Write('present')
        exit 0
      }}
    }}
  }} finally {{
    $key.Dispose()
  }}
}}
$startup = [Environment]::GetFolderPath('Startup')
if (Test-Path -LiteralPath $startup) {{
  foreach ($shortcut in @('一龙开发平台.lnk','一龙PC节点.lnk','ElonNodeAgentTray.lnk','elon-node-agent.lnk')) {{
    if (Test-Path -LiteralPath (Join-Path $startup $shortcut)) {{
      [Console]::Out.Write('present')
      exit 0
    }}
  }}
}}
"#,
        task_names = ps_string_array(&[&[TASK_NAME], LEGACY_TASK_NAMES].concat()),
        run_value_names = ps_string_array(&[&[RUN_VALUE_NAME], LEGACY_RUN_VALUE_NAMES].concat()),
    )
}

#[cfg(any(windows, test))]
fn ps_string_array(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("'{}'", ps_single_quote_for_script(value)))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(any(windows, test))]
fn ps_single_quote_for_script(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn remove_legacy_scheduled_tasks() {
    for name in LEGACY_TASK_NAMES {
        remove_scheduled_task(name);
    }
}

#[cfg(windows)]
fn remove_scheduled_task(name: &str) {
    let mut cmd = launcher_command::silent_command("schtasks");
    cmd.args(["/Delete", "/TN", name, "/F"]);
    let _ = launcher_command::status_hidden(&mut cmd);
}

#[cfg(test)]
mod tests {
    use super::{ShortcutTarget, START_MENU_SHORTCUTS};

    #[test]
    fn start_menu_shortcuts_cover_user_maintenance_flow() {
        let names = START_MENU_SHORTCUTS
            .iter()
            .map(|spec| spec.file_name)
            .collect::<Vec<_>>();

        assert!(names.contains(&"一龙开发平台.lnk"));
        assert!(names.contains(&"打开运行日志.lnk"));
        assert!(names.contains(&"导出诊断.lnk"));
        assert!(names.contains(&"一龙开发平台.lnk"));
        assert!(names.contains(&"修复客户端.lnk"));
        assert!(names.contains(&"卸载一龙开发平台.lnk"));
        assert!(START_MENU_SHORTCUTS
            .iter()
            .all(|spec| spec.file_name.ends_with(".lnk")));
        assert!(START_MENU_SHORTCUTS
            .iter()
            .any(|spec| spec.arguments == "--export-diagnostics"));
        assert!(START_MENU_SHORTCUTS
            .iter()
            .any(|spec| spec.arguments == "--repair"));
        assert!(START_MENU_SHORTCUTS
            .iter()
            .any(|spec| matches!(spec.target, ShortcutTarget::Uninstall)));
    }

    #[test]
    fn autostart_task_targets_installed_client_watchdog() {
        let command = super::autostart_task_run_command(std::path::Path::new(
            r"C:\Users\ELon\AppData\Local\ElonNode\一龙开发平台.exe",
        ));

        assert_eq!(
            command,
            r#""C:\Users\ELon\AppData\Local\ElonNode\一龙开发平台.exe" --watchdog"#,
        );
    }

    #[test]
    fn autostart_repair_probe_checks_tasks_and_legacy_run_values() {
        let script = super::autostart_marker_probe_script();

        assert!(script.contains("Get-ScheduledTask"));
        assert!(script.contains("ElonNodeAgent"));
        assert!(script.contains("CurrentVersion\\Run"));
        assert!(!script.contains("CreateSubKey"));
    }
}

#[cfg(windows)]
fn remove_known_run_values() {
    remove_run_value(RUN_VALUE_NAME);
    for value_name in LEGACY_RUN_VALUE_NAMES {
        remove_run_value(value_name);
    }
}

#[cfg(windows)]
fn remove_run_value(value_name: &str) {
    let mut cmd = launcher_command::silent_command("reg");
    cmd.args([
        "delete",
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
        "/v",
        value_name,
        "/f",
    ]);
    let _ = launcher_command::status_hidden(&mut cmd);
}

#[cfg(windows)]
fn remove_legacy_startup_shortcuts() {
    let script = r#"
$startup = [Environment]::GetFolderPath('Startup')
if (Test-Path -LiteralPath $startup) {
  '一龙开发平台.lnk','一龙开发平台.lnk','一龙PC节点.lnk','ElonNodeAgentTray.lnk','elon-node-agent.lnk' | ForEach-Object {
    Remove-Item -LiteralPath (Join-Path $startup $_) -Force -ErrorAction SilentlyContinue
  }
}
"#;
    let mut cmd = launcher_command::powershell_hidden_command(script);
    let _ = launcher_command::status_hidden(&mut cmd);
}

#[cfg(windows)]
fn reg_add(args: &[&str], message: &str) -> Result<()> {
    let mut cmd = launcher_command::silent_command("reg");
    cmd.args(args);
    let status = launcher_command::status_hidden(&mut cmd).with_context(|| message.to_string())?;
    if !status.success() {
        anyhow::bail!("{message}");
    }
    Ok(())
}
