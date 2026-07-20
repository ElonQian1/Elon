use super::{cdp::CdpClient, cdp::CdpSocket, CaptureDiagnostic};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{process::Child, time::sleep};

use super::browser::ProcessCleanup;

pub(super) struct BrowserProcess {
    child: Child,
    profile_dir: PathBuf,
}

impl BrowserProcess {
    pub(super) async fn launch(executable: &Path) -> Result<(Self, CdpSocket), CaptureDiagnostic> {
        let profile_dir = std::env::temp_dir()
            .join("elon-pwa-runtime")
            .join(uuid::Uuid::new_v4().simple().to_string());
        fs::create_dir_all(&profile_dir).map_err(|_| launch_error())?;
        let mut command = tokio::process::Command::new(executable);
        command
            .args([
                "--headless=new",
                "--remote-debugging-port=0",
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-background-networking",
                "--disable-component-update",
                "--disable-sync",
                "--metrics-recording-only",
                "--password-store=basic",
                "about:blank",
            ])
            .arg(format!("--user-data-dir={}", profile_dir.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::node_agent_exec::hide_tokio_command_window(&mut command);
        let child = match command.spawn() {
            Ok(child) => child,
            Err(_) => {
                let _ = fs::remove_dir_all(&profile_dir);
                return Err(launch_error());
            }
        };
        let mut process = Self { child, profile_dir };
        let active_port = process.profile_dir.join("DevToolsActivePort");
        let started = Instant::now();
        let content = loop {
            if let Ok(content) = fs::read_to_string(&active_port) {
                break content;
            }
            if started.elapsed() > Duration::from_secs(10) {
                process.abort_launch().await;
                return Err(launch_error());
            }
            sleep(Duration::from_millis(50)).await;
        };
        let mut lines = content.lines();
        let Some(port) = lines.next().and_then(|value| value.parse::<u16>().ok()) else {
            process.abort_launch().await;
            return Err(launch_error());
        };
        let Some(path) = lines.next().filter(|value| value.starts_with('/')) else {
            process.abort_launch().await;
            return Err(launch_error());
        };
        let ws = format!("ws://127.0.0.1:{port}{path}");
        let connection =
            tokio::time::timeout(Duration::from_secs(5), tokio_tungstenite::connect_async(ws))
                .await;
        let socket = match connection {
            Ok(Ok((socket, _))) => socket,
            _ => {
                process.abort_launch().await;
                return Err(launch_error());
            }
        };
        Ok((process, socket))
    }

    pub(super) async fn shutdown(&mut self, cdp: &mut CdpClient) -> ProcessCleanup {
        let deadline = Instant::now() + Duration::from_secs(2);
        let _ = cdp
            .command("Browser.close", json!({}), None, deadline)
            .await;
        let _ = tokio::time::timeout(Duration::from_secs(1), cdp.socket.close(None)).await;
        let mut reaped = tokio::time::timeout(Duration::from_secs(5), self.child.wait())
            .await
            .is_ok();
        if !reaped {
            kill_process_tree(&mut self.child).await;
            reaped = tokio::time::timeout(Duration::from_secs(5), self.child.wait())
                .await
                .is_ok();
        }
        let mut removed = false;
        for _ in 0..10 {
            match fs::remove_dir_all(&self.profile_dir) {
                Ok(()) => {
                    removed = true;
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    removed = true;
                    break;
                }
                Err(_) => sleep(Duration::from_millis(100)).await,
            }
        }
        ProcessCleanup {
            browser_process_reaped: reaped,
            temporary_profile_removed: removed,
        }
    }

    async fn abort_launch(&mut self) {
        kill_process_tree(&mut self.child).await;
        let _ = tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await;
        for _ in 0..10 {
            match fs::remove_dir_all(&self.profile_dir) {
                Ok(()) => break,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(_) => sleep(Duration::from_millis(100)).await,
            }
        }
    }
}

async fn kill_process_tree(child: &mut Child) {
    #[cfg(windows)]
    if let Some(pid) = child.id() {
        let mut command = tokio::process::Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/T", "/F"]);
        command.stdout(Stdio::null()).stderr(Stdio::null());
        crate::node_agent_exec::hide_tokio_command_window(&mut command);
        let _ = command.status().await;
    }
    let _ = child.kill().await;
}

pub(super) fn locate_browser() -> Result<PathBuf, CaptureDiagnostic> {
    locate_browser_from(&browser_candidates()).ok_or_else(browser_not_found)
}

fn browser_candidates() -> Vec<PathBuf> {
    let mut values = Vec::new();
    if let Some(path) = std::env::var_os("ELON_PWA_BROWSER_PATH") {
        values.push(PathBuf::from(path));
    }
    #[cfg(windows)]
    for (variable, suffix) in [
        ("ProgramFiles(x86)", "Microsoft/Edge/Application/msedge.exe"),
        ("ProgramFiles", "Microsoft/Edge/Application/msedge.exe"),
        ("ProgramFiles", "Google/Chrome/Application/chrome.exe"),
        ("ProgramFiles(x86)", "Google/Chrome/Application/chrome.exe"),
        ("LOCALAPPDATA", "Google/Chrome/Application/chrome.exe"),
    ] {
        if let Some(root) = std::env::var_os(variable) {
            values.push(PathBuf::from(root).join(suffix));
        }
    }
    for name in [
        "msedge.exe",
        "chrome.exe",
        "chromium.exe",
        "chromium",
        "chromium-browser",
        "google-chrome",
    ] {
        values.push(PathBuf::from(name));
    }
    values
}

fn locate_browser_from(candidates: &[PathBuf]) -> Option<PathBuf> {
    for candidate in candidates {
        if candidate.is_absolute() && candidate.is_file() {
            return candidate.canonicalize().ok();
        }
        if candidate.components().count() == 1 {
            if let Some(path) = std::env::var_os("PATH") {
                for root in std::env::split_paths(&path) {
                    let joined = root.join(candidate);
                    if joined.is_file() {
                        return joined.canonicalize().ok();
                    }
                }
            }
        }
    }
    None
}

fn launch_error() -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "BROWSER_LAUNCH_FAILED",
        "本机无头浏览器启动或 CDP 握手失败",
        true,
        "确认 Edge/Chrome 可执行、未被策略禁用且临时目录可写",
    )
}

fn browser_not_found() -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "BROWSER_NOT_FOUND",
        "PC 节点未找到可用 Edge、Chrome 或 Chromium",
        false,
        "安装 Microsoft Edge/Google Chrome，或把绝对路径写入 ELON_PWA_BROWSER_PATH 后重启 Windows 节点",
    )
}

#[cfg(test)]
pub(super) fn missing_browser_diagnostic_for_test() -> CaptureDiagnostic {
    locate_browser_from(&[PathBuf::from("Z:/definitely-missing/elon-browser.exe")])
        .ok_or_else(browser_not_found)
        .unwrap_err()
}
