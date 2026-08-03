use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::node_agent_config::NodeConfig;

use super::{
    contract::{
        ComputePluginDescriptor, ComputePluginTask, COMPUTE_PLUGIN_HOST_SCHEMA,
        COMPUTE_PLUGIN_MODE_IN_PROCESS_LEGACY, COMPUTE_TASK_KIND_LLM_CHAT_V1,
    },
    legacy_llm::{LegacyLocalLlmAdapter, LEGACY_LOCAL_LLM_PLUGIN_ID},
};

/// Node-internal execution seam. It does not advertise a new wire capability yet.
pub(crate) struct ComputePluginHost {
    descriptors: Vec<ComputePluginDescriptor>,
    legacy_local_llm: LegacyLocalLlmAdapter,
}

impl ComputePluginHost {
    pub(crate) fn new(cfg: NodeConfig) -> Self {
        Self {
            descriptors: vec![ComputePluginDescriptor {
                schema: COMPUTE_PLUGIN_HOST_SCHEMA,
                plugin_id: LEGACY_LOCAL_LLM_PLUGIN_ID,
                task_kinds: vec![COMPUTE_TASK_KIND_LLM_CHAT_V1],
                mode: COMPUTE_PLUGIN_MODE_IN_PROCESS_LEGACY,
            }],
            legacy_local_llm: LegacyLocalLlmAdapter::new(cfg),
        }
    }

    pub(crate) fn descriptor_count(&self) -> usize {
        self.descriptors.len()
    }

    /// Preserve the existing fire-and-stream behavior while routing through the Host seam.
    pub(crate) fn spawn(&self, task: ComputePluginTask, wire_sink: mpsc::UnboundedSender<Message>) {
        debug_assert_eq!(task.task_kind(), COMPUTE_TASK_KIND_LLM_CHAT_V1);
        match task {
            ComputePluginTask::LlmChatV1(task) => {
                self.legacy_local_llm.spawn(task, wire_sink);
            }
        }
    }
}
