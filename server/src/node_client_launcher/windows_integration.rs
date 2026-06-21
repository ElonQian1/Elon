// server/src/node_client_launcher/windows_integration.rs

#[cfg(windows)]
use anyhow::Context;
use anyhow::Result;
use std::path::Path;

#[cfg(windows)]
use super::{command as launcher_command, paths, APP_NAME};

#[cfg(windows)]
const RUN_VALUE_NAME: &str = "ElonNodeAgent";
#[cfg(windows)]
const LEGACY_TASK_NAME: &str = "ElonNodeAgentTray";
#[cfg(windows)]
const PROTOCOL_SCHEME: &str = "elon-node";
#[cfg(windows)]
const LEGACY_RUN_VALUE_NAMES: &[&str] = &[
    "ElonNodeAgentTray",
    "ElonNodeClient",
    "一龙PC节点",
    "elon-node-agent",
];

pub(crate) fn enable_autostart(install_dir: &Path) -> Result<()> {
    #[cfg(not(windows))]
    let _ = install_dir;
    #[cfg(windows)]
    {
        let client = paths::client_exe(install_dir);
        let value = format!("\"{}\"", client.display());
        remove_legacy_scheduled_task();
        remove_legacy_run_values();
        remove_legacy_startup_shortcuts();
        let mut cmd = launcher_command::silent_command("reg");
        cmd.args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            RUN_VALUE_NAME,
            "/t",
            "REG_SZ",
            "/d",
            &value,
            "/f",
        ]);
        let status = launcher_command::status_hidden(&mut cmd).context("无法注册开机自启")?;
        if !status.success() {
            anyhow::bail!("注册开机自启失败");
        }
    }
    Ok(())
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
        let mut cmd = launcher_command::silent_command("reg");
        cmd.args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            RUN_VALUE_NAME,
            "/f",
        ]);
        let _ = launcher_command::status_hidden(&mut cmd);
        remove_legacy_run_values();
        remove_legacy_scheduled_task();
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
fn remove_legacy_scheduled_task() {
    let mut cmd = launcher_command::silent_command("schtasks");
    cmd.args(["/Delete", "/TN", LEGACY_TASK_NAME, "/F"]);
    let _ = launcher_command::status_hidden(&mut cmd);
}

#[cfg(windows)]
fn remove_legacy_run_values() {
    for value_name in LEGACY_RUN_VALUE_NAMES {
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
}

#[cfg(windows)]
fn remove_legacy_startup_shortcuts() {
    let script = r#"
$startup = [Environment]::GetFolderPath('Startup')
if (Test-Path -LiteralPath $startup) {
  '一龙PC节点.lnk','ElonNodeAgentTray.lnk','elon-node-agent.lnk' | ForEach-Object {
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
