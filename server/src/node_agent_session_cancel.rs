//! Fail-closed handling for server Cancel messages.

use anyhow::{Context, Result};

use homecli_proto::CancelRequestAudit;

use crate::NodeRuntime;

pub(crate) async fn apply(
    runtime: &NodeRuntime,
    task_id: &str,
    audit: &CancelRequestAudit,
) -> Result<bool> {
    // Durability comes first: if persistence fails, the session is closed and
    // the server cannot assume this Cancel fenced a later Prompt.
    runtime
        .task_journal
        .record_prestart_cancel_tombstone(task_id)
        .with_context(|| format!("持久化 CLI 启动前取消墓碑失败: {task_id}"))?;
    Ok(runtime.cancel_cli_prompt_with_audit(task_id, audit).await)
}
