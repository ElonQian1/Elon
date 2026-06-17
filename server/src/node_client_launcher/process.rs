use anyhow::{bail, Context, Result};
use std::{
    net::TcpStream,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use super::{env_file, paths, DEFAULT_ADMIN_PORT};

pub(crate) fn start_or_open(install_dir: &Path) -> Result<()> {
    let internal_dir = paths::internal_dir(install_dir);
    let agent = paths::agent_exe(install_dir);
    if !agent.exists() {
        bail!("缺少内部节点程序：{}", agent.display());
    }

    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    let port = env_values
        .get("NODE_ADMIN_PORT")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_ADMIN_PORT);

    if !is_port_open(port) {
        let mut cmd = Command::new(&agent);
        cmd.current_dir(&internal_dir)
            .envs(&env_values)
            .env("NODE_AUTO_OPEN_ADMIN", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        spawn_hidden(&mut cmd).with_context(|| format!("无法启动 {}", agent.display()))?;
        wait_for_port(port, Duration::from_secs(15));
    }

    open_admin_page(port)
}

pub(crate) fn stop_agent() {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/IM", "elon-node-agent.exe", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("pkill")
            .arg("elon-node-agent")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

pub(crate) fn open_admin_page(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{port}/");
    #[cfg(windows)]
    {
        Command::new("explorer")
            .arg(&url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("无法打开管理页 {url}"))?;
    }
    #[cfg(not(windows))]
    {
        Command::new("xdg-open")
            .arg(&url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("无法打开管理页 {url}"))?;
    }
    Ok(())
}

pub(crate) fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if is_port_open(port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

pub(crate) fn is_port_open(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

fn spawn_hidden(command: &mut Command) -> std::io::Result<std::process::Child> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn()
}
