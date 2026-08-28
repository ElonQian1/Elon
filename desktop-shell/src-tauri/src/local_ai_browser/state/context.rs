use serde_json::{json, Value};

use super::super::semantic_context;
use super::SessionRecord;

const BOUNDARY_ACTIONS: [&str; 4] = [
    "new_conversation",
    "open_conversation",
    "open_project",
    "open_cached_conversation",
];

// 适配器静默丢弃一次快照时，悬挂超过这个时长就不再等待原本边界，回退到最新已确认快照。
const PENDING_CONTEXT_TIMEOUT_MS: u64 = 9_000;

impl SessionRecord {
    pub(super) fn context_binding_status(&self) -> &'static str {
        if !self.pending_context_action.is_empty() || self.loading {
            return "restoring";
        }
        if self.semantic_event.is_none() {
            return "empty";
        }
        if !self.semantic_live {
            return "cached";
        }
        let conversation_bound = self.active_conversation_id.is_some()
            && self.active_conversation_id == self.semantic_conversation_id;
        let page_bound = self.active_page_context_key.is_some()
            && self.active_page_context_key == self.semantic_page_context_key;
        if self.window_status == "ready"
            && self.renderer_status == "active"
            && conversation_bound
            && page_bound
        {
            "bound"
        } else {
            "unbound"
        }
    }

    pub(super) fn context_ready(&self) -> bool {
        self.context_binding_status() == "bound"
    }

    pub(super) fn expire_stale_pending_context(&mut self) {
        if self.pending_context_action.is_empty() {
            return;
        }
        let elapsed = super::now_ms().saturating_sub(self.pending_context_since_ms);
        if elapsed < PENDING_CONTEXT_TIMEOUT_MS {
            return;
        }
        if self.pending_context_action == "new_conversation" {
            // Do not roll a timed-out new-chat generation back onto the previous
            // semantic conversation. The frontend may stop showing its recovery
            // spinner, but the old snapshot must stay unaligned and therefore hidden
            // until a fresh empty page (or a deliberate first user turn) is observed.
            self.pending_context_action.clear();
            self.pending_context_request_id = None;
            self.pending_context_since_ms = 0;
            self.pending_send_prompt = None;
            self.pending_send_private_stream_revision = 0;
            self.preserve_conversation_on_navigation = false;
            self.last_event_kind = "new_conversation_transition_timed_out".to_string();
            return;
        }
        self.active_conversation_id = self.semantic_conversation_id.clone();
        self.active_page_context_key = self.semantic_page_context_key.clone();
        self.pending_context_action.clear();
        self.pending_context_request_id = None;
        self.pending_context_since_ms = 0;
        self.pending_send_prompt = None;
        self.pending_send_private_stream_revision = 0;
        self.new_conversation_baseline_user = None;
        self.preserve_conversation_on_navigation = false;
        self.last_event_kind = "context_transition_timed_out".to_string();
    }

    pub(super) fn begin_context_command(
        &mut self,
        action: &str,
        target: Option<&str>,
        request_id: Option<&str>,
    ) {
        if action == "send_prompt" {
            self.pending_context_action = action.to_string();
            self.pending_context_request_id = request_id.map(ToOwned::to_owned);
            self.pending_context_since_ms = super::now_ms();
            self.pending_send_prompt = target.map(|value| value.to_string());
            self.pending_send_private_stream_revision = self
                .semantic_event
                .as_ref()
                .and_then(|event| event.get("privateStreamRevision"))
                .and_then(Value::as_u64)
                .unwrap_or_default();
            self.preserve_conversation_on_navigation = true;
            return;
        }
        if !BOUNDARY_ACTIONS.contains(&action) {
            return;
        }
        if action != "new_conversation" {
            self.new_conversation_baseline_user = None;
        }
        self.pending_send_prompt = None;
        self.pending_send_private_stream_revision = 0;
        self.pending_context_action = action.to_string();
        self.pending_context_request_id = request_id.map(ToOwned::to_owned);
        self.pending_context_since_ms = super::now_ms();
        self.preserve_conversation_on_navigation = false;
        if action == "new_conversation" {
            self.navigation_updated_at_ms = 0;
            self.new_conversation_baseline_user = self
                .semantic_event
                .as_ref()
                .and_then(semantic_context::last_user_fingerprint);
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
        self.new_conversation_baseline_user = None;
        self.pending_context_action = "open_cached_conversation".to_string();
        self.pending_context_request_id = None;
        self.pending_context_since_ms = super::now_ms();
        self.preserve_conversation_on_navigation = false;
        self.pending_send_private_stream_revision = 0;
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
            if self.provider_id == "google-ai-mode"
                && self.pending_context_action == "open_conversation"
            {
                self.active_conversation_id = Some(context_key);
            } else if self.active_conversation_id.is_none() {
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
        self.pending_context_request_id = None;
        self.pending_context_since_ms = super::now_ms();
    }

    pub(super) fn apply_message_snapshot(
        &mut self,
        mut payload: Value,
        page_context_key: Option<&str>,
    ) -> bool {
        let mut boundary = self.pending_context_action.clone();
        let page_changed = page_context_key.is_some()
            && page_context_key != self.semantic_page_context_key.as_deref();
        if boundary == "send_prompt" {
            let prompt_missing = self
                .pending_send_prompt
                .as_deref()
                .is_some_and(|expected| !semantic_context::has_last_user_text(&payload, expected));
            if prompt_missing {
                if !self.bind_chatgpt_private_stream_pending_send(&mut payload) {
                    self.last_event_kind = "pending_send_snapshot_ignored".to_string();
                    return false;
                }
                self.last_event_kind = "private_stream_pending_send_bound".to_string();
            }
        }
        if boundary == "new_conversation" && semantic_context::has_visible_messages(&payload) {
            // A fresh conversation is empty until its first prompt is deliberately sent.
            // Private-stream and DOM observers can still finish an assistant-only snapshot
            // from the previous turn after the new-chat command starts. Such a snapshot has
            // no user fingerprint, so comparing only the last user would incorrectly accept
            // it as the replacement conversation.
            self.last_event_kind = "stale_new_conversation_snapshot_ignored".to_string();
            return false;
        }
        if self.new_conversation_baseline_user.is_some()
            && boundary != "send_prompt"
            && semantic_context::has_visible_messages(&payload)
        {
            let incoming_user = semantic_context::last_user_fingerprint(&payload);
            let lacks_new_user_identity = incoming_user.is_none();
            let repeats_previous_user = incoming_user == self.new_conversation_baseline_user;
            if lacks_new_user_identity || repeats_previous_user {
                self.last_event_kind = "stale_new_conversation_snapshot_ignored".to_string();
                return false;
            }
            self.new_conversation_baseline_user = None;
        }
        if boundary == "send_prompt" {
            self.new_conversation_baseline_user = None;
        }
        if let (Some(expected), Some(actual)) =
            (self.active_page_context_key.as_deref(), page_context_key)
        {
            if expected != actual {
                if self.preserve_conversation_on_navigation || boundary == "new_conversation" {
                    self.active_page_context_key = Some(actual.to_string());
                } else if boundary.is_empty() && self.provider_id == "google-ai-mode" {
                    self.active_page_context_key = Some(actual.to_string());
                    if self.active_conversation_id.is_none() {
                        self.active_conversation_id = Some(actual.to_string());
                    }
                } else if boundary.is_empty() {
                    self.active_page_context_key = Some(actual.to_string());
                    self.active_conversation_id = Some(actual.to_string());
                    boundary = "semantic_navigation".to_string();
                    self.pending_context_action = boundary.clone();
                    self.pending_context_since_ms = super::now_ms();
                } else {
                    self.last_event_kind = "stale_message_snapshot_ignored".to_string();
                    return false;
                }
            }
        }
        if boundary == "new_conversation" {
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
        let boundary_pending = !boundary.is_empty() && boundary != "send_prompt";
        if self.active_page_context_key.is_none() {
            self.active_page_context_key = page_context_key.map(ToOwned::to_owned);
        }
        if self.active_conversation_id.is_none() && semantic_context::has_visible_messages(&payload)
        {
            self.active_conversation_id = page_context_key.map(ToOwned::to_owned);
        }
        let cached_restore = boundary == "open_cached_conversation"
            && self.active_conversation_id.is_some()
            && self.active_conversation_id == self.semantic_conversation_id;
        let same_conversation = (!boundary_pending || cached_restore)
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
        self.pending_context_request_id = None;
        self.pending_context_since_ms = 0;
        self.pending_send_prompt = None;
        self.pending_send_private_stream_revision = 0;
        if self
            .semantic_event
            .as_ref()
            .is_some_and(semantic_context::has_completed_assistant)
        {
            self.preserve_conversation_on_navigation = false;
        }
        true
    }

    pub(super) fn finish_context_command(&mut self, payload: &Value) -> bool {
        let action = payload
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let request_matches = self.pending_context_request_id.is_none()
            || payload.get("requestId").and_then(Value::as_str)
                == self.pending_context_request_id.as_deref();
        if self.pending_context_action == action && !request_matches {
            self.last_event_kind = "stale_context_command_result_ignored".to_string();
            return false;
        }
        if ok
            && action == "new_conversation"
            && self.provider_id == "chatgpt"
            && self.pending_context_action == action
        {
            self.confirm_verified_empty_chatgpt_conversation();
            return true;
        }
        if !ok && self.pending_context_action == action {
            self.active_conversation_id = self.semantic_conversation_id.clone();
            self.active_page_context_key = self.semantic_page_context_key.clone();
            self.pending_context_action.clear();
            self.pending_context_request_id = None;
            self.pending_context_since_ms = 0;
            self.pending_send_prompt = None;
            self.pending_send_private_stream_revision = 0;
            if action == "new_conversation" {
                self.new_conversation_baseline_user = None;
            }
        }
        if action == "send_prompt" && !ok {
            self.preserve_conversation_on_navigation = false;
        }
        false
    }

    fn confirm_verified_empty_chatgpt_conversation(&mut self) {
        let previous = self.semantic_event.as_ref();
        let authenticated = previous
            .and_then(|event| event.get("authenticated"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let login_required = previous
            .and_then(|event| event.get("loginRequired"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let current_model = previous
            .and_then(|event| event.get("currentModel"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let capabilities = previous
            .and_then(|event| event.get("capabilities"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let private_stream_revision = previous
            .and_then(|event| event.get("privateStreamRevision"))
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let private_stream_observed = previous
            .and_then(|event| event.get("privateStreamObserved"))
            .and_then(Value::as_bool)
            .unwrap_or(private_stream_revision > 0);
        let home_key = semantic_context::page_context_key("chatgpt", "https://chatgpt.com/");
        if self.active_page_context_key.is_none() {
            self.active_page_context_key = home_key;
        }
        self.semantic_event = Some(json!({
            "type": "message_snapshot",
            "title": "New chat",
            "url": "https://chatgpt.com/",
            "draft": "",
            "messages": [],
            "observedMessageCount": 0,
            "messageWindowStart": 0,
            "authenticated": authenticated,
            "pageKind": "conversation",
            "loginRequired": login_required,
            "composerReady": true,
            "streaming": false,
            "streamingStatus": "",
            "privateStreamObserved": private_stream_observed,
            "privateStreamRevision": private_stream_revision,
            "privateStreamState": "idle",
            "currentModel": current_model,
            "attachments": [],
            "dictationActive": false,
            "capabilities": capabilities,
        }));
        self.semantic_conversation_id = self.active_conversation_id.clone();
        self.semantic_page_context_key = self.active_page_context_key.clone();
        self.pending_context_action.clear();
        self.pending_context_request_id = None;
        self.pending_context_since_ms = 0;
        self.pending_send_prompt = None;
        self.pending_send_private_stream_revision = 0;
        self.preserve_conversation_on_navigation = false;
        self.window_status = "ready".to_string();
        self.loading = false;
        self.renderer_status = "active".to_string();
        self.message_count = 0;
        self.assistant_message_count = 0;
        self.streaming = false;
        self.semantic_live = true;
        let updated_at_ms = super::now_ms();
        self.cache_updated_at_ms = updated_at_ms;
        self.semantic_updated_at_ms = updated_at_ms;
        self.last_event_kind = "verified_empty_new_conversation".to_string();
    }

    fn bind_chatgpt_private_stream_pending_send(&self, payload: &mut Value) -> bool {
        if self.provider_id != "chatgpt"
            || payload
                .get("privateStreamObserved")
                .and_then(Value::as_bool)
                != Some(true)
        {
            return false;
        }
        let revision = payload
            .get("privateStreamRevision")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        if revision <= self.pending_send_private_stream_revision {
            return false;
        }
        let Some(stream_state) = payload
            .get("privateStreamState")
            .and_then(Value::as_str)
            .filter(|state| matches!(*state, "streaming" | "completed"))
        else {
            return false;
        };
        let Some(prompt) = self
            .pending_send_prompt
            .as_deref()
            .map(str::trim)
            .filter(|prompt| !prompt.is_empty())
        else {
            return false;
        };
        let Some(assistant) = payload
            .get("messages")
            .and_then(Value::as_array)
            .and_then(|messages| {
                messages.iter().rev().find(|message| {
                    message.get("role").and_then(Value::as_str) == Some("assistant")
                        && message.get("state").and_then(Value::as_str) == Some(stream_state)
                        && message
                            .get("id")
                            .and_then(Value::as_str)
                            .is_some_and(|id| id.starts_with("private-stream:"))
                })
            })
            .cloned()
        else {
            return false;
        };
        let request_identity = self
            .pending_context_request_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| revision.to_string());
        let identity =
            semantic_context::opaque_id(&format!("{}:{request_identity}", self.provider_id));
        let previous_observed = self
            .semantic_event
            .as_ref()
            .map(snapshot_observed_message_count)
            .unwrap_or_default();
        let synthesized = json!({
            "id": format!("private-stream-bound:{identity}:user"),
            "role": "user",
            "state": "completed",
            "content": [{"type":"text", "text":prompt}],
        });
        let Some(snapshot) = payload.as_object_mut() else {
            return false;
        };
        snapshot.insert(
            "messages".to_string(),
            Value::Array(vec![synthesized, assistant]),
        );
        snapshot.insert(
            "messageWindowStart".to_string(),
            Value::from(previous_observed),
        );
        snapshot.insert(
            "observedMessageCount".to_string(),
            Value::from(previous_observed.saturating_add(2)),
        );
        true
    }

    pub(super) fn reset_context(&mut self) {
        self.active_conversation_id = None;
        self.semantic_conversation_id = None;
        self.active_page_context_key = None;
        self.semantic_page_context_key = None;
        self.pending_context_action.clear();
        self.pending_context_request_id = None;
        self.pending_context_since_ms = 0;
        self.pending_send_prompt = None;
        self.pending_send_private_stream_revision = 0;
        self.new_conversation_baseline_user = None;
        self.preserve_conversation_on_navigation = false;
    }
}

fn snapshot_observed_message_count(snapshot: &Value) -> u64 {
    let window_start = snapshot
        .get("messageWindowStart")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let message_count = snapshot
        .get("messages")
        .and_then(Value::as_array)
        .map_or(0, |messages| messages.len() as u64);
    snapshot
        .get("observedMessageCount")
        .and_then(Value::as_u64)
        .unwrap_or_default()
        .max(window_start.saturating_add(message_count))
}
