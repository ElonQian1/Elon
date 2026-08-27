use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiWebSessionState {
    pub provider_id: String,
    pub window_label: String,
    pub window_status: String,
    pub window_visible: bool,
    pub current_url: String,
    pub current_host: String,
    pub loading: bool,
    pub renderer_status: String,
    pub last_error: Option<String>,
    pub last_error_code: Option<String>,
    pub semantic_event: Option<Value>,
    pub navigation_event: Option<Value>,
    pub composer_event: Option<Value>,
    pub feature_event: Option<Value>,
    pub ui_manifest_event: Option<Value>,
    pub command_result: Option<Value>,
    pub command_results: Vec<Value>,
    pub diagnostics: Value,
    pub cache_status: String,
    pub semantic_cache_status: String,
    pub navigation_cache_status: String,
    pub local_conversations: Vec<LocalAiCachedConversation>,
    pub active_conversation_id: Option<String>,
    pub semantic_conversation_aligned: bool,
    pub context_ready: bool,
    pub context_status: String,
    pub cache_updated_at_ms: u64,
    pub navigation_updated_at_ms: u64,
    pub semantic_updated_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiCachedConversation {
    pub id: String,
    pub title: String,
    pub active: bool,
    pub updated_at_ms: u64,
}
