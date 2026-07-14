use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, bail, Error, Result};

use crate::node_agent_android_inspector::adb_capture::wake_device_for_user_interaction;
use crate::node_agent_android_inspector::adb_command::run_adb_text;

const INSTALL_TIMEOUT: Duration = Duration::from_secs(180);
const UNINSTALL_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_OUTPUT: usize = 256 * 1024;
const SIGNATURE_MISMATCH: &str = "INSTALL_FAILED_UPDATE_INCOMPATIBLE";

pub(super) async fn install_debug_apk(
    device_id: &str,
    package_name: &str,
    apk: &Path,
    allow_debug_package_reset: bool,
) -> Result<String> {
    // Vendor Android builds can require a visible, on-device confirmation for
    // ADB installs. Public test phones often sleep between users, which leaves
    // that prompt hidden behind a black screen and looks like a PC-side hang.
    // Wake the display and dismiss only an unsecured keyguard immediately
    // before installation. A PIN/fingerprint lock remains protected.
    wake_device_for_user_interaction(device_id).await;
    match run_install(device_id, apk, true).await {
        Ok(output) => require_success(output),
        Err(error)
            if allow_debug_package_reset
                && error
                    .to_string()
                    .to_ascii_uppercase()
                    .contains(SIGNATURE_MISMATCH) =>
        {
            // A side-by-side debug package can outlive a keystore rotation or
            // a node update. It contains no production app data, so reset only
            // this suffixed debug package and retry instead of leaving the PC
            // page stuck on an opaque signature mismatch.
            run_adb_text(
                &[
                    "-s".to_string(),
                    device_id.to_string(),
                    "uninstall".to_string(),
                    package_name.to_string(),
                ],
                UNINSTALL_TIMEOUT,
                MAX_OUTPUT,
            )
            .await?;
            let output = require_success(
                run_install(device_id, apk, false)
                    .await
                    .map_err(actionable_install_error)?,
            )?;
            Ok(format!(
                "Reset incompatible side-by-side debug package {package_name}.\n{output}"
            ))
        }
        Err(error) => Err(actionable_install_error(error)),
    }
}

fn actionable_install_error(error: Error) -> Error {
    let detail = error.to_string();
    if detail.contains("INSTALL_FAILED_USER_RESTRICTED") {
        return anyhow!(
            "手机系统拒绝安装调试 APK。已尝试自动点亮手机，请解锁后在开发者选项中开启“通过 USB 安装”；若手机弹出安装确认，请点允许，然后在 PC 网页点击重试。原始错误：{detail}"
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
    use super::{actionable_install_error, require_success};

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
}
