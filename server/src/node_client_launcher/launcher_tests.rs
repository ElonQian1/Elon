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
        ClientCommand::from_args(&["--repair-background".to_string()], false),
        ClientCommand::InstallBackground
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

    assert!(source.contains("避免已有 /pc 工作页重连时又被插入一个重复 tab"));
    assert!(source.contains("ClientCommand::InstallBackground"));
    assert!(source.contains("let port = process::start_background(&install_dir)"));
    assert!(source.contains("process::verify_background_ready(port)"));
    assert!(source.contains("Rechecking updates here can recursively schedule a second"));
}

#[test]
fn runtime_start_checks_existing_autostart_marker() {
    let source = include_str!("mod.rs");

    assert!(source.contains("repair_autostart_on_runtime_start"));
    assert!(source.contains("repair_existing_autostart"));
    assert!(source.contains("checked existing autostart marker on runtime start"));
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
    let script =
        super::watchdog::watchdog_query_script(Path::new(r"C:\ElonNode\一龙开发平台.exe"), 1234);

    assert!(script.contains("--watchdog"));
    assert!(script.contains("ProcessId -ne"));
    assert!(script.contains("一龙开发平台.exe"));
}

#[cfg(windows)]
#[test]
fn watchdog_election_keeps_only_lowest_pid_for_same_client() {
    let script =
        super::watchdog::watchdog_election_script(Path::new(r"C:\ElonNode\一龙开发平台.exe"), 1234);

    assert!(script.contains("--watchdog"));
    assert!(script.contains("Sort-Object -Property ProcessId"));
    assert!(script.contains("winner.ProcessId"));
    assert!(script.contains("elected"));
    assert!(!script.contains("--agent-runtime"));
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

#[cfg(windows)]
#[test]
fn watchdog_detects_active_cli_sidecars_from_the_same_installation() {
    let script = super::watchdog::active_cli_sidecar_query_script(Path::new(
        r"C:\ElonNode\一龙开发平台.exe",
    ));

    assert!(script.contains("--cli-sidecar"));
    assert!(script.contains("一龙开发平台.exe"));
    assert!(script.contains("and $exeMatch"));
    assert!(!script.contains("--agent-runtime"));
    assert!(!script.contains("--watchdog"));
}

#[test]
fn watchdog_defers_restart_while_cli_sidecar_is_active() {
    let source = include_str!("watchdog.rs");

    assert!(source.contains("watchdog_restart_deferred_active_cli"));
    assert!(source.contains("active_cli_sidecar_running(install_dir)"));
    assert!(source.contains("state.consecutive_admin_failures = 0"));
}
