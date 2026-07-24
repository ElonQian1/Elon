use super::{cdp::CdpClient, cdp::CdpSocket, CaptureDiagnostic};
use serde::Serialize;
use serde_json::json;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{process::Child, time::sleep};

use super::browser::ProcessCleanup;

const PROFILE_CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
const PROFILE_CLEANUP_INITIAL_BACKOFF: Duration = Duration::from_millis(50);
const PROFILE_CLEANUP_MAX_BACKOFF: Duration = Duration::from_millis(500);
const STDERR_TAIL_BYTES: u64 = 8 * 1024;

pub(super) struct BrowserProcess {
    child: Child,
    profile_dir: PathBuf,
    stderr_path: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BrowserStderrDiagnostic {
    captured: bool,
    truncated: bool,
    tail: Vec<String>,
}

impl BrowserProcess {
    pub(super) async fn launch(executable: &Path) -> Result<(Self, CdpSocket), CaptureDiagnostic> {
        let profile_dir = std::env::temp_dir()
            .join("elon-pwa-runtime")
            .join(uuid::Uuid::new_v4().simple().to_string());
        fs::create_dir_all(&profile_dir).map_err(|_| launch_error())?;
        let stderr_path = profile_dir.join("browser.stderr.log");
        let stderr = match fs::File::create(&stderr_path) {
            Ok(stderr) => stderr,
            Err(_) => {
                let _ = fs::remove_dir_all(&profile_dir);
                return Err(launch_error());
            }
        };
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
            .stderr(Stdio::from(stderr));
        crate::node_agent_exec::hide_tokio_command_window(&mut command);
        let child = match command.spawn() {
            Ok(child) => child,
            Err(_) => {
                let _ = fs::remove_dir_all(&profile_dir);
                return Err(launch_error());
            }
        };
        let mut process = Self {
            child,
            profile_dir,
            stderr_path,
        };
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

    pub(super) async fn shutdown(
        &mut self,
        cdp: &mut CdpClient,
    ) -> (ProcessCleanup, BrowserStderrDiagnostic) {
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
        let stderr = read_stderr_diagnostic(&self.stderr_path, &self.profile_dir);
        let removed = remove_temporary_profile(&self.profile_dir).await;
        (
            ProcessCleanup {
                browser_process_reaped: reaped,
                temporary_profile_removed: removed,
            },
            stderr,
        )
    }

    async fn abort_launch(&mut self) {
        kill_process_tree(&mut self.child).await;
        let _ = tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await;
        let _ = remove_temporary_profile(&self.profile_dir).await;
    }
}

fn read_stderr_diagnostic(path: &Path, profile_dir: &Path) -> BrowserStderrDiagnostic {
    let Ok(mut file) = fs::File::open(path) else {
        return BrowserStderrDiagnostic {
            captured: false,
            truncated: false,
            tail: Vec::new(),
        };
    };
    let length = file.metadata().map(|value| value.len()).unwrap_or(0);
    let truncated = length > STDERR_TAIL_BYTES;
    if truncated
        && file
            .seek(SeekFrom::End(-(STDERR_TAIL_BYTES as i64)))
            .is_err()
    {
        return BrowserStderrDiagnostic {
            captured: false,
            truncated,
            tail: Vec::new(),
        };
    }
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return BrowserStderrDiagnostic {
            captured: false,
            truncated,
            tail: Vec::new(),
        };
    }
    let text = String::from_utf8_lossy(&bytes)
        .replace(&profile_dir.display().to_string(), "<temporary-profile>");
    BrowserStderrDiagnostic {
        captured: true,
        truncated,
        tail: text
            .lines()
            .map(redact_stderr_urls)
            .filter(|line| !line.trim().is_empty())
            .take(40)
            .collect(),
    }
}

fn redact_stderr_urls(line: &str) -> String {
    line.split_whitespace()
        .map(|token| {
            if token.contains("http://") || token.contains("https://") {
                "<url-redacted>"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

async fn remove_temporary_profile(profile_dir: &Path) -> bool {
    remove_temporary_profile_with(
        profile_dir,
        PROFILE_CLEANUP_TIMEOUT,
        PROFILE_CLEANUP_INITIAL_BACKOFF,
        PROFILE_CLEANUP_MAX_BACKOFF,
        |path| fs::remove_dir_all(path),
    )
    .await
}

async fn remove_temporary_profile_with<F>(
    profile_dir: &Path,
    timeout: Duration,
    initial_backoff: Duration,
    max_backoff: Duration,
    mut remove: F,
) -> bool
where
    F: FnMut(&Path) -> std::io::Result<()>,
{
    let started = Instant::now();
    let mut backoff = initial_backoff;
    loop {
        match remove(profile_dir) {
            Ok(()) => return true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
            Err(_) => {}
        }
        let Some(remaining) = timeout.checked_sub(started.elapsed()) else {
            return false;
        };
        if remaining.is_zero() {
            return false;
        }
        sleep(backoff.min(remaining)).await;
        backoff = backoff.saturating_mul(2).min(max_backoff);
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

#[cfg(test)]
pub(super) async fn remove_temporary_profile_for_test<F>(
    profile_dir: &Path,
    timeout: Duration,
    initial_backoff: Duration,
    max_backoff: Duration,
    remove: F,
) -> bool
where
    F: FnMut(&Path) -> std::io::Result<()>,
{
    remove_temporary_profile_with(profile_dir, timeout, initial_backoff, max_backoff, remove).await
}
