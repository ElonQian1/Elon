//! Bounded install gating after a verified update has been downloaded.

use super::*;

pub(super) fn wait_for_safe_update_checkpoint(
    install_dir: &Path,
    version_file: &Path,
    remote_text: &str,
) -> Result<()> {
    let max_wait_secs = env_u64(
        "NODE_AGENT_UPDATE_DEFER_MAX_SECS",
        DEFAULT_UPDATE_DEFER_MAX_SECS,
        30,
        60 * 60,
    );
    let started = std::time::Instant::now();
    loop {
        let fresh_runtime_handles = runtime_gate::fresh_runtime_handle_task_ids(install_dir)?;
        let decision = crate::node_agent_update_checkpoint::checkpoint_downloaded_update(
            version_file,
            remote_text,
            &fresh_runtime_handles,
        )?;
        if decision.install_may_proceed() {
            if runtime_gate::desktop_shell_running(install_dir)? {
                crate::node_agent_update_recovery::UpdateRecoveryStore::default()
                    .set_install_gate_phase(
                        "deferred_desktop_in_use",
                        Some("desktop shell is in use; keep it online and retry in background"),
                    )?;
                return Err(UpdateDeferred::DesktopInUse.into());
            }
            return Ok(());
        }
        log_file::record_event(
            install_dir,
            "update_deferred_active_foreground",
            true,
            &format!(
                "等待 {} 个无安全 checkpoint 的前台任务结束后再安装更新",
                decision
                    .active_foreground_task_ids
                    .len()
                    .saturating_sub(decision.checkpointed_task_ids.len())
            ),
        );
        if started.elapsed() >= Duration::from_secs(max_wait_secs) {
            let blockers = decision
                .active_foreground_task_ids
                .iter()
                .filter(|task_id| !decision.checkpointed_task_ids.contains(task_id))
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(",");
            return Err(UpdateDeferred::ActiveForeground {
                wait_secs: max_wait_secs,
                blockers,
            }
            .into());
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}
