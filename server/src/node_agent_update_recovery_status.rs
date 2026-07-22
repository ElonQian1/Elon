use anyhow::Result;

use crate::node_agent_update_recovery::{
    UpdateRecoveryStore, UPDATE_RECOVERY_PROTOCOL, UPDATE_RECOVERY_SCHEMA_VERSION,
};

impl UpdateRecoveryStore {
    pub(crate) fn status_payload(&self, limit: usize) -> Result<serde_json::Value> {
        let ledger = self.load()?;
        let mut receipts = ledger.receipts;
        receipts.sort_by(|left, right| right.updated_at_ms.cmp(&left.updated_at_ms));
        receipts.truncate(limit.clamp(1, 100));
        let active_count = receipts
            .iter()
            .filter(|receipt| !receipt.state.is_terminal())
            .count();
        Ok(serde_json::json!({
            "schema_version": UPDATE_RECOVERY_SCHEMA_VERSION,
            "protocol": UPDATE_RECOVERY_PROTOCOL,
            "expected_downtime_explicit": true,
            "cursor_replay_supported": true,
            "install_gate": ledger.install_gate,
            "active_count": active_count,
            "receipts": receipts,
        }))
    }

    pub(crate) fn status_summary_payload(&self, limit: usize) -> Result<serde_json::Value> {
        self.status_page_payload(0, limit.clamp(1, 20), false)
    }

    pub(crate) fn status_page_payload(
        &self,
        cursor: usize,
        limit: usize,
        include_events: bool,
    ) -> Result<serde_json::Value> {
        let ledger = self.load()?;
        let mut receipts = ledger.receipts;
        receipts.sort_by(|left, right| right.updated_at_ms.cmp(&left.updated_at_ms));
        let receipt_count = receipts.len();
        let active_count = receipts
            .iter()
            .filter(|receipt| !receipt.state.is_terminal())
            .count();
        let cursor = cursor.min(receipt_count);
        let limit = limit.clamp(1, 100);
        let end = cursor.saturating_add(limit).min(receipt_count);
        let page = receipts[cursor..end]
            .iter()
            .map(|receipt| {
                let mut value = serde_json::to_value(receipt).unwrap_or_default();
                if !include_events {
                    if let Some(object) = value.as_object_mut() {
                        object.remove("events");
                        object.insert(
                            "event_count".to_string(),
                            serde_json::json!(receipt.events.len()),
                        );
                    }
                }
                value
            })
            .collect::<Vec<_>>();
        Ok(serde_json::json!({
            "schema_version": UPDATE_RECOVERY_SCHEMA_VERSION,
            "protocol": UPDATE_RECOVERY_PROTOCOL,
            "expected_downtime_explicit": true,
            "cursor_replay_supported": true,
            "summary": !include_events,
            "install_gate": ledger.install_gate,
            "active_count": active_count,
            "receipt_count": receipt_count,
            "cursor": cursor,
            "limit": limit,
            "next_cursor": (end < receipt_count).then_some(end),
            "has_more": end < receipt_count,
            "receipts": page,
        }))
    }
}
