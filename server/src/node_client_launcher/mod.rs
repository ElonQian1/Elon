mod env_file;
mod installer;
mod paths;
mod process;
mod updater;
mod windows_integration;

use anyhow::Result;

pub(crate) const APP_NAME: &str = "一龙PC节点";
/// Canonical user-facing Windows entry. It contains both the launcher and the
/// background node runtime; no separate agent exe is shipped on Windows.
pub(crate) const CLIENT_EXE_NAME: &str = "一龙PC节点.exe";
pub(crate) const UNINSTALL_NAME: &str = "卸载一龙PC节点";
pub(crate) const UNINSTALL_EXE_NAME: &str = "卸载一龙PC节点.exe";
pub(crate) const INTERNAL_DIR_NAME: &str = "_internal";
pub(crate) const AGENT_RUNTIME_ARG: &str = "--agent-runtime";
pub(crate) const DEFAULT_BASE_URL: &str = "http://43.139.149.158:8080";
pub(crate) const DEFAULT_ADMIN_PORT: u16 = 7799;

enum ClientCommand {
    Start,
    Install,
    Uninstall,
    Update,
}

pub(crate) fn run() -> Result<()> {
    let command = ClientCommand::from_env();
    match command {
        ClientCommand::Start => {
            let install_dir = installer::ensure_installed()?;
            if updater::update_client_if_needed(&install_dir)? {
                return Ok(());
            }
            process::start_or_open(&install_dir)?;
        }
        ClientCommand::Install => {
            let install_dir = installer::install_or_repair()?;
            if updater::update_client_if_needed(&install_dir)? {
                return Ok(());
            }
            process::launch_installed_client(&install_dir)?;
        }
        ClientCommand::Uninstall => installer::uninstall()?,
        ClientCommand::Update => {
            let install_dir = paths::install_dir()?;
            let _ = updater::update_client_if_needed(&install_dir)?;
        }
    }
    Ok(())
}

impl ClientCommand {
    fn from_env() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.iter().any(|arg| arg == "--uninstall") || exe_stem_contains(UNINSTALL_NAME) {
            return Self::Uninstall;
        }
        if args.iter().any(|arg| arg == "--install") {
            return Self::Install;
        }
        if args.iter().any(|arg| arg == "--update") {
            return Self::Update;
        }
        Self::Start
    }
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
