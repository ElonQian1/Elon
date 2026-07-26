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

#[derive(Debug)]
enum UpdateDeferred {
    DesktopInUse,
    ActiveForeground { wait_secs: u64, blockers: String },
}

impl std::fmt::Display for UpdateDeferred {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DesktopInUse => formatter.write_str(
                "一龙桌面工作台正在使用；本次只保留已验证下载与恢复审计，后台稍后重试，不关闭当前窗口",
            ),
            Self::ActiveForeground {
                wait_secs,
                blockers,
            } => write!(
                formatter,
                "节点更新等待 {wait_secs} 秒后结束本次尝试；以下任务仍无完整恢复检查点：{blockers}。保持旧 runtime 运行，后台稍后重试"
            ),
        }
    }
}

impl std::error::Error for UpdateDeferred {}

pub(crate) fn update_client_if_needed(install_dir: &Path) -> Result<bool> {
    match try_update_client_if_needed(install_dir) {
        Ok(scheduled_restart) => Ok(scheduled_restart),
        Err(error) if error.downcast_ref::<UpdateDeferred>().is_some() => {
            super::log_file::record_event(
                install_dir,
                "auto_update_deferred",
                true,
                &format!("{error:#}"),
            );
            Ok(false)
        }
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

#[path = "updater_checkpoint_gate.rs"]
mod checkpoint_gate;

#[path = "updater_runtime_gate.rs"]
mod runtime_gate;

#[path = "updater_singleflight.rs"]
mod singleflight;
pub(crate) use self::singleflight::{
    ensure_background_update, run_update_owner, try_acquire_apply_lock,
};

#[path = "update_compat.rs"]
mod update_compat;
