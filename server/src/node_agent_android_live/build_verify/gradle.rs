use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::Mutex;

use super::super::broker::LiveUiSession;

const DEFAULT_BUILD_HARD_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const DEFAULT_BUILD_IDLE_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const BUILD_POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_BUILD_OUTPUT: usize = 256 * 1024;
const KOTLIN_IN_PROCESS_ARGUMENT: &str = "-Pkotlin.compiler.execution.strategy=in-process";

pub(super) fn canonical_project_root(session: &LiveUiSession) -> Result<PathBuf> {
    let root = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow!("Live 会话未绑定 projectRoot"))?;
    PathBuf::from(root)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {root}"))
}

pub(super) fn find_gradle_root(project_root: &Path) -> Result<PathBuf> {
    for candidate in [project_root.to_path_buf(), project_root.join("android")] {
        if candidate.join("gradlew").is_file() || candidate.join("gradlew.bat").is_file() {
            return candidate
                .canonicalize()
                .with_context(|| format!("Gradle 目录不可访问: {}", candidate.display()));
        }
    }
    bail!("项目根目录及 android/ 下均未找到 Gradle Wrapper")
}

pub(super) fn gradle_wrapper(gradle_root: &Path) -> Result<PathBuf> {
    let candidates = if cfg!(windows) {
        [gradle_root.join("gradlew.bat"), gradle_root.join("gradlew")]
    } else {
        [gradle_root.join("gradlew"), gradle_root.join("gradlew.bat")]
    };
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| anyhow!("Gradle Wrapper 不存在"))
}

pub(super) async fn run_debug_build(
    gradle_root: &Path,
    wrapper: &Path,
    debug_application_id_suffix: Option<&str>,
    debug_app_label: Option<&str>,
    force_rerun: bool,
) -> Result<()> {
    let mut command =
        if cfg!(windows) && wrapper.extension().and_then(|v| v.to_str()) == Some("bat") {
            let mut command = Command::new("cmd.exe");
            // cmd.exe applies special quote stripping after /C. Passing an absolute
            // \\?\ path containing spaces can therefore be truncated before Gradle is
            // launched. The command already runs in gradle_root, so invoke only the
            // wrapper file name and avoid both long-path and quoting ambiguity.
            command
                .args(["/D", "/C"])
                .arg(wrapper.file_name().unwrap_or_default());
            command
        } else {
            Command::new(wrapper)
        };
    command
        .current_dir(gradle_root)
        .args(debug_build_arguments(
            gradle_root,
            debug_application_id_suffix,
            debug_app_label,
            force_rerun,
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    crate::node_agent_exec::hide_tokio_command_window(&mut command);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().context("无法启动 Gradle Wrapper")?;
    let process_id = child.id();
    let stdout = child.stdout.take().context("无法读取 Gradle stdout")?;
    let stderr = child.stderr.take().context("无法读取 Gradle stderr")?;
    let capture = Arc::new(Mutex::new(BuildOutputCapture::new()));
    let stdout_task = tokio::spawn(capture_stream(
        stdout,
        capture.clone(),
        BuildOutputStream::Stdout,
    ));
    let stderr_task = tokio::spawn(capture_stream(
        stderr,
        capture.clone(),
        BuildOutputStream::Stderr,
    ));
    let policy = BuildTimeoutPolicy::from_env();
    let started = Instant::now();
    let mut ticker = tokio::time::interval(BUILD_POLL_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let status = loop {
        tokio::select! {
            status = child.wait() => break status.context("等待 Gradle Wrapper 失败")?,
            _ = ticker.tick() => {
                let snapshot = capture.lock().await.snapshot();
                if snapshot.total_bytes > MAX_BUILD_OUTPUT * 4 {
                    terminate_gradle_process_tree(&mut child, process_id).await;
                    let _ = stdout_task.await;
                    let _ = stderr_task.await;
                    bail!("Gradle 输出异常过大，已终止完整构建进程树");
                }
                if let Some(reason) = policy.timeout_reason(
                    started.elapsed(),
                    snapshot.last_activity.elapsed(),
                ) {
                    terminate_gradle_process_tree(&mut child, process_id).await;
                    let _ = stdout_task.await;
                    let _ = stderr_task.await;
                    let output = capture.lock().await;
                    let message = tail_output(&output.stdout, &output.stderr);
                    bail!(
                        "Android Debug 构建超时: {reason}; elapsed={}s idle={}s; tail={message}",
                        started.elapsed().as_secs(),
                        snapshot.last_activity.elapsed().as_secs(),
                    );
                }
            }
        }
    };
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let output = capture.lock().await;
    if !status.success() {
        let message = tail_output(&output.stdout, &output.stderr);
        bail!("Android Debug 构建失败: {message}");
    }
    Ok(())
}

pub(super) fn debug_build_arguments(
    gradle_root: &Path,
    debug_application_id_suffix: Option<&str>,
    debug_app_label: Option<&str>,
    force_rerun: bool,
) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from("assembleDebug"),
        OsString::from("--no-daemon"),
        OsString::from("--console=plain"),
        OsString::from("--build-cache"),
    ];
    // Kotlin's daemon-to-fallback handoff on Windows can stringify a native
    // Unicode source root as JSON escape text (for example `一龙` becomes the
    // literal path segment `u4E00u9F99`). Compile inside the single-use Gradle
    // process whenever the managed checkout path is non-ASCII, so Path/OsStr
    // values stay native and never cross that lossy text boundary.
    if path_contains_non_ascii(gradle_root) {
        arguments.push(OsString::from(KOTLIN_IN_PROCESS_ARGUMENT));
    }
    // Source-parity certification forces every task to run. First-time Runtime
    // preparation instead keeps Gradle incremental so a previously completed
    // manual build can be reused without crossing the MCP timeout again.
    if force_rerun {
        arguments.push(OsString::from("--rerun-tasks"));
    }
    if let Some(suffix) = debug_application_id_suffix {
        arguments.push(OsString::from(format!(
            "-PELON_DEBUG_APPLICATION_ID_SUFFIX={suffix}"
        )));
    }
    if let Some(label) = debug_app_label {
        arguments.push(OsString::from(format!("-PELON_DEBUG_APP_LABEL={label}")));
    }
    arguments
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuildTimeoutReason {
    HardLimit,
    NoOutputProgress,
}

impl std::fmt::Display for BuildTimeoutReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HardLimit => formatter.write_str("达到构建最大时长"),
            Self::NoOutputProgress => formatter.write_str("长时间没有 Gradle 输出进展"),
        }
    }
}

#[derive(Clone, Copy)]
struct BuildTimeoutPolicy {
    hard_timeout: Duration,
    idle_timeout: Duration,
}

impl BuildTimeoutPolicy {
    fn from_env() -> Self {
        Self {
            hard_timeout: timeout_from_env(
                "ELON_ANDROID_DEBUG_BUILD_MAX_SECS",
                DEFAULT_BUILD_HARD_TIMEOUT,
            ),
            idle_timeout: timeout_from_env(
                "ELON_ANDROID_DEBUG_BUILD_IDLE_SECS",
                DEFAULT_BUILD_IDLE_TIMEOUT,
            ),
        }
    }

    fn timeout_reason(
        self,
        total_elapsed: Duration,
        idle_elapsed: Duration,
    ) -> Option<BuildTimeoutReason> {
        if total_elapsed >= self.hard_timeout {
            Some(BuildTimeoutReason::HardLimit)
        } else if idle_elapsed >= self.idle_timeout {
            Some(BuildTimeoutReason::NoOutputProgress)
        } else {
            None
        }
    }
}

fn timeout_from_env(name: &str, fallback: Duration) -> Duration {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(Duration::from_secs)
        .unwrap_or(fallback)
}

#[derive(Clone, Copy)]
enum BuildOutputStream {
    Stdout,
    Stderr,
}

struct BuildOutputCapture {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    total_bytes: usize,
    last_activity: Instant,
}

#[derive(Clone, Copy)]
struct BuildOutputSnapshot {
    total_bytes: usize,
    last_activity: Instant,
}

impl BuildOutputCapture {
    fn new() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
            total_bytes: 0,
            last_activity: Instant::now(),
        }
    }

    fn append(&mut self, stream: BuildOutputStream, bytes: &[u8]) {
        self.total_bytes = self.total_bytes.saturating_add(bytes.len());
        self.last_activity = Instant::now();
        let output = match stream {
            BuildOutputStream::Stdout => &mut self.stdout,
            BuildOutputStream::Stderr => &mut self.stderr,
        };
        output.extend_from_slice(bytes);
        if output.len() > MAX_BUILD_OUTPUT {
            output.drain(..output.len() - MAX_BUILD_OUTPUT);
        }
    }

    fn snapshot(&self) -> BuildOutputSnapshot {
        BuildOutputSnapshot {
            total_bytes: self.total_bytes,
            last_activity: self.last_activity,
        }
    }
}

async fn capture_stream(
    mut stream: impl AsyncRead + Unpin,
    capture: Arc<Mutex<BuildOutputCapture>>,
    kind: BuildOutputStream,
) -> std::io::Result<()> {
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            return Ok(());
        }
        capture.lock().await.append(kind, &buffer[..read]);
    }
}

async fn terminate_gradle_process_tree(child: &mut tokio::process::Child, process_id: Option<u32>) {
    let terminated = tokio::task::spawn_blocking(move || {
        crate::node_agent_cli_runtime_policy::terminate_process_tree(process_id)
    })
    .await
    .unwrap_or(false);
    if !terminated {
        let _ = child.kill().await;
    }
    let _ = child.wait().await;
}

fn path_contains_non_ascii(path: &Path) -> bool {
    path.as_os_str()
        .to_string_lossy()
        .chars()
        .any(|ch| !ch.is_ascii())
}

pub(super) fn validate_debug_application_id_suffix(value: &str) -> Result<&str> {
    if value.is_empty()
        || value.len() > 40
        || !value.starts_with('.')
        || value == "."
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
    {
        bail!("debugApplicationIdSuffix 仅允许以点开头的字母、数字、点和下划线，长度不超过 40")
    }
    Ok(value)
}

pub(super) fn validate_package_name(value: &str) -> Result<&str> {
    let valid = !value.is_empty()
        && value.len() <= 220
        && value.split('.').all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
                && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        });
    if !valid {
        bail!("basePackageName 不是合法的 Android applicationId")
    }
    Ok(value)
}

pub(super) fn infer_debug_application_id_suffix(
    gradle_root: &Path,
    package_name: &str,
) -> Result<Option<String>> {
    for relative in [
        "app/build.gradle",
        "app/build.gradle.kts",
        "build.gradle",
        "build.gradle.kts",
    ] {
        let path = gradle_root.join(relative);
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("无法读取 Gradle 配置: {}", path.display()))?;
        let Some(base_application_id) = application_id_literal(&text) else {
            continue;
        };
        if package_name == base_application_id {
            return Ok(None);
        }
        let Some(suffix) = package_name.strip_prefix(&base_application_id) else {
            bail!(
                "当前会话包 {package_name} 不是项目 applicationId {base_application_id} 的 Debug 变体"
            );
        };
        validate_debug_application_id_suffix(suffix)?;
        return Ok(Some(suffix.to_string()));
    }
    Ok(None)
}

fn application_id_literal(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let remainder = line.strip_prefix("applicationId")?;
        if remainder
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return None;
        }
        let value = remainder
            .trim()
            .strip_prefix('=')
            .unwrap_or(remainder.trim())
            .trim();
        let quote = value.chars().next()?;
        if !matches!(quote, '\'' | '"') {
            return None;
        }
        let rest = &value[quote.len_utf8()..];
        let end = rest.find(quote)?;
        Some(rest[..end].to_string())
    })
}

fn tail_output(stdout: &[u8], stderr: &[u8]) -> String {
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr),
    );
    let start = combined.len().saturating_sub(MAX_BUILD_OUTPUT);
    combined
        .get(start..)
        .unwrap_or(&combined)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_suffix_from_groovy_application_id() {
        let root = std::env::temp_dir().join(format!(
            "debug-suffix-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::write(
            root.join("app/build.gradle"),
            "android { defaultConfig {\n  applicationId \"com.example.app\"\n} }",
        )
        .unwrap();
        assert_eq!(
            infer_debug_application_id_suffix(&root, "com.example.app.uituner").unwrap(),
            Some(".uituner".to_string())
        );
        assert_eq!(
            infer_debug_application_id_suffix(&root, "com.example.app").unwrap(),
            None
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parses_kotlin_assignment_literal() {
        assert_eq!(
            application_id_literal("applicationId = \"com.example.kotlin\""),
            Some("com.example.kotlin".to_string())
        );
    }

    #[test]
    fn debug_build_enables_plain_output_and_cross_generation_build_cache() {
        let arguments = debug_build_arguments(
            Path::new("ascii-project"),
            Some(".uitest"),
            Some("Debug"),
            false,
        );
        assert!(arguments.contains(&OsString::from("--console=plain")));
        assert!(arguments.contains(&OsString::from("--build-cache")));
        assert!(!arguments.contains(&OsString::from("--rerun-tasks")));
    }

    #[test]
    fn progress_resets_idle_deadline_but_not_hard_limit() {
        let policy = BuildTimeoutPolicy {
            hard_timeout: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(20),
        };
        assert_eq!(
            policy.timeout_reason(Duration::from_secs(30), Duration::from_secs(5)),
            None
        );
        assert_eq!(
            policy.timeout_reason(Duration::from_secs(30), Duration::from_secs(20)),
            Some(BuildTimeoutReason::NoOutputProgress)
        );
        assert_eq!(
            policy.timeout_reason(Duration::from_secs(60), Duration::from_secs(1)),
            Some(BuildTimeoutReason::HardLimit)
        );
    }
}
