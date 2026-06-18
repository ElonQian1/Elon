#[cfg(windows)]
use anyhow::Context;
use anyhow::Result;
use std::path::Path;
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
use super::{paths, APP_NAME};

#[cfg(windows)]
use std::process::{ExitStatus, Stdio};
#[cfg(windows)]
const RUN_VALUE_NAME: &str = "ElonNodeAgent";
#[cfg(windows)]
const LEGACY_TASK_NAME: &str = "ElonNodeAgentTray";
#[cfg(windows)]
const PROTOCOL_SCHEME: &str = "elon-node";

pub(crate) fn enable_autostart(install_dir: &Path) -> Result<()> {
    #[cfg(not(windows))]
    let _ = install_dir;
    #[cfg(windows)]
    {
        let client = paths::client_exe(install_dir);
        let value = format!("\"{}\"", client.display());
        remove_legacy_scheduled_task();
        let mut cmd = silent_command("reg");
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
        let status = hidden_status(&mut cmd).context("无法注册开机自启")?;
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
        let mut cmd = silent_command("reg");
        cmd.args(["delete", &key, "/f"]);
        let _ = hidden_status(&mut cmd);
    }
}

pub(crate) fn disable_autostart() {
    #[cfg(windows)]
    {
        let mut cmd = silent_command("reg");
        cmd.args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            RUN_VALUE_NAME,
            "/f",
        ]);
        let _ = hidden_status(&mut cmd);
        remove_legacy_scheduled_task();
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
            ps_single_quote(APP_NAME),
            ps_single_quote(&target.to_string_lossy()),
            ps_single_quote(&workdir.to_string_lossy())
        );
        let mut cmd = silent_command("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ]);
        let status = hidden_status(&mut cmd).context("无法创建桌面快捷方式")?;
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
            ps_single_quote(APP_NAME)
        );
        let mut cmd = silent_command("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ]);
        let _ = hidden_status(&mut cmd);
    }
}

#[cfg(windows)]
fn remove_legacy_scheduled_task() {
    let mut cmd = silent_command("schtasks");
    cmd.args(["/Delete", "/TN", LEGACY_TASK_NAME, "/F"]);
    let _ = hidden_status(&mut cmd);
}

#[cfg(windows)]
fn reg_add(args: &[&str], message: &str) -> Result<()> {
    let mut cmd = silent_command("reg");
    cmd.args(args);
    let status = hidden_status(&mut cmd).with_context(|| message.to_string())?;
    if !status.success() {
        anyhow::bail!("{message}");
    }
    Ok(())
}

#[cfg(windows)]
fn silent_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

#[cfg(windows)]
fn hidden_status(command: &mut Command) -> std::io::Result<ExitStatus> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW).status()
}

#[cfg(windows)]
fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}
