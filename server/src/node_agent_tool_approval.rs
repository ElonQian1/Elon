// server/src/node_agent_tool_approval.rs

use serde::Serialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::{watch, RwLock};

#[derive(Clone, Default)]
pub(crate) struct ToolApprovalState {
    pending: Arc<RwLock<HashMap<String, PendingToolApproval>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolApprovalDecision {
    Approve,
    Deny,
}

#[derive(Clone)]
struct PendingToolApproval {
    tx: watch::Sender<Option<ToolApprovalDecision>>,
    req_id: String,
    approval_id: String,
    registered_at_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PendingToolApprovalView {
    pub approval_id: String,
    pub registered_at_ms: u128,
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
        self.pending.write().await.insert(
            key.clone(),
            PendingToolApproval {
                tx,
                req_id: req_id.to_string(),
                approval_id: approval_id.to_string(),
                registered_at_ms: now_ms(),
            },
        );
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
        let pending = self.pending.write().await.remove(&key);
        pending
            .map(|pending| pending.tx.send(Some(decision)).is_ok())
            .unwrap_or(false)
    }

    pub(crate) async fn pending_for_req(&self, req_id: &str) -> Vec<PendingToolApprovalView> {
        let mut approvals: Vec<_> = self
            .pending
            .read()
            .await
            .values()
            .filter(|pending| pending.req_id == req_id)
            .map(|pending| PendingToolApprovalView {
                approval_id: pending.approval_id.clone(),
                registered_at_ms: pending.registered_at_ms,
            })
            .collect();
        approvals.sort_by(|left, right| {
            left.registered_at_ms
                .cmp(&right.registered_at_ms)
                .then_with(|| left.approval_id.cmp(&right.approval_id))
        });
        approvals
    }

    pub(crate) async fn clear_req(&self, req_id: &str) -> usize {
        let mut pending = self.pending.write().await;
        let keys = pending
            .iter()
            .filter_map(|(key, approval)| (approval.req_id == req_id).then_some(key.clone()))
            .collect::<Vec<_>>();
        let removed = keys.len();
        for key in keys {
            pending.remove(&key);
        }
        removed
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

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
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

    #[tokio::test]
    async fn duplicate_decision_is_rejected_after_first_consume() {
        let state = ToolApprovalState::default();
        let _waiter = state.register("req", "tap_1_1").await;

        assert!(state.decide("req", "tap_1_1", "approve").await);
        assert!(!state.decide("req", "tap_1_1", "approve").await);
    }

    #[tokio::test]
    async fn pending_for_req_lists_only_live_waiters() {
        let state = ToolApprovalState::default();
        let _first = state.register("req", "tap_1_1").await;
        let _other = state.register("other", "tap_2_1").await;

        let pending = state.pending_for_req("req").await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].approval_id, "tap_1_1");

        assert!(state.decide("req", "tap_1_1", "deny").await);
        assert!(state.pending_for_req("req").await.is_empty());
    }

    #[tokio::test]
    async fn clear_req_removes_only_matching_waiters() {
        let state = ToolApprovalState::default();
        let mut first = state.register("req", "tap_1_1").await;
        let mut second = state.register("req", "tap_1_2").await;
        let mut other = state.register("other", "tap_2_1").await;

        assert_eq!(state.clear_req("req").await, 2);
        assert!(state.pending_for_req("req").await.is_empty());
        assert_eq!(state.pending_for_req("other").await.len(), 1);
        assert!(!state.decide("req", "tap_1_1", "approve").await);
        assert!(!first.changed().await);
        assert!(!second.changed().await);

        assert!(state.decide("other", "tap_2_1", "approve").await);
        assert!(other.changed().await);
        assert_eq!(other.decision(), Some(ToolApprovalDecision::Approve));
    }
}
