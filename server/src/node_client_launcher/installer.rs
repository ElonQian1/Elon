use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use super::{
    env_file, paths, process, windows_integration, AGENT_EXE_NAME, DEFAULT_ADMIN_PORT,
    INTERNAL_DIR_NAME,
};

const INTERNAL_FILES: &[&str] = &[
    AGENT_EXE_NAME,
    "node-agent-version.json",
    "node-agent.env.example",
    "README.txt",
];

const LEGACY_TOP_LEVEL_FILES: &[&str] = &[
    "安装一龙PC节点.cmd",
    "启动一龙节点.cmd",
    "卸载一龙PC节点.cmd",
    "install-elon-node.ps1",
    "start-node-agent.ps1",
    "tray-launcher.ps1",
    "uninstall-elon-node.ps1",
    AGENT_EXE_NAME,
    "node-agent-version.json",
    "node-agent.env.example",
    "README.txt",
];

pub(crate) fn ensure_installed() -> Result<PathBuf> {
    let install_dir = paths::install_dir()?;
    if install_layout_ready(&install_dir) && paths::is_running_from_install_dir(&install_dir) {
        if let Err(error) = windows_integration::register_url_protocol(&install_dir) {
            eprintln!("警告：注册网页一键唤起入口失败：{error:#}");
        }
        return Ok(install_dir);
    }
    install_or_repair()
}

pub(crate) fn install_or_repair() -> Result<PathBuf> {
    let install_dir = paths::install_dir()?;
    let internal_dir = paths::internal_dir(&install_dir);
    std::fs::create_dir_all(&internal_dir)
        .with_context(|| format!("无法创建安装目录 {}", internal_dir.display()))?;

    let previous_port = configured_admin_port(&install_dir);
    process::stop_agent();
    process::wait_for_port_closed(previous_port, Duration::from_secs(5));

    let current_exe = std::env::current_exe().context("无法定位当前客户端 exe")?;
    copy_if_needed(&current_exe, &paths::client_exe(&install_dir))?;
    copy_if_needed(&current_exe, &paths::uninstall_exe(&install_dir))?;

    let source_internal = resolve_source_internal_dir(&install_dir)?;
    copy_internal_files(&source_internal, &internal_dir)?;
    preserve_user_env(&install_dir, &internal_dir)?;
    cleanup_legacy_top_level(&install_dir)?;

    if let Err(error) = windows_integration::create_desktop_shortcut(&install_dir) {
        eprintln!("警告：创建桌面快捷方式失败：{error:#}");
    }
    if let Err(error) = windows_integration::enable_autostart(&install_dir) {
        eprintln!("警告：注册开机自启失败：{error:#}");
    }
    if let Err(error) = windows_integration::register_url_protocol(&install_dir) {
        eprintln!("警告：注册网页一键唤起入口失败：{error:#}");
    }
    Ok(install_dir)
}

pub(crate) fn uninstall() -> Result<()> {
    let install_dir = paths::install_dir()?;
    process::stop_agent();
    windows_integration::disable_autostart();
    windows_integration::remove_url_protocol();
    windows_integration::remove_desktop_shortcut();

    if install_dir.exists() {
        let current = std::env::current_exe().unwrap_or_default();
        if current.starts_with(&install_dir) {
            schedule_self_delete(&install_dir)?;
        } else {
            std::fs::remove_dir_all(&install_dir)
                .with_context(|| format!("无法删除安装目录 {}", install_dir.display()))?;
        }
    }
    Ok(())
}

fn install_layout_ready(install_dir: &Path) -> bool {
    paths::client_exe(install_dir).exists()
        && paths::uninstall_exe(install_dir).exists()
        && paths::agent_exe(install_dir).exists()
}

fn resolve_source_internal_dir(install_dir: &Path) -> Result<PathBuf> {
    let packaged = paths::packaged_internal_dir()?;
    if packaged.join(AGENT_EXE_NAME).exists() {
        return Ok(packaged);
    }
    let installed = paths::internal_dir(install_dir);
    if installed.join(AGENT_EXE_NAME).exists() {
        return Ok(installed);
    }
    if install_dir.join(AGENT_EXE_NAME).exists() {
        return Ok(install_dir.to_path_buf());
    }
    anyhow::bail!(
        "客户端包不完整：缺少 {} 或 {}\\{}",
        AGENT_EXE_NAME,
        INTERNAL_DIR_NAME,
        AGENT_EXE_NAME
    )
}

fn copy_internal_files(source: &Path, internal_dir: &Path) -> Result<()> {
    for file in INTERNAL_FILES {
        let src = source.join(file);
        if !src.exists() {
            if *file == AGENT_EXE_NAME {
                anyhow::bail!("缺少内部节点程序 {}", src.display());
            }
            continue;
        }
        copy_if_needed(&src, &internal_dir.join(file))?;
    }
    Ok(())
}

fn preserve_user_env(install_dir: &Path, internal_dir: &Path) -> Result<()> {
    let old_top_level = install_dir.join("node-agent.env");
    let new_env = internal_dir.join("node-agent.env");
    if old_top_level.exists() && !new_env.exists() {
        copy_if_needed(&old_top_level, &new_env)?;
    }
    Ok(())
}

fn cleanup_legacy_top_level(install_dir: &Path) -> Result<()> {
    for file in LEGACY_TOP_LEVEL_FILES {
        let path = install_dir.join(file);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("无法删除旧文件 {}", path.display()))?;
        }
    }
    Ok(())
}

fn configured_admin_port(install_dir: &Path) -> u16 {
    env_file::read_env_file(&paths::env_file(install_dir))
        .ok()
        .and_then(|values| {
            values
                .get("NODE_ADMIN_PORT")
                .and_then(|value| value.parse::<u16>().ok())
        })
        .unwrap_or(DEFAULT_ADMIN_PORT)
}

fn copy_if_needed(source: &Path, dest: &Path) -> Result<()> {
    if same_path(source, dest) {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, dest)
        .with_context(|| format!("复制 {} -> {} 失败", source.display(), dest.display()))?;
    Ok(())
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn schedule_self_delete(install_dir: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let command = format!(
            "timeout /t 1 /nobreak >nul & rmdir /s /q \"{}\"",
            install_dir.display()
        );
        Command::new("cmd")
            .args(["/C", &command])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("无法安排卸载清理")?;
    }
    #[cfg(not(windows))]
    {
        std::fs::remove_dir_all(install_dir)?;
    }
    Ok(())
}
