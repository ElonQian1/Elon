//! Candidate ingestion, deduplication, provenance, and lifecycle transitions.

use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    project_document_index::ProjectDocumentIndex,
    project_document_native_context::{
        bind_current_evidence_hashes, initialize_candidate_schema, normalize_memories,
        validate_memories_current, NativeContextCandidate, NativeContextProvenance,
        ProjectContextMemory,
    },
};

const MAX_CANDIDATES: usize = 200;

pub(crate) fn record_candidate(
    workspace: &Path,
    memory: ProjectContextMemory,
    producer: &str,
) -> Result<NativeContextCandidate> {
    Ok(record_candidates(workspace, vec![memory], producer)?.remove(0))
}

pub(crate) fn record_candidates(
    workspace: &Path,
    memories: Vec<ProjectContextMemory>,
    producer: &str,
) -> Result<Vec<NativeContextCandidate>> {
    record_candidates_internal(workspace, memories, producer, None)
}

pub(crate) fn record_candidates_attested(
    workspace: &Path,
    memories: Vec<ProjectContextMemory>,
    producer: &str,
    session_id: &str,
) -> Result<Vec<NativeContextCandidate>> {
    record_candidates_internal(workspace, memories, producer, Some(session_id))
}

fn record_candidates_internal(
    workspace: &Path,
    memories: Vec<ProjectContextMemory>,
    producer: &str,
    session_id: Option<&str>,
) -> Result<Vec<NativeContextCandidate>> {
    let memories = memories
        .into_iter()
        .map(|memory| bind_current_evidence_hashes(workspace, memory))
        .collect::<Result<Vec<_>>>()?;
    let memories = normalize_memories(memories)?;
    validate_memories_current(workspace, &memories)?;
    let producer = bounded_text(producer, 40);
    if producer.is_empty() {
        bail!("native context candidate producer 不能为空");
    }
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_candidate_schema(&index)?;
    let now = now_millis();
    let transaction = index.conn.unchecked_transaction()?;
    let mut candidates = Vec::with_capacity(memories.len());
    for memory in memories {
        let conflicts =
            crate::project_document_native_context_conflict::inspect(workspace, &memory);
        let provenance = provenance(&producer, session_id, memory.evidence.len(), now);
        if crate::project_document_native_context_conflict::is_shared_duplicate(&conflicts) {
            candidates.push(NativeContextCandidate {
                memory,
                status: "applied".to_string(),
                producer: producer.clone(),
                created_at_ms: now,
                updated_at_ms: now,
                evidence_current: true,
                ingest_action: "shared_duplicate".to_string(),
                provenance,
                conflicts,
            });
            continue;
        }
        let existing = transaction
            .query_row(
                "SELECT status,candidate_json,created_at_ms FROM native_context_candidates WHERE id=?1",
                params![memory.candidate_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?;
        if let Some((stored_status, encoded, _)) = existing.as_ref() {
            let mut stored: NativeContextCandidate = serde_json::from_str(encoded)?;
            let preserves_human_revision = !stored.provenance.last_editor.is_empty()
                && stored.memory.evidence == memory.evidence;
            if stored.memory == memory || preserves_human_revision {
                stored.status = stored_status.clone();
                stored.evidence_current = true;
                stored.ingest_action = "deduplicated".to_string();
                candidates.push(stored);
                continue;
            }
        }
        let created_at_ms = existing
            .as_ref()
            .map(|(_, _, value)| (*value).max(0) as u64)
            .unwrap_or(now);
        let ingest_action = existing
            .as_ref()
            .map(|(status, _, _)| {
                if matches!(status.as_str(), "reviewed" | "applied") {
                    "replacement"
                } else {
                    "updated"
                }
            })
            .unwrap_or("created");
        let candidate = NativeContextCandidate {
            memory,
            status: "pending".to_string(),
            producer: producer.clone(),
            created_at_ms,
            updated_at_ms: now,
            evidence_current: true,
            ingest_action: ingest_action.to_string(),
            provenance,
            conflicts,
        };
        transaction.execute(
            "INSERT INTO native_context_candidates(id,status,candidate_json,created_at_ms,updated_at_ms)
             VALUES(?1,'pending',?2,?3,?4)
             ON CONFLICT(id) DO UPDATE SET status='pending',candidate_json=excluded.candidate_json,
             updated_at_ms=excluded.updated_at_ms",
            params![
                candidate.memory.candidate_id,
                serde_json::to_string(&candidate)?,
                to_i64(created_at_ms),
                to_i64(now)
            ],
        )?;
        candidates.push(candidate);
    }
    transaction.execute(
        "DELETE FROM native_context_candidates WHERE id IN (
           SELECT id FROM native_context_candidates ORDER BY updated_at_ms DESC LIMIT -1 OFFSET ?1
         )",
        params![MAX_CANDIDATES as i64],
    )?;
    transaction.commit()?;
    Ok(candidates)
}

fn provenance(
    producer: &str,
    session_id: Option<&str>,
    evidence_path_count: usize,
    recorded_at_ms: u64,
) -> NativeContextProvenance {
    let session_fingerprint = session_id
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            format!("{:x}", Sha256::digest(value.as_bytes()))
                .chars()
                .take(24)
                .collect()
        })
        .unwrap_or_default();
    NativeContextProvenance {
        schema: "elon.native_context_provenance.v1".to_string(),
        source: if session_fingerprint.is_empty() {
            "governance_mcp"
        } else {
            "receipt_profile"
        }
        .to_string(),
        assurance: if session_fingerprint.is_empty() {
            "producer_asserted"
        } else {
            "local_mcp_session_attested"
        }
        .to_string(),
        session_fingerprint,
        evidence_path_count,
        recorded_at_ms,
        last_editor: String::new(),
        last_edited_at_ms: 0,
    }
}

fn bounded_text(value: &str, limit: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(limit)
        .collect()
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
