use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub(crate) struct LmChatRequest {
    /// OpenAI 格式的消息数组。
    pub(crate) messages: Vec<Value>,
    pub(crate) agent: Option<String>,
    pub(crate) conversation_id: Option<String>,
    pub(crate) conversation_title: Option<String>,
    pub(crate) scope: Option<String>,
    #[serde(
        default,
        alias = "runtimeRoute",
        alias = "pcRuntimeRoute",
        alias = "pc_runtime_route"
    )]
    pub(crate) runtime_route: Option<String>,
}
