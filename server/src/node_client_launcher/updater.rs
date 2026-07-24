// server/src/node_client_launcher/updater.rs

use anyhow::{Context, Result};
use std::{collections::HashMap, path::Path, time::Duration};

pub(crate) use super::log_file;
use super::{
    command as launcher_command, env_file, paths, process,
    update_integrity::{preferred_sha256, read_local_git_sha, verify_optional_sha256, VersionInfo},
    watchdog, DEFAULT_BASE_URL,
};

const DEFAULT_UPDATE_CONNECT_TIMEOUT_SECS: u64 = 20;
const DEFAULT_UPDATE_DOWNLOAD_TIMEOUT_SECS: u64 = 15 * 60;
const DEFAULT_UPDATE_DOWNLOAD_RETRIES: usize = 3;
const DEFAULT_UPDATE_DEFER_MAX_SECS: u64 = 90;

pub(crate) fn update_client_if_needed(install_dir: &Path) -> Result<bool> {
    match try_update_client_if_needed(install_dir) {
        Ok(scheduled_restart) => Ok(scheduled_restart),
        Err(error) => {
            super::log_file::record_event(
                install_dir,
                "auto_update_failed",
                false,
                &format!("{error:#}"),
            );
            eprintln!("自动更新检查失败，继续使用本地版本: {error:#}");
            Ok(false)
        }
    }
}

#[path = "updater_impl.rs"]
mod updater_impl;
use self::updater_impl::*;

#[path = "updater_runtime_gate.rs"]
mod runtime_gate;

#[path = "update_compat.rs"]
mod update_compat;
