use std::sync::Arc;

use crate::NodeRuntime;

use super::{
    classification, drain_admission_lock, load_checkpoint, now_ms, save_checkpoint, DRAIN_POLL_SECS,
};

pub(super) fn spawn_startup_checkpoint_reconciler(runtime: Arc<NodeRuntime>, update_id: String) {
    tokio::spawn(async move {
        loop {
            let Some(observed) = load_checkpoint().ok().flatten() else {
                return;
            };
            if observed.update_id != update_id || observed.state != "resume_required" {
                return;
            }
            let classification = match classification::classify_startup_checkpoint_tasks(
                runtime.as_ref(),
                &observed.active_task_ids,
            )
            .await
            {
                Ok(classification) => classification,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        %update_id,
                        "启动后重核更新恢复所有权失败，保留现有恢复状态"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(DRAIN_POLL_SECS)).await;
                    continue;
                }
            };
            let Ok(_admission) = drain_admission_lock().lock() else {
                tokio::time::sleep(std::time::Duration::from_secs(DRAIN_POLL_SECS)).await;
                continue;
            };
            let Ok(Some(mut current)) = load_checkpoint() else {
                return;
            };
            if current.update_id != update_id || current.state != "resume_required" {
                return;
            }
            current.active_task_ids = classification.blocking;
            current
                .recoverable_task_ids
                .extend(classification.recoverable);
            current.recoverable_task_ids.sort();
            current.recoverable_task_ids.dedup();
            if current.active_task_ids.is_empty() {
                current.transition(
                    "runtime_online",
                    "节点已恢复在线；无执行所有权的历史任务已保留为可恢复记录，不再阻塞当前更新。",
                );
                let _ = save_checkpoint(&current);
                return;
            }
            current.message =
                "仍有真实执行句柄、进程或新鲜恢复心跳；仅这些任务继续等待恢复。".to_string();
            current.updated_at_ms = now_ms();
            let _ = save_checkpoint(&current);
            drop(_admission);
            tokio::time::sleep(std::time::Duration::from_secs(DRAIN_POLL_SECS)).await;
        }
    });
}
