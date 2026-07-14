use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::{command as launcher_command, log_file};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MaintenanceProtocolTarget {
    Logs,
    LauncherLogs,
    TaskJournal,
    ConfigDir,
    DiagnosticsDir,
    InstallDir,
}

pub(crate) fn protocol_start_requested(args: &[String]) -> bool {
    args.iter().any(|arg| {
        protocol_action(arg)
            .as_deref()
            .map(|action| matches!(action, "open" | "start"))
            .unwrap_or(false)
    })
}

pub(crate) fn target_from_args(args: &[String]) -> Option<MaintenanceProtocolTarget> {
    args.iter()
        .find_map(|arg| target_from_flag(arg).or_else(|| target_from_protocol(arg)))
}

pub(crate) fn open_target(target: MaintenanceProtocolTarget, install_dir: &Path) -> Result<()> {
    let path = target_path(target, install_dir);
    if target.ensure_dir() {
        std::fs::create_dir_all(&path)
            .with_context(|| format!("create maintenance directory {}", path.display()))?;
    }
    open_path(&path).with_context(|| format!("open maintenance target {}", path.display()))
}

impl MaintenanceProtocolTarget {
    pub(crate) fn action_name(self) -> &'static str {
        match self {
            Self::Logs => "open_logs",
            Self::LauncherLogs => "open_launcher_logs",
            Self::TaskJournal => "open_task_journal",
            Self::ConfigDir => "open_config_dir",
            Self::DiagnosticsDir => "open_diagnostics_dir",
            Self::InstallDir => "open_install_dir",
        }
    }

    fn ensure_dir(self) -> bool {
        !matches!(self, Self::InstallDir)
    }
}

fn target_from_flag(arg: &str) -> Option<MaintenanceProtocolTarget> {
    match arg.trim().to_ascii_lowercase().as_str() {
        "--logs" | "--open-logs" => Some(MaintenanceProtocolTarget::Logs),
        "--launcher-logs" | "--open-launcher-logs" => Some(MaintenanceProtocolTarget::LauncherLogs),
        "--task-journal" | "--open-task-journal" => Some(MaintenanceProtocolTarget::TaskJournal),
        "--config-dir" | "--open-config" => Some(MaintenanceProtocolTarget::ConfigDir),
        "--diagnostics" | "--diagnostics-dir" | "--open-diagnostics" => {
            Some(MaintenanceProtocolTarget::DiagnosticsDir)
        }
        "--install-dir" | "--open-install-dir" => Some(MaintenanceProtocolTarget::InstallDir),
        _ => None,
    }
}

fn target_from_protocol(arg: &str) -> Option<MaintenanceProtocolTarget> {
    protocol_action(arg).and_then(|action| target_from_action(&action))
}

fn protocol_action(arg: &str) -> Option<String> {
    let value = arg.trim().trim_matches('"').to_ascii_lowercase();
    let rest = value.strip_prefix("elon-node://")?;
    let mut parts = rest
        .split(['/', '?', '#'])
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let first = parts.next()?;
    if matches!(first, "maintenance" | "open-target" | "open_target") {
        return parts.next().map(ToOwned::to_owned);
    }
    Some(first.to_string())
}

fn target_from_action(action: &str) -> Option<MaintenanceProtocolTarget> {
    match action.trim().to_ascii_lowercase().as_str() {
        "logs" | "logs-dir" | "logs_dir" | "client-logs" | "client_logs" => {
            Some(MaintenanceProtocolTarget::Logs)
        }
        "launcher-logs" | "launcher_logs" | "launcher-log" | "launcher_log" => {
            Some(MaintenanceProtocolTarget::LauncherLogs)
        }
        "task-journal" | "task_journal" | "tasks" => Some(MaintenanceProtocolTarget::TaskJournal),
        "config" | "config-dir" | "config_dir" => Some(MaintenanceProtocolTarget::ConfigDir),
        "diagnostics" | "diagnostics-dir" | "diagnostics_dir" => {
            Some(MaintenanceProtocolTarget::DiagnosticsDir)
        }
        "install-dir" | "install_dir" => Some(MaintenanceProtocolTarget::InstallDir),
        _ => None,
    }
}

fn target_path(target: MaintenanceProtocolTarget, install_dir: &Path) -> PathBuf {
    match target {
        MaintenanceProtocolTarget::Logs => config_dir().join("logs"),
        MaintenanceProtocolTarget::LauncherLogs => log_file::logs_dir(install_dir),
        MaintenanceProtocolTarget::TaskJournal => {
            crate::state_path().with_file_name("task-journal")
        }
        MaintenanceProtocolTarget::ConfigDir => config_dir(),
        MaintenanceProtocolTarget::DiagnosticsDir => config_dir().join("diagnostics"),
        MaintenanceProtocolTarget::InstallDir => install_dir.to_path_buf(),
    }
}

fn config_dir() -> PathBuf {
    crate::state_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn open_path(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let mut command = open_path_command(path);
        launcher_command::spawn_hidden(&mut command)?;
    }
    #[cfg(not(windows))]
    {
        let mut command = launcher_command::silent_command("xdg-open");
        command.arg(path);
        launcher_command::spawn_hidden(&mut command)?;
    }
    Ok(())
}

#[cfg(windows)]
fn open_path_command(path: &Path) -> std::process::Command {
    let mut command = launcher_command::silent_command("explorer.exe");
    command.arg(path);
    command
}

#[cfg(test)]
#[path = "maintenance_protocol_tests.rs"]
mod tests;
