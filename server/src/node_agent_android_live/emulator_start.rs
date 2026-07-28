use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

const BOOT_TIMEOUT: Duration = Duration::from_secs(150);
const DEFAULT_MAX_EMULATOR_SLOTS: usize = 2;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeviceSelection {
    pub(super) device_id: String,
    pub(super) source: &'static str,
    pub(super) avd_name: Option<String>,
    pub(super) emulator_slot_id: Option<String>,
    pub(super) fallback_reason: Option<String>,
}

pub(super) async fn select_or_start(
    requested: Option<String>,
    auto_start_emulator: bool,
    fallback_to_emulator: bool,
    prefer_emulator: bool,
    require_visual_ready: bool,
    excluded_device_ids: &HashSet<String>,
) -> Result<DeviceSelection> {
    let devices =
        crate::node_agent_android_inspector::adb_wireless::list_device_inventory().await?;
    let available = devices
        .iter()
        .filter(|device| {
            device.state == "device" && !excluded_device_ids.contains(device.serial.as_str())
        })
        .collect::<Vec<_>>();

    if prefer_emulator {
        return select_emulator_or_start(
            &available,
            auto_start_emulator,
            requested
                .as_deref()
                .filter(|device_id| !device_id.starts_with("emulator-"))
                .map(|device_id| format!("显式跳过物理设备 {device_id}")),
            "preferred_existing_emulator",
            excluded_device_ids,
        )
        .await;
    }

    if let Some(device_id) = requested.as_deref() {
        if available
            .iter()
            .any(|device| device.serial == device_id && device.state == "device")
        {
            if !device_id.starts_with("emulator-") && require_visual_ready {
                if let Some(reason) =
                    crate::node_agent_android_inspector::adb_capture::visual_unavailable_reason(
                        device_id,
                    )
                    .await?
                {
                    if !fallback_to_emulator {
                        bail!(
                            "请求的 Android 设备 {device_id} 当前无法视觉验收，且 fallbackToEmulator=false：{reason}"
                        );
                    }
                    return select_emulator_or_start(
                        &available,
                        auto_start_emulator,
                        Some(reason),
                        "fallback_visual_unavailable_emulator",
                        excluded_device_ids,
                    )
                    .await;
                }
            }
            return Ok(DeviceSelection {
                device_id: device_id.to_string(),
                source: "requested",
                emulator_slot_id: device_id
                    .starts_with("emulator-")
                    .then(|| device_id.to_string()),
                avd_name: None,
                fallback_reason: None,
            });
        }
        if !fallback_to_emulator {
            let reason = if excluded_device_ids.contains(device_id) {
                "已被其他 Renderer 会话占用"
            } else {
                "不在线"
            };
            bail!("请求的 Android 设备 {device_id} {reason}，且 fallbackToEmulator=false");
        }
        return select_emulator_or_start(
            &available,
            auto_start_emulator,
            Some(if excluded_device_ids.contains(device_id) {
                format!("请求的 Android 设备 {device_id} 已被其他 Renderer 会话占用")
            } else {
                format!("请求的 Android 设备 {device_id} 不在线")
            }),
            "fallback_existing_emulator",
            excluded_device_ids,
        )
        .await;
    }

    if let Some(device_id) = choose_emulator_serial(
        available.iter().map(|device| device.serial.as_str()),
        excluded_device_ids,
    ) {
        return Ok(DeviceSelection {
            device_id: device_id.clone(),
            source: "existing_emulator",
            avd_name: None,
            emulator_slot_id: Some(device_id),
            fallback_reason: None,
        });
    }
    if let Some(device) = available.first() {
        if require_visual_ready {
            if let Some(reason) =
                crate::node_agent_android_inspector::adb_capture::visual_unavailable_reason(
                    &device.serial,
                )
                .await?
            {
                if fallback_to_emulator {
                    return select_emulator_or_start(
                        &available,
                        auto_start_emulator,
                        Some(reason),
                        "fallback_visual_unavailable_emulator",
                        excluded_device_ids,
                    )
                    .await;
                }
                bail!(
                    "Android 设备 {} 当前无法视觉验收，且 fallbackToEmulator=false：{reason}",
                    device.serial
                );
            }
        }
        return Ok(DeviceSelection {
            device_id: device.serial.clone(),
            source: "existing_device",
            avd_name: None,
            emulator_slot_id: None,
            fallback_reason: None,
        });
    }

    if !auto_start_emulator {
        bail!("没有空闲 Android 设备或模拟器，且 autoStartEmulator=false");
    }
    start_default_emulator(excluded_device_ids).await
}

async fn select_emulator_or_start(
    devices: &[&crate::node_agent_android_inspector::types::AndroidDevice],
    auto_start_emulator: bool,
    fallback_reason: Option<String>,
    existing_source: &'static str,
    excluded_device_ids: &HashSet<String>,
) -> Result<DeviceSelection> {
    if let Some(device_id) = choose_emulator_serial(
        devices.iter().map(|device| device.serial.as_str()),
        excluded_device_ids,
    ) {
        return Ok(DeviceSelection {
            device_id: device_id.clone(),
            source: existing_source,
            avd_name: None,
            emulator_slot_id: Some(device_id),
            fallback_reason,
        });
    }
    if !auto_start_emulator {
        bail!("没有空闲 Android 模拟器，且 autoStartEmulator=false");
    }
    let mut selection = start_default_emulator(excluded_device_ids).await?;
    selection.fallback_reason = fallback_reason;
    Ok(selection)
}

fn choose_emulator_serial<'a>(
    serials: impl IntoIterator<Item = &'a str>,
    excluded_device_ids: &HashSet<String>,
) -> Option<String> {
    serials
        .into_iter()
        .find(|serial| serial.starts_with("emulator-") && !excluded_device_ids.contains(*serial))
        .map(str::to_string)
}

async fn start_default_emulator(excluded_device_ids: &HashSet<String>) -> Result<DeviceSelection> {
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
    let max_slots = std::env::var("ELON_ANDROID_EMULATOR_MAX_SLOTS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MAX_EMULATOR_SLOTS);
    if existing.len() >= max_slots {
        bail!(
            "本机 Android 模拟器池已满（{}/{}），且没有空闲 slot；等待现有 Renderer lease 释放后重试",
            existing.len(),
            max_slots
        );
    }
    let mut command = Command::new(&executable);
    command
        .arg("-avd")
        .arg(&avd_name)
        .args([
            "-no-window",
            "-no-snapshot-save",
            "-read-only",
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
                && !existing.contains(&device.serial)
                && !excluded_device_ids.contains(device.serial.as_str())
        }) {
            return Ok(DeviceSelection {
                device_id: device.serial.clone(),
                source: "auto_started_emulator",
                avd_name: Some(avd_name),
                emulator_slot_id: Some(device.serial.clone()),
                fallback_reason: None,
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
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn avd_name_filter_rejects_argument_injection() {
        for value in ["", "-wipe-data", "bad\nname", "bad\0name"] {
            assert!(
                value.is_empty() || value.contains(['\r', '\n', '\0']) || value.starts_with('-')
            );
        }
    }

    #[test]
    fn occupied_emulator_is_skipped_in_favor_of_an_idle_slot() {
        let mut excluded = HashSet::new();
        excluded.insert("emulator-5554".to_string());
        assert_eq!(
            choose_emulator_serial(["phone-a", "emulator-5554", "emulator-5556"], &excluded),
            Some("emulator-5556".to_string())
        );
    }

    #[test]
    fn all_occupied_emulators_yield_no_slot() {
        let excluded = HashSet::from(["emulator-5554".to_string(), "emulator-5556".to_string()]);
        assert_eq!(
            choose_emulator_serial(["emulator-5554", "emulator-5556"], &excluded),
            None
        );
    }
}
