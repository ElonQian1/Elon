// server/src/node_client_launcher/mod.rs

pub(crate) mod command;
mod env_file;
mod fallback_page;
mod installer;
pub(crate) mod log_file;
mod maintenance_protocol;
mod paths;
mod process;
mod updater;
mod watchdog;
mod windows_integration;

use anyhow::Result;
#[cfg(windows)]
use std::path::Path;

pub(crate) const APP_NAME: &str = "一龙开发平台";
/// Canonical user-facing Windows entry. It contains both the launcher and the
/// background node runtime; no separate agent exe is shipped on Windows.
pub(crate) const CLIENT_EXE_NAME: &str = "一龙开发平台.exe";
pub(crate) const UNINSTALL_NAME: &str = "卸载一龙开发平台";
pub(crate) const UNINSTALL_EXE_NAME: &str = "卸载一龙开发平台.exe";
pub(crate) const INTERNAL_DIR_NAME: &str = "_internal";
pub(crate) const AGENT_RUNTIME_ARG: &str = "--agent-runtime";
pub(crate) const BACKGROUND_START_ARG: &str = "--background";
pub(crate) const WATCHDOG_ARG: &str = "--watchdog";
pub(crate) const DEFAULT_BASE_URL: &str = "http://43.139.149.158:8080";
pub(crate) const DEFAULT_ADMIN_PORT: u16 = 7799;
pub(crate) const AUTOSTART_RUN_VALUE_NAME: &str = windows_integration::RUN_VALUE_NAME;

#[derive(Clone, Copy)]
enum ClientCommand {
    Start,
    BackgroundStart,
    Watchdog,
    Install,
    Uninstall,
    Update,
    ExportDiagnostics,
    OpenMaintenance(maintenance_protocol::MaintenanceProtocolTarget),
}

pub(crate) fn run() -> Result<()> {
    let command = ClientCommand::from_env();
    let install_dir = paths::install_dir().ok();
    if let Some(install_dir) = install_dir.as_deref() {
        log_file::record_event(
            install_dir,
            "launcher_command_started",
            true,
            command.as_str(),
        );
    }
    let result = run_command(command);
    if let Some(install_dir) = install_dir.as_deref() {
        match &result {
            Ok(()) => log_file::record_event(
                install_dir,
                "launcher_command_finished",
                true,
                command.as_str(),
            ),
            Err(error) => log_file::record_event(
                install_dir,
                "launcher_command_finished",
                false,
                &format!("{}: {error:#}", command.as_str()),
            ),
        }
    }
    result
}

#[cfg(windows)]
pub(crate) fn set_autostart_enabled(install_dir: &Path, enabled: bool) -> Result<()> {
    if enabled {
        windows_integration::enable_autostart(install_dir)
    } else {
        windows_integration::disable_autostart();
        Ok(())
    }
}

#[cfg(not(windows))]
pub(crate) fn set_autostart_enabled(_install_dir: &std::path::Path, _enabled: bool) -> Result<()> {
    anyhow::bail!("当前平台不支持 Windows 开机自启动设置")
}

fn run_command(command: ClientCommand) -> Result<()> {
    match command {
        ClientCommand::Start => {
            let install_dir = installer::ensure_installed()?;
            if updater::update_client_if_needed(&install_dir)? {
                // 自动更新脚本会重启后台 runtime；不要在这里主动打开浏览器，
                // 避免已有 /pc 工作页重连时又被插入一个重复 tab。
                return Ok(());
            }
            let start_result = process::start_or_open(&install_dir);
            let watchdog_result = watchdog::ensure_running(&install_dir);
            start_result?;
            watchdog_result?;
        }
        ClientCommand::BackgroundStart => {
            let install_dir = installer::ensure_installed()?;
            if updater::update_client_if_needed(&install_dir)? {
                return Ok(());
            }
            let start_result = process::start_background(&install_dir);
            let watchdog_result = watchdog::ensure_running(&install_dir);
            start_result?;
            watchdog_result?;
        }
        ClientCommand::Watchdog => {
            let install_dir = paths::install_dir()?;
            watchdog::run_loop(&install_dir)?;
        }
        ClientCommand::Install => {
            let install_dir = installer::install_or_repair()?;
            if updater::update_client_if_needed(&install_dir)? {
                // 旧安装包触发自更新时保持浏览器不动，更新脚本负责重启 runtime。
                return Ok(());
            }
            let launch_result = process::launch_installed_client(&install_dir);
            let watchdog_result = watchdog::ensure_running(&install_dir);
            launch_result?;
            watchdog_result?;
        }
        ClientCommand::Uninstall => installer::uninstall()?,
        ClientCommand::Update => {
            let install_dir = paths::install_dir()?;
            let _ = updater::update_client_if_needed(&install_dir)?;
        }
        ClientCommand::ExportDiagnostics => {
            crate::node_agent_client_diagnostics::export_diagnostics_file()
                .map_err(anyhow::Error::msg)?;
        }
        ClientCommand::OpenMaintenance(target) => {
            let install_dir = paths::install_dir()?;
            maintenance_protocol::open_target(target, &install_dir)?;
        }
    }
    Ok(())
}

impl ClientCommand {
    fn from_env() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Self::from_args(&args, exe_stem_contains(UNINSTALL_NAME))
    }

    fn from_args(args: &[String], uninstall_exe: bool) -> Self {
        if args.iter().any(|arg| arg == "--uninstall") || uninstall_exe {
            return Self::Uninstall;
        }
        if args.iter().any(|arg| arg == WATCHDOG_ARG) {
            return Self::Watchdog;
        }
        if args
            .iter()
            .any(|arg| arg == BACKGROUND_START_ARG || arg == "--autostart")
        {
            return Self::BackgroundStart;
        }
        if args
            .iter()
            .any(|arg| arg == "--install" || arg == "--repair")
            || repair_requested(args)
        {
            return Self::Install;
        }
        if args
            .iter()
            .any(|arg| arg == "--update" || arg == "--check-update")
        {
            return Self::Update;
        }
        if diagnostics_export_requested(args) {
            return Self::ExportDiagnostics;
        }
        if let Some(target) = maintenance_protocol::target_from_args(&args) {
            return Self::OpenMaintenance(target);
        }
        if maintenance_protocol::protocol_start_requested(&args) {
            return Self::Start;
        }
        Self::Start
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::BackgroundStart => "background_start",
            Self::Watchdog => "watchdog",
            Self::Install => "install",
            Self::Uninstall => "uninstall",
            Self::Update => "update",
            Self::ExportDiagnostics => "export_diagnostics",
            Self::OpenMaintenance(target) => target.action_name(),
        }
    }
}

fn diagnostics_export_requested(args: &[String]) -> bool {
    args.iter().any(|arg| {
        let value = arg.trim().trim_matches('"').to_ascii_lowercase();
        matches!(
            value.as_str(),
            "--export-diagnostics" | "--diagnostics-export" | "--support-bundle"
        ) || matches!(
            value.as_str(),
            "elon-node://diagnostics/export"
                | "elon-node://maintenance/diagnostics/export"
                | "elon-node://support-bundle"
        )
    })
}

fn repair_requested(args: &[String]) -> bool {
    args.iter().any(|arg| {
        let value = arg.trim().trim_matches('"').to_ascii_lowercase();
        matches!(
            value.as_str(),
            "elon-node://repair"
                | "elon-node://maintenance/repair"
                | "elon-node://install"
                | "elon-node://maintenance/install"
        )
    })
}

fn exe_stem_contains(needle: &str) -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .map(|stem| stem.contains(needle))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{ClientCommand, UNINSTALL_NAME};
    use std::time::Duration;

    #[cfg(windows)]
    use std::path::Path;

    #[test]
    fn client_command_accepts_productized_maintenance_aliases() {
        assert!(matches!(
            ClientCommand::from_args(&["--check-update".to_string()], false),
            ClientCommand::Update
        ));
        assert!(matches!(
            ClientCommand::from_args(&["--background".to_string()], false),
            ClientCommand::BackgroundStart
        ));
        assert!(matches!(
            ClientCommand::from_args(&["--autostart".to_string()], false),
            ClientCommand::BackgroundStart
        ));
        assert!(matches!(
            ClientCommand::from_args(&["--watchdog".to_string()], false),
            ClientCommand::Watchdog
        ));
        assert!(matches!(
            ClientCommand::from_args(&["--repair".to_string()], false),
            ClientCommand::Install
        ));
        assert!(matches!(
            ClientCommand::from_args(&["elon-node://repair".to_string()], false),
            ClientCommand::Install
        ));
        assert!(matches!(
            ClientCommand::from_args(&["--export-diagnostics".to_string()], false),
            ClientCommand::ExportDiagnostics
        ));
        assert!(matches!(
            ClientCommand::from_args(&["--support-bundle".to_string()], false),
            ClientCommand::ExportDiagnostics
        ));
        assert!(matches!(
            ClientCommand::from_args(&["elon-node://diagnostics/export".to_string()], false),
            ClientCommand::ExportDiagnostics
        ));
    }

    #[test]
    fn uninstall_exe_name_still_routes_to_uninstall() {
        assert!(matches!(
            ClientCommand::from_args(&[UNINSTALL_NAME.to_string()], true),
            ClientCommand::Uninstall
        ));
    }

    #[test]
    fn scheduled_update_paths_do_not_open_browser_tabs() {
        let source = include_str!("mod.rs");
        let removed_open_helper = ["open_installed", "pc_web_page"].join("_");

        assert!(!source.contains(&removed_open_helper));
        assert!(source.contains("避免已有 /pc 工作页重连时又被插入一个重复 tab"));
    }

    #[test]
    fn watchdog_restart_requires_consecutive_failures() {
        assert!(!super::watchdog::should_restart(0, 3));
        assert!(!super::watchdog::should_restart(2, 3));
        assert!(super::watchdog::should_restart(3, 3));
    }

    #[test]
    fn watchdog_interval_env_is_clamped() {
        assert_eq!(
            super::watchdog::watchdog_interval_from(Some("1")),
            Duration::from_secs(5)
        );
        assert_eq!(
            super::watchdog::watchdog_interval_from(Some("9999")),
            Duration::from_secs(300)
        );
        assert_eq!(
            super::watchdog::watchdog_interval_from(Some("not-a-number")),
            Duration::from_secs(15)
        );
    }

    #[cfg(windows)]
    #[test]
    fn watchdog_query_matches_same_client_and_skips_current_pid() {
        let script = super::watchdog::watchdog_query_script(
            Path::new(r"C:\ElonNode\一龙开发平台.exe"),
            1234,
        );

        assert!(script.contains("--watchdog"));
        assert!(script.contains("ProcessId -ne"));
        assert!(script.contains("一龙开发平台.exe"));
    }

    #[cfg(windows)]
    #[test]
    fn watchdog_stop_terminates_only_other_watchdog_processes() {
        let script =
            super::watchdog::watchdog_stop_script(Path::new(r"C:\ElonNode\一龙开发平台.exe"), 1234);

        assert!(script.contains("--watchdog"));
        assert!(script.contains("ProcessId -ne"));
        assert!(script.contains("Terminate"));
        assert!(!script.contains("--agent-runtime"));
    }

    #[test]
    fn watchdog_uses_client_exe_identity() {
        assert_eq!(super::CLIENT_EXE_NAME, "一龙开发平台.exe");
    }
}
