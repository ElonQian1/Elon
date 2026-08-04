//! Session-local delivery ledger for the lightweight project context profile.
//!
//! The ledger stores only query digests, revision-bound plan IDs, document
//! paths, and content hashes in the existing short-lived MCP session folder.
//! It never stores task text or document/source bodies.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    node_agent_project_context_cache::WorkspaceRevision,
    node_agent_project_context_projection::not_modified_response,
};

const RECEIPT_SCHEMA: &str = "elon.project_context_session.v1";
const MAX_RECEIPT_BYTES: u64 = 64 * 1024;
const MAX_ENTRIES: usize = 8;
const MAX_SOURCES: usize = 16;

#[derive(Default, Serialize, Deserialize)]
struct ReceiptStore {
    #[serde(default)]
    schema: String,
    #[serde(default)]
    entries: Vec<ReceiptEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
struct ReceiptEntry {
    scope_key: String,
    plan_id: String,
    #[serde(default)]
    sources: Vec<SourceReceipt>,
    updated_at_ms: u64,
    delivery_count: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct SourceReceipt {
    path: String,
    content_hash: String,
}

pub(super) struct DeliveryInput<'a> {
    pub(super) receipt_path: Option<&'a Path>,
    pub(super) query: &'a str,
    pub(super) max_tokens: u64,
    pub(super) max_documents: usize,
    pub(super) max_response_tokens: u64,
    pub(super) previous_plan_id: Option<&'a str>,
    pub(super) force_refresh: bool,
    pub(super) cache_status: &'a str,
    pub(super) cache_age_ms: Option<u64>,
    pub(super) planning_ms: u64,
    pub(super) revision: &'a WorkspaceRevision,
}

pub(super) fn finish(mut plan: Value, input: DeliveryInput<'_>) -> Result<Value> {
    let plan_id = plan
        .pointer("/plan_receipt/plan_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let full_plan_tokens = plan
        .pointer("/plan_receipt/estimated_full_plan_tokens")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let scope_key = request_scope_key(
        input.query,
        input.max_tokens,
        input.max_documents,
        input.max_response_tokens,
    );
    let current_sources = source_receipts(&plan);
    let _guard = receipt_lock();
    let mut store = input
        .receipt_path
        .map(load_store)
        .unwrap_or_else(ReceiptStore::default);
    let previous = store
        .entries
        .iter()
        .find(|entry| entry.scope_key == scope_key)
        .cloned();
    let session_plan_id = previous.as_ref().map(|entry| entry.plan_id.as_str());
    let effective_previous = input.previous_plan_id.or(session_plan_id);
    let automatic_reuse = input.previous_plan_id.is_none() && session_plan_id.is_some();
    let unchanged = !input.force_refresh && effective_previous == Some(plan_id.as_str());
    let delta = if !input.force_refresh && !unchanged {
        previous
            .as_ref()
            .map(|entry| apply_source_delta(&mut plan, &entry.sources, &current_sources))
    } else {
        None
    };
    let delivery_count = previous
        .as_ref()
        .map(|entry| entry.delivery_count.saturating_add(1))
        .unwrap_or(1);
    let persistence = record_receipt(
        input.receipt_path,
        &mut store,
        ReceiptEntry {
            scope_key,
            plan_id: plan_id.clone(),
            sources: current_sources.clone(),
            updated_at_ms: unix_millis(),
            delivery_count,
        },
    );
    let mode = if unchanged {
        "not_modified"
    } else if delta.is_some() {
        "incremental"
    } else {
        "full"
    };
    let delivery_receipt = json!({
        "schema": RECEIPT_SCHEMA,
        "mode": mode,
        "session_memory_enabled": input.receipt_path.is_some(),
        "automatic_previous_plan_reuse": automatic_reuse,
        "known_source_count": previous.as_ref().map(|entry| entry.sources.len()).unwrap_or(0),
        "current_source_count": current_sources.len(),
        "delivery_count": delivery_count,
        "persistence": persistence,
        "content_hash_drift_only": true,
        "semantic_content_compared": false,
    });

    if unchanged {
        let mut response = not_modified_response(
            &plan,
            input.cache_status,
            input.cache_age_ms,
            input.max_response_tokens,
            input.planning_ms,
        );
        response["delivery_receipt"] = delivery_receipt;
        refresh_response_estimate(&mut response, full_plan_tokens);
        return Ok(response);
    }

    plan["cache"] = json!({
        "status": input.cache_status,
        "age_ms": input.cache_age_ms,
        "ttl_seconds": 300,
        "revision_mode": &input.revision.fingerprint_status,
        "fingerprinted_files": input.revision.fingerprint_file_count,
        "fingerprinted_bytes": input.revision.fingerprint_total_bytes,
        "bypass_reason": &input.revision.cache_bypass_reason,
        "dirty_requires_complete_fingerprint": true,
    });
    plan["delivery_receipt"] = delivery_receipt;
    plan["performance_receipt"] = json!({
        "planning_ms": input.planning_ms,
        "cache_reused": input.cache_status == "hit",
        "estimated_full_plan_tokens": full_plan_tokens,
        "estimated_tokens_avoided": 0,
        "measurement_kind": "local_structural_estimate",
        "token_estimate_method": "utf8_bytes_div_4",
        "baseline": "same locally generated full plan before delivery projection",
        "not_vendor_billing": true,
        "not_total_task_tokens": true,
    });
    refresh_response_estimate(&mut plan, full_plan_tokens);
    Ok(plan)
}

pub(super) fn attach_tool_result_size(response: &mut Value) -> Result<()> {
    for _ in 0..4 {
        let structured_bytes = serde_json::to_vec(&response["structuredContent"])?.len() as u64;
        let bytes = serde_json::to_vec(response)?.len() as u64;
        if let Some(budget) = response.pointer_mut("/structuredContent/response_budget") {
            budget["actual_bytes"] = json!(structured_bytes);
            budget["estimated_tokens"] = json!(structured_bytes.div_ceil(4));
        }
        let Some(performance) = response.pointer_mut("/structuredContent/performance_receipt")
        else {
            return Ok(());
        };
        let receipt = json!({
            "mcp_tool_result_bytes": bytes,
            "estimated_mcp_tool_result_tokens": bytes.div_ceil(4),
            "structured_content_bytes": structured_bytes,
            "measurement_kind": "local_structural_estimate",
            "token_estimate_method": "utf8_bytes_div_4",
            "not_vendor_billing": true,
            "not_total_task_tokens": true,
        });
        if performance.get("transport") == Some(&receipt) {
            break;
        }
        performance["transport"] = receipt;
    }
    Ok(())
}

fn apply_source_delta(
    plan: &mut Value,
    previous: &[SourceReceipt],
    current: &[SourceReceipt],
) -> Value {
    let previous_by_path = previous
        .iter()
        .map(|source| (source.path.as_str(), source.content_hash.as_str()))
        .collect::<HashMap<_, _>>();
    let current_paths = current
        .iter()
        .map(|source| source.path.as_str())
        .collect::<HashSet<_>>();
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged = Vec::new();
    for source in current {
        match previous_by_path.get(source.path.as_str()) {
            None => added.push(source.path.clone()),
            Some(hash)
                if !source.content_hash.is_empty() && *hash == source.content_hash.as_str() =>
            {
                unchanged.push(source.path.clone());
            }
            Some(_) => changed.push(source.path.clone()),
        }
    }
    let removed = previous
        .iter()
        .filter(|source| !current_paths.contains(source.path.as_str()))
        .map(|source| source.path.clone())
        .collect::<Vec<_>>();
    let unchanged_set = unchanged.iter().map(String::as_str).collect::<HashSet<_>>();
    for key in ["mandatory_rules", "relevant_documents"] {
        if let Some(items) = plan.get_mut(key).and_then(Value::as_array_mut) {
            items.retain(|item| {
                item.pointer("/document/path")
                    .and_then(Value::as_str)
                    .map_or(true, |path| !unchanged_set.contains(path))
            });
        }
    }
    let source_set_changed = !added.is_empty() || !removed.is_empty();
    let content_hash_drift_detected = !changed.is_empty();
    let removed_count = removed.len();
    let removed_paths = removed.into_iter().take(4).collect::<Vec<_>>();
    plan["status"] = json!("context_delta");
    plan["context_delta"] = json!({
        "schema": "elon.project_context_delta.v1",
        "added_source_count": added.len(),
        "content_changed_source_count": changed.len(),
        "removed_source_count": removed_count,
        "removed_paths": removed_paths,
        "removed_paths_omitted": removed_count.saturating_sub(4),
        "unchanged_source_count": unchanged.len(),
        "source_set_changed": source_set_changed,
        "content_hash_drift_detected": content_hash_drift_detected,
        "semantic_content_compared": false,
        "instruction": "Reuse unchanged source metadata from the prior delivery; open only added or content-changed paths, then verify current implementation with native tools."
    });
    plan["response_projection"]["delta_only"] = json!(true);
    plan["context_delta"].clone()
}

fn source_receipts(plan: &Value) -> Vec<SourceReceipt> {
    let mut sources = Vec::new();
    for key in ["mandatory_rules", "relevant_documents"] {
        for item in plan
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(source) = source_receipt(item.get("document").unwrap_or(&Value::Null)) {
                sources.push(source);
            }
        }
    }
    for path in plan
        .get("selected_paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        sources.push(SourceReceipt {
            path: path.to_string(),
            content_hash: String::new(),
        });
    }
    sources.sort_by(|left, right| left.path.cmp(&right.path));
    sources.dedup_by(|left, right| left.path == right.path);
    sources.truncate(MAX_SOURCES);
    sources
}

fn source_receipt(document: &Value) -> Option<SourceReceipt> {
    let path = document.get("path").and_then(Value::as_str)?.trim();
    if path.is_empty() || path.len() > 500 {
        return None;
    }
    Some(SourceReceipt {
        path: path.to_string(),
        content_hash: document
            .get("content_hash")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .chars()
            .take(128)
            .collect(),
    })
}

fn refresh_response_estimate(response: &mut Value, full_plan_tokens: u64) {
    for _ in 0..4 {
        let tokens = serde_json::to_vec(response).map_or(0, |bytes| bytes.len().div_ceil(4)) as u64;
        let Some(performance) = response.get_mut("performance_receipt") else {
            return;
        };
        let avoided = full_plan_tokens.saturating_sub(tokens);
        if performance
            .get("estimated_response_tokens_before_transport")
            .and_then(Value::as_u64)
            == Some(tokens)
            && performance
                .get("estimated_tokens_avoided")
                .and_then(Value::as_u64)
                == Some(avoided)
        {
            break;
        }
        performance["estimated_response_tokens_before_transport"] = json!(tokens);
        performance["estimated_tokens_avoided"] = json!(avoided);
    }
}

fn request_scope_key(
    query: &str,
    max_tokens: u64,
    max_documents: usize,
    max_response_tokens: u64,
) -> String {
    let normalized = query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let material = format!(
        "elon.project_context.session.scope.v1\0{normalized}\0{max_tokens}\0{max_documents}\0{max_response_tokens}"
    );
    hex::encode(Sha256::digest(material.as_bytes()))
}

fn load_store(path: &Path) -> ReceiptStore {
    let Some(bytes) = fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.len() <= MAX_RECEIPT_BYTES)
        .and_then(|_| fs::read(path).ok())
    else {
        return ReceiptStore::default();
    };
    serde_json::from_slice::<ReceiptStore>(&bytes)
        .ok()
        .filter(|store| store.schema == RECEIPT_SCHEMA && store.entries.len() <= MAX_ENTRIES)
        .unwrap_or_default()
}

fn record_receipt(
    path: Option<&Path>,
    store: &mut ReceiptStore,
    entry: ReceiptEntry,
) -> &'static str {
    let Some(path) = path else {
        return "disabled";
    };
    store.schema = RECEIPT_SCHEMA.to_string();
    store
        .entries
        .retain(|item| item.scope_key != entry.scope_key);
    store.entries.push(entry);
    store.entries.sort_by_key(|item| item.updated_at_ms);
    let excess = store.entries.len().saturating_sub(MAX_ENTRIES);
    if excess > 0 {
        store.entries.drain(..excess);
    }
    let Ok(bytes) = serde_json::to_vec(store) else {
        return "write_failed";
    };
    if bytes.len() as u64 > MAX_RECEIPT_BYTES {
        return "size_limit";
    }
    match crate::node_agent_atomic_file::write(path, &bytes) {
        Ok(()) => "stored",
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "项目上下文会话回执写入失败");
            "write_failed"
        }
    }
}

fn receipt_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{apply_source_delta, source_receipts, SourceReceipt};
    use serde_json::json;

    #[test]
    fn delta_omits_unchanged_sources_and_reports_content_drift() {
        let mut plan = json!({
            "mandatory_rules":[],
            "relevant_documents":[
                {"document":{"path":"AI_CURRENT.md","content_hash":"new"}},
                {"document":{"path":"AI_INDEX.md","content_hash":"same"}}
            ],
            "response_projection":{}
        });
        let previous = vec![
            SourceReceipt {
                path: "AI_CURRENT.md".into(),
                content_hash: "old".into(),
            },
            SourceReceipt {
                path: "AI_INDEX.md".into(),
                content_hash: "same".into(),
            },
        ];
        let current = source_receipts(&plan);
        let delta = apply_source_delta(&mut plan, &previous, &current);
        assert_eq!(plan["relevant_documents"].as_array().unwrap().len(), 1);
        assert_eq!(delta["content_changed_source_count"], 1);
        assert_eq!(delta["unchanged_source_count"], 1);
        assert_eq!(delta["semantic_content_compared"], false);
    }
}
