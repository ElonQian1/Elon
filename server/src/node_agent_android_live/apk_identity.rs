use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use super::debug_integration::DebugArtifactStatus;

pub(crate) fn verify_and_stage_apk(
    apk: &Path,
    gradle_root: &Path,
    artifact_root: &Path,
    expected_package: &str,
    expected_label: Option<&str>,
    generation: u64,
) -> Result<DebugArtifactStatus> {
    let bytes = fs::read(apk)?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    let tool_apk = ApkToolInput::prepare(apk, &bytes, &sha256)?;
    let aapt = find_sdk_tool(gradle_root, if cfg!(windows) { "aapt.exe" } else { "aapt" })?;
    let badging = command_output(
        &aapt,
        &[
            OsStr::new("dump"),
            OsStr::new("badging"),
            tool_apk.path().as_os_str(),
        ],
    )?;
    let (package_name, version_code, version_name, app_label) = parse_badging(&badging)?;
    if package_name != expected_package {
        bail!("APK_PACKAGE_MISMATCH: 期望 {expected_package}，实际 {package_name}");
    }
    if let Some(expected) = expected_label {
        if app_label != expected {
            bail!("APK_LABEL_MISMATCH: 期望节点固定标签 {expected:?}，实际 {app_label:?}");
        }
    }
    let apksigner = find_sdk_tool(
        gradle_root,
        if cfg!(windows) {
            "apksigner.bat"
        } else {
            "apksigner"
        },
    )?;
    let signer_output = command_output(
        &apksigner,
        &[
            OsStr::new("verify"),
            OsStr::new("--print-certs"),
            tool_apk.path().as_os_str(),
        ],
    )?;
    let signer_sha256 = parse_signer_sha256(&signer_output)?;
    fs::create_dir_all(artifact_root)?;
    let staged = artifact_root.join(format!("{sha256}.apk"));
    if !staged.exists() {
        let temporary = artifact_root.join(format!(".{sha256}.{}.tmp", std::process::id()));
        fs::write(&temporary, &bytes)?;
        fs::rename(&temporary, &staged).with_context(|| {
            format!(
                "无法原子暂存 APK: {} -> {}",
                temporary.display(),
                staged.display()
            )
        })?;
    }
    Ok(DebugArtifactStatus {
        apk_path: staged.display().to_string(),
        sha256,
        package_name,
        version_code,
        version_name,
        app_label,
        signer_sha256,
        generation,
    })
}

struct ApkToolInput {
    path: PathBuf,
    temporary: bool,
}

impl ApkToolInput {
    fn prepare(apk: &Path, bytes: &[u8], sha256: &str) -> Result<Self> {
        if !cfg!(windows) || is_ascii_path(apk) {
            return Ok(Self {
                path: apk.to_path_buf(),
                temporary: false,
            });
        }

        for root in ascii_inspection_roots() {
            if !is_ascii_path(&root) {
                continue;
            }
            let directory = root.join("elon-apk-identity-v1");
            if fs::create_dir_all(&directory).is_err() {
                continue;
            }
            let path = directory.join(format!(
                "{}-{}-{}.apk",
                std::process::id(),
                uuid::Uuid::new_v4().simple(),
                sha256
            ));
            if fs::write(&path, bytes).is_ok() {
                return Ok(Self {
                    path,
                    temporary: true,
                });
            }
        }

        bail!("APK_IDENTITY_UNICODE_STAGING_FAILED: 无法创建 ASCII 工具检查副本");
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ApkToolInput {
    fn drop(&mut self) {
        if self.temporary {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn ascii_inspection_roots() -> Vec<PathBuf> {
    let mut roots = vec![std::env::temp_dir()];
    #[cfg(windows)]
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        roots.push(PathBuf::from(system_root).join("Temp"));
    }
    roots
}

#[cfg(windows)]
fn is_ascii_path(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str().encode_wide().all(|unit| unit <= 0x7f)
}

#[cfg(not(windows))]
fn is_ascii_path(path: &Path) -> bool {
    path.as_os_str().to_string_lossy().is_ascii()
}

fn find_sdk_tool(gradle_root: &Path, name: &str) -> Result<PathBuf> {
    let sdk_root = ["ANDROID_SDK_ROOT", "ANDROID_HOME"]
        .into_iter()
        .find_map(|key| std::env::var_os(key).map(PathBuf::from))
        .or_else(|| sdk_dir_from_local_properties(gradle_root));
    let sdk_root = sdk_root.context("APK_IDENTITY_TOOL_MISSING: 未找到 Android SDK 根目录")?;
    let build_tools = sdk_root.join("build-tools");
    let mut versions = fs::read_dir(&build_tools)
        .with_context(|| format!("无法读取 Android build-tools: {}", build_tools.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .collect::<Vec<_>>();
    versions.sort_by_key(|entry| entry.file_name());
    versions.reverse();
    versions
        .into_iter()
        .map(|entry| entry.path().join(name))
        .find(|path| path.is_file())
        .with_context(|| format!("APK_IDENTITY_TOOL_MISSING: build-tools 中未找到 {name}"))
}

fn sdk_dir_from_local_properties(gradle_root: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(gradle_root.join("local.properties")).ok()?;
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("sdk.dir=")?.replace("\\\\", "\\");
        (!value.trim().is_empty()).then(|| PathBuf::from(value.trim()))
    })
}

fn command_output(program: &Path, args: &[&OsStr]) -> Result<String> {
    let mut command =
        if cfg!(windows) && program.extension().and_then(|value| value.to_str()) == Some("bat") {
            let mut command = Command::new("cmd.exe");
            command.args(["/D", "/C"]).arg(program);
            command
        } else {
            Command::new(program)
        };
    let output = command.args(args).output()?;
    if !output.status.success() {
        bail!(
            "APK 身份工具失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_badging(output: &str) -> Result<(String, String, String, String)> {
    let package = output
        .lines()
        .find(|line| line.starts_with("package: "))
        .context("APK badging 缺少 package 行")?;
    let package_name = quoted_field(package, "name").context("APK badging 缺少 package name")?;
    let version_code =
        quoted_field(package, "versionCode").context("APK badging 缺少 versionCode")?;
    let version_name =
        quoted_field(package, "versionName").context("APK badging 缺少 versionName")?;
    let app_label = output
        .lines()
        .find_map(|line| {
            line.strip_prefix("application-label:")
                .and_then(single_quoted)
        })
        .context("APK badging 缺少 application-label")?;
    Ok((package_name, version_code, version_name, app_label))
}

fn parse_signer_sha256(output: &str) -> Result<String> {
    output
        .lines()
        .find_map(|line| {
            line.split_once("certificate SHA-256 digest:")
                .map(|(_, value)| value.trim().replace(':', "").to_ascii_lowercase())
        })
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .context("APK 签名校验没有返回有效 SHA-256 证书摘要")
}

fn quoted_field(line: &str, field: &str) -> Option<String> {
    let marker = format!("{field}='");
    let rest = line.split_once(&marker)?.1;
    Some(rest.split_once('\'')?.0.to_string())
}

fn single_quoted(value: &str) -> Option<String> {
    let value = value.trim();
    Some(value.strip_prefix('\'')?.strip_suffix('\'')?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_version_label_and_signer() {
        let badging = "package: name='com.elon.app.uituner_abcd1234' versionCode='462' versionName='1.1.454-uituner'\napplication-label:'一龙调试 abcd1234'\n";
        let parsed = parse_badging(badging).unwrap();
        assert_eq!(parsed.0, "com.elon.app.uituner_abcd1234");
        assert_eq!(parsed.1, "462");
        assert_eq!(parsed.3, "一龙调试 abcd1234");
        let signer = parse_signer_sha256(
            "Signer #1 certificate SHA-256 digest: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();
        assert_eq!(signer.len(), 64);
    }

    #[cfg(windows)]
    #[test]
    fn unicode_node_data_root_stages_apk_for_legacy_identity_tools() {
        let root = std::env::temp_dir().join(format!(
            "elon-apk-identity-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let unicode_root = root.join("一龙").join("ElonNodeData");
        fs::create_dir_all(&unicode_root).unwrap();
        let apk = unicode_root.join("app-debug.apk");
        let bytes = b"unicode-apk-path";
        fs::write(&apk, bytes).unwrap();
        let sha256 = hex::encode(Sha256::digest(bytes));

        let tool_apk = ApkToolInput::prepare(&apk, bytes, &sha256).unwrap();
        assert_ne!(tool_apk.path(), apk);
        assert!(is_ascii_path(tool_apk.path()));
        assert_eq!(fs::read(tool_apk.path()).unwrap(), bytes);
        let temporary = tool_apk.path().to_path_buf();
        drop(tool_apk);
        assert!(!temporary.exists());

        fs::remove_dir_all(root).unwrap();
    }
}
