// server/src/node_agent_tool_approval.rs

use serde::Serialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::{watch, RwLock};

pub(crate) const TOOL_APPROVAL_TIMEOUT_SECS: u64 = 30 * 60;

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
    expires_at_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PendingToolApprovalView {
    pub approval_id: String,
    pub registered_at_ms: u128,
    pub expires_at_ms: u128,
}

pub(crate) struct ToolApprovalWaiter {
    key: String,
    rx: watch::Receiver<Option<ToolApprovalDecision>>,
    state: ToolApprovalState,
    registered_at_ms: u128,
    expires_at_ms: u128,
}

impl ToolApprovalState {
    pub(crate) async fn register(&self, req_id: &str, approval_id: &str) -> ToolApprovalWaiter {
        let key = approval_key(req_id, approval_id);
        let (tx, rx) = watch::channel(None);
        let registered_at_ms = now_ms();
        let expires_at_ms =
            registered_at_ms.saturating_add(u128::from(TOOL_APPROVAL_TIMEOUT_SECS) * 1_000);
        self.pending.write().await.insert(
            key.clone(),
            PendingToolApproval {
                tx,
                req_id: req_id.to_string(),
                approval_id: approval_id.to_string(),
                registered_at_ms,
                expires_at_ms,
            },
        );
        ToolApprovalWaiter {
            key,
            rx,
            state: self.clone(),
            registered_at_ms,
            expires_at_ms,
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
                expires_at_ms: pending.expires_at_ms,
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

    pub(crate) fn registered_at_ms(&self) -> u128 {
        self.registered_at_ms
    }

    pub(crate) fn expires_at_ms(&self) -> u128 {
        self.expires_at_ms
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
#[path = "node_agent_tool_approval_tests.rs"]
mod tests;
