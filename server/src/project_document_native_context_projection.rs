//! Bounded query projection for reviewed project navigation memory.

use chrono::{NaiveDate, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_document_native_context::{validate_evidence_current, ProjectContextMemory},
    project_document_native_context_conflict::inspect_shared_set,
    project_document_native_context_git::{relocation_candidates_from_index, relocation_index},
};

const MAX_VALIDATION_CANDIDATES: usize = 24;
const MAX_SELECTED_MEMORIES: usize = 3;

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct MemoryRetrievalScope {
    pub(crate) task_paths: Vec<String>,
    pub(crate) scope_id: String,
    pub(crate) git_branch: String,
    pub(crate) release: String,
    pub(crate) worktree_clean: Option<bool>,
}

pub(crate) fn relevant_memories(
    workspace: &Path,
    query: &str,
    retrieval_scope: &MemoryRetrievalScope,
    memories: &[ProjectContextMemory],
    limit: usize,
) -> Value {
    let mut scope_filtered_count = 0usize;
    let mut ranked = memories
        .iter()
        .filter_map(|memory| {
            if !scope_matches(query, retrieval_scope, memory) {
                scope_filtered_count += 1;
                return None;
            }
            let score = relevance_score(query, memory);
            (score > 0).then_some((score, memory))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.candidate_id.cmp(&right.1.candidate_id))
    });

    let selected_limit = limit.min(MAX_SELECTED_MEMORIES);
    let mut selected = Vec::new();
    let mut invalidated = Vec::new();
    let mut invalidated_count = 0usize;
    let mut relocation_cache = None;
    let shared_conflicts = inspect_shared_set(memories);
    for (score, memory) in ranked.into_iter().take(MAX_VALIDATION_CANDIDATES) {
        if memory.reviewed_at.trim().is_empty() {
            invalidated_count += 1;
            invalidated.push(json!({
                "candidate_id": memory.candidate_id,
                "reason": "missing_review_receipt",
                "action": "Review through the project document suggestion/apply flow."
            }));
            continue;
        }
        if memory_expired(memory) {
            invalidated_count += 1;
            invalidated.push(json!({
                "candidate_id": memory.candidate_id,
                "reason": "memory_expired",
                "action": "Reverify the evidence and renew or retire this memory through the reviewed suggestions/apply flow."
            }));
            continue;
        }
        let drifted = memory
            .evidence
            .iter()
            .find(|evidence| validate_evidence_current(workspace, evidence).is_err());
        if let Some(evidence) = drifted {
            invalidated_count += 1;
            let index = relocation_cache.get_or_insert_with(|| relocation_index(workspace));
            let relocations = relocation_candidates_from_index(evidence, index);
            invalidated.push(json!({
                "candidate_id": memory.candidate_id,
                "drifted_path": evidence.path,
                "reason": if relocations.is_empty() { "evidence_drifted" } else { "path_relocation_suggested" },
                "relocation_candidates": relocations,
                "action": "Re-open current files with native tools before proposing a reviewed replacement; never rewrite evidence automatically."
            }));
            continue;
        }
        if let Some(conflicts) = shared_conflicts.get(&memory.candidate_id) {
            invalidated_count += 1;
            invalidated.push(json!({
                "candidate_id": memory.candidate_id,
                "reason": "shared_memory_conflict",
                "conflicts": conflicts,
                "action": "Resolve against current source and binding documents through a reviewed replacement; do not inject either conflicting memory."
            }));
            continue;
        }
        selected.push(json!({
            "candidate_id": memory.candidate_id,
            "summary": memory.summary,
            "topics": memory.topics,
            "evidence": memory.evidence,
            "owner": memory.owner,
            "scope": memory.scope,
            "review": memory.review,
            "score": score,
            "authority": "navigation_only",
            "native_verification_required_before_edit": true
        }));
        if selected.len() >= selected_limit {
            break;
        }
    }
    let selected_count = selected.len();
    json!({
        "schema": "elon.project_context_memory.v3",
        "selected": selected,
        "selected_count": selected_count,
        "invalidated": invalidated,
        "invalidated_count": invalidated_count,
        "scope_filtered_count": scope_filtered_count,
        "task_scope": retrieval_scope,
        "validation_candidate_limit": MAX_VALIDATION_CANDIDATES,
        "source_bodies_returned": 0,
        "conflict_rule": "Current files/tests and binding project documents always override this navigation memory."
    })
}

fn scope_matches(
    query: &str,
    retrieval: &MemoryRetrievalScope,
    memory: &ProjectContextMemory,
) -> bool {
    let scope = &memory.scope;
    if scope.kind == "paths" && !path_scope_matches(query, &retrieval.task_paths, &scope.paths) {
        return false;
    }
    if !scope.scope_ids.is_empty()
        && (retrieval.scope_id.is_empty()
            || !scope
                .scope_ids
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&retrieval.scope_id)))
    {
        return false;
    }
    if !scope.branches.is_empty()
        && (retrieval.git_branch.is_empty()
            || !scope
                .branches
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&retrieval.git_branch)))
    {
        return false;
    }
    if !scope.releases.is_empty()
        && (retrieval.release.is_empty()
            || !scope
                .releases
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&retrieval.release)))
    {
        return false;
    }
    match scope.worktree_state.as_str() {
        "clean" => retrieval.worktree_clean == Some(true),
        "dirty" => retrieval.worktree_clean == Some(false),
        _ => true,
    }
}

fn path_scope_matches(query: &str, task_paths: &[String], scope_paths: &[String]) -> bool {
    let query = query.replace('\\', "/").to_ascii_lowercase();
    scope_paths.iter().any(|scope_path| {
        let scope_path = scope_path.to_ascii_lowercase();
        query.contains(&scope_path)
            || task_paths
                .iter()
                .any(|task_path| paths_overlap(&scope_path, &task_path.to_ascii_lowercase()))
    })
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn memory_expired(memory: &ProjectContextMemory) -> bool {
    NaiveDate::parse_from_str(memory.review.expires_at.trim(), "%Y-%m-%d")
        .is_ok_and(|expires| expires < Utc::now().date_naive())
}

fn relevance_score(query: &str, memory: &ProjectContextMemory) -> usize {
    let query = query.to_lowercase();
    let summary = memory.summary.to_lowercase();
    let mut score = usize::from(summary.contains(&query)) * 12;
    for topic in &memory.topics {
        let topic = topic.to_lowercase();
        if query.contains(&topic) || topic.contains(&query) {
            score += 8;
        } else if query
            .split(|value: char| !value.is_alphanumeric())
            .any(|term| {
                term.chars().count() >= 2 && (topic.contains(term) || summary.contains(term))
            })
        {
            score += 3;
        }
    }
    for evidence in &memory.evidence {
        if query.contains(&evidence.path.to_lowercase()) {
            score += 4;
        }
    }
    score
}
