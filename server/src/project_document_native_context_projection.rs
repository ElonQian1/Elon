//! Bounded query projection for reviewed project navigation memory.

use serde_json::{json, Value};
use std::path::Path;

use crate::project_document_native_context::{validate_evidence_current, ProjectContextMemory};

const MAX_VALIDATION_CANDIDATES: usize = 8;
const MAX_SELECTED_MEMORIES: usize = 3;

pub(crate) fn relevant_memories(
    workspace: &Path,
    query: &str,
    memories: &[ProjectContextMemory],
    limit: usize,
) -> Value {
    let mut ranked = memories
        .iter()
        .filter_map(|memory| {
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
        let drifted = memory
            .evidence
            .iter()
            .find(|evidence| validate_evidence_current(workspace, evidence).is_err());
        if let Some(evidence) = drifted {
            invalidated_count += 1;
            invalidated.push(json!({
                "candidate_id": memory.candidate_id,
                "drifted_path": evidence.path,
                "action": "Re-open with native tools before proposing a replacement."
            }));
            continue;
        }
        selected.push(json!({
            "candidate_id": memory.candidate_id,
            "summary": memory.summary,
            "topics": memory.topics,
            "evidence": memory.evidence,
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
        "schema": "elon.project_context_memory.v1",
        "selected": selected,
        "selected_count": selected_count,
        "invalidated": invalidated,
        "invalidated_count": invalidated_count,
        "validation_candidate_limit": MAX_VALIDATION_CANDIDATES,
        "source_bodies_returned": 0,
        "conflict_rule": "Current files/tests and binding project documents always override this navigation memory."
    })
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
