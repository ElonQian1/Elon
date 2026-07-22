use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, bail, Error, Result};

use crate::node_agent_android_inspector::adb_capture::wake_device_for_user_interaction;
use crate::node_agent_android_inspector::adb_command::run_adb_text;

// Honor and other vendor package installers can spend nearly three minutes in
// their security scan before rendering the on-device confirmation. Keep the
// same adb transaction alive long enough for that prompt to be accepted.
const INSTALL_TIMEOUT: Duration = Duration::from_secs(360);
const MAX_OUTPUT: usize = 256 * 1024;
const DEVICE_PROBE_ATTEMPTS: usize = 3;
const INSTALL_ATTEMPTS: usize = 2;

#[derive(Debug, Clone)]
pub(super) struct InstallDebugApkEvidence {
    pub(super) output: String,
    pub(super) device_state: String,
    pub(super) device_probe_attempts: usize,
    pub(super) reconnect_output: Option<String>,
    pub(super) install_attempts: usize,
}

pub(super) async fn install_debug_apk(
    device_id: &str,
    package_name: &str,
    apk: &Path,
    _allow_debug_package_reset: bool,
) -> Result<String> {
    Ok(
        install_debug_apk_with_evidence(device_id, package_name, apk, false)
            .await?
            .output,
    )
}

pub(super) async fn install_debug_apk_with_evidence(
    device_id: &str,
    package_name: &str,
    apk: &Path,
    _allow_debug_package_reset: bool,
) -> Result<InstallDebugApkEvidence> {
    let device = ensure_device_ready(device_id).await?;
    let mut first_error = None;
    for attempt in 1..=INSTALL_ATTEMPTS {
        match install_debug_apk_once(device_id, package_name, apk).await {
            Ok(output) => {
                return Ok(InstallDebugApkEvidence {
                    output,
                    device_state: device.state,
                    device_probe_attempts: device.attempts,
                    reconnect_output: device.reconnect_output,
                    install_attempts: attempt,
                })
            }
            Err(error) if attempt < INSTALL_ATTEMPTS && is_transient_adb_error(&error) => {
                first_error = Some(error.to_string());
                tokio::time::sleep(Duration::from_millis(900)).await;
            }
            Err(error) => {
                return Err(anyhow!(
                    "ADB 安装在 {attempt}/{INSTALL_ATTEMPTS} 次有界尝试后失败；firstError={}; finalError={error:#}",
                    first_error.as_deref().unwrap_or("none")
                ));
            }
        }
    }
    unreachable!("bounded install loop always returns")
}

pub(super) async fn list_legacy_debug_packages(
    device_id: &str,
    expected_package: &str,
) -> Result<Vec<String>> {
    let output = run_adb_text(
        &[
            "-s".into(),
            device_id.into(),
            "shell".into(),
            "pm".into(),
            "list".into(),
            "packages".into(),
        ],
        Duration::from_secs(15),
        MAX_OUTPUT,
    )
    .await?;
    Ok(parse_legacy_debug_packages(&output, expected_package))
}

fn parse_legacy_debug_packages(output: &str, expected_package: &str) -> Vec<String> {
    let base = crate::node_agent_android_live::debug_base_package_name(expected_package);
    let mut packages = output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("package:"))
        .filter(|package| *package != expected_package)
        .filter(|package| {
            package.strip_prefix(base).is_some_and(|suffix| {
                suffix == ".uituner"
                    || suffix.starts_with(".uituner_")
                    || suffix == ".uitest"
                    || suffix.starts_with(".uitest_")
            })
        })
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    packages.sort();
    packages.dedup();
    packages
}

async fn install_debug_apk_once(device_id: &str, package_name: &str, apk: &Path) -> Result<String> {
    // Vendor Android builds can require a visible, on-device confirmation for
    // ADB installs. Public test phones often sleep between users, which leaves
    // that prompt hidden behind a black screen and looks like a PC-side hang.
    // Wake the display and dismiss only an unsecured keyguard immediately
    // before installation. A PIN/fingerprint lock remains protected.
    wake_device_for_user_interaction(device_id).await;
    match run_install(device_id, apk, true).await {
        Ok(output) => require_success(output),
        Err(error) => Err(actionable_install_error(error)),
    }
}

struct DeviceReadyEvidence {
    state: String,
    attempts: usize,
    reconnect_output: Option<String>,
}

async fn ensure_device_ready(device_id: &str) -> Result<DeviceReadyEvidence> {
    let mut first_error = None;
    let mut reconnect_output = None;
    for attempt in 1..=DEVICE_PROBE_ATTEMPTS {
        let args = vec![
            "-s".to_string(),
            device_id.to_string(),
            "get-state".to_string(),
        ];
        match run_adb_text(&args, Duration::from_secs(8), 16 * 1024).await {
            Ok(output) if output.trim() == "device" => {
                return Ok(DeviceReadyEvidence {
                    state: "device".to_string(),
                    attempts: attempt,
                    reconnect_output,
                })
            }
            Ok(output) => first_error = Some(format!("unexpected state: {}", output.trim())),
            Err(error) => {
                let detail = error.to_string();
                if detail.to_ascii_lowercase().contains("unauthorized") {
                    bail!(
                        "ADB_DEVICE_UNAUTHORIZED: 设备 {device_id} 尚未授权此 PC；请在手机上确认调试授权后重试。原始错误：{detail}"
                    );
                }
                first_error = Some(detail);
            }
        }
        if attempt == 1 && is_tcp_device_id(device_id) {
            let connect_args = vec!["connect".to_string(), device_id.to_string()];
            reconnect_output = Some(
                run_adb_text(&connect_args, Duration::from_secs(10), 64 * 1024)
                    .await
                    .unwrap_or_else(|error| format!("connect failed: {error:#}")),
            );
        }
        if attempt < DEVICE_PROBE_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(800)).await;
        }
    }
    bail!(
        "ADB_DEVICE_NOT_READY: 设备 {device_id} 在 {DEVICE_PROBE_ATTEMPTS} 次有界探测后仍未进入 device；reconnect={}; lastError={}",
        reconnect_output.as_deref().unwrap_or("not-attempted"),
        first_error.as_deref().unwrap_or("unknown")
    )
}

fn is_tcp_device_id(device_id: &str) -> bool {
    device_id
        .rsplit_once(':')
        .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok())
}

fn is_transient_adb_error(error: &Error) -> bool {
    let detail = error.to_string().to_ascii_lowercase();
    [
        "device offline",
        "device not found",
        "connection reset",
        "closed",
        "transport error",
    ]
    .iter()
    .any(|signature| detail.contains(signature))
}

fn actionable_install_error(error: Error) -> Error {
    let detail = error.to_string();
    if detail
        .to_ascii_uppercase()
        .contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE")
    {
        return anyhow!(
            "DEBUG_APK_SIGNATURE_MISMATCH: 手机上的固定节点调试包与新 APK 签名不一致。系统已 fail-closed：不会创建新包，也不会自动卸载手机已有应用；请恢复节点原调试签名或由用户明确处理旧应用。原始错误：{detail}"
        );
    }
    if detail.contains("adb 命令超时") && detail.contains(" install ") {
        return anyhow!(
            "手机安装器等待确认超过 6 分钟。首次安装或更新节点专属 Debug 包时，荣耀、小米等系统可能先执行较长的安全扫描，再要求在手机上勾选风险提示并点“继续安装”；请保持手机解锁、完成确认后在 PC 网页重试。后续同签名 Debug 包更新通常会自动完成。原始错误：{detail}"
        );
    }
    if detail.contains("INSTALL_FAILED_USER_RESTRICTED") {
        return anyhow!(
            "手机系统拒绝安装调试 APK。已尝试自动点亮手机，请解锁后在开发者选项中开启“通过 USB 安装”；若手机弹出安装确认，请点允许，然后在 PC 网页点击重试。原始错误：{detail}"
        );
    }
    if detail.contains("INSTALL_FAILED_ABORTED")
        && detail
            .to_ascii_lowercase()
            .contains("user rejected permissions")
    {
        return anyhow!(
            "手机安装器等待用户确认时拒绝了本次 Debug APK 更新。荣耀等系统可能在 adb 已返回失败后仍保留“未经安全审核”提示；请保持手机解锁，点“继续”，再在 PC 网页点击重试。完成一次确认后，后续同签名更新可继续自动安装。原始错误：{detail}"
        );
    }
    error
}

async fn run_install(device_id: &str, apk: &Path, replace: bool) -> Result<String> {
    let mut args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "install".to_string(),
    ];
    if replace {
        args.push("-r".to_string());
    }
    args.extend(["-t".to_string(), apk.display().to_string()]);
    run_adb_text(&args, INSTALL_TIMEOUT, MAX_OUTPUT).await
}

fn require_success(output: String) -> Result<String> {
    if !output.to_ascii_lowercase().contains("success") {
        bail!("Debug APK 安装未返回 Success: {}", output.trim());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{
        actionable_install_error, is_tcp_device_id, is_transient_adb_error,
        parse_legacy_debug_packages, require_success,
    };

    #[test]
    fn install_output_requires_success_marker() {
        assert!(require_success("Success\n".to_string()).is_ok());
        assert!(require_success("Failure [INSTALL_FAILED]\n".to_string()).is_err());
    }

    #[test]
    fn user_restricted_install_explains_visible_phone_action_and_retry() {
        let error = actionable_install_error(anyhow::anyhow!(
            "Failure [INSTALL_FAILED_USER_RESTRICTED: Install canceled by user]"
        ));
        let message = error.to_string();
        assert!(message.contains("已尝试自动点亮手机"));
        assert!(message.contains("在 PC 网页点击重试"));
    }

    #[test]
    fn install_timeout_explains_vendor_confirmation_and_first_install() {
        let error = actionable_install_error(anyhow::anyhow!(
            "adb 命令超时: -s phone install -r -t app-debug.apk"
        ));
        let message = error.to_string();
        assert!(message.contains("超过 6 分钟"));
        assert!(message.contains("安全扫描"));
        assert!(message.contains("首次安装或更新节点专属 Debug 包"));
        assert!(message.contains("继续安装"));
        assert!(message.contains("后续同签名 Debug 包更新通常会自动完成"));
    }

    #[test]
    fn aborted_honor_install_explains_late_confirmation_and_retry() {
        let error = actionable_install_error(anyhow::anyhow!(
            "Failure [INSTALL_FAILED_ABORTED: User rejected permissions]"
        ));
        let message = error.to_string();
        assert!(message.contains("adb 已返回失败后仍保留"));
        assert!(message.contains("点“继续”"));
        assert!(message.contains("在 PC 网页点击重试"));
    }

    #[test]
    fn retries_only_transient_transport_failures() {
        assert!(is_transient_adb_error(&anyhow::anyhow!("device offline")));
        assert!(!is_transient_adb_error(&anyhow::anyhow!(
            "INSTALL_FAILED_USER_RESTRICTED"
        )));
        assert!(is_tcp_device_id("192.168.31.171:5555"));
        assert!(!is_tcp_device_id("emulator-5554"));
    }

    #[test]
    fn signature_mismatch_never_suggests_automatic_uninstall() {
        let message = actionable_install_error(anyhow::anyhow!(
            "Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE]"
        ))
        .to_string();
        assert!(message.contains("不会自动卸载"));
        assert!(message.contains("不会创建新包"));
    }

    #[test]
    fn legacy_debug_packages_are_reported_without_the_formal_or_fixed_package() {
        let output = "package:com.elon.app\npackage:com.elon.app.uitest\npackage:com.elon.app.uitest_anim\npackage:com.elon.app.uituner_oldnode\npackage:com.elon.app.uituner_fixednode\n";
        assert_eq!(
            parse_legacy_debug_packages(output, "com.elon.app.uituner_fixednode"),
            vec![
                "com.elon.app.uitest",
                "com.elon.app.uitest_anim",
                "com.elon.app.uituner_oldnode"
            ]
        );
    }
}
