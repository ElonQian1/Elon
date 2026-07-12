use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tokio::process::Command;

use super::super::broker::LiveUiSession;

const BUILD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const MAX_BUILD_OUTPUT: usize = 256 * 1024;

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
        // Build verification must produce a fresh artifact. Otherwise a stale
        // APK from an UP-TO-DATE task can be installed and falsely certified.
        .args(["assembleDebug", "--no-daemon", "--rerun-tasks"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(suffix) = debug_application_id_suffix {
        command.arg(format!("-PELON_DEBUG_APPLICATION_ID_SUFFIX={suffix}"));
    }
    crate::node_agent_exec::hide_tokio_command_window(&mut command);
    let output = tokio::time::timeout(BUILD_TIMEOUT, command.output())
        .await
        .context("Android Debug 构建超时")?
        .context("无法启动 Gradle Wrapper")?;
    if output.stdout.len() + output.stderr.len() > MAX_BUILD_OUTPUT * 4 {
        bail!("Gradle 输出异常过大，已停止验收");
    }
    if !output.status.success() {
        let message = tail_output(&output.stdout, &output.stderr);
        bail!("Android Debug 构建失败: {message}");
    }
    Ok(())
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
        let value = remainder.trim().strip_prefix('=').unwrap_or(remainder.trim()).trim();
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
}
