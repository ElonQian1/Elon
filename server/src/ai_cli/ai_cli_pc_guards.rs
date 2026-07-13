use super::{ai_cli_pc_execution::record_pc_execution_finished, NativeSessionScope};
use crate::{homecli_agent::CliPromptCancelHandle, types::AppState};
use std::sync::Arc;

pub(super) struct PcCliCancelOnDrop {
    handle: Option<CliPromptCancelHandle>,
}

impl PcCliCancelOnDrop {
    pub(super) fn armed(handle: CliPromptCancelHandle) -> Self {
        Self {
            handle: Some(handle),
        }
    }

    pub(super) fn disarm(&mut self) {
        self.handle = None;
    }
}

impl Drop for PcCliCancelOnDrop {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let sent = handle.cancel();
            tracing::info!(
                req_id = handle.req_id(),
                sent,
                "PC CLI task dropped; sent cancel to agent"
            );
        }
    }
}

pub(super) struct PcExecutionFinishOnDrop {
    state: Option<Arc<AppState>>,
    scope: Option<NativeSessionScope>,
    node_id: String,
    request_id: String,
    model: Option<String>,
}

impl PcExecutionFinishOnDrop {
    pub(super) fn armed(
        state: Arc<AppState>,
        scope: Option<NativeSessionScope>,
        node_id: String,
        request_id: String,
        model: Option<String>,
    ) -> Self {
        Self {
            state: scope.as_ref().map(|_| state),
            scope,
            node_id,
            request_id,
            model,
        }
    }

    pub(super) fn disarm(&mut self) {
        self.state = None;
        self.scope = None;
    }
}

impl Drop for PcExecutionFinishOnDrop {
    fn drop(&mut self) {
        let (Some(state), Some(scope)) = (self.state.as_ref(), self.scope.as_ref()) else {
            return;
        };
        record_pc_execution_finished(
            state.as_ref(),
            Some(scope),
            &self.node_id,
            &self.request_id,
            false,
            Some("PC CLI 请求在收到终态前被取消或连接断开"),
            self.model.as_deref(),
            None,
            None,
            None,
        );
    }
}
