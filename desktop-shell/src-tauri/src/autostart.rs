// desktop-shell/src-tauri/src/autostart.rs
//
// 开机自启动：直接写"当前用户启动文件夹"快捷方式，不用 tauri-plugin-autostart
// 默认的注册表 Run key 机制。
//
// 原因：项目在 elon-pc-node 上已经踩过注册表 Run key 触发 Windows Defender
// `Behavior:Win32/Persistence.A!ml` 云查杀的坑（见 docs/windows-client-defender.md），
// 修复后的安全做法是"当前用户 Startup 文件夹 .lnk 快捷方式 / 计划任务 + 用户
// 明确 opt-in"。新客户端不发明新的持久化方式，复用同一个已验证安全的机制：
// Startup 文件夹快捷方式，且只有用户在托盘菜单里主动勾选才会创建。

use anyhow::{Context, Result};
use std::path::PathBuf;

#[cfg(windows)]
const SHORTCUT_NAME: &str = "一龙工作台.lnk";

/// 当前是否已开启开机自启动（通过检测 Startup 文件夹快捷方式是否存在）。
#[cfg(windows)]
pub(crate) fn is_enabled() -> bool {
    shortcut_path().map(|path| path.exists()).unwrap_or(false)
}

#[cfg(not(windows))]
pub(crate) fn is_enabled() -> bool {
    false
}

/// 开启开机自启动：在当前用户 Startup 文件夹里创建指向本 exe 的快捷方式。
#[cfg(windows)]
pub(crate) fn enable() -> Result<()> {
    let exe = std::env::current_exe().context("无法定位当前 exe 路径")?;
    let shortcut = shortcut_path().context("无法定位当前用户 Startup 文件夹")?;
    if let Some(parent) = shortcut.parent() {
        std::fs::create_dir_all(parent).context("无法创建 Startup 文件夹")?;
    }
    let script = format!(
        r#"
$shortcut = '{shortcut}'
$target = '{target}'
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($shortcut)
$link.TargetPath = $target
$link.WorkingDirectory = Split-Path -Parent $target
$link.IconLocation = "$target,0"
$link.Description = '开机自动启动一龙工作台'
$link.Save()
"#,
        shortcut = ps_single_quote(&shortcut.to_string_lossy()),
        target = ps_single_quote(&exe.to_string_lossy()),
    );
    run_hidden_powershell(&script).context("创建开机自启动快捷方式失败")
}

#[cfg(not(windows))]
pub(crate) fn enable() -> Result<()> {
    anyhow::bail!("开机自启动目前只支持 Windows")
}

/// 关闭开机自启动：删除 Startup 文件夹里的快捷方式。
#[cfg(windows)]
pub(crate) fn disable() -> Result<()> {
    if let Some(path) = shortcut_path() {
        if path.exists() {
            std::fs::remove_file(&path).context("无法删除开机自启动快捷方式")?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn disable() -> Result<()> {
    Ok(())
}

#[cfg(windows)]
fn shortcut_path() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(appdata)
            .join(r"Microsoft\Windows\Start Menu\Programs\Startup")
            .join(SHORTCUT_NAME),
    )
}

#[cfg(windows)]
fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn run_hidden_powershell(script: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let status = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .context("无法启动 PowerShell")?;
    if !status.success() {
        anyhow::bail!("PowerShell 脚本执行失败，退出码 {:?}", status.code());
    }
    Ok(())
}
