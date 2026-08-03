use serde::Serialize;

pub(crate) const COMPUTE_PLUGIN_HOST_SCHEMA: &str = "elon.compute-plugin-host.v1";
pub(crate) const COMPUTE_TASK_KIND_LLM_CHAT_V1: &str = "llm_chat.v1";
pub(crate) const COMPUTE_PLUGIN_MODE_IN_PROCESS_LEGACY: &str = "in_process_legacy";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputePluginDescriptor {
    pub schema: &'static str,
    pub plugin_id: &'static str,
    pub task_kinds: Vec<&'static str>,
    pub mode: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct LlmChatTask {
    pub req_id: String,
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub(crate) enum ComputePluginTask {
    LlmChatV1(LlmChatTask),
}

impl ComputePluginTask {
    pub(crate) fn task_kind(&self) -> &'static str {
        match self {
            Self::LlmChatV1(_) => COMPUTE_TASK_KIND_LLM_CHAT_V1,
        }
    }
}
