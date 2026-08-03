pub(crate) const INTERNAL_COMPUTE_HOST_REVISION: u32 = 1;
pub(crate) const COMPUTE_TASK_KIND_LLM_CHAT: &str = "llm_chat";
pub(crate) const COMPUTE_PLUGIN_MODE_IN_PROCESS_LEGACY: &str = "in_process_legacy";

/// Internal runner registration only; this is not a downloadable plugin manifest or ready offer.
#[derive(Debug, Clone)]
pub(crate) struct InternalRunnerRegistration {
    pub host_revision: u32,
    pub runner_id: &'static str,
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
            Self::LlmChatV1(_) => COMPUTE_TASK_KIND_LLM_CHAT,
        }
    }
}
