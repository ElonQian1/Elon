use super::SessionRecord;

impl SessionRecord {
    pub(super) fn begin_document_navigation(&mut self) {
        if self.provider_id == "chatgpt" {
            self.document_token = None;
        }
    }

    pub(super) fn accept_document_event(
        &mut self,
        kind: &str,
        document_token: Option<&str>,
    ) -> bool {
        if self.provider_id != "chatgpt" || document_token.is_none() {
            return true;
        }
        let token = document_token.unwrap_or_default();
        if kind == "adapter_ready" && self.document_token.is_none() {
            self.document_token = Some(token.to_string());
            return true;
        }
        if self.document_token.as_deref() == Some(token) {
            return true;
        }
        self.last_event_kind = "stale_document_event_ignored".to_string();
        false
    }
}
