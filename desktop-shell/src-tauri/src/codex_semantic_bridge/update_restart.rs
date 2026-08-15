use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{Manager, WebviewWindow};

const GUARD_SCRIPT: &str = include_str!("update_restart_guard.ps1");
const EXIT_DELAY: Duration = Duration::from_millis(2_500);
static RESTART_SCHEDULED: AtomicBool = AtomicBool::new(false);

pub(super) fn schedule(
    window: &WebviewWindow,
    action_id: &str,
    requested_by: &str,
    target_release_identity: &str,
) -> Result<String, String> {
    if requested_by != "codex_mcp" {
        return Err("更新重启只允许项目绑定的 Codex MCP 发起".to_string());
    }
    schedule_target_is_valid(target_release_identity)?;
    if RESTART_SCHEDULED.swap(true, Ordering::SeqCst) {
        return Err("本桌面进程已经安排更新重启，拒绝重复触发".to_string());
    }
    if let Err(error) = schedule_platform_guard(window, action_id, target_release_identity) {
        RESTART_SCHEDULED.store(false, Ordering::SeqCst);
        return Err(error);
    }
    Ok(format!(
        "已安排精确更新重启；目标版本 {target_release_identity}，桌面壳将在回执写回后优雅退出"
    ))
}

pub(super) fn schedule_target_is_valid(value: &str) -> Result<(), String> {
    let (version, git_sha) = value
        .trim()
        .rsplit_once('+')
        .ok_or("目标发布身份必须是 version+git_sha")?;
    let version_ok = !version.is_empty()
        && version.len() <= 48
        && version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'));
    let git_sha_ok =
        (40..=64).contains(&git_sha.len()) && git_sha.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !version_ok || !git_sha_ok {
        return Err("目标发布身份不是合法的一龙 Win 精确版本".to_string());
    }
    Ok(())
}

#[cfg(windows)]
fn schedule_platform_guard(
    window: &WebviewWindow,
    action_id: &str,
    target_release_identity: &str,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    let current_exe =
        std::env::current_exe().map_err(|error| format!("无法定位一龙桌面壳: {error}"))?;
    let internal_dir = current_exe
        .parent()
        .ok_or("一龙桌面壳缺少 _internal 父目录")?;
    if !internal_dir
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("_internal"))
    {
        return Err("更新重启只允许正式安装目录中的一龙桌面壳执行".to_string());
    }
    let install_dir = internal_dir.parent().ok_or("一龙桌面壳缺少安装根目录")?;
    let client_path = install_dir.join("一龙开发平台.exe");
    if !client_path.is_file() {
        return Err("正式安装目录缺少一龙开发平台更新入口".to_string());
    }
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .ok_or("无法读取 LOCALAPPDATA，不能创建独立更新守卫")?;
    let guard_root = local_app_data
        .join("Elon")
        .join("desktop-update-restart-v1");
    std::fs::create_dir_all(&guard_root)
        .map_err(|error| format!("无法创建更新守卫目录: {error}"))?;
    let script_path = guard_root.join(format!("guard-{}-{}.ps1", std::process::id(), now_ms()));
    std::fs::write(&script_path, GUARD_SCRIPT.as_bytes())
        .map_err(|error| format!("无法写入更新守卫脚本: {error}"))?;

    let mut command = std::process::Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .arg("-DesktopPid")
        .arg(std::process::id().to_string())
        .arg("-ClientPath")
        .arg(&client_path)
        .arg("-ExpectedReleaseIdentity")
        .arg(target_release_identity)
        .arg("-ActionId")
        .arg(action_id)
        .current_dir(install_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    command
        .spawn()
        .map_err(|error| format!("无法启动独立更新重启守卫: {error}"))?;

    let _ = window.eval("window.dispatchEvent(new CustomEvent('elon:update-restart-scheduled'));");
    let app = window.app_handle().clone();
    std::thread::spawn(move || {
        std::thread::sleep(EXIT_DELAY);
        app.exit(0);
    });
    Ok(())
}

#[cfg(not(windows))]
fn schedule_platform_guard(
    _window: &WebviewWindow,
    _action_id: &str,
    _target_release_identity: &str,
) -> Result<(), String> {
    Err("更新重启编排只支持正式 Windows 安装端".to_string())
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_identity_is_exact_and_bounded() {
        assert!(schedule_target_is_valid(&format!("0.3.69+{}", "a".repeat(40))).is_ok());
        assert!(schedule_target_is_valid("latest").is_err());
        assert!(schedule_target_is_valid("0.3.69+short").is_err());
        assert!(schedule_target_is_valid(&format!("bad version+{}", "a".repeat(40))).is_err());
    }

    #[test]
    fn guard_waits_for_exact_health_before_reopening() {
        for marker in [
            "Wait-Process -Id $DesktopPid",
            "ELON_EXPECTED_UPDATE_RELEASE_IDENTITY",
            "--update",
            "release_identity",
            "ExpectedReleaseIdentity",
            "update.apply.lock",
            "Start-Process -FilePath $ClientPath",
        ] {
            assert!(
                GUARD_SCRIPT.contains(marker),
                "missing guard marker: {marker}"
            );
        }
    }
}
