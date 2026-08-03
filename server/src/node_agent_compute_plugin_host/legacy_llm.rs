use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::{node_agent_config::NodeConfig, node_agent_local_llm::run_llm_inference};

use super::contract::LlmChatTask;

pub(super) const LEGACY_LOCAL_LLM_PLUGIN_ID: &str = "builtin.legacy-local-llm.v1";

#[derive(Clone)]
pub(super) struct LegacyLocalLlmAdapter {
    cfg: NodeConfig,
}

impl LegacyLocalLlmAdapter {
    pub(super) fn new(cfg: NodeConfig) -> Self {
        Self { cfg }
    }

    pub(super) fn spawn(&self, task: LlmChatTask, wire_sink: mpsc::UnboundedSender<Message>) {
        let cfg = self.cfg.clone();
        tokio::spawn(async move {
            let LlmChatTask {
                req_id,
                model,
                messages,
                max_tokens,
            } = task;
            run_llm_inference(&cfg, req_id, &model, messages, max_tokens, wire_sink).await;
        });
    }
}
