//! Project-isolated, bounded bridge queue. No page content enters diagnostics.
#[path = "node_agent_browser_research_api.rs"]
mod api;
#[path = "node_agent_browser_research_contract.rs"]
pub(crate) mod contract;
pub(crate) use api::routes;

use contract::{identifier, valid_error, validate_result, ResearchCommand, ResearchResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_ACTIONS: usize = 128;
const ACTION_TTL_MS: u64 = 120_000;
const RETENTION_MS: u64 = 600_000;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ResearchAction {
    pub action_id: String,
    pub project_key: String,
    pub command: ResearchCommand,
    pub requested_at_ms: u64,
    pub expires_at_ms: u64,
    pub status: String,
    pub receipt: Option<ResearchReceipt>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ResearchReceipt {
    pub status: String,
    pub result: Option<Value>,
    pub error_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptInput {
    pub claim_token: String,
    pub status: String,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClaimedAction {
    pub action: ResearchAction,
    pub claim_token: String,
}

struct Entry {
    action: ResearchAction,
    claim_token: Option<String>,
}

#[derive(Default)]
pub(crate) struct BrowserResearchHub {
    inner: Mutex<VecDeque<Entry>>,
}

pub(crate) fn project_key(workspace: &Path) -> ResearchResult<String> {
    let path = workspace.canonicalize().map_err(|_| "invalid_project")?;
    if !path.is_dir() {
        return Err("invalid_project");
    }
    let value = path.to_string_lossy().replace('\\', "/");
    let value = if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    };
    Ok(format!("{:x}", Sha256::digest(value.as_bytes())))
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(crate) fn terminal(status: &str) -> bool {
    matches!(
        status,
        "succeeded" | "failed" | "host_unavailable" | "expired" | "cancelled"
    )
}

fn sweep(entries: &mut VecDeque<Entry>, now: u64) {
    entries.retain(|entry| now < entry.action.requested_at_ms.saturating_add(RETENTION_MS));
    for entry in entries {
        if !terminal(&entry.action.status) && now >= entry.action.expires_at_ms {
            entry.action.status = "expired".into();
            entry.claim_token = None;
        }
    }
}

impl BrowserResearchHub {
    pub(crate) fn enqueue(
        &self,
        workspace: &Path,
        command: ResearchCommand,
    ) -> ResearchResult<ResearchAction> {
        command.validate()?;
        let project_key = project_key(workspace)?;
        let now = now_ms();
        let mut entries = self.inner.lock().map_err(|_| "queue_unavailable")?;
        sweep(&mut entries, now);
        if entries
            .iter()
            .filter(|entry| {
                entry.action.project_key == project_key && !terminal(&entry.action.status)
            })
            .count()
            >= 16
        {
            return Err("queue_full");
        }
        if entries.len() >= MAX_ACTIONS {
            if let Some(index) = entries
                .iter()
                .position(|entry| terminal(&entry.action.status))
            {
                entries.remove(index);
            } else {
                return Err("queue_full");
            }
        }
        let action = ResearchAction {
            action_id: format!("research_{}", uuid::Uuid::new_v4().simple()),
            project_key,
            command,
            requested_at_ms: now,
            expires_at_ms: now.saturating_add(ACTION_TTL_MS),
            status: "queued".into(),
            receipt: None,
        };
        entries.push_back(Entry {
            action: action.clone(),
            claim_token: None,
        });
        Ok(action)
    }

    pub(crate) fn pending(&self, limit: usize) -> ResearchResult<Vec<ResearchAction>> {
        if !(1..=16).contains(&limit) {
            return Err("invalid_limit");
        }
        let mut entries = self.inner.lock().map_err(|_| "queue_unavailable")?;
        sweep(&mut entries, now_ms());
        let mut actions = Vec::new();
        let mut bytes = 128;
        for entry in entries
            .iter()
            .filter(|entry| entry.action.status == "queued")
            .take(limit)
        {
            let size = serde_json::to_vec(&entry.action)
                .map_err(|_| "invalid_result")?
                .len()
                + 1;
            if bytes + size > contract::MAX_RESULT_BYTES {
                break;
            }
            bytes += size;
            actions.push(entry.action.clone());
        }
        Ok(actions)
    }

    pub(crate) fn claim(&self, id: &str) -> ResearchResult<ClaimedAction> {
        if !identifier(id) {
            return Err("action_not_found");
        }
        let mut entries = self.inner.lock().map_err(|_| "queue_unavailable")?;
        sweep(&mut entries, now_ms());
        let entry = entries
            .iter_mut()
            .find(|entry| entry.action.action_id == id)
            .ok_or("action_not_found")?;
        // A second bridge never receives a token and must never execute again.
        if entry.action.status != "queued" {
            return Err("action_not_claimable");
        }
        let token = format!("claim_{}", uuid::Uuid::new_v4().simple());
        entry.claim_token = Some(token.clone());
        entry.action.status = "executing".into();
        Ok(ClaimedAction {
            action: entry.action.clone(),
            claim_token: token,
        })
    }

    pub(crate) fn record_receipt(
        &self,
        id: &str,
        input: ReceiptInput,
    ) -> ResearchResult<ResearchAction> {
        if !identifier(id) || !identifier(&input.claim_token) {
            return Err("invalid_receipt");
        }
        match input.status.as_str() {
            "succeeded" if input.result.is_some() && input.error_code.is_none() => {
                validate_result(input.result.as_ref().ok_or("invalid_receipt")?)?;
            }
            "failed" | "host_unavailable"
                if input.result.is_none()
                    && input.error_code.as_deref().is_some_and(valid_error) => {}
            _ => return Err("invalid_receipt"),
        }
        let receipt = ResearchReceipt {
            status: input.status,
            result: input.result,
            error_code: input.error_code,
        };
        let mut entries = self.inner.lock().map_err(|_| "queue_unavailable")?;
        sweep(&mut entries, now_ms());
        let entry = entries
            .iter_mut()
            .find(|entry| entry.action.action_id == id)
            .ok_or("action_not_found")?;
        if entry.claim_token.as_deref() != Some(input.claim_token.as_str()) {
            return Err("invalid_claim");
        }
        if let Some(existing) = &entry.action.receipt {
            return if *existing == receipt {
                Ok(entry.action.clone())
            } else {
                Err("receipt_conflict")
            };
        }
        if entry.action.status != "executing" {
            return Err("action_not_executing");
        }
        entry.action.status = receipt.status.clone();
        entry.action.receipt = Some(receipt);
        Ok(entry.action.clone())
    }

    pub(crate) fn action(&self, workspace: &Path, id: &str) -> ResearchResult<ResearchAction> {
        let project = project_key(workspace)?;
        let action = self.admin_action(id)?;
        if action.project_key != project {
            return Err("action_not_found");
        }
        Ok(action)
    }

    pub(crate) fn cancel(&self, workspace: &Path, id: &str) -> ResearchResult<ResearchAction> {
        let project = project_key(workspace)?;
        let mut entries = self.inner.lock().map_err(|_| "queue_unavailable")?;
        sweep(&mut entries, now_ms());
        let entry = entries
            .iter_mut()
            .find(|entry| entry.action.action_id == id && entry.action.project_key == project)
            .ok_or("action_not_found")?;
        if !terminal(&entry.action.status) {
            entry.action.status = "cancelled".into();
            entry.claim_token = None;
        }
        Ok(entry.action.clone())
    }

    pub(crate) fn admin_action(&self, id: &str) -> ResearchResult<ResearchAction> {
        if !identifier(id) {
            return Err("action_not_found");
        }
        let mut entries = self.inner.lock().map_err(|_| "queue_unavailable")?;
        sweep(&mut entries, now_ms());
        entries
            .iter()
            .find(|entry| entry.action.action_id == id)
            .map(|entry| entry.action.clone())
            .ok_or("action_not_found")
    }
}

#[cfg(test)]
#[path = "node_agent_browser_research_tests.rs"]
mod tests;
