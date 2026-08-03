//! Small in-process cache for the read-only project context MCP profile.
//!
//! Clean workspaces bind cache entries to HEAD. Dirty workspaces are cacheable
//! only when every changed/untracked regular file fits a bounded content hash;
//! incomplete, oversized, symlinked, or racing snapshots fail closed.

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
    pub(crate) worktree_fingerprint: Option<String>,
    pub(crate) fingerprint_status: String,
    pub(crate) fingerprint_file_count: usize,
    pub(crate) fingerprint_total_bytes: u64,
    pub(crate) cache_bypass_reason: Option<String>,
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
    let status = crate::node_agent_update_checkpoint::git_output(
        workspace,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    );
    let git_clean = status.as_ref().map(|status| status.trim().is_empty());
    let fingerprint = crate::node_agent_project_context_fingerprint::inspect(
        workspace,
        git_clean,
        status.as_deref(),
    );
    WorkspaceRevision {
        git_head,
        git_branch,
        git_clean,
        worktree_fingerprint: fingerprint.digest,
        fingerprint_status: fingerprint.status,
        fingerprint_file_count: fingerprint.file_count,
        fingerprint_total_bytes: fingerprint.total_bytes,
        cache_bypass_reason: fingerprint.bypass_reason,
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
    let git_head = revision.git_head.as_deref()?;
    let source_revision = match revision.git_clean {
        Some(true) => "clean".to_string(),
        Some(false) => format!("dirty:{}", revision.worktree_fingerprint.as_deref()?),
        None => return None,
    };
    let canonical = fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
    let seed = format!(
        "elon.project_context.cache.v3\0{}\0{git_head}\0{source_revision}\0{}\0{max_tokens}\0{max_documents}\0{max_response_tokens}",
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

    fn dirty_revision(fingerprint: Option<&str>) -> WorkspaceRevision {
        WorkspaceRevision {
            git_head: Some("abc".into()),
            git_branch: Some("main".into()),
            git_clean: Some(false),
            worktree_fingerprint: fingerprint.map(str::to_string),
            fingerprint_status: if fingerprint.is_some() {
                "content_hashed"
            } else {
                "incomplete"
            }
            .into(),
            fingerprint_file_count: 1,
            fingerprint_total_bytes: 10,
            cache_bypass_reason: fingerprint
                .is_none()
                .then(|| "changed_file_unreadable_or_unsafe".into()),
        }
    }

    #[test]
    fn dirty_workspace_requires_a_complete_fingerprint() {
        let revision = dirty_revision(None);
        assert!(request_cache_key(Path::new("."), &revision, "task", 1200, 6, 1200).is_none());
    }

    #[test]
    fn fingerprinted_dirty_workspace_is_cacheable() {
        let revision = dirty_revision(Some("fingerprint"));
        assert!(request_cache_key(Path::new("."), &revision, "task", 1200, 6, 1200).is_some());
    }

    #[test]
    fn plan_receipt_is_stable_for_identical_material() {
        let material = json!({"head":"abc","documents":["AI_CURRENT.md"]});
        assert_eq!(stable_plan_id(&material), stable_plan_id(&material));
        assert!(stable_plan_id(&material).starts_with("ctx_"));
    }
}
