use anyhow::{Context, Result};
use std::{path::Path, process::Command};

use super::{paths, APP_NAME};

const RUN_VALUE_NAME: &str = "ElonNodeAgent";
const LEGACY_TASK_NAME: &str = "ElonNodeAgentTray";

pub(crate) fn enable_autostart(install_dir: &Path) -> Result<()> {
    let client = paths::client_exe(install_dir);
    let value = format!("\"{}\"", client.display());
    #[cfg(windows)]
    {
        remove_legacy_scheduled_task();
        let status = Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                RUN_VALUE_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .status()
            .context("无法注册开机自启")?;
        if !status.success() {
            anyhow::bail!("注册开机自启失败");
        }
    }
    Ok(())
}

pub(crate) fn disable_autostart() {
    #[cfg(windows)]
    {
        let _ = Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                RUN_VALUE_NAME,
                "/f",
            ])
            .status();
        remove_legacy_scheduled_task();
    }
}

pub(crate) fn create_desktop_shortcut(install_dir: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let shortcut = desktop_path().join(format!("{APP_NAME}.lnk"));
        let target = paths::client_exe(install_dir);
        let workdir = install_dir.to_path_buf();
        if let Some(parent) = shortcut.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建桌面目录 {}", parent.display()))?;
        }
        let script = format!(
            r#"
$shortcut = '{}'
$target = '{}'
$workdir = '{}'
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($shortcut)
$link.TargetPath = $target
$link.WorkingDirectory = $workdir
$link.IconLocation = $target
$link.Save()
"#,
            ps_single_quote(&shortcut.to_string_lossy()),
            ps_single_quote(&target.to_string_lossy()),
            ps_single_quote(&workdir.to_string_lossy())
        );
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .status()
            .context("无法创建桌面快捷方式")?;
        if !status.success() {
            anyhow::bail!("创建桌面快捷方式失败");
        }
    }
    Ok(())
}

pub(crate) fn remove_desktop_shortcut() {
    #[cfg(windows)]
    {
        let shortcut = desktop_path().join(format!("{APP_NAME}.lnk"));
        let _ = std::fs::remove_file(shortcut);
    }
}

#[cfg(windows)]
fn desktop_path() -> std::path::PathBuf {
    if let Ok(output) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::GetFolderPath('Desktop')",
        ])
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return std::path::PathBuf::from(path);
            }
        }
    }

    std::env::var("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Desktop")
}

#[cfg(windows)]
fn remove_legacy_scheduled_task() {
    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", LEGACY_TASK_NAME, "/F"])
        .status();
}

#[cfg(windows)]
fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}
