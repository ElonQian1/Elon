//! Persistent ownership and disposition workflow for deterministic document issues.

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    project_document_federation::{health_node_matches_path, KnowledgeFederationHealth},
    project_document_governance::DocumentSectionManifest,
    project_document_index::ProjectDocumentIndex,
};

const ISSUE_STATUSES: &[&str] = &["open", "assigned", "snoozed", "ignored", "resolved"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IssueWorkflowUpdate {
    pub fingerprint: String,
    pub status: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub due_at: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub snoozed_until: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IssueWorkflowState {
    pub status: String,
    pub owner: String,
    pub due_at: String,
    pub reason: String,
    pub snoozed_until: String,
    pub updated_at_ms: u64,
}

pub(crate) fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS issue_actions(
           fingerprint TEXT PRIMARY KEY,status TEXT NOT NULL,owner TEXT NOT NULL,due_at TEXT NOT NULL,
           reason TEXT NOT NULL,snoozed_until TEXT NOT NULL,updated_at_ms INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS health_snapshots(
           id INTEGER PRIMARY KEY AUTOINCREMENT,signature TEXT NOT NULL UNIQUE,created_at_ms INTEGER NOT NULL,
           overall_score INTEGER NOT NULL,architecture_score INTEGER NOT NULL,quality_score INTEGER NOT NULL,
           federation_score INTEGER NOT NULL,issue_count INTEGER NOT NULL,actionable_count INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS health_snapshots_recent ON health_snapshots(created_at_ms DESC);",
    )?;
    Ok(())
}

pub(crate) fn synchronize(
    index: &ProjectDocumentIndex,
    raw_issues: Vec<Value>,
    manifest: &DocumentSectionManifest,
    federation: &KnowledgeFederationHealth,
    scores: (u8, u8, u8, u8),
) -> Result<Value> {
    let mut issues = Vec::with_capacity(raw_issues.len());
    for mut issue in raw_issues {
        let fingerprint = string(&issue, "fingerprint");
        let path = string(&issue, "path").replace('\\', "/");
        let state = effective_state(index, &fingerprint)?;
        let metadata_owner = manifest
            .document_metadata
            .get(&path)
            .map(|metadata| metadata.owner.trim())
            .filter(|owner| !owner.is_empty())
            .unwrap_or_default();
        let owner = if state.owner.is_empty() {
            metadata_owner
        } else {
            &state.owner
        };
        let primary_topic = manifest.assignments.get(&path).cloned().unwrap_or_default();
        let secondary_topics = manifest
            .secondary_assignments
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let scope_id = federation
            .nodes
            .iter()
            .filter(|node| health_node_matches_path(node, &path))
            .max_by_key(|node| {
                (
                    node.scope_path.len(),
                    node.include_globs
                        .iter()
                        .map(String::len)
                        .max()
                        .unwrap_or_default(),
                )
            })
            .map(|node| node.id.clone())
            .unwrap_or_else(|| federation.root_id.clone());
        issue["workflow"] = serde_json::to_value(IssueWorkflowState {
            owner: owner.to_string(),
            ..state
        })?;
        issue["context"] = json!({
            "primary_topic": primary_topic,
            "secondary_topics": secondary_topics,
            "scope_id": scope_id,
        });
        issues.push(issue);
    }
    index.replace_issues(&issues)?;
    let summary = workflow_summary(&issues);
    record_snapshot(
        index,
        &issues,
        scores,
        summary["actionable"].as_u64().unwrap_or_default(),
    )?;
    Ok(json!({
        "version": 1,
        "summary": summary,
        "issues": issues.iter().take(100).collect::<Vec<_>>(),
        "returned_issues": issues.len().min(100),
        "total_issues": issues.len(),
        "filters": filter_options(&issues),
        "trend": health_trend(index, 30)?,
        "score_explanation": score_explanation(scores),
    }))
}

pub(crate) fn update_issue(
    index: &ProjectDocumentIndex,
    mut update: IssueWorkflowUpdate,
) -> Result<IssueWorkflowState> {
    update.fingerprint = clean(&update.fingerprint, 128);
    update.status = clean_id(&update.status, 24);
    update.owner = clean(&update.owner, 80);
    update.due_at = clean_date(&update.due_at)?;
    update.reason = clean(&update.reason, 500);
    update.snoozed_until = clean_date(&update.snoozed_until)?;
    if update.fingerprint.is_empty() || !ISSUE_STATUSES.contains(&update.status.as_str()) {
        bail!("文档问题 fingerprint 或状态无效");
    }
    if update.status == "assigned" && update.owner.is_empty() {
        bail!("分派问题时必须指定负责人");
    }
    if matches!(update.status.as_str(), "ignored" | "snoozed") && update.reason.is_empty() {
        bail!("忽略或延期问题必须填写原因");
    }
    if update.status == "snoozed" && update.snoozed_until.is_empty() {
        bail!("延期问题必须填写恢复日期");
    }
    let exists = index.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE fingerprint=?1)",
        params![update.fingerprint],
        |row| row.get::<_, bool>(0),
    )?;
    if !exists {
        bail!("文档问题已不存在或尚未完成分析，请刷新后重试");
    }
    let now = now_millis();
    index.conn.execute(
        "INSERT INTO issue_actions(fingerprint,status,owner,due_at,reason,snoozed_until,updated_at_ms)
         VALUES(?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(fingerprint) DO UPDATE SET status=excluded.status,owner=excluded.owner,
         due_at=excluded.due_at,reason=excluded.reason,snoozed_until=excluded.snoozed_until,
         updated_at_ms=excluded.updated_at_ms",
        params![update.fingerprint, update.status, update.owner, update.due_at, update.reason, update.snoozed_until, to_i64(now)],
    )?;
    let state = IssueWorkflowState {
        status: update.status,
        owner: update.owner,
        due_at: update.due_at,
        reason: update.reason,
        snoozed_until: update.snoozed_until,
        updated_at_ms: now,
    };
    if let Some(raw) = index
        .conn
        .query_row(
            "SELECT issue_json FROM issues WHERE fingerprint=?1",
            params![update.fingerprint],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        if let Ok(mut issue) = serde_json::from_str::<Value>(&raw) {
            issue["workflow"] = serde_json::to_value(&state)?;
            index.conn.execute(
                "UPDATE issues SET issue_json=?1,updated_at_ms=?2 WHERE fingerprint=?3",
                params![
                    serde_json::to_string(&issue)?,
                    to_i64(now),
                    update.fingerprint
                ],
            )?;
        }
    }
    Ok(state)
}

pub(crate) fn list_filtered(
    index: &ProjectDocumentIndex,
    issue_types: &[String],
    statuses: &[String],
    severities: &[String],
    owner: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<Value>> {
    Ok(index
        .list_issues(issue_types, 0, 100_000)?
        .into_iter()
        .filter(|issue| {
            statuses.is_empty()
                || statuses.iter().any(|status| {
                    status
                        == issue
                            .pointer("/workflow/status")
                            .and_then(Value::as_str)
                            .unwrap_or("open")
                })
        })
        .filter(|issue| {
            severities.is_empty()
                || severities.iter().any(|severity| {
                    severity
                        == issue
                            .get("severity")
                            .and_then(Value::as_str)
                            .unwrap_or("info")
                })
        })
        .filter(|issue| {
            owner.trim().is_empty()
                || issue
                    .pointer("/workflow/owner")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(owner.trim()))
        })
        .skip(offset.min(100_000))
        .take(limit.clamp(1, 200))
        .collect())
}

pub(crate) fn health_trend(index: &ProjectDocumentIndex, limit: usize) -> Result<Vec<Value>> {
    let mut statement = index.conn.prepare(
        "SELECT created_at_ms,overall_score,architecture_score,quality_score,federation_score,
         issue_count,actionable_count FROM health_snapshots ORDER BY created_at_ms DESC LIMIT ?1",
    )?;
    let mut values = statement
        .query_map(params![limit.clamp(1, 365) as i64], |row| {
            Ok(json!({
                "created_at_ms": row.get::<_, i64>(0)?.max(0) as u64,
                "overall_score": row.get::<_, i64>(1)?,
                "architecture_score": row.get::<_, i64>(2)?,
                "quality_score": row.get::<_, i64>(3)?,
                "federation_score": row.get::<_, i64>(4)?,
                "issue_count": row.get::<_, i64>(5)?,
                "actionable_count": row.get::<_, i64>(6)?,
            }))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    values.reverse();
    Ok(values)
}

fn effective_state(index: &ProjectDocumentIndex, fingerprint: &str) -> Result<IssueWorkflowState> {
    let state = index.conn.query_row(
        "SELECT status,owner,due_at,reason,snoozed_until,updated_at_ms FROM issue_actions WHERE fingerprint=?1",
        params![fingerprint],
        |row| Ok(IssueWorkflowState {
            status: row.get(0)?, owner: row.get(1)?, due_at: row.get(2)?, reason: row.get(3)?,
            snoozed_until: row.get(4)?, updated_at_ms: row.get::<_, i64>(5)?.max(0) as u64,
        }),
    ).optional()?.unwrap_or_else(|| IssueWorkflowState { status: "open".to_string(), ..Default::default() });
    if state.status == "snoozed" && !state.snoozed_until.is_empty() && state.snoozed_until < today()
    {
        return Ok(IssueWorkflowState {
            status: "open".to_string(),
            ..state
        });
    }
    Ok(state)
}

fn workflow_summary(issues: &[Value]) -> Value {
    let count = |status: &str| {
        issues
            .iter()
            .filter(|issue| {
                issue.pointer("/workflow/status").and_then(Value::as_str) == Some(status)
            })
            .count()
    };
    let ignored = count("ignored");
    let snoozed = count("snoozed");
    let resolved = count("resolved");
    let today = today();
    json!({
        "open": count("open"), "assigned": count("assigned"), "snoozed": snoozed,
        "ignored": ignored, "resolved": resolved,
        "actionable": issues.len().saturating_sub(ignored + snoozed + resolved),
        "overdue": issues.iter().filter(|issue| issue.pointer("/workflow/due_at").and_then(Value::as_str).is_some_and(|date| !date.is_empty() && date < today.as_str())).count(),
    })
}

fn filter_options(issues: &[Value]) -> Value {
    fn unique(issues: &[Value], pointer: &str) -> Vec<String> {
        let mut values = issues
            .iter()
            .filter_map(|issue| issue.pointer(pointer).and_then(Value::as_str))
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        values
    }
    json!({
        "types": unique(issues, "/type"), "severities": unique(issues, "/severity"),
        "owners": unique(issues, "/workflow/owner"), "topics": unique(issues, "/context/primary_topic"),
        "scopes": unique(issues, "/context/scope_id"), "statuses": ISSUE_STATUSES,
    })
}

fn record_snapshot(
    index: &ProjectDocumentIndex,
    issues: &[Value],
    scores: (u8, u8, u8, u8),
    actionable: u64,
) -> Result<()> {
    let mut hasher = Sha256::new();
    for issue in issues {
        hasher.update(string(issue, "fingerprint"));
    }
    hasher.update([scores.0, scores.1, scores.2, scores.3]);
    hasher.update(actionable.to_le_bytes());
    let signature = format!("{:x}", hasher.finalize());
    index.conn.execute(
        "INSERT OR IGNORE INTO health_snapshots(signature,created_at_ms,overall_score,architecture_score,
         quality_score,federation_score,issue_count,actionable_count) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![signature, to_i64(now_millis()), scores.0, scores.1, scores.2, scores.3, issues.len() as i64, actionable as i64],
    )?;
    let cutoff = now_millis().saturating_sub(365 * 24 * 60 * 60 * 1_000);
    index.conn.execute(
        "DELETE FROM health_snapshots WHERE created_at_ms<?1",
        params![to_i64(cutoff)],
    )?;
    Ok(())
}

fn score_explanation(scores: (u8, u8, u8, u8)) -> Value {
    json!({
        "formula": "architecture × 35% + quality × 50% + federation × 15%",
        "overall": scores.0,
        "components": [
            {"key":"architecture","label":"知识结构","score":scores.1,"weight":35,"contribution":u16::from(scores.1)*35/100},
            {"key":"quality","label":"文档质量","score":scores.2,"weight":50,"contribution":u16::from(scores.2)*50/100},
            {"key":"federation","label":"联邦架构","score":scores.3,"weight":15,"contribution":u16::from(scores.3)*15/100}
        ]
    })
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
fn clean(value: &str, limit: usize) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_control())
        .take(limit)
        .collect()
}
fn clean_id(value: &str, limit: usize) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .take(limit)
        .collect()
}
fn clean_date(value: &str) -> Result<String> {
    let value = clean(value, 10);
    if value.is_empty() || chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_ok() {
        Ok(value)
    } else {
        bail!("日期必须使用 YYYY-MM-DD")
    }
}
fn today() -> String {
    chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
fn to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

#[cfg(test)]
#[path = "project_document_issue_workflow_tests.rs"]
mod tests;
