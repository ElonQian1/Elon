//! Small in-process cache for the read-only project context MCP profile.
//!
//! Only clean Git workspaces are cacheable: HEAD is then a sufficient source
//! revision for tracked and untracked project inputs. Dirty workspaces always
//! bypass the cache so an index receipt cannot hide local edits.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_CACHE_ENTRIES: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceRevision {
    pub(crate) git_head: Option<String>,
    pub(crate) git_branch: Option<String>,
    pub(crate) git_clean: Option<bool>,
}

#[derive(Debug, Clone)]
pub(crate) struct CacheHit {
    pub(crate) plan: Value,
    pub(crate) age_ms: u64,
}

#[derive(Clone)]
struct CachedPlan {
    created_at: Instant,
    last_used_at: Instant,
    plan: Value,
}

static PLAN_CACHE: OnceLock<Mutex<HashMap<String, CachedPlan>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, CachedPlan>> {
    PLAN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn inspect_workspace(workspace: &Path) -> WorkspaceRevision {
    let git_head = crate::node_agent_update_checkpoint::git_output(
        workspace,
        &["rev-parse", "--verify", "HEAD^{commit}"],
    );
    let git_branch =
        crate::node_agent_update_checkpoint::git_output(workspace, &["branch", "--show-current"]);
    let git_clean =
        crate::node_agent_update_checkpoint::git_output(workspace, &["status", "--porcelain"])
            .map(|status| status.trim().is_empty());
    WorkspaceRevision {
        git_head,
        git_branch,
        git_clean,
    }
}

pub(crate) fn request_cache_key(
    workspace: &Path,
    revision: &WorkspaceRevision,
    query: &str,
    max_tokens: u64,
    max_documents: usize,
    max_response_tokens: u64,
) -> Option<String> {
    if revision.git_clean != Some(true) {
        return None;
    }
    let git_head = revision.git_head.as_deref()?;
    let canonical = fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
    let seed = format!(
        "elon.project_context.cache.v2\0{}\0{git_head}\0{}\0{max_tokens}\0{max_documents}\0{max_response_tokens}",
        canonical.to_string_lossy().to_lowercase(),
        normalize_query(query),
    );
    Some(sha256_hex(seed.as_bytes()))
}

pub(crate) fn lookup(key: &str) -> Option<CacheHit> {
    let now = Instant::now();
    let mut cache = cache().lock().ok()?;
    cache.retain(|_, entry| now.duration_since(entry.created_at) <= CACHE_TTL);
    let entry = cache.get_mut(key)?;
    entry.last_used_at = now;
    Some(CacheHit {
        plan: entry.plan.clone(),
        age_ms: now
            .duration_since(entry.created_at)
            .as_millis()
            .min(u64::MAX as u128) as u64,
    })
}

pub(crate) fn store(key: String, plan: Value) {
    let now = Instant::now();
    let Ok(mut cache) = cache().lock() else {
        return;
    };
    cache.retain(|_, entry| now.duration_since(entry.created_at) <= CACHE_TTL);
    if cache.len() >= MAX_CACHE_ENTRIES && !cache.contains_key(&key) {
        if let Some(oldest) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_used_at)
            .map(|(key, _)| key.clone())
        {
            cache.remove(&oldest);
        }
    }
    cache.insert(
        key,
        CachedPlan {
            created_at: now,
            last_used_at: now,
            plan,
        },
    );
}

pub(crate) fn stable_plan_id(receipt_material: &Value) -> String {
    let encoded = serde_json::to_vec(receipt_material).unwrap_or_default();
    format!("ctx_{}", &sha256_hex(&encoded)[..24])
}

fn normalize_query(query: &str) -> String {
    query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn sha256_hex(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use super::{request_cache_key, stable_plan_id, WorkspaceRevision};
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn dirty_workspace_is_never_cacheable() {
        let revision = WorkspaceRevision {
            git_head: Some("abc".into()),
            git_branch: Some("main".into()),
            git_clean: Some(false),
        };
        assert!(request_cache_key(Path::new("."), &revision, "task", 1200, 6, 1200).is_none());
    }

    #[test]
    fn plan_receipt_is_stable_for_identical_material() {
        let material = json!({"head":"abc","documents":["AI_CURRENT.md"]});
        assert_eq!(stable_plan_id(&material), stable_plan_id(&material));
        assert!(stable_plan_id(&material).starts_with("ctx_"));
    }
}
