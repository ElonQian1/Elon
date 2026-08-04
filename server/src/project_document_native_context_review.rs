//! Review bridge between local native-agent context candidates and shared suggestions.

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

use crate::{
    project_document_authorization::DocumentAutomationMode,
    project_document_files::read_project_document_file,
    project_document_governance::{
        parse_suggestions, DocumentOrganizationSuggestions, OrganizationStatus,
        SUGGESTIONS_CONFIG_PATH,
    },
    project_document_governance_service::save_suggestions,
    project_document_index::ProjectDocumentIndex,
    project_document_native_context::{
        initialize_candidate_schema, merge, validate_memories_current, NativeContextCandidate,
        ProjectContextMemory,
    },
};

const MAX_PAGE_SIZE: usize = 20;
const MAX_REVIEW_CANDIDATES: usize = 50;

pub(crate) fn candidate_page(
    workspace: &Path,
    requested_status: &str,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    if offset > 10_000 {
        bail!("native context candidate offset 不能超过 10000");
    }
    let status = normalized_status(requested_status)?;
    let limit = if limit == 0 {
        10
    } else {
        limit.clamp(1, MAX_PAGE_SIZE)
    };
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_candidate_schema(&index)?;
    let total = index.conn.query_row(
        "SELECT COUNT(*) FROM native_context_candidates WHERE ?1='all' OR status=?1",
        params![status],
        |row| row.get::<_, i64>(0),
    )?;
    let mut statement = index.conn.prepare(
        "SELECT status,candidate_json FROM native_context_candidates
         WHERE ?1='all' OR status=?1
         ORDER BY updated_at_ms DESC,id ASC LIMIT ?2 OFFSET ?3",
    )?;
    let rows = statement
        .query_map(params![status, limit as i64, offset as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let candidates = rows
        .into_iter()
        .map(|(stored_status, encoded)| {
            let mut candidate: NativeContextCandidate = serde_json::from_str(&encoded)?;
            candidate.status = stored_status;
            candidate.evidence_current =
                validate_memories_current(workspace, std::slice::from_ref(&candidate.memory))
                    .is_ok();
            Ok(candidate)
        })
        .collect::<Result<Vec<_>>>()?;
    let counts = status_counts(&index)?;
    let returned = candidates.len();
    let total = total.max(0) as usize;
    Ok(json!({
        "status": status,
        "counts": counts,
        "pagination": {
            "offset": offset,
            "limit": limit,
            "returned": returned,
            "total": total,
            "next_offset": (offset + returned < total).then_some(offset + returned),
        },
        "candidates": candidates,
        "authority": "candidate_only",
        "storage": "external_project_document_index",
        "repository_changed": false,
        "source_bodies_returned": 0,
    }))
}

pub(crate) fn review_candidates(
    workspace: &Path,
    candidate_ids: Vec<String>,
    action: &str,
    authorization_mode: DocumentAutomationMode,
    expected_catalog_revision: Option<&str>,
    expected_suggestions_revision: Option<&str>,
) -> Result<Value> {
    let ids = normalize_candidate_ids(candidate_ids)?;
    let index = ProjectDocumentIndex::open(workspace)?;
    initialize_candidate_schema(&index)?;
    let candidates = load_candidates(&index, &ids)?;
    match action.trim() {
        "reject" => change_local_status(index, candidates, "rejected"),
        "restore" => change_local_status(index, candidates, "pending"),
        "accept" => accept_into_suggestions(
            workspace,
            index,
            candidates,
            authorization_mode,
            expected_catalog_revision.unwrap_or_default(),
            expected_suggestions_revision,
        ),
        _ => bail!("native context review action 仅支持 accept、reject 或 restore"),
    }
}

fn accept_into_suggestions(
    workspace: &Path,
    index: ProjectDocumentIndex,
    candidates: Vec<NativeContextCandidate>,
    authorization_mode: DocumentAutomationMode,
    expected_catalog_revision: &str,
    expected_suggestions_revision: Option<&str>,
) -> Result<Value> {
    if expected_catalog_revision.trim().is_empty() {
        bail!("接受候选前必须提供当前 catalog revision");
    }
    for candidate in &candidates {
        if !matches!(candidate.status.as_str(), "pending" | "reviewed") {
            bail!(
                "候选 {} 当前状态为 {}，必须先恢复为 pending",
                candidate.memory.candidate_id,
                candidate.status
            );
        }
    }
    let mut memories = candidates
        .iter()
        .map(|candidate| candidate.memory.clone())
        .collect::<Vec<_>>();
    validate_memories_current(workspace, &memories)?;
    let review_revision = format!(
        "catalog:{}",
        expected_catalog_revision
            .chars()
            .take(32)
            .collect::<String>()
    );
    for memory in &mut memories {
        memory.reviewed_at = review_revision.clone();
    }

    let (mut suggestions, current_revision) = load_or_create_suggestions(workspace)?;
    if current_revision.is_some() && suggestions.status == OrganizationStatus::Requested {
        bail!("AI 文档整理任务仍在生成建议，请等待任务完成后再并入候选");
    }
    merge(&mut suggestions.proposed_context_memories, &memories)?;
    suggestions.status = OrganizationStatus::Ready;
    if suggestions.summary.trim().is_empty() {
        suggestions.summary = "已审核原生工具项目理解候选，等待应用到共享项目记忆。".into();
    } else if !suggestions.summary.contains("原生工具项目理解候选") {
        suggestions
            .summary
            .push_str(" 已审核原生工具项目理解候选，等待应用到共享项目记忆。");
    }
    let saved = save_suggestions(
        workspace,
        suggestions,
        authorization_mode,
        expected_catalog_revision,
        expected_suggestions_revision,
    )?;
    mark_reviewed(&index, &memories)?;
    Ok(json!({
        "action": "accept",
        "accepted": memories.len(),
        "candidate_ids": memories.iter().map(|memory| memory.candidate_id.as_str()).collect::<Vec<_>>(),
        "candidate_status": "reviewed",
        "catalog_revision": saved.get("catalog_revision"),
        "suggestions_revision": saved.get("suggestions_revision"),
        "suggestions": saved.get("suggestions"),
        "already_saved": saved.get("already_saved"),
        "previous_suggestions_revision": current_revision,
        "repository_changed": saved.get("already_saved").and_then(Value::as_bool) != Some(true),
        "source_bodies_stored": 0,
        "next": "Use the existing apply-suggestions flow to promote reviewed memories into the Git-backed manifest.",
    }))
}

fn change_local_status(
    mut index: ProjectDocumentIndex,
    candidates: Vec<NativeContextCandidate>,
    target: &str,
) -> Result<Value> {
    let action = if target == "rejected" {
        "reject"
    } else {
        "restore"
    };
    for candidate in &candidates {
        let valid = match target {
            "rejected" => matches!(candidate.status.as_str(), "pending" | "rejected"),
            "pending" => matches!(candidate.status.as_str(), "rejected" | "pending"),
            _ => false,
        };
        if !valid {
            bail!(
                "候选 {} 当前状态为 {}，不能执行 {}",
                candidate.memory.candidate_id,
                candidate.status,
                action
            );
        }
    }
    let transaction = index.conn.transaction()?;
    for candidate in &candidates {
        let changed = transaction.execute(
            "UPDATE native_context_candidates SET status=?1,updated_at_ms=updated_at_ms+1
             WHERE id=?2 AND status=?3",
            params![target, candidate.memory.candidate_id, candidate.status],
        )?;
        if changed != 1 {
            bail!(
                "候选 {} 已被其他会话修改，请刷新后重试",
                candidate.memory.candidate_id
            );
        }
    }
    transaction.commit()?;
    Ok(json!({
        "action": action,
        "changed": candidates.len(),
        "candidate_ids": candidates.iter().map(|candidate| candidate.memory.candidate_id.as_str()).collect::<Vec<_>>(),
        "candidate_status": target,
        "repository_changed": false,
        "source_bodies_stored": 0,
    }))
}

fn load_candidates(
    index: &ProjectDocumentIndex,
    ids: &[String],
) -> Result<Vec<NativeContextCandidate>> {
    ids.iter()
        .map(|id| {
            let stored = index
                .conn
                .query_row(
                    "SELECT status,candidate_json FROM native_context_candidates WHERE id=?1",
                    params![id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
                .ok_or_else(|| anyhow!("native context candidate 不存在：{id}"))?;
            let mut candidate: NativeContextCandidate = serde_json::from_str(&stored.1)?;
            candidate.status = stored.0;
            Ok(candidate)
        })
        .collect()
}

fn load_or_create_suggestions(
    workspace: &Path,
) -> Result<(DocumentOrganizationSuggestions, Option<String>)> {
    if workspace.join(SUGGESTIONS_CONFIG_PATH).is_file() {
        let file = read_project_document_file(workspace, SUGGESTIONS_CONFIG_PATH)
            .map_err(|error| anyhow!(error.message))?;
        let suggestions = parse_suggestions(Some(&file.content))?
            .ok_or_else(|| anyhow!("AI 整理建议文件为空"))?;
        Ok((suggestions, Some(file.revision)))
    } else {
        let suggestions =
            parse_suggestions(Some("{}"))?.ok_or_else(|| anyhow!("无法初始化 AI 整理建议"))?;
        Ok((suggestions, None))
    }
}

fn mark_reviewed(index: &ProjectDocumentIndex, memories: &[ProjectContextMemory]) -> Result<()> {
    for memory in memories {
        index.conn.execute(
            "UPDATE native_context_candidates SET status='reviewed',updated_at_ms=updated_at_ms+1
             WHERE id=?1 AND status IN ('pending','reviewed')",
            params![memory.candidate_id],
        )?;
    }
    Ok(())
}

fn status_counts(index: &ProjectDocumentIndex) -> Result<BTreeMap<String, usize>> {
    let mut counts = ["pending", "reviewed", "rejected", "applied"]
        .into_iter()
        .map(|status| (status.to_string(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut statement = index
        .conn
        .prepare("SELECT status,COUNT(*) FROM native_context_candidates GROUP BY status")?;
    for row in statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })? {
        let (status, count) = row?;
        counts.insert(status, count.max(0) as usize);
    }
    Ok(counts)
}

fn normalized_status(value: &str) -> Result<&str> {
    match value.trim() {
        "" | "pending" => Ok("pending"),
        status @ ("reviewed" | "rejected" | "applied" | "all") => Ok(status),
        _ => bail!(
            "native context candidate status 仅支持 pending、reviewed、rejected、applied 或 all"
        ),
    }
}

fn normalize_candidate_ids(ids: Vec<String>) -> Result<Vec<String>> {
    let mut normalized = ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if normalized.is_empty() || normalized.len() > MAX_REVIEW_CANDIDATES {
        bail!("一次必须审核 1 至 {MAX_REVIEW_CANDIDATES} 条候选");
    }
    if normalized.iter().any(|id| {
        id.len() > 80
            || !id
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.'))
    }) {
        bail!("candidate_id 只能包含字母、数字、点、下划线和连字符");
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_filters_accept_only_bounded_known_values() {
        assert_eq!(normalized_status("").unwrap(), "pending");
        assert_eq!(normalized_status("reviewed").unwrap(), "reviewed");
        assert!(normalized_status("private_memory").is_err());
        assert_eq!(
            normalize_candidate_ids(vec![
                "native-b".into(),
                "native-a".into(),
                "native-a".into()
            ])
            .unwrap(),
            vec!["native-a", "native-b"]
        );
        assert!(normalize_candidate_ids(vec!["../source".into()]).is_err());
    }
}
