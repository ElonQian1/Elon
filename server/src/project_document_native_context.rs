//! Evidence-bound project navigation memory derived from native agent reads.
//!
//! Pending candidates live in the external project-document SQLite index. Only
//! reviewed memories copied through the existing suggestion/apply flow enter
//! the Git-backed manifest. Neither layer stores prompts, chats, tool output, or
//! source bodies.

use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    project_document_file_operation_model::normalize_document_path,
    project_document_index::ProjectDocumentIndex,
};

const MAX_MEMORIES: usize = 64;
const MAX_EVIDENCE_PER_MEMORY: usize = 8;
const MAX_CANDIDATES: usize = 200;
const MAX_EVIDENCE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectContextEvidence {
    pub path: String,
    pub content_hash: String,
    #[serde(default)]
    pub locator: String,
    #[serde(default)]
    pub evidence_kind: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProjectContextMemory {
    #[serde(default)]
    pub candidate_id: String,
    pub summary: String,
    #[serde(default)]
    pub topics: Vec<String>,
    pub evidence: Vec<ProjectContextEvidence>,
    #[serde(default)]
    pub reviewed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NativeContextCandidate {
    #[serde(flatten)]
    pub memory: ProjectContextMemory,
    pub status: String,
    pub producer: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(default)]
    pub evidence_current: bool,
}

pub(crate) fn normalize_memories(
    memories: Vec<ProjectContextMemory>,
) -> Result<Vec<ProjectContextMemory>> {
    if memories.len() > MAX_MEMORIES {
        bail!("项目导航记忆最多 {MAX_MEMORIES} 条");
    }
    let mut ids = HashSet::new();
    memories
        .into_iter()
        .map(normalize_memory)
        .map(|memory| {
            let memory = memory?;
            if !ids.insert(memory.candidate_id.clone()) {
                bail!("项目导航记忆包含重复 candidate_id：{}", memory.candidate_id);
            }
            Ok(memory)
        })
        .collect()
}

pub(crate) fn merge(
    current: &mut Vec<ProjectContextMemory>,
    proposed: &[ProjectContextMemory],
) -> Result<()> {
    let replacement_ids = proposed
        .iter()
        .map(|memory| memory.candidate_id.as_str())
        .collect::<HashSet<_>>();
    current.retain(|memory| !replacement_ids.contains(memory.candidate_id.as_str()));
    current.extend(proposed.iter().cloned());
    *current = normalize_memories(std::mem::take(current))?;
    Ok(())
}

pub(crate) fn validate_memories_current(
    workspace: &Path,
    memories: &[ProjectContextMemory],
) -> Result<()> {
    for memory in memories {
        for evidence in &memory.evidence {
            validate_evidence_current(workspace, evidence)
                .with_context(|| format!("项目导航记忆 {} 的证据已漂移", memory.candidate_id))?;
        }
    }
    Ok(())
}

pub(crate) fn validate_reviewed_memories_current(
    workspace: &Path,
    memories: &[ProjectContextMemory],
) -> Result<()> {
    for memory in memories {
        if memory.reviewed_at.trim().is_empty() {
            bail!(
                "项目导航记忆 {} 未提供 reviewed_at，不能进入共享 manifest",
                memory.candidate_id
            );
        }
    }
    validate_memories_current(workspace, memories)
}

pub(crate) fn record_candidate(
    workspace: &Path,
    memory: ProjectContextMemory,
    producer: &str,
) -> Result<NativeContextCandidate> {
    let memory = normalize_memory(memory)?;
    validate_memories_current(workspace, std::slice::from_ref(&memory))?;
    let producer = bounded_text(producer, 40);
    if producer.is_empty() {
        bail!("native context candidate producer 不能为空");
    }
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_candidate_schema(&index)?;
    let now = now_millis();
    let created_at_ms = index
        .conn
        .query_row(
            "SELECT created_at_ms FROM native_context_candidates WHERE id=?1",
            params![memory.candidate_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .map(|value| value.max(0) as u64)
        .unwrap_or(now);
    let candidate = NativeContextCandidate {
        memory,
        status: "pending".to_string(),
        producer,
        created_at_ms,
        updated_at_ms: now,
        evidence_current: true,
    };
    index.conn.execute(
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
    prune_candidates(&index)?;
    Ok(candidate)
}

pub(crate) fn list_candidates(
    workspace: &Path,
    requested_status: &str,
    limit: usize,
) -> Result<Vec<NativeContextCandidate>> {
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_candidate_schema(&index)?;
    let status = match requested_status.trim() {
        "" | "pending" => Some("pending"),
        "applied" => Some("applied"),
        "all" => None,
        _ => bail!("native context candidate status 仅支持 pending、applied 或 all"),
    };
    let mut statement = index.conn.prepare(
        "SELECT status,candidate_json FROM native_context_candidates
         ORDER BY updated_at_ms DESC LIMIT 200",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .filter(|(stored_status, _)| status.map_or(true, |value| value == stored_status))
        .take(limit.clamp(1, 100))
        .map(|(stored_status, encoded)| {
            let mut candidate: NativeContextCandidate = serde_json::from_str(&encoded)?;
            candidate.status = stored_status;
            candidate.evidence_current =
                validate_memories_current(workspace, std::slice::from_ref(&candidate.memory))
                    .is_ok();
            Ok(candidate)
        })
        .collect()
}

pub(crate) fn mark_candidates_applied(
    workspace: &Path,
    memories: &[ProjectContextMemory],
) -> Result<usize> {
    if memories.is_empty() {
        return Ok(0);
    }
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_candidate_schema(&index)?;
    let mut changed = 0usize;
    for memory in memories {
        changed += index.conn.execute(
            "UPDATE native_context_candidates SET status='applied',updated_at_ms=?1 WHERE id=?2",
            params![to_i64(now_millis()), memory.candidate_id],
        )?;
    }
    Ok(changed)
}

fn normalize_memory(mut memory: ProjectContextMemory) -> Result<ProjectContextMemory> {
    memory.summary = bounded_text(&memory.summary, 800);
    if memory.summary.chars().count() < 12 {
        bail!("项目导航记忆 summary 至少 12 个字符");
    }
    memory.topics = unique_bounded_strings(memory.topics, 8, 48);
    if memory.topics.is_empty() {
        bail!("项目导航记忆至少需要一个 topic");
    }
    if memory.evidence.is_empty() || memory.evidence.len() > MAX_EVIDENCE_PER_MEMORY {
        bail!("项目导航记忆 evidence 需要 1 至 {MAX_EVIDENCE_PER_MEMORY} 条");
    }
    memory.evidence = memory
        .evidence
        .into_iter()
        .map(normalize_evidence)
        .collect::<Result<Vec<_>>>()?;
    memory
        .evidence
        .sort_by(|left, right| left.path.cmp(&right.path));
    memory
        .evidence
        .dedup_by(|left, right| left.path == right.path && left.locator == right.locator);
    memory.reviewed_at = bounded_text(&memory.reviewed_at, 40);
    memory.candidate_id = normalize_or_derive_id(&memory)?;
    Ok(memory)
}

fn normalize_evidence(mut evidence: ProjectContextEvidence) -> Result<ProjectContextEvidence> {
    evidence.path = normalize_document_path(&evidence.path)?;
    evidence.content_hash = evidence.content_hash.trim().to_ascii_lowercase();
    if evidence.content_hash.len() != 64
        || !evidence
            .content_hash
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        bail!("项目导航记忆 evidence.content_hash 必须是 SHA-256 hex");
    }
    evidence.locator = bounded_text(&evidence.locator, 120);
    evidence.evidence_kind = match evidence.evidence_kind.trim() {
        "" | "source" => "source".to_string(),
        "test" | "document" | "configuration" => evidence.evidence_kind.trim().to_string(),
        _ => bail!("evidence_kind 仅支持 source、test、document 或 configuration"),
    };
    Ok(evidence)
}

fn normalize_or_derive_id(memory: &ProjectContextMemory) -> Result<String> {
    let supplied = memory.candidate_id.trim();
    if !supplied.is_empty() {
        if supplied.len() > 80
            || !supplied
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.'))
        {
            bail!("candidate_id 只能包含字母、数字、点、下划线和连字符");
        }
        return Ok(supplied.to_string());
    }
    let material =
        serde_json::to_vec(&(memory.summary.as_str(), &memory.topics, &memory.evidence))?;
    Ok(format!("native-{:x}", Sha256::digest(material))
        .chars()
        .take(31)
        .collect())
}

pub(crate) fn validate_evidence_current(
    workspace: &Path,
    evidence: &ProjectContextEvidence,
) -> Result<()> {
    let canonical_workspace = workspace
        .canonicalize()
        .with_context(|| format!("无法解析项目工作区：{}", workspace.display()))?;
    let path = workspace.join(&evidence.path);
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("证据文件不存在：{}", evidence.path))?;
    if !canonical_path.starts_with(&canonical_workspace) {
        bail!("证据路径越过项目工作区：{}", evidence.path);
    }
    let metadata = fs::metadata(&canonical_path)
        .with_context(|| format!("证据文件不存在：{}", evidence.path))?;
    if !metadata.is_file() || metadata.len() > MAX_EVIDENCE_BYTES {
        bail!("证据文件不是普通文件或超过 8 MiB：{}", evidence.path);
    }
    let actual = sha256_file(&canonical_path)?;
    if actual != evidence.content_hash {
        bail!("证据 hash 与当前文件不一致：{}", evidence.path);
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn initialize_candidate_schema(index: &ProjectDocumentIndex) -> Result<()> {
    index.conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS native_context_candidates(
           id TEXT PRIMARY KEY,status TEXT NOT NULL,candidate_json TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,updated_at_ms INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS native_context_candidates_status
           ON native_context_candidates(status,updated_at_ms);",
    )?;
    Ok(())
}

fn prune_candidates(index: &ProjectDocumentIndex) -> Result<()> {
    index.conn.execute(
        "DELETE FROM native_context_candidates WHERE id IN (
           SELECT id FROM native_context_candidates ORDER BY updated_at_ms DESC LIMIT -1 OFFSET ?1
         )",
        params![MAX_CANDIDATES as i64],
    )?;
    Ok(())
}

fn unique_bounded_strings(values: Vec<String>, limit: usize, char_limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| bounded_text(&value, char_limit))
        .filter(|value| !value.is_empty() && seen.insert(value.to_ascii_lowercase()))
        .take(limit)
        .collect()
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
