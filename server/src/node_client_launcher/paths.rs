use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

use super::{CLIENT_EXE_NAME, INTERNAL_DIR_NAME, UNINSTALL_EXE_NAME};

pub(crate) fn install_dir() -> Result<PathBuf> {
    let local_app_data =
        std::env::var("LOCALAPPDATA").context("无法读取 LOCALAPPDATA，不能确定安装目录")?;
    Ok(PathBuf::from(local_app_data).join("ElonNode"))
}

pub(crate) fn internal_dir(install_dir: &std::path::Path) -> PathBuf {
    install_dir.join(INTERNAL_DIR_NAME)
}

pub(crate) fn client_exe(install_dir: &std::path::Path) -> PathBuf {
    install_dir.join(CLIENT_EXE_NAME)
}

pub(crate) fn uninstall_exe(install_dir: &std::path::Path) -> PathBuf {
    install_dir.join(UNINSTALL_EXE_NAME)
}

pub(crate) fn version_file(install_dir: &std::path::Path) -> PathBuf {
    internal_dir(install_dir).join("node-agent-version.json")
}

pub(crate) fn env_file(install_dir: &std::path::Path) -> PathBuf {
    internal_dir(install_dir).join("node-agent.env")
}

pub(crate) fn current_exe_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("无法定位当前客户端 exe")?;
    exe.parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| anyhow!("当前客户端 exe 没有父目录"))
}

pub(crate) fn packaged_internal_dir() -> Result<PathBuf> {
    Ok(current_exe_dir()?.join(INTERNAL_DIR_NAME))
}

pub(crate) fn is_running_from_install_dir(install_dir: &std::path::Path) -> bool {
    let Ok(current) = std::env::current_exe() else {
        return false;
    };
    current.starts_with(install_dir)
}
