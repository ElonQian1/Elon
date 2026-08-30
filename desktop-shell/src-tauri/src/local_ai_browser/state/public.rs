use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

use super::{diagnostics, private_stream, SessionRecord};

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
    pub composer_events: BTreeMap<String, Value>,
    pub feature_event: Option<Value>,
    pub interaction_live: bool,
    pub interaction_updated_at_ms: u64,
    pub ui_manifest_event: Option<Value>,
    pub realtime_voice_event: Option<Value>,
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

impl From<SessionRecord> for LocalAiWebSessionState {
    fn from(record: SessionRecord) -> Self {
        let cache_status = record.cache_status().to_string();
        let semantic_cache_status = record
            .event_cache_status(record.semantic_event.is_some(), record.semantic_live)
            .to_string();
        let navigation_cache_status = record
            .event_cache_status(record.navigation_event.is_some(), record.navigation_live)
            .to_string();
        let diagnostics = diagnostic_summary(&record);
        let context_ready = record.context_ready();
        let context_status = record.context_binding_status().to_string();
        let semantic_conversation_aligned = record.active_conversation_id.is_none()
            || record.semantic_conversation_id.is_none()
            || record.active_conversation_id == record.semantic_conversation_id;
        let local_conversations = record
            .conversation_snapshots
            .iter()
            .map(|entry| LocalAiCachedConversation {
                id: entry.id.clone(),
                title: entry.title.clone(),
                active: record.active_conversation_id.as_deref() == Some(entry.id.as_str()),
                updated_at_ms: entry.updated_at_ms,
            })
            .collect();
        Self {
            provider_id: record.provider_id,
            window_label: record.window_label,
            window_status: record.window_status,
            window_visible: record.window_visible,
            current_url: record.current_url,
            current_host: record.current_host,
            loading: record.loading,
            renderer_status: record.renderer_status,
            last_error: record.last_error,
            last_error_code: record.last_error_code,
            semantic_event: record.semantic_event,
            navigation_event: record.navigation_event,
            composer_event: record.composer_event,
            composer_events: record.composer_events,
            feature_event: record.feature_event,
            interaction_live: record.interaction_live,
            interaction_updated_at_ms: record.interaction_updated_at_ms,
            ui_manifest_event: record.ui_manifest_event,
            realtime_voice_event: record.realtime_voice_event,
            command_result: record.command_result,
            command_results: record.command_results,
            diagnostics,
            cache_status,
            semantic_cache_status,
            navigation_cache_status,
            local_conversations,
            active_conversation_id: record.active_conversation_id,
            semantic_conversation_aligned,
            context_ready,
            context_status,
            cache_updated_at_ms: record.cache_updated_at_ms,
            navigation_updated_at_ms: record.navigation_updated_at_ms,
            semantic_updated_at_ms: record.semantic_updated_at_ms,
            updated_at_ms: record.updated_at_ms,
        }
    }
}

fn diagnostic_summary(record: &SessionRecord) -> Value {
    let coverage = diagnostics::content_coverage(record.semantic_event.as_ref());
    serde_json::json!({
        "lastEventKind": record.last_event_kind,
        "lastCommandAction": record.last_command_action,
        "lastCommandRequestId": record.last_command_request_id,
        "lastCommandOk": record.last_command_ok,
        "messageCount": record.message_count,
        "assistantMessageCount": record.assistant_message_count,
        "contentPartCounts": coverage.part_counts,
        "richCardKindCounts": coverage.rich_kind_counts,
        "citationCount": coverage.citation_count,
        "linkedCitationCount": coverage.linked_citation_count,
        "citationLogoCount": coverage.citation_logo_count,
        "streaming": record.streaming,
        "privateStreamObserved": private_stream::observed(record.semantic_event.as_ref()),
        "privateStreamRevision": private_stream::revision(record.semantic_event.as_ref()),
        "privateStreamState": private_stream::state(record.semantic_event.as_ref()),
        "privateRichRecovery": diagnostics::private_rich_recovery(record.semantic_event.as_ref()),
        "semanticUpdatedAtMs": record.semantic_updated_at_ms,
        "updatedAtMs": record.updated_at_ms,
    })
}
