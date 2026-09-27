use serde_json::Value;

use super::{now_ms, semantic_context, snapshot_cache, LocalAiBrowserRuntime, SessionRecord};

impl LocalAiBrowserRuntime {
    pub fn activate_cached_conversation(&self, label: &str, id: &str) -> Option<String> {
        let mut sessions = self.sessions();
        let record = sessions.get_mut(label)?;
        let cached = record
            .conversation_snapshots
            .iter()
            .find(|entry| {
                entry.id == id
                    || semantic_context::page_context_key(
                        &record.provider_id,
                        &entry.restorable_url,
                    )
                    .as_deref()
                        == Some(id)
            })
            .cloned()?;
        record.semantic_event = Some(cached.semantic_event);
        record.active_restorable_url = Some(cached.restorable_url.clone());
        record.semantic_conversation_id = Some(cached.id.clone());
        record.semantic_page_context_key =
            semantic_context::page_context_key(&record.provider_id, &cached.restorable_url);
        record.begin_cached_conversation(cached.id, &cached.restorable_url);
        record.semantic_live = false;
        record.streaming = false;
        record.last_event_kind = "cached_conversation_activated".to_string();
        record.updated_at_ms = now_ms();
        Some(cached.restorable_url)
    }
}

impl SessionRecord {
    pub(super) fn remember_current_conversation(&mut self) {
        let Some(event) = self.semantic_event.as_ref() else {
            return;
        };
        if event.get("streaming").and_then(Value::as_bool) == Some(true)
            || event
                .get("messages")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
        {
            return;
        }
        let Some(restorable_url) = self.active_restorable_url.clone() else {
            return;
        };
        let title = conversation_title(event, &self.provider_id);
        let Some(id) = self.active_conversation_id.clone() else {
            return;
        };
        self.conversation_snapshots.retain(|entry| entry.id != id);
        self.conversation_snapshots.insert(
            0,
            snapshot_cache::StoredConversationSnapshot {
                id,
                title,
                restorable_url,
                semantic_event: event.clone(),
                updated_at_ms: now_ms(),
            },
        );
        self.conversation_snapshots.truncate(200);
    }

    pub(super) fn mark_snapshot_cached(&mut self) {
        if self.semantic_event.is_some() {
            self.semantic_live = false;
        }
        if self.navigation_event.is_some() {
            self.navigation_live = false;
        }
    }

    pub(super) fn cache_status(&self) -> &'static str {
        let has_semantic = self.semantic_event.is_some();
        let has_navigation = self.navigation_event.is_some();
        if !has_semantic && !has_navigation {
            return "empty";
        }
        if (!has_semantic || self.semantic_live) && (!has_navigation || self.navigation_live) {
            "live"
        } else {
            "cached"
        }
    }

    pub(super) fn event_cache_status(&self, present: bool, live: bool) -> &'static str {
        if !present {
            "empty"
        } else if live {
            "live"
        } else {
            "cached"
        }
    }
}

fn conversation_title(event: &Value, provider_id: &str) -> String {
    let explicit = event
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let first_user_text = event
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| {
            messages.iter().find_map(|message| {
                if message.get("role").and_then(Value::as_str) != Some("user") {
                    return None;
                }
                message
                    .get("content")
                    .and_then(Value::as_array)
                    .and_then(|parts| {
                        parts.iter().find_map(|part| {
                            matches!(
                                part.get("type").and_then(Value::as_str),
                                Some("text" | "markdown")
                            )
                            .then(|| part.get("text").and_then(Value::as_str))
                            .flatten()
                        })
                    })
            })
        })
        .unwrap_or_default();
    let fallback = if provider_id == "google-ai-mode" {
        "Google AI 会话"
    } else {
        "ChatGPT 会话"
    };
    let source = [explicit, first_user_text]
        .into_iter()
        .find(|value| !value.trim().is_empty())
        .unwrap_or(fallback);
    let title = source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(80)
        .collect::<String>();
    if title.is_empty() {
        fallback.to_string()
    } else {
        title
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn settled_partial_refresh_records_freshness_without_passive_updates_extending_it() {
        let runtime = LocalAiBrowserRuntime::default();
        runtime.ensure_session("directory", "chatgpt", "active");
        runtime.record_adapter_event(
            "directory",
            "conversation_snapshot",
            json!({
                "type":"conversation_snapshot", "conversations":[], "projects":[],
                "collection":{"complete":false,"refreshSettled":true,"continueRefresh":false}
            }),
        );
        let first = runtime
            .snapshot("directory")
            .unwrap()
            .navigation_updated_at_ms;
        assert!(first > 0);
        runtime.record_adapter_event(
            "directory",
            "conversation_snapshot",
            json!({
                "type":"conversation_snapshot", "conversations":[], "projects":[],
                "collection":{"complete":false,"refreshSettled":null}
            }),
        );
        let next = runtime.snapshot("directory").unwrap();
        assert_eq!(next.navigation_updated_at_ms, first);
        assert_eq!(
            next.navigation_event.unwrap()["collection"]["refreshSettled"],
            true
        );
    }
}
