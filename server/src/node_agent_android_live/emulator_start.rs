use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

const BOOT_TIMEOUT: Duration = Duration::from_secs(150);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeviceSelection {
    pub(super) device_id: String,
    pub(super) source: &'static str,
    pub(super) avd_name: Option<String>,
}

pub(super) async fn select_or_start(
    requested: Option<String>,
    auto_start_emulator: bool,
    fallback_to_emulator: bool,
) -> Result<DeviceSelection> {
    let devices =
        crate::node_agent_android_inspector::adb_wireless::list_device_inventory().await?;
    if let Some(device_id) = requested.as_deref() {
        if devices
            .iter()
            .any(|device| device.serial == device_id && device.state == "device")
        {
            return Ok(DeviceSelection {
                device_id: device_id.to_string(),
                source: "requested",
                avd_name: None,
            });
        }
        if !fallback_to_emulator {
            bail!("请求的 Android 设备 {device_id} 不在线，且 fallbackToEmulator=false");
        }
        if let Some(device) = devices
            .iter()
            .find(|device| device.state == "device" && device.serial.starts_with("emulator-"))
        {
            return Ok(DeviceSelection {
                device_id: device.serial.clone(),
                source: "fallback_existing_emulator",
                avd_name: None,
            });
        }
    } else if let Some(device) = devices
        .iter()
        .find(|device| device.state == "device" && device.serial.starts_with("emulator-"))
    {
        return Ok(DeviceSelection {
            device_id: device.serial.clone(),
            source: "existing_emulator",
            avd_name: None,
        });
    } else if let Some(device) = devices.iter().find(|device| device.state == "device") {
        return Ok(DeviceSelection {
            device_id: device.serial.clone(),
            source: "existing_device",
            avd_name: None,
        });
    }

    if !auto_start_emulator {
        bail!("没有可用 Android 设备或模拟器，且 autoStartEmulator=false");
    }
    start_default_emulator().await
}

async fn start_default_emulator() -> Result<DeviceSelection> {
    let executable = find_emulator_executable()?;
    let avds = list_avds(&executable).await?;
    let requested = std::env::var("ELON_ANDROID_AVD")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let avd_name = match requested {
        Some(requested) if avds.iter().any(|avd| avd == &requested) => requested,
        Some(requested) => bail!(
            "ELON_ANDROID_AVD={requested} 不存在；可用 AVD: {}",
            avds.join(", ")
        ),
        None => avds
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("Android SDK 没有已创建的 AVD"))?,
    };
    let existing = crate::node_agent_android_inspector::adb_wireless::list_device_inventory()
        .await?
        .into_iter()
        .filter(|device| device.serial.starts_with("emulator-"))
        .map(|device| device.serial)
        .collect::<Vec<_>>();
    let mut command = Command::new(&executable);
    command
        .arg("-avd")
        .arg(&avd_name)
        .args([
            "-no-window",
            "-no-snapshot-save",
            "-no-boot-anim",
            "-gpu",
            "swiftshader_indirect",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false);
    crate::node_agent_exec::hide_tokio_command_window(&mut command);
    command
        .spawn()
        .with_context(|| format!("无法启动 Android AVD {avd_name}"))?;

    let started = Instant::now();
    loop {
        let devices =
            crate::node_agent_android_inspector::adb_wireless::list_device_inventory().await?;
        if let Some(device) = devices.iter().find(|device| {
            device.state == "device"
                && device.serial.starts_with("emulator-")
                && (!existing.contains(&device.serial)
                    || existing.len() == 1
                    || started.elapsed() > Duration::from_secs(5))
        }) {
            return Ok(DeviceSelection {
                device_id: device.serial.clone(),
                source: "auto_started_emulator",
                avd_name: Some(avd_name),
            });
        }
        if started.elapsed() >= BOOT_TIMEOUT {
            bail!(
                "自动启动 AVD {avd_name} 后 {} 秒仍未出现在线 emulator 设备",
                BOOT_TIMEOUT.as_secs()
            );
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn list_avds(executable: &Path) -> Result<Vec<String>> {
    let output = Command::new(executable)
        .arg("-list-avds")
        .output()
        .await
        .context("无法列出 Android AVD")?;
    if !output.status.success() {
        bail!("Android emulator -list-avds 失败");
    }
    let mut avds = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && !value.contains(['\r', '\n', '\0'])
                && !value.starts_with('-')
        })
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    avds.sort();
    avds.dedup();
    Ok(avds)
}

fn find_emulator_executable() -> Result<PathBuf> {
    let mut candidates = Vec::new();
    for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(root) = std::env::var_os(key).filter(|value| !value.is_empty()) {
            candidates.push(PathBuf::from(root).join("emulator").join(executable_name()));
        }
    }
    if cfg!(windows) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local)
                    .join("Android")
                    .join("Sdk")
                    .join("emulator")
                    .join("emulator.exe"),
            );
        }
        candidates.extend([
            PathBuf::from(r"D:\Android\sdk\emulator\emulator.exe"),
            PathBuf::from(r"C:\Android\sdk\emulator\emulator.exe"),
        ]);
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| anyhow!("未找到 Android emulator 可执行文件"))
}

fn executable_name() -> &'static str {
    if cfg!(windows) {
        "emulator.exe"
    } else {
        "emulator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avd_name_filter_rejects_argument_injection() {
        for value in ["", "-wipe-data", "bad\nname", "bad\0name"] {
            assert!(
                value.is_empty() || value.contains(['\r', '\n', '\0']) || value.starts_with('-')
            );
        }
    }
}
