use std::sync::Arc;

use crate::{store::NodeComputeRun, types::AppState};

pub(super) async fn cleanup_failed_credential_delivery(
    state: &Arc<AppState>,
    lease_id: &str,
    consumer_user_id: &str,
    consumer_node_id: &str,
    midrun: Option<&NodeComputeRun>,
    reason: &str,
) {
    match state
        .store
        .clear_codex_vault_emergency_lease_for_node_with_cancel_targets(
            consumer_user_id,
            consumer_node_id,
            Some(lease_id),
        ) {
        Ok(Some(issue)) => {
            for (node_id, req_id) in issue.cancel_targets {
                if !state
                    .agent_manager
                    .cancel_cli_prompt_on_agent(&node_id, &req_id)
                    .await
                {
                    tracing::warn!(
                        %node_id,
                        %req_id,
                        %lease_id,
                        "共享凭据返回失败，关联运行取消消息未送达"
                    );
                }
            }
        }
        Ok(None) => {}
        Err(error) => tracing::error!(
            %lease_id,
            %error,
            "共享凭据返回失败，租约清理也失败"
        ),
    }
    if let Some(run) = midrun {
        if let Err(error) = state
            .store
            .release_dispatch_billing_hold_before_send(&run.consumer_user_id, &run.compute_call_id)
        {
            tracing::error!(
                compute_call_id = %run.compute_call_id,
                %error,
                "共享凭据从未返回，但 dispatch hold 释放失败"
            );
        }
    }
    let _ = state.store.record_codex_vault_event(
        consumer_user_id,
        "emergency_credential_delivery_rejected",
        Some(consumer_node_id),
        false,
        Some(reason),
    );
}
