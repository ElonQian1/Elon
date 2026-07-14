// server/src/node_client_launcher/installer.rs

use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use super::{
    command as launcher_command, env_file, log_file, paths, process, watchdog, windows_integration,
    DEFAULT_ADMIN_PORT,
};

const INTERNAL_FILES: &[&str] = &[
    "node-agent-version.json",
    "node-agent.env.example",
    "README.txt",
    // 一龙桌面壳（desktop-shell/ 独立 crate 构建产物），随主客户端一起分发。
    // 找不到时 node_agent_admin_open 会自动回退到系统浏览器，不影响旧客户端。
    "elon-desktop.exe",
];

const INTERNAL_DIRS: &[&str] = &["pc-next-dist"];

const LEGACY_TOP_LEVEL_FILES: &[&str] = &[
    "安装一龙PC节点.cmd",
    "启动一龙节点.cmd",
    "卸载一龙PC节点.cmd",
    "install-elon-node.ps1",
    "start-node-agent.ps1",
    "tray-launcher.ps1",
    "uninstall-elon-node.ps1",
    "elon-node-agent.exe",
    "elon-node-client.exe",
    "node-agent-version.json",
    "node-agent.env.example",
    "README.txt",
    // 旧名称 exe，重命名后自动清除
    "一龙PC节点.exe",
    "卸载一龙PC节点.exe",
];

const LEGACY_INTERNAL_FILES: &[&str] = &[
    "elon-node-agent.exe",
    "elon-node-agent.exe.new",
    "elon-node-client.exe",
    "elon-node-client.exe.new",
    "elon-pc-node.exe",
    "elon-pc-node.exe.new",
    "一龙PC节点.exe.new",
    "一龙开发平台.exe.new", // 名称更新后的临时文件
    "elon-node-agent-windows.zip.new",
    "node-agent-version.json.new",
];

pub(crate) fn ensure_installed() -> Result<PathBuf> {
    let install_dir = paths::install_dir()?;
    if install_layout_ready(&install_dir) && paths::is_running_from_install_dir(&install_dir) {
        cleanup_legacy_files(&install_dir)?;
        if let Err(error) = windows_integration::create_start_menu_shortcuts(&install_dir) {
            eprintln!("警告：创建开始菜单入口失败：{error:#}");
        }
        if let Err(error) = windows_integration::repair_existing_autostart(&install_dir) {
            eprintln!("警告：修复已有开机自启失败：{error:#}");
        }
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
    watchdog::stop_running(&install_dir);
    process::stop_agent();
    process::stop_installed_client_processes(&install_dir);
    process::wait_for_port_closed(previous_port, Duration::from_secs(5));

    let current_exe = std::env::current_exe().context("无法定位当前客户端 exe")?;
    copy_if_needed(&current_exe, &paths::client_exe(&install_dir))?;
    copy_if_needed(&current_exe, &paths::uninstall_exe(&install_dir))?;

    if let Some(source_internal) = resolve_source_internal_dir(&install_dir)? {
        copy_internal_files(&source_internal, &internal_dir)?;
    }
    preserve_user_env(&install_dir, &internal_dir)?;
    cleanup_legacy_files(&install_dir)?;

    if let Err(error) = windows_integration::create_desktop_shortcut(&install_dir) {
        eprintln!("警告：创建桌面快捷方式失败：{error:#}");
    }
    if let Err(error) = windows_integration::create_start_menu_shortcuts(&install_dir) {
        eprintln!("警告：创建开始菜单入口失败：{error:#}");
    }
    if let Err(error) = windows_integration::repair_existing_autostart(&install_dir) {
        eprintln!("警告：修复已有开机自启失败：{error:#}");
    }
    log_file::record_event(
        &install_dir,
        "autostart_default_opt_in",
        true,
        "install_or_repair does not create startup persistence; user opt-in is required",
    );
    if let Err(error) = windows_integration::register_url_protocol(&install_dir) {
        eprintln!("警告：注册网页一键唤起入口失败：{error:#}");
    }
    Ok(install_dir)
}

pub(crate) fn uninstall() -> Result<()> {
    let install_dir = paths::install_dir()?;
    watchdog::stop_running(&install_dir);
    process::stop_agent();
    windows_integration::disable_autostart();
    windows_integration::remove_url_protocol();
    windows_integration::remove_desktop_shortcut();
    windows_integration::remove_start_menu_shortcuts();

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
    paths::client_exe(install_dir).exists() && paths::uninstall_exe(install_dir).exists()
}

fn resolve_source_internal_dir(install_dir: &Path) -> Result<Option<PathBuf>> {
    let packaged = paths::packaged_internal_dir()?;
    if packaged.exists() {
        return Ok(Some(packaged));
    }
    let installed = paths::internal_dir(install_dir);
    if installed.exists() {
        return Ok(Some(installed));
    }
    Ok(None)
}

fn copy_internal_files(source: &Path, internal_dir: &Path) -> Result<()> {
    for file in INTERNAL_FILES {
        let src = source.join(file);
        if !src.exists() {
            continue;
        }
        copy_if_needed(&src, &internal_dir.join(file))?;
    }
    for dir in INTERNAL_DIRS {
        let src = source.join(dir);
        if !src.exists() {
            continue;
        }
        copy_dir_fresh(&src, &internal_dir.join(dir))?;
    }
    Ok(())
}

fn copy_dir_fresh(source: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)
            .with_context(|| format!("无法清理旧内部目录 {}", dest.display()))?;
    }
    copy_dir_recursive(source, dest)
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)
        .with_context(|| format!("无法创建内部目录 {}", dest.display()))?;
    for entry in
        std::fs::read_dir(source).with_context(|| format!("无法读取目录 {}", source.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else if file_type.is_file() {
            copy_if_needed(&src_path, &dest_path)?;
        }
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

fn cleanup_legacy_files(install_dir: &Path) -> Result<()> {
    for file in LEGACY_TOP_LEVEL_FILES {
        let path = install_dir.join(file);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("无法删除旧文件 {}", path.display()))?;
        }
    }
    let internal_dir = paths::internal_dir(install_dir);
    for file in LEGACY_INTERNAL_FILES {
        let path = internal_dir.join(file);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("无法删除旧内部文件 {}", path.display()))?;
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
        // 自删除必须等当前进程退出，仍保留 cmd，但统一走隐藏启动 helper。
        let mut cmd = launcher_command::cmd_hidden_command(&command);
        launcher_command::spawn_hidden(&mut cmd).context("无法安排卸载清理")?;
    }
    #[cfg(not(windows))]
    {
        std::fs::remove_dir_all(install_dir)?;
    }
    Ok(())
}
