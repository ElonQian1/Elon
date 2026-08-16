use serde_json::Value;

use super::super::semantic_context;
use super::SessionRecord;

const BOUNDARY_ACTIONS: [&str; 4] = [
    "new_conversation",
    "open_conversation",
    "open_project",
    "open_cached_conversation",
];

impl SessionRecord {
    pub(super) fn begin_context_command(&mut self, action: &str, target: Option<&str>) {
        if action == "send_prompt" {
            self.preserve_conversation_on_navigation = true;
            return;
        }
        if !BOUNDARY_ACTIONS.contains(&action) {
            return;
        }
        self.pending_context_action = action.to_string();
        self.preserve_conversation_on_navigation = false;
        if action == "new_conversation" {
            self.active_conversation_id =
                Some(semantic_context::fresh_conversation_id(&self.provider_id));
            return;
        }
        if let Some(context_key) =
            target.and_then(|value| semantic_context::target_context_key(&self.provider_id, value))
        {
            self.active_page_context_key = Some(context_key.clone());
            self.active_conversation_id = Some(context_key);
        }
    }

    pub(super) fn begin_cached_conversation(&mut self, id: String, restorable_url: &str) {
        self.pending_context_action = "open_cached_conversation".to_string();
        self.preserve_conversation_on_navigation = false;
        self.active_conversation_id = Some(id);
        self.active_page_context_key =
            semantic_context::page_context_key(&self.provider_id, restorable_url);
    }

    pub(super) fn mark_context_navigation(&mut self, raw_url: &str) {
        let Some(context_key) = semantic_context::page_context_key(&self.provider_id, raw_url)
        else {
            return;
        };
        if self.pending_context_action == "new_conversation" {
            self.active_page_context_key = Some(context_key);
            return;
        }
        if !self.pending_context_action.is_empty() {
            self.active_page_context_key = Some(context_key.clone());
            if self.active_conversation_id.is_none() {
                self.active_conversation_id = Some(context_key);
            }
            return;
        }
        if self.preserve_conversation_on_navigation {
            self.active_page_context_key = Some(context_key);
            return;
        }
        if self.provider_id == "google-ai-mode"
            && self.active_conversation_id.is_some()
            && !semantic_context::is_new_conversation_surface(&self.provider_id, raw_url)
        {
            self.active_page_context_key = Some(context_key);
            return;
        }
        if self.active_page_context_key.as_deref() == Some(context_key.as_str()) {
            return;
        }
        self.active_page_context_key = Some(context_key.clone());
        self.active_conversation_id = Some(
            if semantic_context::is_new_conversation_surface(&self.provider_id, raw_url) {
                semantic_context::fresh_conversation_id(&self.provider_id)
            } else {
                context_key
            },
        );
        self.pending_context_action = "navigation".to_string();
    }

    pub(super) fn apply_message_snapshot(
        &mut self,
        payload: Value,
        page_context_key: Option<&str>,
    ) -> bool {
        if let (Some(expected), Some(actual)) =
            (self.active_page_context_key.as_deref(), page_context_key)
        {
            if expected != actual {
                self.last_event_kind = "stale_message_snapshot_ignored".to_string();
                return false;
            }
        }
        let boundary = self.pending_context_action.clone();
        if boundary == "new_conversation" {
            let page_changed = page_context_key.is_some()
                && page_context_key != self.semantic_page_context_key.as_deref();
            if !page_changed
                && semantic_context::has_same_last_user(self.semantic_event.as_ref(), &payload)
            {
                self.last_event_kind = "stale_message_snapshot_ignored".to_string();
                return false;
            }
            self.active_page_context_key = page_context_key.map(ToOwned::to_owned);
            if self.active_conversation_id.is_none() {
                self.active_conversation_id =
                    Some(semantic_context::fresh_conversation_id(&self.provider_id));
            }
        }
        let boundary_pending = !boundary.is_empty();
        if self.active_page_context_key.is_none() {
            self.active_page_context_key = page_context_key.map(ToOwned::to_owned);
        }
        if self.active_conversation_id.is_none() && semantic_context::has_visible_messages(&payload)
        {
            self.active_conversation_id = page_context_key.map(ToOwned::to_owned);
        }
        let same_conversation = !boundary_pending
            && self.active_conversation_id.is_some()
            && self.active_conversation_id == self.semantic_conversation_id;
        let merged = semantic_context::merge_message_snapshot(
            &self.provider_id,
            self.semantic_event.as_ref(),
            payload,
            same_conversation,
        );
        self.semantic_event = Some(merged);
        self.semantic_conversation_id = self.active_conversation_id.clone();
        self.semantic_page_context_key = page_context_key
            .map(ToOwned::to_owned)
            .or_else(|| self.active_page_context_key.clone());
        self.pending_context_action.clear();
        if self
            .semantic_event
            .as_ref()
            .is_some_and(semantic_context::has_completed_assistant)
        {
            self.preserve_conversation_on_navigation = false;
        }
        true
    }

    pub(super) fn finish_context_command(&mut self, payload: &Value) {
        let action = payload
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if !ok && self.pending_context_action == action {
            self.active_conversation_id = self.semantic_conversation_id.clone();
            self.active_page_context_key = self.semantic_page_context_key.clone();
            self.pending_context_action.clear();
        }
        if action == "send_prompt" && !ok {
            self.preserve_conversation_on_navigation = false;
        }
    }

    pub(super) fn reset_context(&mut self) {
        self.active_conversation_id = None;
        self.semantic_conversation_id = None;
        self.active_page_context_key = None;
        self.semantic_page_context_key = None;
        self.pending_context_action.clear();
        self.preserve_conversation_on_navigation = false;
    }
}
