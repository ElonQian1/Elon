// server/src/node_agent_tool_approval.rs

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{watch, RwLock};

#[derive(Clone, Default)]
pub(crate) struct ToolApprovalState {
    pending: Arc<RwLock<HashMap<String, watch::Sender<Option<ToolApprovalDecision>>>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolApprovalDecision {
    Approve,
    Deny,
}

pub(crate) struct ToolApprovalWaiter {
    key: String,
    rx: watch::Receiver<Option<ToolApprovalDecision>>,
    state: ToolApprovalState,
}

impl ToolApprovalState {
    pub(crate) async fn register(&self, req_id: &str, approval_id: &str) -> ToolApprovalWaiter {
        let key = approval_key(req_id, approval_id);
        let (tx, rx) = watch::channel(None);
        self.pending.write().await.insert(key.clone(), tx);
        ToolApprovalWaiter {
            key,
            rx,
            state: self.clone(),
        }
    }

    pub(crate) async fn decide(&self, req_id: &str, approval_id: &str, decision: &str) -> bool {
        let Some(decision) = normalize_decision(decision) else {
            return false;
        };
        let key = approval_key(req_id, approval_id);
        let tx = self.pending.write().await.remove(&key);
        tx.map(|tx| tx.send(Some(decision)).is_ok())
            .unwrap_or(false)
    }

    async fn remove_key(&self, key: &str) {
        self.pending.write().await.remove(key);
    }
}

impl ToolApprovalWaiter {
    pub(crate) async fn changed(&mut self) -> bool {
        self.rx.changed().await.is_ok()
    }

    pub(crate) fn decision(&self) -> Option<ToolApprovalDecision> {
        self.rx.borrow().clone()
    }

    pub(crate) async fn cleanup(&self) {
        self.state.remove_key(&self.key).await;
    }
}

fn approval_key(req_id: &str, approval_id: &str) -> String {
    format!("{req_id}:{approval_id}")
}

fn normalize_decision(value: &str) -> Option<ToolApprovalDecision> {
    match value.trim().to_ascii_lowercase().as_str() {
        "approve" | "approved" => Some(ToolApprovalDecision::Approve),
        "deny" | "denied" | "reject" | "rejected" => Some(ToolApprovalDecision::Deny),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ToolApprovalDecision, ToolApprovalState};

    #[tokio::test]
    async fn decision_wakes_registered_waiter() {
        let state = ToolApprovalState::default();
        let mut waiter = state.register("req", "tap_1_1").await;

        assert!(state.decide("req", "tap_1_1", "approve").await);
        assert!(waiter.changed().await);
        assert_eq!(waiter.decision(), Some(ToolApprovalDecision::Approve));
    }

    #[tokio::test]
    async fn unknown_or_invalid_decision_is_rejected() {
        let state = ToolApprovalState::default();
        let _waiter = state.register("req", "tap_1_1").await;

        assert!(!state.decide("req", "tap_1_2", "approve").await);
        assert!(!state.decide("req", "tap_1_1", "maybe").await);
    }
}
