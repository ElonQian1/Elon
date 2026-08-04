//! Explicit post-task receipt and optimistic candidate editing.

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    project_document_index::ProjectDocumentIndex,
    project_document_native_context::{
        initialize_candidate_schema, normalize_memories, record_candidates,
        record_candidates_attested, validate_memories_current, NativeContextCandidate,
        ProjectContextMemory,
    },
};

const MAX_RECEIPT_CANDIDATES: usize = 8;

pub(crate) fn record_receipt(
    workspace: &Path,
    memories: Vec<ProjectContextMemory>,
    producer: &str,
) -> Result<Value> {
    record_receipt_internal(workspace, memories, producer, None)
}

pub(crate) fn record_attested_receipt(
    workspace: &Path,
    memories: Vec<ProjectContextMemory>,
    producer: &str,
    session_id: &str,
) -> Result<Value> {
    record_receipt_internal(workspace, memories, producer, Some(session_id))
}

fn record_receipt_internal(
    workspace: &Path,
    memories: Vec<ProjectContextMemory>,
    producer: &str,
    session_id: Option<&str>,
) -> Result<Value> {
    if memories.is_empty() || memories.len() > MAX_RECEIPT_CANDIDATES {
        bail!("一次原生理解回执需要 1 至 {MAX_RECEIPT_CANDIDATES} 条候选");
    }
    let candidates = match session_id {
        Some(session_id) => record_candidates_attested(workspace, memories, producer, session_id)?,
        None => record_candidates(workspace, memories, producer)?,
    };
    let count = |action: &str| {
        candidates
            .iter()
            .filter(|candidate| candidate.ingest_action == action)
            .count()
    };
    let recorded_count = count("created") + count("updated") + count("replacement");
    let deduplicated_count = count("deduplicated") + count("shared_duplicate");
    let evidence_path_count = candidates
        .iter()
        .map(|candidate| candidate.memory.evidence.len())
        .sum::<usize>();
    let conflict_hint_count = candidates
        .iter()
        .map(|candidate| candidate.conflicts.len())
        .sum::<usize>();
    Ok(json!({
        "status": if recorded_count == 0 { "no_new_candidate" } else { "pending_review" },
        "recorded_count": recorded_count,
        "created_count": count("created"),
        "updated_count": count("updated"),
        "replacement_count": count("replacement"),
        "deduplicated_count": deduplicated_count,
        "shared_duplicate_count": count("shared_duplicate"),
        "candidates": candidates,
        "identity_binding": "server_current_file_sha256_plus_git_object_when_available",
        "storage": "external_project_document_index",
        "authority": "candidate_only",
        "lifecycle": "candidate -> reviewed suggestion -> applied Git memory -> drifted/replacement",
        "effect_receipt": {
            "submitted_candidate_count": candidates.len(),
            "evidence_path_count": evidence_path_count,
            "recorded_count": recorded_count,
            "deduplicated_count": deduplicated_count,
            "conflict_hint_count": conflict_hint_count,
            "source_bodies_stored": 0,
            "measurement_kind": "local_structural_count",
            "not_vendor_billing": true,
            "not_total_task_tokens": true
        },
        "repository_changed": false,
        "source_bodies_stored": 0,
        "next": "Review in the project document workspace, then use the existing suggestions/apply flow."
    }))
}

pub(crate) fn revise_candidate(
    workspace: &Path,
    candidate_id: &str,
    expected_updated_at_ms: u64,
    summary: String,
    topics: Vec<String>,
) -> Result<Value> {
    if expected_updated_at_ms == 0 {
        bail!("修订候选必须提供 expected_updated_at_ms");
    }
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_candidate_schema(&index)?;
    let (stored_status, encoded, stored_updated_at_ms) = index
        .conn
        .query_row(
            "SELECT status,candidate_json,updated_at_ms FROM native_context_candidates WHERE id=?1",
            params![candidate_id.trim()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("原生理解候选不存在：{}", candidate_id.trim()))?;
    let stored_updated_at_ms = stored_updated_at_ms.max(0) as u64;
    if stored_updated_at_ms != expected_updated_at_ms {
        bail!("原生理解候选已更新，请刷新后重试");
    }
    if !matches!(stored_status.as_str(), "pending" | "rejected") {
        bail!("只能修订 pending 或 rejected 候选");
    }

    let mut candidate: NativeContextCandidate = serde_json::from_str(&encoded)?;
    candidate.memory.summary = summary;
    candidate.memory.topics = topics;
    candidate.memory.reviewed_at.clear();
    candidate.memory = normalize_memories(vec![candidate.memory])?.remove(0);
    candidate.conflicts =
        crate::project_document_native_context_conflict::inspect(workspace, &candidate.memory);
    candidate.provenance.last_editor = "pc_document_review".to_string();
    candidate.provenance.last_edited_at_ms = now_millis();
    candidate.status = stored_status.clone();
    candidate.updated_at_ms = candidate
        .provenance
        .last_edited_at_ms
        .max(expected_updated_at_ms.saturating_add(1));
    candidate.evidence_current =
        validate_memories_current(workspace, std::slice::from_ref(&candidate.memory)).is_ok();
    let changed = index.conn.execute(
        "UPDATE native_context_candidates SET candidate_json=?1,updated_at_ms=?2
         WHERE id=?3 AND status=?4 AND updated_at_ms=?5",
        params![
            serde_json::to_string(&candidate)?,
            to_i64(candidate.updated_at_ms),
            candidate.memory.candidate_id,
            stored_status,
            to_i64(expected_updated_at_ms),
        ],
    )?;
    if changed != 1 {
        bail!("原生理解候选已更新，请刷新后重试");
    }
    Ok(json!({
        "status": "revised",
        "candidate": candidate,
        "authority": "candidate_only",
        "repository_changed": false,
        "evidence_changed": false,
    }))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}
