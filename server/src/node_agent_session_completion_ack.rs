//! ACK handling that cannot bypass durable local terminal reconciliation.

use anyhow::{anyhow, Context, Result};
use homecli_proto::CliCompletionProducerIdentity;

use crate::{node_agent_local_terminal_reconcile::LocalTerminalReconciler, NodeRuntime};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply(
    runtime: &NodeRuntime,
    authenticated_producer: &CliCompletionProducerIdentity,
    event_id: &str,
    req_id: &str,
    accepted: bool,
    retryable: bool,
    error: Option<&str>,
) -> Result<()> {
    let completion = runtime
        .completion_outbox
        .completion_for_binding(event_id, req_id)
        .context("读取 CLI completion ACK 绑定")?
        .ok_or_else(|| anyhow!("未知或不匹配的 CLI completion ACK binding"))?;
    if completion.producer_identity.as_ref() != Some(authenticated_producer) {
        return Err(anyhow!("CLI completion ACK 不属于当前登录/节点/安装身份"));
    }
    if completion.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN {
        LocalTerminalReconciler::from_runtime(runtime)
            .reconcile(&completion)
            .await?;
        let display_updated = if accepted {
            runtime.local_tasks.mark_synced(event_id)
        } else {
            runtime.local_tasks.mark_sync_error(event_id, retryable)
        }?;
        anyhow::ensure!(
            display_updated,
            "本机任务尚未绑定 completion event，保留 outbox 等待重试"
        );
    }

    if accepted {
        anyhow::ensure!(
            runtime.completion_outbox.acknowledge(event_id, req_id)?,
            "CLI completion ACK binding 在迁移前消失"
        );
        anyhow::ensure!(
            runtime.completion_outbox.delete_acked(event_id)?,
            "已确认 CLI completion 未能安全清理"
        );
    } else {
        anyhow::ensure!(
            runtime.completion_outbox.reject(
                event_id,
                req_id,
                retryable,
                error.unwrap_or("服务器拒绝 CLI completion 补传"),
            )?,
            "CLI completion rejection binding 在迁移前消失"
        );
    }
    Ok(())
}
