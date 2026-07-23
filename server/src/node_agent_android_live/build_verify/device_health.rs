use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tokio::process::Command;

use crate::node_agent_android_inspector::adb_command::{run_adb_text, validate_device_id};
use crate::node_agent_android_inspector::adb_path::adb_path;

const PROBE_ATTEMPTS: usize = 3;
const RECOVERY_WAIT_ATTEMPTS: usize = 150;
const MAX_OUTPUT: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub(super) struct AndroidFrameworkHealth {
    pub(super) probe_attempts: usize,
    pub(super) recovery_action: Option<&'static str>,
    pub(super) recovery_wait_attempts: usize,
    pub(super) detail: String,
}

#[derive(Debug, Clone)]
struct FrameworkProbe {
    ready: bool,
    detail: String,
}

pub(super) async fn ensure_android_framework_ready(
    device_id: &str,
) -> Result<AndroidFrameworkHealth> {
    validate_device_id(device_id)?;
    let mut last_probe = None;
    for attempt in 1..=PROBE_ATTEMPTS {
        let probe = probe_android_framework(device_id).await;
        if probe.ready {
            return Ok(AndroidFrameworkHealth {
                probe_attempts: attempt,
                recovery_action: None,
                recovery_wait_attempts: 0,
                detail: probe.detail,
            });
        }
        last_probe = Some(probe);
        if attempt < PROBE_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(700)).await;
        }
    }

    let detail = last_probe
        .as_ref()
        .map(|probe| probe.detail.as_str())
        .unwrap_or("unknown");
    if !device_id.starts_with("emulator-") {
        bail!(
            "ADB_FRAMEWORK_NOT_READY: 设备 {device_id} 的 Android framework 服务未就绪；为避免中断真机，不自动重启。probe={detail}"
        );
    }

    cold_relaunch_emulator(device_id)
        .await
        .with_context(|| {
            format!(
                "ADB_EMULATOR_RECOVERY_FAILED: 模拟器 {device_id} 伪就绪且无数据冷启动恢复失败；initialProbe={detail}"
            )
        })?;

    for attempt in 1..=RECOVERY_WAIT_ATTEMPTS {
        let probe = probe_android_framework(device_id).await;
        if probe.ready {
            return Ok(AndroidFrameworkHealth {
                probe_attempts: PROBE_ATTEMPTS,
                recovery_action: Some("COLD_RELAUNCH_NO_WIPE"),
                recovery_wait_attempts: attempt,
                detail: probe.detail,
            });
        }
        last_probe = Some(probe);
        if attempt < RECOVERY_WAIT_ATTEMPTS {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    bail!(
        "ADB_EMULATOR_FRAMEWORK_TIMEOUT: 模拟器 {device_id} 无数据冷启动后在 {} 次有界探测内仍未就绪；lastProbe={}",
        RECOVERY_WAIT_ATTEMPTS,
        last_probe
            .as_ref()
            .map(|probe| probe.detail.as_str())
            .unwrap_or("unknown")
    )
}

async fn probe_android_framework(device_id: &str) -> FrameworkProbe {
    let state = adb_probe(device_id, &["get-state"], Duration::from_secs(8)).await;
    let boot = adb_probe(
        device_id,
        &["shell", "getprop", "sys.boot_completed"],
        Duration::from_secs(8),
    )
    .await;
    let package = adb_probe(
        device_id,
        &["shell", "service", "check", "package"],
        Duration::from_secs(8),
    )
    .await;
    let settings = adb_probe(
        device_id,
        &["shell", "service", "check", "settings"],
        Duration::from_secs(8),
    )
    .await;
    classify_framework_probe(&state, &boot, &package, &settings)
}

async fn adb_probe(device_id: &str, suffix: &[&str], timeout: Duration) -> String {
    let mut args = vec!["-s".to_string(), device_id.to_string()];
    args.extend(suffix.iter().map(|value| value.to_string()));
    run_adb_text(&args, timeout, MAX_OUTPUT)
        .await
        .map(|output| output.trim().to_string())
        .unwrap_or_else(|error| format!("ERROR:{error:#}"))
}

fn classify_framework_probe(
    state: &str,
    boot: &str,
    package: &str,
    settings: &str,
) -> FrameworkProbe {
    let package_ready = service_found(package, "package");
    let settings_ready = service_found(settings, "settings");
    FrameworkProbe {
        ready: state.trim() == "device" && boot.trim() == "1" && package_ready && settings_ready,
        detail: format!(
            "state={}; bootCompleted={}; packageService={}; settingsService={}",
            compact(state),
            compact(boot),
            compact(package),
            compact(settings)
        ),
    }
}

fn service_found(output: &str, service: &str) -> bool {
    output
        .trim()
        .eq_ignore_ascii_case(&format!("Service {service}: found"))
}

fn compact(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}

async fn cold_relaunch_emulator(device_id: &str) -> Result<()> {
    let avd_output = adb_probe(device_id, &["emu", "avd", "name"], Duration::from_secs(10)).await;
    let avd_name = parse_avd_name(&avd_output)?;
    let port = emulator_console_port(device_id)?;
    let executable = find_emulator_executable()?;

    let kill_args = vec![
        "-s".to_string(),
        device_id.to_string(),
        "emu".to_string(),
        "kill".to_string(),
    ];
    run_adb_text(&kill_args, Duration::from_secs(15), MAX_OUTPUT)
        .await
        .context("无法停止伪就绪模拟器")?;

    for _ in 0..40 {
        let state = adb_probe(device_id, &["get-state"], Duration::from_secs(2)).await;
        if state.trim() != "device" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let args = emulator_relaunch_args(&avd_name, port);
    let mut command = Command::new(&executable);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false);
    crate::node_agent_exec::hide_tokio_command_window(&mut command);
    command.spawn().with_context(|| {
        format!(
            "无法启动 Android 模拟器 {}（AVD={avd_name}）",
            executable.display()
        )
    })?;
    Ok(())
}

fn parse_avd_name(output: &str) -> Result<String> {
    let name = output
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty() && !line.eq_ignore_ascii_case("OK") && !line.starts_with("ERROR:")
        })
        .ok_or_else(|| anyhow!("无法读取当前模拟器 AVD 名称: {}", compact(output)))?;
    if name.chars().count() > 128 || name.chars().any(char::is_control) {
        bail!("模拟器 AVD 名称无效");
    }
    Ok(name.to_string())
}

fn emulator_console_port(device_id: &str) -> Result<u16> {
    let port = device_id
        .strip_prefix("emulator-")
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| (5554..=5682).contains(port) && port % 2 == 0)
        .ok_or_else(|| anyhow!("模拟器 serial 不包含有效 console 端口: {device_id}"))?;
    Ok(port)
}

fn emulator_relaunch_args(avd_name: &str, port: u16) -> Vec<String> {
    vec![
        "-avd".into(),
        avd_name.into(),
        "-port".into(),
        port.to_string(),
        "-no-snapshot-load".into(),
        "-no-boot-anim".into(),
    ]
}

fn find_emulator_executable() -> Result<PathBuf> {
    let mut candidates = Vec::new();
    let adb = PathBuf::from(adb_path());
    if let Some(sdk_root) = adb.parent().and_then(Path::parent) {
        candidates.push(sdk_root.join("emulator").join(emulator_executable_name()));
    }
    for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(root) = std::env::var_os(key).filter(|value| !value.is_empty()) {
            candidates.push(
                PathBuf::from(root)
                    .join("emulator")
                    .join(emulator_executable_name()),
            );
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
        .ok_or_else(|| anyhow!("未找到 Android emulator 可执行文件，无法无数据重启 AVD"))
}

fn emulator_executable_name() -> &'static str {
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
    fn boot_property_alone_does_not_mark_framework_ready() {
        let probe = classify_framework_probe(
            "device",
            "1",
            "Service package: not found",
            "Service settings: not found",
        );
        assert!(!probe.ready);
        assert!(probe.detail.contains("bootCompleted=1"));
        assert!(probe
            .detail
            .contains("packageService=Service package: not found"));
    }

    #[test]
    fn package_and_settings_services_are_required() {
        assert!(
            classify_framework_probe(
                "device",
                "1",
                "Service package: found",
                "Service settings: found",
            )
            .ready
        );
        assert!(
            !classify_framework_probe(
                "device",
                "1",
                "Service package: found",
                "Service settings: not found",
            )
            .ready
        );
    }

    #[test]
    fn avd_relaunch_is_cold_and_never_wipes_data() {
        let args = emulator_relaunch_args("Medium_Phone_API_36", 5554);
        assert!(args.contains(&"-no-snapshot-load".to_string()));
        assert!(args.contains(&"-port".to_string()));
        assert!(!args.iter().any(|arg| arg == "-wipe-data"));
        assert_eq!(emulator_console_port("emulator-5554").unwrap(), 5554);
        assert!(emulator_console_port("emulator-5555").is_err());
    }

    #[test]
    fn avd_name_ignores_console_acknowledgement() {
        assert_eq!(
            parse_avd_name("Medium_Phone_API_36\r\nOK\r\n").unwrap(),
            "Medium_Phone_API_36"
        );
    }
}
